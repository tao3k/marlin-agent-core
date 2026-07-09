#![cfg(feature = "turso")]

use marlin_agent_storage::{
    AgentId, AgentStorage, ArtifactHash, ArtifactRecord, EventId, MemoryKey, MemoryProposalId,
    MemoryProposalRecord, ProjectId, SessionId, StorageError, TopologyEdgeId, TopologyEdgeRecord,
    TopologyNodeId, TursoAgentStorage, TursoAgentStorageConfig, TursoMvccMode,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn turso_persists_memory_proposals_and_topology_edges() {
    let tempdir = tempfile::tempdir().expect("create tempdir");
    let database_path = tempdir.path().join("agent-storage.db");
    let storage = TursoAgentStorage::open_local(TursoAgentStorageConfig {
        path: database_path.clone(),
        mvcc: TursoMvccMode::DisabledForCompatibility,
    })
    .await
    .expect("open turso storage");

    let project_id = ProjectId::new("project-db").expect("project id");
    let session_id = SessionId::new("session-alpha").expect("session id");
    let agent_id = AgentId::new("agent-memory").expect("agent id");
    let event_id = EventId::new("event-memory-001").expect("event id");
    let artifact_hash = ArtifactHash::new("sha256:org-memory-source").expect("artifact hash");
    let artifact_body = b"* Memory\n:PROPERTIES:\n:MARLIN_MEMORY_KEY: db/session\n:END:\n".to_vec();

    storage
        .put_artifact(ArtifactRecord {
            project_id: project_id.clone(),
            artifact_hash: artifact_hash.clone(),
            artifact_kind: "org.memory.source".to_string(),
            producer_session_id: session_id.clone(),
            producer_agent_id: agent_id.clone(),
            producer_event_id: event_id.clone(),
            media_type: "text/org".to_string(),
            size_bytes: artifact_body.len() as u64,
            body: artifact_body,
            created_at_unix_ms: 1,
        })
        .await
        .expect("put org memory source artifact");

    let memory_key = MemoryKey::new("db/session").expect("memory key");
    let proposal_id = MemoryProposalId::new("memory-proposal-001").expect("proposal id");
    let proposal = MemoryProposalRecord {
        project_id: project_id.clone(),
        proposal_id: proposal_id.clone(),
        memory_key: memory_key.clone(),
        proposal_status: "accepted".to_string(),
        source_artifact_hash: artifact_hash.clone(),
        source_session_id: session_id.clone(),
        source_agent_id: agent_id.clone(),
        source_event_id: event_id.clone(),
        org_source_path: ".marlin/memory/session.org".to_string(),
        org_source_begin: 1,
        org_source_end: 4,
        memory_kind: "session.fact".to_string(),
        body: b"db storage backs org memory recall".to_vec(),
        created_at_unix_ms: 2,
    };
    storage
        .put_memory_proposal(proposal.clone())
        .await
        .expect("put memory proposal");

    let duplicate_memory = storage.put_memory_proposal(proposal.clone()).await;
    assert!(matches!(
        duplicate_memory,
        Err(StorageError::DuplicateMemoryProposal { .. })
    ));

    let edge = TopologyEdgeRecord {
        project_id: project_id.clone(),
        edge_id: TopologyEdgeId::new("topology-edge-session-memory").expect("edge id"),
        from_node_id: TopologyNodeId::new("session:session-alpha").expect("from node"),
        to_node_id: TopologyNodeId::new("memory:db/session").expect("to node"),
        edge_kind: "session.proposes_memory".to_string(),
        source_session_id: session_id.clone(),
        source_agent_id: agent_id.clone(),
        source_event_id: event_id,
        body: b"session imported org memory proposal".to_vec(),
        created_at_unix_ms: 3,
    };
    storage
        .append_topology_edge(edge.clone())
        .await
        .expect("append topology edge");

    let duplicate_edge = storage.append_topology_edge(edge.clone()).await;
    assert!(matches!(
        duplicate_edge,
        Err(StorageError::DuplicateTopologyEdge { .. })
    ));

    let proposals = storage
        .list_memory_proposals(&project_id, Some(&memory_key))
        .await
        .expect("list memory proposals by key");
    assert_eq!(proposals, vec![proposal.clone()]);

    let edges = storage
        .list_topology_edges(&project_id)
        .await
        .expect("list topology edges");
    assert_eq!(edges, vec![edge.clone()]);

    drop(storage);

    let reopened = TursoAgentStorage::open_local(TursoAgentStorageConfig {
        path: database_path,
        mvcc: TursoMvccMode::DisabledForCompatibility,
    })
    .await
    .expect("reopen turso storage");
    assert_eq!(
        reopened
            .list_memory_proposals(&project_id, Some(&memory_key))
            .await
            .expect("reopened memory proposals"),
        vec![proposal]
    );
    assert_eq!(
        reopened
            .list_topology_edges(&project_id)
            .await
            .expect("reopened topology edges"),
        vec![edge]
    );
}
