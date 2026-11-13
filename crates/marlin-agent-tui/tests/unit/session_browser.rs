// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use marlin_agent_storage::SessionEventStore;
use std::time::{Duration, Instant};

use super::replay::{ReplayError, ReplayLimits, ReplayOutcome};
use super::session_browser::{
    SessionBrowser, SessionOpenError, SessionPageRequestError, validate_page,
};
use marlin_agent_protocol::{
    ModelEndpoint, ModelGatewayCompletionChoice, ModelGatewayCompletionResponse,
    assistant_gateway_message,
};
use marlin_agent_sessions::{ConversationJournal, ConversationJournalError, ConversationTurnStart};
use marlin_agent_storage::{
    AgentId, EventId, InMemoryAgentStorage, ProjectId, SessionEventRecord, SessionId, StoragePage,
    TurnId,
};

fn sid(value: &str) -> SessionId {
    SessionId::new(value).unwrap()
}

#[test]
fn session_browser_requests_one_page_at_a_time_and_advances() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let storage = InMemoryAgentStorage::new();
    let project = ProjectId::new("project:test").unwrap();
    let agent = AgentId::new("agent:test").unwrap();
    runtime.block_on(async {
        for index in 0..21 {
            storage
                .append_session_event(SessionEventRecord {
                    project_id: project.clone(),
                    agent_id: agent.clone(),
                    session_id: sid(&format!("s{index:02}")),
                    turn_id: TurnId::new("turn").unwrap(),
                    event_id: EventId::new("event").unwrap(),
                    event_kind: "test".into(),
                    causality_parent_event_id: None,
                    body: Vec::new(),
                    created_at_unix_ms: 0,
                })
                .await
                .unwrap();
        }
    });
    let mut browser = SessionBrowser::new(storage, project, agent);
    assert_eq!(
        browser.request(&runtime, true),
        Err(SessionPageRequestError::NoNextPage)
    );
    browser.request(&runtime, false).unwrap();
    assert_eq!(
        browser.request(&runtime, false),
        Err(SessionPageRequestError::Busy)
    );
    let wait = |browser: &mut SessionBrowser<InMemoryAgentStorage>| {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(result) = browser.poll() {
                break result.unwrap();
            }
            assert!(Instant::now() < deadline, "session page task timed out");
            std::thread::yield_now();
        }
    };
    let first = wait(&mut browser);
    assert_eq!(first.items.len(), 20);
    assert_eq!(first.items[0], sid("s00"));
    assert_eq!(first.next_cursor, Some(sid("s19")));
    browser.request(&runtime, true).unwrap();
    let last = wait(&mut browser);
    assert_eq!(last.items, vec![sid("s20")]);
    assert_eq!(
        browser.request(&runtime, true),
        Err(SessionPageRequestError::NoNextPage)
    );
}

#[test]
fn invalid_session_pages_fail_closed() {
    let invalid = [
        StoragePage {
            items: vec![sid("b"), sid("a")],
            next_cursor: None,
        },
        StoragePage {
            items: vec![sid("a"), sid("a")],
            next_cursor: None,
        },
        StoragePage {
            items: vec![sid("a")],
            next_cursor: Some(sid("b")),
        },
        StoragePage {
            items: Vec::new(),
            next_cursor: Some(sid("a")),
        },
        StoragePage {
            items: (0..21).map(|n| sid(&format!("s{n:02}"))).collect(),
            next_cursor: None,
        },
    ];
    for page in invalid {
        assert!(validate_page(page, None).is_err());
    }
    let replay = StoragePage {
        items: vec![sid("a")],
        next_cursor: None,
    };
    assert!(validate_page(replay, Some(&sid("a"))).is_err());
}

#[test]
fn opening_unfinished_or_invalid_sessions_preserves_ownership_boundaries() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let storage = InMemoryAgentStorage::new();
    let project = ProjectId::new("project:test").unwrap();
    let agent = AgentId::new("agent:test").unwrap();
    let start = || ConversationTurnStart {
        endpoint: ModelEndpoint::new("provider", "model"),
        user_text: "prompt".into(),
    };
    let mut current = runtime
        .block_on(ConversationJournal::open(
            storage.clone(),
            project.clone(),
            sid("current"),
            agent.clone(),
            |_| {},
        ))
        .unwrap();
    runtime.block_on(async {
        let mut unfinished = ConversationJournal::open(
            storage.clone(),
            project.clone(),
            sid("unfinished"),
            agent.clone(),
            |_| {},
        )
        .await
        .unwrap();
        let id = unfinished.start(start(), 1).await.unwrap();
        unfinished.checkpoint(id, "partial", 2).await.unwrap();
        let mut foreign = ConversationJournal::open(
            storage.clone(),
            project.clone(),
            sid("foreign"),
            AgentId::new("agent:other").unwrap(),
            |_| {},
        )
        .await
        .unwrap();
        foreign.start(start(), 1).await.unwrap();
    });
    let mut browser = SessionBrowser::new(storage, project, agent);
    let wait = |browser: &mut SessionBrowser<InMemoryAgentStorage>| {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(result) = browser.poll_open() {
                break result;
            }
            assert!(Instant::now() < deadline, "session open timed out");
            std::thread::yield_now();
        }
    };
    browser.open(&runtime, sid("unfinished")).unwrap();
    assert!(browser.cancel_open());
    assert!(!browser.is_opening());
    assert!(browser.poll_open().is_none());
    assert!(!browser.cancel_open());
    browser.open(&runtime, sid("foreign")).unwrap();
    assert_eq!(
        browser.open(&runtime, sid("unfinished")),
        Err(SessionPageRequestError::Busy)
    );
    assert!(wait(&mut browser).is_err());
    let id = runtime.block_on(current.start(start(), 3)).unwrap();
    assert_eq!(id.get(), 1, "failed switch must not alter current journal");
    browser.open(&runtime, sid("unfinished")).unwrap();
    let mut opened = wait(&mut browser).unwrap();
    assert_eq!(opened.id, sid("unfinished"));
    assert!(
        matches!(&opened.view, super::session_browser::OpenedView::Full(turns)
        if matches!(&turns[0].outcome, ReplayOutcome::Unfinished { partial } if partial == "partial"))
    );
    assert!(matches!(
        runtime.block_on(opened.journal.start(start(), 4)),
        Err(ConversationJournalError::UnfinishedTurn)
    ));
}

#[test]
fn replay_budget_rejection_does_not_replace_current_session() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let storage = InMemoryAgentStorage::new();
    let project = ProjectId::new("project:test").unwrap();
    let agent = AgentId::new("agent:test").unwrap();
    let start = || ConversationTurnStart {
        endpoint: ModelEndpoint::new("provider", "model"),
        user_text: "prompt".into(),
    };
    let mut current = runtime
        .block_on(ConversationJournal::open(
            storage.clone(),
            project.clone(),
            sid("current"),
            agent.clone(),
            |_| {},
        ))
        .unwrap();
    runtime.block_on(async {
        let mut target = ConversationJournal::open(
            storage.clone(),
            project.clone(),
            sid("target"),
            agent.clone(),
            |_| {},
        )
        .await
        .unwrap();
        for time in 0..2 {
            let id = target.start(start(), time).await.unwrap();
            let response = ModelGatewayCompletionResponse::new(
                "id",
                "model",
                vec![ModelGatewayCompletionChoice::new(
                    0,
                    assistant_gateway_message("answer"),
                    None,
                )],
            );
            target
                .finish(
                    id,
                    marlin_agent_sessions::ConversationTerminalObservation::Completed { response },
                    time,
                )
                .await
                .unwrap();
        }
    });
    let mut browser =
        SessionBrowser::new(storage, project, agent).with_replay_limits(ReplayLimits {
            turns: 1,
            bytes: 4096,
        });
    browser.open(&runtime, sid("target")).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    let result = loop {
        if let Some(result) = browser.poll_open() {
            break result;
        }
        assert!(Instant::now() < deadline, "session open timed out");
        std::thread::yield_now();
    };
    assert!(matches!(
        result,
        Err(SessionOpenError::Replay(ReplayError::ProjectionLimit {
            turns: 1,
            ..
        }))
    ));
    assert_eq!(
        runtime.block_on(current.start(start(), 10)).unwrap().get(),
        1
    );
}
