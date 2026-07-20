#![cfg(feature = "turso")]

use marlin_agent_sessions::{
    AgentSessionContext, ContextNamespace, ContextVisibility, SessionId as RuntimeSessionId,
    SessionIsolationPolicy, SessionKind,
};
use marlin_agent_storage::{
    AgentId, AgentStorage, EventId, ProjectId, SessionEventRecord, SessionId as StorageSessionId,
    StorageResult, TurnId, TursoAgentStorage, TursoAgentStorageConfig, VisibilityReceipt,
};
use tempfile::tempdir;

fn project_id() -> StorageResult<ProjectId> {
    ProjectId::new("project:rfcdb-session-storage")
}

fn storage_session_id(session_id: &RuntimeSessionId) -> StorageResult<StorageSessionId> {
    StorageSessionId::new(session_id.as_str())
}

fn event_id(label: &str) -> StorageResult<EventId> {
    EventId::new(format!("event:session-storage:{label}"))
}

fn turn_id(label: &str) -> StorageResult<TurnId> {
    TurnId::new(format!("turn:session-storage:{label}"))
}

fn agent_id(label: &str) -> StorageResult<AgentId> {
    AgentId::new(format!("agent:session-storage:{label}"))
}

fn session_context_event(
    project_id: &ProjectId,
    context: &AgentSessionContext,
    label: &str,
) -> StorageResult<SessionEventRecord> {
    Ok(SessionEventRecord {
        project_id: project_id.clone(),
        session_id: storage_session_id(context.session_id())?,
        agent_id: agent_id(label)?,
        turn_id: turn_id(label)?,
        event_id: event_id(label)?,
        event_kind: "rfcdb.session.context".to_owned(),
        causality_parent_event_id: None,
        body: serde_json::to_vec(context).expect("session context should serialize"),
        created_at_unix_ms: 1_800_001_000_000,
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn turso_persists_session_identity_and_isolation_receipts() -> StorageResult<()> {
    let project_id = project_id()?;
    let root_visibility = ContextVisibility::from_namespaces([
        ContextNamespace::System,
        ContextNamespace::User,
        ContextNamespace::Workspace,
        ContextNamespace::Memory,
        ContextNamespace::Tools,
    ])
    .with_max_history_items(Some(128));
    let root_context = AgentSessionContext::root_with_policy(
        RuntimeSessionId::new("runtime-root-session"),
        root_visibility,
        SessionIsolationPolicy::strict(),
    );
    let requested_child_visibility = ContextVisibility::from_namespaces([
        ContextNamespace::Memory,
        ContextNamespace::Tools,
        ContextNamespace::Secrets,
    ])
    .with_max_history_items(Some(4096));
    let (child_context, isolation_receipt) = root_context.child_session(
        SessionKind::SubAgent,
        RuntimeSessionId::new("runtime-subagent-session"),
        requested_child_visibility,
    );

    assert_eq!(
        isolation_receipt.parent_session_id().as_str(),
        "runtime-root-session"
    );
    assert_eq!(
        isolation_receipt.child_session_id().as_str(),
        "runtime-subagent-session"
    );

    let tempdir = tempdir().expect("tempdir should be available");
    let db_path = tempdir.path().join("sessions.turso");
    let storage = TursoAgentStorage::open_local(TursoAgentStorageConfig { path: db_path.clone(), optimization_profile: marlin_agent_storage::TursoOptimizationProfile::AsyncIoWithMvccAndPassiveCheckpointExperimental, batch_transaction_mode: marlin_agent_storage::TursoBatchTransactionMode::Concurrent })
    .await?;

    storage
        .append_session_event(session_context_event(&project_id, &root_context, "root")?)
        .await?;
    storage
        .append_session_event(session_context_event(&project_id, &child_context, "child")?)
        .await?;
    storage
        .record_visibility(VisibilityReceipt {
            project_id: project_id.clone(),
            receipt_id: "session-isolation-runtime-subagent-session".to_owned(),
            receipt_kind: "rfcdb.session.isolation".to_owned(),
            body: serde_json::to_vec(&isolation_receipt)
                .expect("session isolation receipt should serialize"),
            created_at_unix_ms: 1_800_001_000_100,
        })
        .await?;

    drop(storage);

    let reopened = TursoAgentStorage::open_local(TursoAgentStorageConfig { path: db_path, optimization_profile: marlin_agent_storage::TursoOptimizationProfile::AsyncIoWithMvccAndPassiveCheckpointExperimental, batch_transaction_mode: marlin_agent_storage::TursoBatchTransactionMode::Concurrent })
    .await?;
    let root_events = reopened
        .list_session_events_page(marlin_agent_storage::SessionEventPageRequest::new(
            project_id.clone(),
            storage_session_id(root_context.session_id())?,
            marlin_agent_storage::StoragePageLimit::MAXIMUM,
        ))
        .await?
        .items;
    let child_events = reopened
        .list_session_events_page(marlin_agent_storage::SessionEventPageRequest::new(
            project_id.clone(),
            storage_session_id(child_context.session_id())?,
            marlin_agent_storage::StoragePageLimit::MAXIMUM,
        ))
        .await?
        .items;
    let visibility = reopened
        .list_visibility_page(marlin_agent_storage::VisibilityPageRequest::new(
            project_id.clone(),
            marlin_agent_storage::StoragePageLimit::MAXIMUM,
        ))
        .await?
        .items;

    assert_eq!(root_events.len(), 1);
    assert_eq!(child_events.len(), 1);
    assert!(String::from_utf8_lossy(&root_events[0].body).contains("runtime-root-session"));
    assert!(String::from_utf8_lossy(&child_events[0].body).contains("runtime-subagent-session"));
    assert!(visibility.iter().any(|receipt| {
        receipt.receipt_kind == "rfcdb.session.isolation"
            && String::from_utf8_lossy(&receipt.body).contains("runtime-subagent-session")
    }));

    Ok(())
}
