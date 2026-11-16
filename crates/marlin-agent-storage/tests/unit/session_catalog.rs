// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use std::num::NonZeroU32;

use marlin_agent_storage::{
    AgentId, EventId, InMemoryAgentStorage, ProjectId, SessionEventCursor, SessionEventPageRequest,
    SessionEventRecord, SessionId, SessionPageRequest, StoragePageLimit, TurnId,
};
use marlin_agent_storage::{SessionEventStore, StorageError};

fn project() -> ProjectId {
    ProjectId::new("project").unwrap()
}

#[tokio::test]
async fn event_counts_follow_atomic_writes_and_exact_session_scope() {
    let storage = InMemoryAgentStorage::new();
    let first = session_event(session("a"), agent("one"), turn("1"), event("1"));
    let second = session_event(session("a"), agent("two"), turn("1"), event("2"));
    let other = session_event(session("b"), agent("one"), turn("1"), event("1"));
    let mut foreign = other.clone();
    foreign.project_id = ProjectId::new("other-project").unwrap();
    storage.append_session_event(first.clone()).await.unwrap();
    storage
        .append_session_events_atomically(vec![second, other, foreign.clone()])
        .await
        .unwrap();
    let count = |project: ProjectId, session: SessionId| {
        let storage = storage.clone();
        async move {
            storage
                .count_session_events(&project, &session)
                .await
                .unwrap()
                .get()
        }
    };
    assert_eq!(count(project(), session("a")).await, 2);
    assert_eq!(count(project(), session("b")).await, 1);
    assert_eq!(count(foreign.project_id, session("b")).await, 1);
    assert_eq!(count(project(), session("absent")).await, 0);
    let fresh = session_event(session("b"), agent("one"), turn("1"), event("2"));
    assert!(
        storage
            .append_session_events_atomically(vec![fresh, first])
            .await
            .is_err()
    );
    assert_eq!(count(project(), session("a")).await, 2);
    assert_eq!(count(project(), session("b")).await, 1);
}

#[tokio::test]
async fn event_pages_seek_within_exact_project_and_session_scope() {
    let storage = InMemoryAgentStorage::new();
    for (session_id, turn_id, event_id) in [
        ("a", "1", "1"),
        ("b", "1", "1"),
        ("b", "1", "3"),
        ("b", "2", "1"),
        ("b-extra", "1", "1"),
    ] {
        storage
            .append_session_event(session_event(
                session(session_id),
                agent("one"),
                turn(turn_id),
                event(event_id),
            ))
            .await
            .unwrap();
    }
    let mut foreign = session_event(session("b"), agent("one"), turn("1"), event("2"));
    foreign.project_id = ProjectId::new("project-extra").unwrap();
    storage.append_session_event(foreign).await.unwrap();
    let limit = StoragePageLimit::new(1).unwrap();
    let page = |cursor: Option<SessionEventCursor>| {
        let mut request = SessionEventPageRequest::new(project(), session("b"), limit);
        if let Some(cursor) = cursor {
            request = request.after(cursor);
        }
        request
    };
    let first = storage.list_session_events_page(page(None)).await.unwrap();
    assert_eq!(
        (
            first.items[0].turn_id.as_str(),
            first.items[0].event_id.as_str()
        ),
        ("1", "1")
    );
    let second = storage
        .list_session_events_page(page(first.next_cursor))
        .await
        .unwrap();
    assert_eq!(
        (
            second.items[0].turn_id.as_str(),
            second.items[0].event_id.as_str()
        ),
        ("1", "3")
    );
    let third = storage
        .list_session_events_page(page(second.next_cursor))
        .await
        .unwrap();
    assert_eq!(
        (
            third.items[0].turn_id.as_str(),
            third.items[0].event_id.as_str()
        ),
        ("2", "1")
    );
    assert!(third.next_cursor.is_none());
    let gap = session_event(session("b"), agent("one"), turn("1"), event("2"));
    let from_gap = storage
        .list_session_events_page(page(Some(SessionEventCursor::from_record(&gap))))
        .await
        .unwrap();
    assert_eq!(from_gap.items[0].event_id.as_str(), "3");
    let after_end = session_event(session("b"), agent("one"), turn("9"), event("9"));
    let empty = storage
        .list_session_events_page(page(Some(SessionEventCursor::from_record(&after_end))))
        .await
        .unwrap();
    assert!(empty.items.is_empty());
    assert!(empty.next_cursor.is_none());
}
fn session(value: &str) -> SessionId {
    SessionId::new(value).unwrap()
}

#[tokio::test]
async fn memory_turn_page_bounds_raw_bodies_and_preserves_the_cursor() {
    verify_turn_page_budget(&InMemoryAgentStorage::new()).await;
}

#[cfg(feature = "turso")]
#[tokio::test]
async fn turso_turn_page_bounds_raw_bodies_and_preserves_the_cursor() {
    use marlin_agent_storage::{
        TursoAgentStorage, TursoAgentStorageConfig, TursoBatchTransactionMode,
        TursoOptimizationProfile,
    };

    let root = tempfile::tempdir().unwrap();
    let storage = TursoAgentStorage::open_local(TursoAgentStorageConfig {
        path: root.path().join("turn-pages.turso"),
        optimization_profile: TursoOptimizationProfile::AsyncIoOnlyCompatibility,
        batch_transaction_mode: TursoBatchTransactionMode::Immediate,
    })
    .await
    .unwrap();
    verify_turn_page_budget(&storage).await;
}

async fn verify_turn_page_budget(storage: &impl SessionEventStore) {
    for (turn_id, event_id, bytes) in [
        ("1", "1", 4),
        ("1", "2", 4),
        ("1", "3", 2),
        ("1", "4", 1024 * 1024),
        ("1", "5", 1),
        ("2", "1", 4),
    ] {
        let mut record = session_event(session("a"), agent("one"), turn(turn_id), event(event_id));
        record.body = vec![b'x'; bytes];
        storage.append_session_event(record).await.unwrap();
    }
    let budget = NonZeroU32::new(6).unwrap();
    let request = |after, count| {
        SessionEventPageRequest::new(
            project(),
            session("a"),
            StoragePageLimit::new(count).unwrap(),
        )
        .within_turn_after(SessionEventCursor::new(turn("1"), event(after)), budget)
    };
    let first = storage
        .list_session_events_page(request("0", 16))
        .await
        .unwrap();
    assert_eq!(first.items.len(), 1);
    assert_eq!(first.next_cursor.unwrap().event_id(), &event("1"));
    let second = storage
        .list_session_events_page(request("1", 16))
        .await
        .unwrap();
    assert_eq!(
        second.items.iter().map(|row| row.body.len()).sum::<usize>(),
        6
    );
    assert_eq!(second.next_cursor.unwrap().event_id(), &event("3"));
    let count_limited = storage
        .list_session_events_page(request("1", 1))
        .await
        .unwrap();
    assert_eq!(count_limited.items.len(), 1);
    assert_eq!(count_limited.next_cursor.unwrap().event_id(), &event("2"));
    assert!(matches!(
        storage.list_session_events_page(request("3", 16)).await,
        Err(StorageError::SessionEventBodyExceedsPageBudget { budget: 6 })
    ));
    let last = storage
        .list_session_events_page(request("4", 16))
        .await
        .unwrap();
    assert_eq!(last.items[0].event_id, event("5"));
    assert!(last.next_cursor.is_none());
    let end = storage
        .list_session_events_page(request("5", 16))
        .await
        .unwrap();
    assert!(end.items.is_empty() && end.next_cursor.is_none());

    let session_request = || {
        SessionEventPageRequest::new(project(), session("a"), StoragePageLimit::new(16).unwrap())
            .with_body_budget(budget)
    };
    let session_first = storage
        .list_session_events_page(session_request())
        .await
        .unwrap();
    assert_eq!(session_first.items.len(), 1);
    assert_eq!(session_first.next_cursor.unwrap().event_id(), &event("1"));
    let across_turns = storage
        .list_session_events_page(
            session_request().after(SessionEventCursor::new(turn("1"), event("4"))),
        )
        .await
        .unwrap();
    assert_eq!(
        across_turns
            .items
            .iter()
            .map(|row| (row.turn_id.as_str(), row.event_id.as_str(), row.body.len()))
            .collect::<Vec<_>>(),
        [("1", "5", 1), ("2", "1", 4)]
    );
    assert!(across_turns.next_cursor.is_none());
}
fn agent(value: &str) -> AgentId {
    AgentId::new(value).unwrap()
}
fn turn(value: &str) -> TurnId {
    TurnId::new(value).unwrap()
}
fn event(value: &str) -> EventId {
    EventId::new(value).unwrap()
}
fn session_event(
    session_id: SessionId,
    agent_id: AgentId,
    turn_id: TurnId,
    event_id: EventId,
) -> SessionEventRecord {
    SessionEventRecord {
        project_id: project(),
        session_id,
        agent_id,
        turn_id,
        event_id,
        event_kind: "tool.output".into(),
        causality_parent_event_id: None,
        body: b"event".to_vec(),
        created_at_unix_ms: 1,
    }
}

#[tokio::test]
async fn sessions_are_project_agent_scoped_unique_and_paginated() {
    let storage = InMemoryAgentStorage::new();
    for name in ["c", "a", "b"] {
        storage
            .append_session_event(session_event(
                session(name),
                agent("one"),
                turn("1"),
                event("1"),
            ))
            .await
            .unwrap();
        storage
            .append_session_event(session_event(
                session(name),
                agent("one"),
                turn("1"),
                event("2"),
            ))
            .await
            .unwrap();
    }
    storage
        .append_session_event(session_event(
            session("other-agent"),
            agent("two"),
            turn("1"),
            event("1"),
        ))
        .await
        .unwrap();
    let mut other = session_event(
        session("other-project"),
        agent("one"),
        turn("1"),
        event("1"),
    );
    other.project_id = ProjectId::new("other").unwrap();
    storage.append_session_event(other).await.unwrap();
    let limit = StoragePageLimit::new(2).unwrap();
    let first = storage
        .list_sessions_page(SessionPageRequest::new(project(), agent("one"), limit))
        .await
        .unwrap();
    assert_eq!(first.items, vec![session("a"), session("b")]);
    assert_eq!(first.next_cursor, Some(session("b")));
    let second = storage
        .list_sessions_page(
            SessionPageRequest::new(project(), agent("one"), limit)
                .after(first.next_cursor.unwrap()),
        )
        .await
        .unwrap();
    assert_eq!(second.items, vec![session("c")]);
    assert_eq!(second.next_cursor, None);
    let absent = storage
        .list_sessions_page(SessionPageRequest::new(project(), agent("none"), limit))
        .await
        .unwrap();
    assert!(absent.items.is_empty());
    let first_batch = session_event(session("d"), agent("one"), turn("1"), event("1"));
    storage
        .append_session_events_atomically(vec![first_batch.clone()])
        .await
        .unwrap();
    let failed_batch = vec![
        first_batch,
        session_event(
            session("must-not-appear"),
            agent("one"),
            turn("1"),
            event("1"),
        ),
    ];
    assert!(
        storage
            .append_session_events_atomically(failed_batch)
            .await
            .is_err()
    );
    let after_c = storage
        .list_sessions_page(
            SessionPageRequest::new(project(), agent("one"), limit).after(session("c")),
        )
        .await
        .unwrap();
    assert_eq!(after_c.items, vec![session("d")]);
}
