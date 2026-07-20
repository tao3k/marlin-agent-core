#![cfg(feature = "turso")]

use marlin_agent_storage::{
    AgentId, AgentStorage, ArtifactHash, ArtifactRecord, EventId, MemoryKey, MemoryProposalId,
    MemoryProposalRecord, ProjectId, SessionId, StorageError, TopologyEdgeId, TopologyEdgeRecord,
    TopologyNodeId, TursoAgentStorage, TursoAgentStorageConfig, TursoMvccMode,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rfcdb_turso_memory_topology_acceptance_flow() {
    let tempdir = tempfile::tempdir().expect("create tempdir");
    let database_path = tempdir.path().join("agent-storage.db");
    let storage = TursoAgentStorage::open_local(TursoAgentStorageConfig { path: database_path.clone(), optimization_profile: marlin_agent_storage::TursoOptimizationProfile::AsyncIoWithMvccAndPassiveCheckpointExperimental, batch_transaction_mode: marlin_agent_storage::TursoBatchTransactionMode::Concurrent })
    .await
    .expect("open turso storage");
    assert_eq!(
        storage.optimization_receipt(),
        marlin_agent_storage::TursoOptimizationReceipt {
            async_io: marlin_agent_storage::TursoAsyncIoMode::Enabled,
            mvcc: TursoMvccMode::Required,
            mvcc_checkpoint: marlin_agent_storage::TursoMvccCheckpointMode::PassiveExperimental,
            connection_lanes: 4,
            statement_cache:
                marlin_agent_storage::TursoStatementCacheMode::PreparedCachedPerConnection,
        }
    );

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
    let later_proposal = MemoryProposalRecord {
        proposal_id: MemoryProposalId::new("memory-proposal-002").expect("later proposal id"),
        proposal_status: "candidate".to_string(),
        body: b"later memory proposal remains ordered".to_vec(),
        created_at_unix_ms: 4,
        ..proposal.clone()
    };
    storage
        .put_memory_proposal(later_proposal.clone())
        .await
        .expect("put later memory proposal");

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
    let later_edge = TopologyEdgeRecord {
        edge_id: TopologyEdgeId::new("topology-edge-memory-evidence").expect("later edge id"),
        to_node_id: TopologyNodeId::new("artifact:sha256:org-memory-source")
            .expect("later to node"),
        edge_kind: "memory.references_artifact".to_string(),
        body: b"memory proposal retains source evidence".to_vec(),
        created_at_unix_ms: 5,
        ..edge.clone()
    };
    storage
        .append_topology_edge(later_edge.clone())
        .await
        .expect("append later topology edge");

    let duplicate_edge = storage.append_topology_edge(edge.clone()).await;
    assert!(matches!(
        duplicate_edge,
        Err(StorageError::DuplicateTopologyEdge { .. })
    ));

    let concurrent_proposals = (0..6)
        .map(|index| MemoryProposalRecord {
            proposal_id: MemoryProposalId::new(format!("memory-proposal-concurrent-{index:03}"))
                .expect("concurrent proposal id"),
            proposal_status: "accepted".to_string(),
            body: format!("concurrent memory proposal {index}").into_bytes(),
            created_at_unix_ms: 10 + index,
            ..proposal.clone()
        })
        .collect::<Vec<_>>();
    let concurrent_edges = (0..6)
        .map(|index| TopologyEdgeRecord {
            edge_id: TopologyEdgeId::new(format!("topology-edge-concurrent-{index:03}"))
                .expect("concurrent edge id"),
            to_node_id: TopologyNodeId::new(format!("memory:db/session/{index:03}"))
                .expect("concurrent to node"),
            edge_kind: "session.proposes_memory.concurrent".to_string(),
            body: format!("concurrent topology evidence {index}").into_bytes(),
            created_at_unix_ms: 20 + index,
            ..edge.clone()
        })
        .collect::<Vec<_>>();
    let mut concurrent_writers = tokio::task::JoinSet::new();
    for (proposal_record, edge_record) in concurrent_proposals
        .iter()
        .cloned()
        .zip(concurrent_edges.iter().cloned())
    {
        let writer_storage = storage.clone();
        concurrent_writers.spawn(async move {
            writer_storage.put_memory_proposal(proposal_record).await?;
            writer_storage.append_topology_edge(edge_record).await?;
            Ok::<(), StorageError>(())
        });
    }
    while let Some(result) = concurrent_writers.join_next().await {
        result
            .expect("concurrent memory/topology writer should not panic")
            .expect("concurrent memory/topology writer should retry lock contention");
    }

    let mut expected_proposals = vec![proposal.clone(), later_proposal.clone()];
    expected_proposals.extend(concurrent_proposals);
    expected_proposals.sort_by_key(|record| record.created_at_unix_ms);
    let mut expected_edges = vec![edge.clone(), later_edge.clone()];
    expected_edges.extend(concurrent_edges);
    expected_edges.sort_by_key(|record| record.created_at_unix_ms);

    let proposals = storage
        .list_memory_proposals_page(
            marlin_agent_storage::MemoryProposalPageRequest::new(
                project_id.clone(),
                marlin_agent_storage::StoragePageLimit::MAXIMUM,
            )
            .for_memory_key(memory_key.clone()),
        )
        .await
        .expect("list memory proposals by key")
        .items;
    assert_eq!(proposals, expected_proposals.clone());
    assert!(
        storage
            .list_memory_proposals_page(
                marlin_agent_storage::MemoryProposalPageRequest::new(
                    project_id.clone(),
                    marlin_agent_storage::StoragePageLimit::MAXIMUM,
                )
                .for_memory_key(MemoryKey::new("db/absent").expect("absent memory key")),
            )
            .await
            .expect("list absent memory key")
            .items
            .is_empty()
    );

    let edges = storage
        .list_topology_edges_page(marlin_agent_storage::TopologyEdgePageRequest::new(
            project_id.clone(),
            marlin_agent_storage::StoragePageLimit::MAXIMUM,
        ))
        .await
        .expect("list topology edges")
        .items;
    assert_eq!(edges, expected_edges.clone());

    drop(storage);

    let reopened = TursoAgentStorage::open_local(TursoAgentStorageConfig { path: database_path, optimization_profile: marlin_agent_storage::TursoOptimizationProfile::AsyncIoWithMvccAndPassiveCheckpointExperimental, batch_transaction_mode: marlin_agent_storage::TursoBatchTransactionMode::Concurrent })
    .await
    .expect("reopen turso storage");
    assert_eq!(
        reopened.optimization_receipt(),
        marlin_agent_storage::TursoOptimizationReceipt {
            async_io: marlin_agent_storage::TursoAsyncIoMode::Enabled,
            mvcc: TursoMvccMode::Required,
            mvcc_checkpoint: marlin_agent_storage::TursoMvccCheckpointMode::PassiveExperimental,
            connection_lanes: 4,
            statement_cache:
                marlin_agent_storage::TursoStatementCacheMode::PreparedCachedPerConnection,
        }
    );
    assert_eq!(
        reopened
            .list_memory_proposals_page(
                marlin_agent_storage::MemoryProposalPageRequest::new(
                    project_id.clone(),
                    marlin_agent_storage::StoragePageLimit::MAXIMUM,
                )
                .for_memory_key(memory_key.clone()),
            )
            .await
            .expect("reopened memory proposals")
            .items,
        expected_proposals
    );
    assert_eq!(
        reopened
            .list_topology_edges_page(marlin_agent_storage::TopologyEdgePageRequest::new(
                project_id.clone(),
                marlin_agent_storage::StoragePageLimit::MAXIMUM,
            ))
            .await
            .expect("reopened topology edges")
            .items,
        expected_edges
    );
}
