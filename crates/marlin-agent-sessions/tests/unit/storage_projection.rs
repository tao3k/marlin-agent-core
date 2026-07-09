use marlin_agent_sessions::{
    AgentSessionContext, ContextNamespace, ContextVisibility, SessionKind, SessionStorageAdapter,
    SessionStorageEvent,
};
use marlin_agent_storage::InMemoryAgentStorage;

#[tokio::test]
async fn session_storage_adapter_projects_context_events_with_isolation() {
    let root_context = AgentSessionContext::root(
        "root-session",
        ContextVisibility::from_namespaces([
            ContextNamespace::System,
            ContextNamespace::Workspace,
            ContextNamespace::Memory,
        ]),
    );
    let (child_context, isolation_receipt) = root_context.child_session(
        SessionKind::SubAgent,
        "subagent-session",
        ContextVisibility::from_namespaces([ContextNamespace::Memory, ContextNamespace::Secrets]),
    );
    let adapter = SessionStorageAdapter::new(InMemoryAgentStorage::new());

    adapter
        .append_context_event(
            &root_context,
            SessionStorageEvent::builder()
                .with_project_id("project:sessions")
                .with_agent_id("agent:root")
                .with_turn_id("turn:1")
                .with_event_id("event:root")
                .with_event_kind("session.context")
                .with_body(b"root context".to_vec())
                .with_created_at_unix_ms(1)
                .build()
                .expect("root event should build"),
        )
        .await
        .expect("root session event should persist");
    adapter
        .append_context_event(
            &child_context,
            SessionStorageEvent::builder()
                .with_project_id("project:sessions")
                .with_agent_id("agent:child")
                .with_turn_id("turn:1")
                .with_event_id("event:child")
                .with_event_kind("session.context")
                .with_parent_event_id("event:root")
                .with_body(
                    format!(
                        "child={} denied={}",
                        child_context.session_id(),
                        isolation_receipt.denied_namespaces().len()
                    )
                    .into_bytes(),
                )
                .with_created_at_unix_ms(2)
                .build()
                .expect("child event should build"),
        )
        .await
        .expect("child session event should persist");

    let root_events = adapter
        .list_context_events("project:sessions", &root_context)
        .await
        .expect("root events should list");
    let child_events = adapter
        .list_context_events("project:sessions", &child_context)
        .await
        .expect("child events should list");

    assert_eq!(root_events.len(), 1);
    assert_eq!(child_events.len(), 1);
    assert_eq!(root_events[0].session_id.as_str(), "root-session");
    assert_eq!(child_events[0].session_id.as_str(), "subagent-session");
    assert_eq!(
        child_events[0]
            .causality_parent_event_id
            .as_ref()
            .expect("child event should link parent")
            .as_str(),
        "event:root"
    );
    assert!(String::from_utf8_lossy(&child_events[0].body).contains("denied=1"));
}
