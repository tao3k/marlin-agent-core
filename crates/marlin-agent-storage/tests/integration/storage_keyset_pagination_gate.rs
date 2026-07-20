#![cfg(feature = "turso")]

use marlin_agent_storage::{
    AgentId, AgentStorage, ArtifactHash, ArtifactRecord, EventId, InMemoryAgentStorage, MemoryKey,
    MemoryProposalId, MemoryProposalPageRequest, MemoryProposalRecord, ProjectId,
    SessionEventPageRequest, SessionEventRecord, SessionId, StorageError, StoragePageLimit,
    StorageResult, TopologyEdgeId, TopologyEdgePageRequest, TopologyEdgeRecord, TopologyNodeId,
    TurnId, TursoAgentStorage, TursoAgentStorageConfig, TursoOptimizationProfile,
    VisibilityPageRequest, VisibilityReceipt,
};

fn project_id() -> ProjectId {
    ProjectId::new("project:keyset-pagination").expect("project id")
}

fn session_id() -> SessionId {
    SessionId::new("session:keyset-pagination").expect("session id")
}

fn agent_id() -> AgentId {
    AgentId::new("agent:keyset-pagination").expect("agent id")
}

fn event_id(index: usize) -> EventId {
    EventId::new(format!("event:{index:03}")).expect("event id")
}

fn session_event(index: usize) -> SessionEventRecord {
    SessionEventRecord {
        project_id: project_id(),
        session_id: session_id(),
        agent_id: agent_id(),
        turn_id: TurnId::new("turn:shared").expect("turn id"),
        event_id: event_id(index),
        event_kind: "storage.keyset.session".to_owned(),
        causality_parent_event_id: (index > 0).then(|| event_id(index - 1)),
        body: format!("session event {index}").into_bytes(),
        created_at_unix_ms: 100,
    }
}

async fn exercise_keyset_contract(storage: &dyn AgentStorage) -> StorageResult<()> {
    storage
        .append_session_events_atomically((0..5).map(session_event).collect())
        .await?;
    let mut other_session_event = session_event(999);
    other_session_event.session_id = SessionId::new("session:keyset-other")?;
    storage.append_session_event(other_session_event).await?;
    for index in 0..5 {
        storage
            .record_visibility(VisibilityReceipt {
                project_id: project_id(),
                receipt_id: format!("receipt:{index:03}"),
                receipt_kind: "storage.keyset.visibility".to_owned(),
                body: format!("visibility {index}").into_bytes(),
                created_at_unix_ms: 200,
            })
            .await?;
    }
    storage
        .record_visibility(VisibilityReceipt {
            project_id: ProjectId::new("project:keyset-other")?,
            receipt_id: "receipt:foreign".to_owned(),
            receipt_kind: "storage.keyset.visibility".to_owned(),
            body: b"foreign visibility".to_vec(),
            created_at_unix_ms: 200,
        })
        .await?;

    let artifact_hash = ArtifactHash::new("sha256:keyset-source")?;
    storage
        .put_artifact(ArtifactRecord {
            project_id: project_id(),
            artifact_hash: artifact_hash.clone(),
            artifact_kind: "storage.keyset.source".to_owned(),
            producer_session_id: session_id(),
            producer_agent_id: agent_id(),
            producer_event_id: event_id(0),
            media_type: "application/octet-stream".to_owned(),
            size_bytes: 6,
            body: b"source".to_vec(),
            created_at_unix_ms: 250,
        })
        .await?;

    let memory_key = MemoryKey::new("memory:keyset-pagination")?;
    for index in 0..5 {
        storage
            .put_memory_proposal(MemoryProposalRecord {
                project_id: project_id(),
                proposal_id: MemoryProposalId::new(format!("proposal:{index:03}"))?,
                memory_key: memory_key.clone(),
                proposal_status: "accepted".to_owned(),
                source_artifact_hash: artifact_hash.clone(),
                source_session_id: session_id(),
                source_agent_id: agent_id(),
                source_event_id: event_id(index),
                org_source_path: "memory/keyset.org".to_owned(),
                org_source_begin: index as u32,
                org_source_end: index as u32 + 1,
                memory_kind: "session.fact".to_owned(),
                body: format!("proposal {index}").into_bytes(),
                created_at_unix_ms: 300,
            })
            .await?;
        storage
            .append_topology_edge(TopologyEdgeRecord {
                project_id: project_id(),
                edge_id: TopologyEdgeId::new(format!("edge:{index:03}"))?,
                from_node_id: TopologyNodeId::new("session:keyset-pagination")?,
                to_node_id: TopologyNodeId::new(format!("memory:keyset-pagination:{index:03}"))?,
                edge_kind: "session.proposes_memory".to_owned(),
                source_session_id: session_id(),
                source_agent_id: agent_id(),
                source_event_id: event_id(index),
                body: format!("edge {index}").into_bytes(),
                created_at_unix_ms: 400,
            })
            .await?;
    }
    storage
        .put_memory_proposal(MemoryProposalRecord {
            project_id: project_id(),
            proposal_id: MemoryProposalId::new("proposal:foreign-key")?,
            memory_key: MemoryKey::new("memory:keyset-other")?,
            proposal_status: "accepted".to_owned(),
            source_artifact_hash: artifact_hash,
            source_session_id: session_id(),
            source_agent_id: agent_id(),
            source_event_id: event_id(0),
            org_source_path: "memory/keyset-other.org".to_owned(),
            org_source_begin: 0,
            org_source_end: 1,
            memory_kind: "session.fact".to_owned(),
            body: b"foreign memory key".to_vec(),
            created_at_unix_ms: 300,
        })
        .await?;
    storage
        .append_topology_edge(TopologyEdgeRecord {
            project_id: ProjectId::new("project:keyset-other")?,
            edge_id: TopologyEdgeId::new("edge:foreign-project")?,
            from_node_id: TopologyNodeId::new("session:keyset-other")?,
            to_node_id: TopologyNodeId::new("memory:keyset-other")?,
            edge_kind: "session.proposes_memory".to_owned(),
            source_session_id: SessionId::new("session:keyset-other")?,
            source_agent_id: agent_id(),
            source_event_id: event_id(0),
            body: b"foreign topology".to_vec(),
            created_at_unix_ms: 400,
        })
        .await?;

    let limit = StoragePageLimit::new(2)?;

    let mut session_cursor = None;
    let mut session_ids = Vec::new();
    loop {
        let mut request = SessionEventPageRequest::new(project_id(), session_id(), limit);
        if let Some(cursor) = session_cursor.take() {
            request = request.after(cursor);
        }
        let page = storage.list_session_events_page(request).await?;
        assert!(page.items.len() <= 2);
        session_ids.extend(page.items.into_iter().map(|record| record.event_id));
        let Some(cursor) = page.next_cursor else {
            break;
        };
        session_cursor = Some(cursor);
    }
    assert_eq!(session_ids, (0..5).map(event_id).collect::<Vec<_>>());

    let mut visibility_cursor = None;
    let mut receipt_ids = Vec::new();
    loop {
        let mut request = VisibilityPageRequest::new(project_id(), limit);
        if let Some(cursor) = visibility_cursor.take() {
            request = request.after(cursor);
        }
        let page = storage.list_visibility_page(request).await?;
        assert!(page.items.len() <= 2);
        receipt_ids.extend(page.items.into_iter().map(|record| record.receipt_id));
        let Some(cursor) = page.next_cursor else {
            break;
        };
        visibility_cursor = Some(cursor);
    }
    assert_eq!(
        receipt_ids,
        (0..5)
            .map(|index| format!("receipt:{index:03}"))
            .collect::<Vec<_>>()
    );

    let mut proposal_cursor = None;
    let mut proposal_ids = Vec::new();
    loop {
        let mut request =
            MemoryProposalPageRequest::new(project_id(), limit).for_memory_key(memory_key.clone());
        if let Some(cursor) = proposal_cursor.take() {
            request = request.after(cursor);
        }
        let page = storage.list_memory_proposals_page(request).await?;
        assert!(page.items.len() <= 2);
        proposal_ids.extend(page.items.into_iter().map(|record| record.proposal_id));
        let Some(cursor) = page.next_cursor else {
            break;
        };
        proposal_cursor = Some(cursor);
    }
    assert_eq!(
        proposal_ids,
        (0..5)
            .map(|index| MemoryProposalId::new(format!("proposal:{index:03}")).unwrap())
            .collect::<Vec<_>>()
    );

    let mut edge_cursor = None;
    let mut edge_ids = Vec::new();
    loop {
        let mut request = TopologyEdgePageRequest::new(project_id(), limit);
        if let Some(cursor) = edge_cursor.take() {
            request = request.after(cursor);
        }
        let page = storage.list_topology_edges_page(request).await?;
        assert!(page.items.len() <= 2);
        edge_ids.extend(page.items.into_iter().map(|record| record.edge_id));
        let Some(cursor) = page.next_cursor else {
            break;
        };
        edge_cursor = Some(cursor);
    }
    assert_eq!(
        edge_ids,
        (0..5)
            .map(|index| TopologyEdgeId::new(format!("edge:{index:03}")).unwrap())
            .collect::<Vec<_>>()
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn keyset_pagination_contract_matches_memory_and_turso_backends() -> StorageResult<()> {
    let memory = InMemoryAgentStorage::new();
    exercise_keyset_contract(&memory).await?;

    let tempdir = tempfile::tempdir().expect("tempdir");
    let turso = TursoAgentStorage::open_local(TursoAgentStorageConfig {
        path: tempdir.path().join("keyset-pagination.turso"),
        optimization_profile: TursoOptimizationProfile::AsyncIoWithMvcc,
        batch_transaction_mode: marlin_agent_storage::TursoBatchTransactionMode::Concurrent,
    })
    .await?;
    exercise_keyset_contract(&turso).await
}

#[test]
fn page_limit_is_a_typed_bounded_protocol() {
    assert_eq!(
        StoragePageLimit::new(0),
        Err(StorageError::InvalidPageLimit { limit: 0 })
    );
    assert_eq!(
        StoragePageLimit::new(StoragePageLimit::MAX + 1),
        Err(StorageError::InvalidPageLimit {
            limit: StoragePageLimit::MAX + 1,
        })
    );
    assert_eq!(StoragePageLimit::MAXIMUM.get(), 1_000);
}
