// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Real on-disk `TursoAgentStorage` persistence, restart recovery, live Org
//! parser admission, and cross-generation edge isolation integration tests for
//! Marlin's converged search engine.

use std::sync::Arc;

use marlin_agent_search_engine::{
    AgentTopologyRelationship, MarlinAgentStorageSearchPort, MarlinContextOrgPort,
    MarlinSearchStorageProvenance, SearchGenerationDigest, SearchProjectId, SearchStoragePortError,
    SearchTopologyEdgeId, SearchTopologyEdgeRecord, SearchWorkspaceId, marlin_search_engine,
};
use marlin_agent_storage::{
    TursoAgentStorage, TursoAgentStorageConfig, TursoBatchTransactionMode, TursoOptimizationProfile,
};
use tempfile::tempdir;

const LIVE_MANIFEST_ORG: &str = "\
:PROPERTIES:
:CONTRACT_ORG: [[../contracts/project.workspace-manifest.v1.org][project.workspace-manifest.v1]]
:END:

* Project Workspace
:PROPERTIES:
:PROJECT_WORKSPACE_ID: marlin-turso-live
:PROJECT_WORKSPACE_IDENTITY: git+https://github.com/tao3k/marlin-agent-core.git#workspace/turso
:WORKSPACE_ROOT_PATH: .
:PORTABILITY: cross-machine
:REPOSITORY_ALIASES: []
:END:
";

async fn open_turso_engine(
    db_path: std::path::PathBuf,
) -> (
    Arc<TursoAgentStorage>,
    marlin_agent_search_engine::MarlinConvergedSearchEngine,
) {
    let storage = Arc::new(
        TursoAgentStorage::open_local(TursoAgentStorageConfig {
            path: db_path,
            optimization_profile: TursoOptimizationProfile::AsyncIoWithMvcc,
            batch_transaction_mode: TursoBatchTransactionMode::Concurrent,
        })
        .await
        .expect("open local TursoAgentStorage"),
    );
    let org_port = Arc::new(MarlinContextOrgPort::with_live_org_parser());
    let storage_port = Arc::new(MarlinAgentStorageSearchPort::new(
        Arc::clone(&storage),
        MarlinSearchStorageProvenance::new("turso-session", "agent-search", "evt-turso")
            .expect("valid provenance"),
    ));
    let engine = marlin_search_engine(org_port, storage_port);
    (storage, engine)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn turso_persists_cas_generations_edges_and_live_org_across_reopen() {
    let temp = tempdir().expect("create tempdir");
    let db_path = temp.path().join("marlin-search-engine.turso");

    let project_id = SearchProjectId::new("proj-marlin-turso").expect("project id");
    let workspace_id = SearchWorkspaceId::new("ws-main").expect("workspace id");
    let gen1 = SearchGenerationDigest::new(
        "blake3-256:1111111111111111111111111111111111111111111111111111111111111111",
    )
    .expect("gen1");
    let gen2 = SearchGenerationDigest::new(
        "blake3-256:2222222222222222222222222222222222222222222222222222222222222222",
    )
    .expect("gen2");
    let gen3 = SearchGenerationDigest::new(
        "blake3-256:3333333333333333333333333333333333333333333333333333333333333333",
    )
    .expect("gen3");

    let ptr2_hash = {
        let (storage, engine) = open_turso_engine(db_path.clone()).await;

        // 1. Live zero-FFI Org parser admits real Org manifest + overlay directly.
        let manifest = engine
            .admit_topology_manifest(LIVE_MANIFEST_ORG)
            .expect("live Org parser admits topology manifest");
        assert_eq!(
            manifest.project_workspace().project_workspace_identity(),
            "git+https://github.com/tao3k/marlin-agent-core.git#workspace/turso"
        );
        let overlay = engine
            .admit_org_overlay(
                gen1.as_str(),
                gen2.as_str(),
                "crates/marlin-agent-search-engine/src/storage_port.rs#item=MarlinAgentStorageSearchPort",
                gen3.as_str(),
                "blake3-256:4444444444444444444444444444444444444444444444444444444444444444",
                "blake3-256:5555555555555555555555555555555555555555555555555555555555555555",
                "Turso-backed converged search overlay",
                LIVE_MANIFEST_ORG,
                vec![AgentTopologyRelationship {
                    from_selector: "crates/marlin-agent-search-engine/src/storage_port.rs#item=MarlinAgentStorageSearchPort".to_owned(),
                    relation: "persists-in".to_owned(),
                    to_selector: "crates/marlin-agent-storage/src/turso_storage.rs#item=TursoAgentStorage".to_owned(),
                }],
            )
            .expect("live Org parser admits overlay");
        assert_eq!(overlay.terminal.state, "admitted");

        // 2. CAS publish gen1 -> gen2 + stale conflict rejection on real Turso.
        let ptr1 = engine
            .publish_generation_artifact(
                project_id.clone(),
                workspace_id.clone(),
                None,
                gen1.clone(),
                "application/octet-stream",
                b"turso-gen1-segment".to_vec(),
            )
            .expect("publish gen1 to Turso");
        assert_eq!(ptr1.revision, 1);

        let conflict = engine.publish_generation_artifact(
            project_id.clone(),
            workspace_id.clone(),
            None,
            gen2.clone(),
            "application/octet-stream",
            b"stale-writer".to_vec(),
        );
        assert!(matches!(
            conflict,
            Err(SearchStoragePortError::PointerConflict { .. })
        ));

        let ptr2 = engine
            .publish_generation_artifact(
                project_id.clone(),
                workspace_id.clone(),
                Some(&ptr1.artifact_hash),
                gen2.clone(),
                "application/octet-stream",
                b"turso-gen2-segment".to_vec(),
            )
            .expect("CAS advance to gen2 on Turso");
        assert_eq!(ptr2.revision, 2);

        // 3. Same logical edge_id across two generations must NOT collide on real Turso.
        let shared_edge_gen1 = SearchTopologyEdgeRecord {
            project_id: project_id.clone(),
            generation_digest: gen1.clone(),
            edge_id: SearchTopologyEdgeId::new("edge-shared").expect("edge id"),
            from_node: "owner:gen1/a.rs".to_owned(),
            to_node: "owner:gen1/b.rs".to_owned(),
            relation: "calls".to_owned(),
            payload: b"gen1-edge".to_vec(),
        };
        let shared_edge_gen2 = SearchTopologyEdgeRecord {
            project_id: project_id.clone(),
            generation_digest: gen2.clone(),
            edge_id: SearchTopologyEdgeId::new("edge-shared").expect("edge id"),
            from_node: "owner:gen2/a.rs".to_owned(),
            to_node: "owner:gen2/b.rs".to_owned(),
            relation: "calls".to_owned(),
            payload: b"gen2-edge".to_vec(),
        };
        assert_eq!(
            engine
                .persist_topology_edges(&[shared_edge_gen1.clone(), shared_edge_gen2.clone()])
                .expect("persist cross-generation shared edge ids"),
            2
        );
        assert_eq!(
            engine
                .persist_topology_edges(std::slice::from_ref(&shared_edge_gen2))
                .expect("duplicate append is idempotent"),
            0
        );

        drop(engine);
        drop(storage);
        ptr2.artifact_hash
    };

    // 4. Reopen Turso database from disk and prove crash/restart recovery + monotonic CAS.
    let (_reopened_storage, reopened_engine) = open_turso_engine(db_path).await;
    let (loaded_ptr, loaded_blob) = reopened_engine
        .load_active_generation_artifact(&project_id, &workspace_id)
        .expect("load active generation after reopen")
        .expect("active pointer persisted on disk");
    assert_eq!(loaded_ptr.revision, 2);
    assert_eq!(loaded_ptr.generation_digest, gen2);
    assert_eq!(loaded_ptr.artifact_hash, ptr2_hash);
    assert_eq!(loaded_blob.body, b"turso-gen2-segment");
    loaded_blob.verify_digest().expect("BLAKE3 digest verified");

    let gen1_edges = reopened_engine
        .load_topology_edges(&project_id, &gen1)
        .expect("load gen1 edges after reopen");
    let gen2_edges = reopened_engine
        .load_topology_edges(&project_id, &gen2)
        .expect("load gen2 edges after reopen");
    assert_eq!(gen1_edges.len(), 1);
    assert_eq!(gen1_edges[0].edge_id.as_str(), "edge-shared");
    assert_eq!(gen1_edges[0].payload, b"gen1-edge");
    assert_eq!(gen2_edges.len(), 1);
    assert_eq!(gen2_edges[0].edge_id.as_str(), "edge-shared");
    assert_eq!(gen2_edges[0].payload, b"gen2-edge");

    let ptr3 = reopened_engine
        .publish_generation_artifact(
            project_id.clone(),
            workspace_id.clone(),
            Some(&ptr2_hash),
            gen3.clone(),
            "application/octet-stream",
            b"turso-gen3-segment".to_vec(),
        )
        .expect("CAS advance gen2 -> gen3 after reopen");
    assert_eq!(ptr3.revision, 3);
    assert_eq!(ptr3.generation_digest, gen3);
}
