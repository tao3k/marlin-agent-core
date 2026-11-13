// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later
#![cfg(feature = "turso")]

use asp_rust::{AspRustRunScope, default_asp_rust_config, run_asp_rust_with_config_for_scope};
use marlin_agent_storage::{
    AgentId, AgentStorage, ArtifactHash, ArtifactRecord, EventId, ProjectId, SessionId,
    StorageResult, TopologyEdgeId, TopologyEdgeRecord, TopologyNodeId, TursoAgentStorage,
    TursoAgentStorageConfig, VisibilityReceipt,
};
use tempfile::tempdir;

#[derive(Clone, Debug, PartialEq, Eq)]
struct RfcdbAspReportProjectionReceipt {
    package_name: String,
    file_count: usize,
    parsed_count: usize,
}

impl RfcdbAspReportProjectionReceipt {
    fn visibility_body(&self) -> Vec<u8> {
        format!(
            "package_name={}\nfile_count={}\nparsed_count={}\n",
            self.package_name, self.file_count, self.parsed_count
        )
        .into_bytes()
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn rfcdb_turso_asp_report_survives_reopen() -> StorageResult<()> {
    let crate_root = crate::paths::crate_root();
    let package_name = crate::paths::STORAGE_PACKAGE_NAME.to_owned();
    let report = run_asp_rust_with_config_for_scope(
        &crate_root,
        &default_asp_rust_config(),
        AspRustRunScope::Package,
    )
    .expect("storage crate should produce an ASP Rust report");
    assert!(report.file_count() > 0);
    assert_eq!(report.file_count(), report.parsed_count());

    let projection = RfcdbAspReportProjectionReceipt {
        package_name,
        file_count: report.file_count(),
        parsed_count: report.parsed_count(),
    };
    let projection_body = projection.visibility_body();
    let project_id = ProjectId::new("project:rfcdb-turso-asp-report")?;
    let artifact_hash = ArtifactHash::new("sha256:rfcdb-turso-package-asp-report")?;
    let session_id = SessionId::new("session:rfcdb-asp-report")?;
    let agent_id = AgentId::new("agent:rfcdb-asp-report")?;
    let event_id = EventId::new("event:rfcdb-asp-report")?;
    let tempdir = tempdir().expect("tempdir should be available");
    let database_path = tempdir.path().join("rfcdb-asp-report.turso");
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
            artifact_kind: "rfcdb.package-asp-report".to_owned(),
            producer_session_id: session_id.clone(),
            producer_agent_id: agent_id.clone(),
            producer_event_id: event_id.clone(),
            media_type: "application/vnd.marlin.rfcdb-asp-report-receipt".to_owned(),
            size_bytes: projection_body.len() as u64,
            body: projection_body.clone(),
            created_at_unix_ms: 1_800_006_000_000,
        })
        .await?;
    storage
        .record_visibility(VisibilityReceipt {
            project_id: project_id.clone(),
            receipt_id: "rfcdb-turso-package-asp-report".to_owned(),
            receipt_kind: "rfcdb.package-asp-report".to_owned(),
            body: projection_body.clone(),
            created_at_unix_ms: 1_800_006_000_100,
        })
        .await?;
    let edge = TopologyEdgeRecord {
        project_id: project_id.clone(),
        edge_id: TopologyEdgeId::new("edge:rfc-40.160-to-package-evidence")?,
        from_node_id: TopologyNodeId::new("rfc:40.160")?,
        to_node_id: TopologyNodeId::new("evidence:rfcdb.package-asp-report")?,
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
            .expect("ASP report artifact should survive reopen")
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
            .any(|receipt| receipt.receipt_kind == "rfcdb.package-asp-report")
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
