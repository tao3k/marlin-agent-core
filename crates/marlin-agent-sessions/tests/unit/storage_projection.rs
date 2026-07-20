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
        .list_context_events_page(
            "project:sessions",
            &root_context,
            marlin_agent_storage::StoragePageLimit::new(10).expect("valid page limit"),
            None,
        )
        .await
        .expect("root events should list")
        .items;
    let child_events = adapter
        .list_context_events_page(
            "project:sessions",
            &child_context,
            marlin_agent_storage::StoragePageLimit::new(10).expect("valid page limit"),
            None,
        )
        .await
        .expect("child events should list")
        .items;

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

#[tokio::test]
async fn session_storage_adapter_appends_context_events_atomically() {
    let context = AgentSessionContext::root(
        "batch-session",
        ContextVisibility::from_namespaces([ContextNamespace::Memory]),
    );
    let adapter = SessionStorageAdapter::new(InMemoryAgentStorage::new());
    let event = |turn_id: &str, event_id: &str, created_at_unix_ms: i64| {
        SessionStorageEvent::builder()
            .with_project_id("project:sessions:batch")
            .with_agent_id("agent:batch")
            .with_turn_id(turn_id)
            .with_event_id(event_id)
            .with_event_kind("session.context.batch")
            .with_body(event_id.as_bytes().to_vec())
            .with_created_at_unix_ms(created_at_unix_ms)
            .build()
            .expect("batch event should build")
    };

    let receipt = adapter
        .append_context_events_atomically(
            &context,
            vec![event("turn:1", "event:1", 1), event("turn:2", "event:2", 2)],
        )
        .await
        .expect("batch should persist atomically");

    assert_eq!(receipt.item_count, 2);
    assert_eq!(receipt.rows_affected, 2);
    let first_page = adapter
        .list_context_events_page(
            "project:sessions:batch",
            &context,
            marlin_agent_storage::StoragePageLimit::new(1).expect("valid page limit"),
            None,
        )
        .await
        .expect("first batch event page should list");
    assert_eq!(first_page.items.len(), 1);
    assert_eq!(first_page.items[0].event_id.as_str(), "event:1");
    let second_page = adapter
        .list_context_events_page(
            "project:sessions:batch",
            &context,
            marlin_agent_storage::StoragePageLimit::new(1).expect("valid page limit"),
            first_page.next_cursor,
        )
        .await
        .expect("second batch event page should list");
    assert_eq!(second_page.items.len(), 1);
    assert_eq!(second_page.items[0].event_id.as_str(), "event:2");
    assert!(second_page.next_cursor.is_none());
}

#[tokio::test]
async fn session_storage_adapter_rejects_duplicate_batch_without_partial_write() {
    let context = AgentSessionContext::root(
        "duplicate-batch-session",
        ContextVisibility::from_namespaces([ContextNamespace::Memory]),
    );
    let adapter = SessionStorageAdapter::new(InMemoryAgentStorage::new());
    let duplicate = || {
        SessionStorageEvent::builder()
            .with_project_id("project:sessions:duplicate-batch")
            .with_agent_id("agent:batch")
            .with_turn_id("turn:1")
            .with_event_id("event:duplicate")
            .with_event_kind("session.context.batch")
            .with_body(b"duplicate".to_vec())
            .with_created_at_unix_ms(1)
            .build()
            .expect("duplicate event should build")
    };

    let error = adapter
        .append_context_events_atomically(&context, vec![duplicate(), duplicate()])
        .await
        .expect_err("duplicate batch should fail atomically");
    assert!(error.to_string().contains("duplicate session event"));

    let events = adapter
        .list_context_events_page(
            "project:sessions:duplicate-batch",
            &context,
            marlin_agent_storage::StoragePageLimit::new(10).expect("valid page limit"),
            None,
        )
        .await
        .expect("events should list after rejected batch")
        .items;
    assert!(events.is_empty());
}
