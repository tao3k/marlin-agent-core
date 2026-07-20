#![cfg(feature = "turso")]

use marlin_agent_storage::{
    AgentId, AgentStorage, ArtifactHash, ArtifactRecord, EventId, ProjectId, SessionId,
    StorageResult, TopologyEdgeId, TopologyEdgeRecord, TopologyNodeId, TursoAgentStorage,
    TursoAgentStorageConfig, VisibilityReceipt,
};
use marlin_rust_project_harness_policy::{
    RustProjectHarnessPackageEvidenceGraphRequest, RustProjectHarnessQualityFindingEvidencePaths,
    build_package_evidence_graph_receipt, run_marlin_rust_project_harness_for_package,
    rust_project_harness_config_for_project,
};
use tempfile::tempdir;

#[derive(Clone, Debug, PartialEq, Eq)]
struct RfcdbPackageEvidenceGraphProjectionReceipt {
    package_name: String,
    node_count: usize,
    edge_count: usize,
    gate_success: bool,
}

impl RfcdbPackageEvidenceGraphProjectionReceipt {
    fn visibility_body(&self) -> Vec<u8> {
        format!(
            "package_name={}\nnode_count={}\nedge_count={}\ngate_success={}\n",
            self.package_name, self.node_count, self.edge_count, self.gate_success
        )
        .into_bytes()
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn rfcdb_turso_package_evidence_graph_survives_reopen() -> StorageResult<()> {
    let crate_root = crate::paths::crate_root();
    let package_name = crate::paths::STORAGE_PACKAGE_NAME.to_owned();
    let config = rust_project_harness_config_for_project(&crate_root);
    let harness_report = run_marlin_rust_project_harness_for_package(&crate_root, &config)
        .expect("storage crate should produce a Rust harness report");
    let graph_receipt =
        build_package_evidence_graph_receipt(RustProjectHarnessPackageEvidenceGraphRequest {
            config: &config,
            harness_report,
            project_root: crate_root.to_path_buf(),
            package_name: package_name.clone(),
            evidence_paths: RustProjectHarnessQualityFindingEvidencePaths::new(
                "evidence-graph.json",
                "verification-plan.json",
                "task-index.json",
                "verification-policy.json",
            ),
        });
    assert!(
        graph_receipt.is_success(),
        "package evidence graph failed for {package_name}: {graph_receipt:#?}"
    );

    let projection = RfcdbPackageEvidenceGraphProjectionReceipt {
        package_name,
        node_count: graph_receipt.evidence_graph_summary.nodes,
        edge_count: graph_receipt.evidence_graph_summary.edges,
        gate_success: graph_receipt.gate_receipt.is_success(),
    };
    let projection_body = projection.visibility_body();
    let project_id = ProjectId::new("project:rfcdb-turso-evidence-graph")?;
    let artifact_hash = ArtifactHash::new("sha256:rfcdb-turso-package-evidence-graph")?;
    let session_id = SessionId::new("session:rfcdb-evidence-graph")?;
    let agent_id = AgentId::new("agent:rfcdb-evidence-graph")?;
    let event_id = EventId::new("event:rfcdb-evidence-graph")?;
    let tempdir = tempdir().expect("tempdir should be available");
    let database_path = tempdir.path().join("rfcdb-evidence-graph.turso");
    let storage = TursoAgentStorage::open_local(TursoAgentStorageConfig {
        path: database_path.clone(),
        optimization_profile: marlin_agent_storage::TursoOptimizationProfile::AsyncIoWithMvccAndPassiveCheckpointExperimental,
        batch_transaction_mode: marlin_agent_storage::TursoBatchTransactionMode::Concurrent,
    })
    .await?;

    storage
        .put_artifact(ArtifactRecord {
            project_id: project_id.clone(),
            artifact_hash: artifact_hash.clone(),
            artifact_kind: "rfcdb.package-evidence-graph".to_owned(),
            producer_session_id: session_id.clone(),
            producer_agent_id: agent_id.clone(),
            producer_event_id: event_id.clone(),
            media_type: "application/vnd.marlin.rfcdb-evidence-graph-receipt".to_owned(),
            size_bytes: projection_body.len() as u64,
            body: projection_body.clone(),
            created_at_unix_ms: 1_800_006_000_000,
        })
        .await?;
    storage
        .record_visibility(VisibilityReceipt {
            project_id: project_id.clone(),
            receipt_id: "rfcdb-turso-package-evidence-graph".to_owned(),
            receipt_kind: "rfcdb.package-evidence-graph".to_owned(),
            body: projection_body.clone(),
            created_at_unix_ms: 1_800_006_000_100,
        })
        .await?;
    let edge = TopologyEdgeRecord {
        project_id: project_id.clone(),
        edge_id: TopologyEdgeId::new("edge:rfc-40.160-to-package-evidence")?,
        from_node_id: TopologyNodeId::new("rfc:40.160")?,
        to_node_id: TopologyNodeId::new("evidence:rfcdb.package-evidence-graph")?,
        edge_kind: "rfc.accepted_by_evidence".to_owned(),
        source_session_id: session_id,
        source_agent_id: agent_id,
        source_event_id: event_id,
        body: projection_body.clone(),
        created_at_unix_ms: 1_800_006_000_200,
    };
    storage.append_topology_edge(edge.clone()).await?;

    drop(storage);
    let reopened = TursoAgentStorage::open_local(TursoAgentStorageConfig {
        path: database_path,
        optimization_profile: marlin_agent_storage::TursoOptimizationProfile::AsyncIoWithMvccAndPassiveCheckpointExperimental,
        batch_transaction_mode: marlin_agent_storage::TursoBatchTransactionMode::Concurrent,
    })
    .await?;
    assert_eq!(
        reopened
            .get_artifact(&project_id, &artifact_hash)
            .await?
            .expect("evidence graph artifact should survive reopen")
            .body,
        projection_body
    );
    assert!(
        reopened
            .list_visibility_page(marlin_agent_storage::VisibilityPageRequest::new(
                project_id.clone(),
                marlin_agent_storage::StoragePageLimit::MAXIMUM,
            ))
            .await?
            .items
            .iter()
            .any(|receipt| receipt.receipt_kind == "rfcdb.package-evidence-graph")
    );
    assert_eq!(
        reopened
            .list_topology_edges_page(marlin_agent_storage::TopologyEdgePageRequest::new(
                project_id.clone(),
                marlin_agent_storage::StoragePageLimit::MAXIMUM,
            ))
            .await?
            .items,
        vec![edge]
    );

    Ok(())
}
