// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use marlin_agent_protocol::{
    ModelEndpoint, ModelGatewayCompletionChoice, ModelGatewayCompletionResponse,
    assistant_gateway_message,
};
use marlin_agent_sessions::{
    ConversationJournal, ConversationJournalError, ConversationTerminalObservation,
    ConversationTurnOutcome, ConversationTurnStart, ConversationUncertaintyReason,
};
use marlin_agent_storage::SessionEventStore;
use marlin_agent_storage::{
    AgentId, EventId, InMemoryAgentStorage, ProjectId, SessionEventRecord, SessionId, StorageError,
    TurnId,
};

fn ids() -> (ProjectId, SessionId, AgentId) {
    (
        ProjectId::new("project:fixture").unwrap(),
        SessionId::new("session:fixture").unwrap(),
        AgentId::new("agent:fixture").unwrap(),
    )
}

fn start(text: &str) -> ConversationTurnStart {
    ConversationTurnStart {
        endpoint: ModelEndpoint::new("openai", "gpt-fixture"),
        user_text: text.into(),
    }
}

fn completion(content: &str) -> ConversationTerminalObservation {
    ConversationTerminalObservation::Completed {
        response: ModelGatewayCompletionResponse::new(
            "response-fixture",
            "gpt-fixture",
            vec![ModelGatewayCompletionChoice::new(
                0,
                assistant_gateway_message(content),
                Some("stop".into()),
            )],
        ),
    }
}

fn recorded_completion(content: &str) -> ConversationTurnOutcome {
    let ConversationTerminalObservation::Completed { response } = completion(content) else {
        unreachable!("fixture always creates a completion")
    };
    ConversationTurnOutcome::Completed { response }
}

#[tokio::test]
async fn alternating_checkpoint_runs_are_bounded_before_write() {
    let storage = InMemoryAgentStorage::new();
    let (project, session, agent) = ids();
    let mut journal = ConversationJournal::open(
        storage.clone(),
        project.clone(),
        session.clone(),
        agent,
        |_| {},
    )
    .await
    .unwrap();
    let id = journal.start(start("prompt"), 1).await.unwrap();
    for index in 0..4096 {
        if index % 2 == 0 {
            journal.checkpoint(id, "a", 2).await.unwrap();
        } else {
            journal.checkpoint_thinking(id, "b", 2).await.unwrap();
        }
    }
    assert!(matches!(
        journal.checkpoint(id, "c", 3).await,
        Err(ConversationJournalError::InvalidEvent(_))
    ));
    assert_eq!(
        storage
            .count_session_events(&project, &session)
            .await
            .unwrap()
            .get(),
        4097
    );
    // A further fragment in the existing run does not allocate a new span.
    journal.checkpoint_thinking(id, "d", 4).await.unwrap();
    assert_eq!(
        storage
            .count_session_events(&project, &session)
            .await
            .unwrap()
            .get(),
        4098
    );
}

#[tokio::test]
async fn replay_rejects_pre_dispatch_rejection_after_thinking() {
    let storage = InMemoryAgentStorage::new();
    let (project, session, agent) = ids();
    let mut journal = ConversationJournal::open(
        storage.clone(),
        project.clone(),
        session.clone(),
        agent.clone(),
        |_| {},
    )
    .await
    .unwrap();
    let id = journal.start(start("prompt"), 1).await.unwrap();
    journal
        .checkpoint_thinking(id, "observed", 2)
        .await
        .unwrap();
    let outcome = ConversationTurnOutcome::RejectedBeforeDispatch {
        diagnostic: "not sent".into(),
    };
    storage
        .append_session_event(SessionEventRecord {
            project_id: project.clone(),
            session_id: session.clone(),
            agent_id: agent.clone(),
            turn_id: TurnId::new("conversation:00000000000000000001").unwrap(),
            event_id: EventId::new("conversation:00000000000000000001:01-finish").unwrap(),
            event_kind: "marlin.conversation.turn.finished.v1".into(),
            causality_parent_event_id: Some(
                EventId::new("conversation:00000000000000000001:00-start").unwrap(),
            ),
            body: serde_json::to_vec(&outcome).unwrap(),
            created_at_unix_ms: 3,
        })
        .await
        .unwrap();
    assert!(matches!(
        ConversationJournal::open(storage, project, session, agent, |_| {}).await,
        Err(ConversationJournalError::InvalidEvent(_))
    ));
}

#[tokio::test]
async fn thinking_and_answer_checkpoints_replay_in_recorded_order() {
    use marlin_agent_storage::{SessionEventPageRequest, StoragePageLimit};
    let storage = InMemoryAgentStorage::new();
    let (project, session, agent) = ids();
    let mut journal = ConversationJournal::open(
        storage.clone(),
        project.clone(),
        session.clone(),
        agent.clone(),
        |_| {},
    )
    .await
    .unwrap();
    let id = journal.start(start("prompt"), 1).await.unwrap();
    journal.checkpoint_thinking(id, "plan ", 2).await.unwrap();
    journal.checkpoint(id, "answer", 3).await.unwrap();
    journal.checkpoint_thinking(id, "next", 4).await.unwrap();
    assert!(matches!(
        journal
            .finish(
                id,
                ConversationTerminalObservation::RejectedBeforeDispatch {
                    diagnostic: "not sent".into(),
                },
                5
            )
            .await,
        Err(ConversationJournalError::InvalidEvent(_))
    ));
    assert!(matches!(
        journal
            .mark_uncertain(id, ConversationUncertaintyReason::StreamOpenTimeout, 5)
            .await,
        Err(ConversationJournalError::InvalidEvent(_))
    ));

    let mut before = Vec::new();
    let reopened = ConversationJournal::open(
        storage.clone(),
        project.clone(),
        session.clone(),
        agent.clone(),
        |turn| before.push(turn.clone()),
    )
    .await
    .unwrap();
    assert!(reopened.unfinished().is_some());
    assert_eq!(before[0].partial_thinking, "plan next");
    assert!(!format!("{:?}", before[0]).contains("plan next"));
    assert_eq!(before[0].partial_text, "answer");
    use marlin_agent_sessions::ConversationCheckpointChannel::{Text, Thinking};
    assert_eq!(
        before[0]
            .checkpoint_spans
            .iter()
            .map(|span| (span.channel, span.start, span.end))
            .collect::<Vec<_>>(),
        [(Thinking, 0, 5), (Text, 0, 6), (Thinking, 5, 9)]
    );
    journal
        .finish(id, completion("answer and more"), 6)
        .await
        .unwrap();
    let mut after = Vec::new();
    ConversationJournal::open(
        storage.clone(),
        project.clone(),
        session.clone(),
        agent,
        |turn| after.push(turn.clone()),
    )
    .await
    .unwrap();
    assert_eq!(after[0].partial_thinking, "plan next");
    let events = storage
        .list_session_events_page(SessionEventPageRequest::new(
            project,
            session,
            StoragePageLimit::new(10).unwrap(),
        ))
        .await
        .unwrap();
    let kinds: Vec<_> = events
        .items
        .iter()
        .map(|event| event.event_kind.as_str())
        .collect();
    assert_eq!(
        kinds,
        [
            "marlin.conversation.turn.started.v1",
            "marlin.conversation.turn.thinking-checkpoint.v1",
            "marlin.conversation.turn.checkpoint.v1",
            "marlin.conversation.turn.thinking-checkpoint.v1",
            "marlin.conversation.turn.finished.v1"
        ]
    );
}

#[tokio::test]
async fn concurrent_append_during_paged_replay_is_rejected_for_that_session() {
    use std::{sync::mpsc, time::Duration};
    for same_session in [true, false] {
        let storage = InMemoryAgentStorage::new();
        let (project, session, agent) = ids();
        let mut writer = ConversationJournal::open(
            storage.clone(),
            project.clone(),
            session.clone(),
            agent.clone(),
            |_| {},
        )
        .await
        .unwrap();
        for index in 0..10 {
            let id = writer
                .start(start(&format!("prompt {index}")), index)
                .await
                .unwrap();
            writer
                .finish(id, completion("answer"), index)
                .await
                .unwrap();
        }
        let (signal_tx, signal_rx) = mpsc::channel();
        let (done_tx, done_rx) = mpsc::channel();
        let writer_storage = storage.clone();
        let write_project = project.clone();
        let write_session = if same_session {
            session.clone()
        } else {
            SessionId::new("session:other").unwrap()
        };
        let write_agent = agent.clone();
        let worker = std::thread::spawn(move || {
            signal_rx.recv_timeout(Duration::from_secs(5)).unwrap();
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            runtime
                .block_on(writer_storage.append_session_event(SessionEventRecord {
                    project_id: write_project,
                    session_id: write_session,
                    agent_id: write_agent,
                    turn_id: TurnId::new("z-unrelated").unwrap(),
                    event_id: EventId::new("event").unwrap(),
                    event_kind: "unrelated.event".into(),
                    causality_parent_event_id: None,
                    body: Vec::new(),
                    created_at_unix_ms: 1,
                }))
                .unwrap();
            done_tx.send(()).unwrap();
        });
        let mut visited = 0;
        let result = ConversationJournal::open(storage, project, session, agent, |_| {
            if visited == 0 {
                signal_tx.send(()).unwrap();
                done_rx.recv_timeout(Duration::from_secs(5)).unwrap();
            }
            visited += 1;
        })
        .await;
        worker.join().unwrap();
        if same_session {
            assert!(matches!(
                result,
                Err(ConversationJournalError::ReplayInconsistent)
            ));
        } else {
            assert!(result.is_ok());
        }
    }
}

#[tokio::test]
async fn checkpoints_replay_in_order_and_terminal_must_include_them() {
    let storage = InMemoryAgentStorage::new();
    let (project, session, agent) = ids();
    let mut journal = ConversationJournal::open(
        storage.clone(),
        project.clone(),
        session.clone(),
        agent.clone(),
        |_| {},
    )
    .await
    .unwrap();
    let id = journal.start(start("prompt"), 1).await.unwrap();
    journal.checkpoint(id, "中", 2).await.unwrap();
    journal.checkpoint(id, "文", 3).await.unwrap();
    assert!(matches!(
        journal.finish(id, completion("different"), 4).await,
        Err(ConversationJournalError::InvalidEvent(_))
    ));
    let mut seen = Vec::new();
    let mut reopened = ConversationJournal::open(
        storage.clone(),
        project.clone(),
        session.clone(),
        agent.clone(),
        |turn| seen.push(turn.clone()),
    )
    .await
    .unwrap();
    assert_eq!(seen[0].partial_text, "中文");
    assert!(matches!(
        reopened.checkpoint(id, "forbidden", 5).await,
        Err(ConversationJournalError::UnownedTurn)
    ));
    journal
        .finish(id, completion("中文 reply"), 6)
        .await
        .unwrap();
    let mut finished = Vec::new();
    ConversationJournal::open(storage, project, session, agent, |turn| {
        finished.push(turn.clone())
    })
    .await
    .unwrap();
    assert_eq!(finished[0].partial_text, "中文");
    assert!(matches!(
        finished[0].outcome,
        Some(ConversationTurnOutcome::Completed { .. })
    ));
}

#[tokio::test]
async fn replay_rejects_a_checkpoint_sequence_gap() {
    let storage = InMemoryAgentStorage::new();
    let (project, session, agent) = ids();
    let mut journal = ConversationJournal::open(
        storage.clone(),
        project.clone(),
        session.clone(),
        agent.clone(),
        |_| {},
    )
    .await
    .unwrap();
    journal.start(start("prompt"), 1).await.unwrap();
    storage
        .append_session_event(SessionEventRecord {
            project_id: project.clone(),
            session_id: session.clone(),
            agent_id: agent.clone(),
            turn_id: TurnId::new("conversation:00000000000000000001").unwrap(),
            event_id: EventId::new(
                "conversation:00000000000000000001:00-start:chunk:00000000000000000002",
            )
            .unwrap(),
            event_kind: "marlin.conversation.turn.checkpoint.v1".into(),
            causality_parent_event_id: Some(
                EventId::new("conversation:00000000000000000001:00-start").unwrap(),
            ),
            body: serde_json::to_vec("orphan").unwrap(),
            created_at_unix_ms: 2,
        })
        .await
        .unwrap();
    assert!(matches!(
        ConversationJournal::open(storage, project, session, agent, |_| {}).await,
        Err(ConversationJournalError::InvalidEvent(_))
    ));
}

#[tokio::test]
async fn replay_crosses_pages_and_preserves_unfinished_and_failed_turns() {
    let storage = InMemoryAgentStorage::new();
    let (project, session, agent) = ids();
    // An unrelated record shifts the page boundary between a start and finish.
    storage
        .append_session_event(SessionEventRecord {
            project_id: project.clone(),
            session_id: session.clone(),
            agent_id: agent.clone(),
            turn_id: TurnId::new("0-unrelated").unwrap(),
            event_id: EventId::new("other").unwrap(),
            event_kind: "other.event".into(),
            causality_parent_event_id: None,
            body: Vec::new(),
            created_at_unix_ms: 0,
        })
        .await
        .unwrap();
    let mut journal = ConversationJournal::open(
        storage.clone(),
        project.clone(),
        session.clone(),
        agent.clone(),
        |_| {},
    )
    .await
    .unwrap();
    for index in 0..10 {
        let id = journal
            .start(start(&format!("prompt {index}")), index)
            .await
            .unwrap();
        assert_eq!(id.get(), index as u64 + 1);
        journal
            .finish(id, completion(&format!("answer {index}")), index)
            .await
            .unwrap();
    }
    let failed = journal.start(start("failure"), 20).await.unwrap();
    journal
        .finish(
            failed,
            ConversationTerminalObservation::RejectedBeforeDispatch {
                diagnostic: "offline".into(),
            },
            21,
        )
        .await
        .unwrap();
    let unfinished = journal.start(start("in flight"), 22).await.unwrap();
    drop(journal);
    let mut replay = Vec::new();
    let mut reopened = ConversationJournal::open(storage, project, session, agent, |turn| {
        replay.push(turn.clone())
    })
    .await
    .unwrap();
    assert_eq!(replay.len(), 12);
    assert_eq!(replay[9].id.get(), 10);
    assert!(
        matches!(&replay[10].outcome, Some(ConversationTurnOutcome::RejectedBeforeDispatch { diagnostic }) if diagnostic == "offline")
    );
    assert_eq!(replay[11].id, unfinished);
    assert!(replay[11].outcome.is_none());
    assert!(matches!(
        reopened.start(start("must not dispatch"), 23).await,
        Err(ConversationJournalError::UnfinishedTurn)
    ));
    assert!(matches!(
        reopened
            .finish(
                unfinished,
                ConversationTerminalObservation::RejectedBeforeDispatch {
                    diagnostic: "not dispatched".into()
                },
                24
            )
            .await,
        Err(ConversationJournalError::UnownedTurn)
    ));
}

#[tokio::test]
async fn stale_writer_cannot_claim_same_turn_before_dispatch() {
    let storage = InMemoryAgentStorage::new();
    let (project, session, agent) = ids();
    let mut first = ConversationJournal::open(
        storage.clone(),
        project.clone(),
        session.clone(),
        agent.clone(),
        |_| {},
    )
    .await
    .unwrap();
    let mut stale = ConversationJournal::open(storage, project, session, agent, |_| {})
        .await
        .unwrap();
    first.start(start("first"), 1).await.unwrap();
    assert!(matches!(
        stale.start(start("second"), 2).await,
        Err(ConversationJournalError::Storage(
            StorageError::DuplicateSessionEvent { .. }
        ))
    ));
    assert!(stale.unfinished().is_none());
}

#[tokio::test]
async fn simultaneous_writers_admit_exactly_one_turn() {
    let storage = InMemoryAgentStorage::new();
    let (project, session, agent) = ids();
    let mut first = ConversationJournal::open(
        storage.clone(),
        project.clone(),
        session.clone(),
        agent.clone(),
        |_| {},
    )
    .await
    .unwrap();
    let mut second = ConversationJournal::open(storage, project, session, agent, |_| {})
        .await
        .unwrap();
    let (left, right) = tokio::join!(
        first.start(start("left"), 1),
        second.start(start("right"), 1)
    );
    assert_ne!(left.is_ok(), right.is_ok());
    let failed = if left.is_err() { left } else { right };
    assert!(matches!(
        failed,
        Err(ConversationJournalError::Storage(
            StorageError::DuplicateSessionEvent { .. }
        ))
    ));
}

#[tokio::test]
async fn invalid_completion_does_not_consume_the_active_turn() {
    let storage = InMemoryAgentStorage::new();
    let (project, session, agent) = ids();
    let mut journal = ConversationJournal::open(storage, project, session, agent, |_| {})
        .await
        .unwrap();
    let id = journal.start(start("hello"), 1).await.unwrap();
    let invalid = ConversationTerminalObservation::Completed {
        response: ModelGatewayCompletionResponse::new("id", "model", vec![]),
    };
    assert!(matches!(
        journal.finish(id, invalid, 2).await,
        Err(ConversationJournalError::InvalidEvent(_))
    ));
    assert_eq!(journal.unfinished().unwrap().id, id);
    journal.finish(id, completion("ok"), 3).await.unwrap();
    assert!(journal.unfinished().is_none());
}

#[tokio::test]
async fn terminal_storage_conflict_does_not_clear_active_turn() {
    let storage = InMemoryAgentStorage::new();
    let (project, session, agent) = ids();
    let mut journal = ConversationJournal::open(
        storage.clone(),
        project.clone(),
        session.clone(),
        agent.clone(),
        |_| {},
    )
    .await
    .unwrap();
    let id = journal.start(start("hello"), 1).await.unwrap();
    storage
        .append_session_event(SessionEventRecord {
            project_id: project,
            session_id: session,
            agent_id: agent,
            turn_id: TurnId::new("conversation:00000000000000000001").unwrap(),
            event_id: EventId::new("conversation:00000000000000000001:01-finish").unwrap(),
            event_kind: "marlin.conversation.turn.finished.v1".into(),
            causality_parent_event_id: Some(
                EventId::new("conversation:00000000000000000001:00-start").unwrap(),
            ),
            body: serde_json::to_vec(&recorded_completion("other writer")).unwrap(),
            created_at_unix_ms: 2,
        })
        .await
        .unwrap();
    assert!(matches!(
        journal.finish(id, completion("local result"), 3).await,
        Err(ConversationJournalError::Storage(
            StorageError::DuplicateSessionEvent { .. }
        ))
    ));
    assert_eq!(journal.unfinished().unwrap().id, id);
    assert!(matches!(
        journal.finish(id, completion("retry"), 4).await,
        Err(ConversationJournalError::UnownedTurn)
    ));
}

#[tokio::test]
async fn legacy_failed_and_cancelled_records_remain_readable() {
    for outcome in [
        ConversationTurnOutcome::Failed {
            diagnostic: "legacy".into(),
            partial_text: "partial".into(),
        },
        ConversationTurnOutcome::CancelledLocally {
            partial_text: "partial".into(),
        },
    ] {
        let storage = InMemoryAgentStorage::new();
        let (project, session, agent) = ids();
        let mut journal = ConversationJournal::open(
            storage.clone(),
            project.clone(),
            session.clone(),
            agent.clone(),
            |_| {},
        )
        .await
        .unwrap();
        journal.start(start("legacy"), 1).await.unwrap();
        storage
            .append_session_event(SessionEventRecord {
                project_id: project.clone(),
                session_id: session.clone(),
                agent_id: agent.clone(),
                turn_id: TurnId::new("conversation:00000000000000000001").unwrap(),
                event_id: EventId::new("conversation:00000000000000000001:01-finish").unwrap(),
                event_kind: "marlin.conversation.turn.finished.v1".into(),
                causality_parent_event_id: Some(
                    EventId::new("conversation:00000000000000000001:00-start").unwrap(),
                ),
                body: serde_json::to_vec(&outcome).unwrap(),
                created_at_unix_ms: 2,
            })
            .await
            .unwrap();
        for (event_suffix, kind, parent, body) in [
            (
                "00-start",
                "marlin.conversation.turn.started.v1",
                None,
                serde_json::to_vec(&start("later turn")).unwrap(),
            ),
            (
                "01-finish",
                "marlin.conversation.turn.finished.v1",
                Some(EventId::new("conversation:00000000000000000002:00-start").unwrap()),
                serde_json::to_vec(&recorded_completion("later answer")).unwrap(),
            ),
        ] {
            storage
                .append_session_event(SessionEventRecord {
                    project_id: project.clone(),
                    session_id: session.clone(),
                    agent_id: agent.clone(),
                    turn_id: TurnId::new("conversation:00000000000000000002").unwrap(),
                    event_id: EventId::new(format!(
                        "conversation:00000000000000000002:{event_suffix}"
                    ))
                    .unwrap(),
                    event_kind: kind.into(),
                    causality_parent_event_id: parent,
                    body,
                    created_at_unix_ms: 3,
                })
                .await
                .unwrap();
        }
        drop(journal);
        let mut turns = Vec::new();
        let mut reopened = ConversationJournal::open(storage, project, session, agent, |turn| {
            turns.push(turn.clone())
        })
        .await
        .unwrap();
        assert_eq!(turns[0].outcome.as_ref(), Some(&outcome));
        assert!(matches!(
            turns[1].outcome,
            Some(ConversationTurnOutcome::Completed { .. })
        ));
        assert!(reopened.unfinished().is_none());
        assert!(matches!(
            reopened.start(start("new request"), 3).await,
            Err(ConversationJournalError::LegacyOutcomeUnresolved)
        ));
    }
}

#[tokio::test]
async fn uncertain_write_conflict_relinquishes_authority_and_replays_typed_reason() {
    let storage = InMemoryAgentStorage::new();
    let (project, session, agent) = ids();
    let mut journal = ConversationJournal::open(
        storage.clone(),
        project.clone(),
        session.clone(),
        agent.clone(),
        |_| {},
    )
    .await
    .unwrap();
    let id = journal.start(start("hello"), 1).await.unwrap();
    storage
        .append_session_event(SessionEventRecord {
            project_id: project.clone(),
            session_id: session.clone(),
            agent_id: agent.clone(),
            turn_id: TurnId::new("conversation:00000000000000000001").unwrap(),
            event_id: EventId::new("conversation:00000000000000000001:01-uncertain").unwrap(),
            event_kind: "marlin.conversation.turn.uncertain.v1".into(),
            causality_parent_event_id: Some(
                EventId::new("conversation:00000000000000000001:00-start").unwrap(),
            ),
            body: serde_json::to_vec(&ConversationUncertaintyReason::StreamOpenTimeout).unwrap(),
            created_at_unix_ms: 2,
        })
        .await
        .unwrap();
    assert!(matches!(
        journal
            .mark_uncertain(id, ConversationUncertaintyReason::StreamOpenTimeout, 3)
            .await,
        Err(ConversationJournalError::Storage(
            StorageError::DuplicateSessionEvent { .. }
        ))
    ));
    assert!(matches!(
        journal.finish(id, completion("should not commit"), 4).await,
        Err(ConversationJournalError::UnownedTurn)
    ));
    assert!(matches!(
        journal.start(start("next"), 5).await,
        Err(ConversationJournalError::UnfinishedTurn)
    ));
    let mut turns = Vec::new();
    let reopened = ConversationJournal::open(storage, project, session, agent, |turn| {
        turns.push(turn.clone())
    })
    .await
    .unwrap();
    assert_eq!(turns.len(), 1);
    assert!(matches!(
        turns[0].outcome,
        Some(ConversationTurnOutcome::Uncertain {
            reason: ConversationUncertaintyReason::StreamOpenTimeout
        })
    ));
    assert!(reopened.unfinished().is_some());
}

#[tokio::test]
async fn checkpoint_write_conflict_relinquishes_authority() {
    let storage = InMemoryAgentStorage::new();
    let (project, session, agent) = ids();
    let mut journal = ConversationJournal::open(
        storage.clone(),
        project.clone(),
        session.clone(),
        agent.clone(),
        |_| {},
    )
    .await
    .unwrap();
    let id = journal.start(start("hello"), 1).await.unwrap();
    storage
        .append_session_event(SessionEventRecord {
            project_id: project.clone(),
            session_id: session.clone(),
            agent_id: agent.clone(),
            turn_id: TurnId::new("conversation:00000000000000000001").unwrap(),
            event_id: EventId::new(
                "conversation:00000000000000000001:00-start:chunk:00000000000000000001",
            )
            .unwrap(),
            event_kind: "marlin.conversation.turn.checkpoint.v1".into(),
            causality_parent_event_id: Some(
                EventId::new("conversation:00000000000000000001:00-start").unwrap(),
            ),
            body: serde_json::to_vec("foreign").unwrap(),
            created_at_unix_ms: 2,
        })
        .await
        .unwrap();
    assert!(matches!(
        journal.checkpoint(id, "local", 3).await,
        Err(ConversationJournalError::Storage(
            StorageError::DuplicateSessionEvent { .. }
        ))
    ));
    assert!(matches!(
        journal.finish(id, completion("local"), 4).await,
        Err(ConversationJournalError::UnownedTurn)
    ));
    let reopened = ConversationJournal::open(storage, project, session, agent, |_| {})
        .await
        .unwrap();
    assert_eq!(reopened.unfinished().unwrap().partial_text, "foreign");
}

#[tokio::test]
async fn stream_open_uncertainty_and_pre_dispatch_rejection_cannot_follow_a_checkpoint() {
    let storage = InMemoryAgentStorage::new();
    let (project, session, agent) = ids();
    let mut journal = ConversationJournal::open(storage, project, session, agent, |_| {})
        .await
        .unwrap();
    let id = journal.start(start("hello"), 1).await.unwrap();
    journal.checkpoint(id, "prefix", 3).await.unwrap();
    assert!(matches!(
        journal
            .finish(
                id,
                ConversationTerminalObservation::RejectedBeforeDispatch {
                    diagnostic: "not dispatched".into()
                },
                4
            )
            .await,
        Err(ConversationJournalError::InvalidEvent(_))
    ));
    assert!(matches!(
        journal
            .mark_uncertain(id, ConversationUncertaintyReason::StreamOpenTimeout, 4)
            .await,
        Err(ConversationJournalError::InvalidEvent(_))
    ));
    journal
        .finish(id, completion("prefix and done"), 5)
        .await
        .unwrap();
}

#[tokio::test]
async fn replay_rejects_orphaned_or_corrupt_conversation_events() {
    let storage = InMemoryAgentStorage::new();
    let (project, session, agent) = ids();
    storage
        .append_session_event(SessionEventRecord {
            project_id: project.clone(),
            session_id: session.clone(),
            agent_id: agent.clone(),
            turn_id: TurnId::new("conversation:00000000000000000001").unwrap(),
            event_id: EventId::new("conversation:00000000000000000001:01-finish").unwrap(),
            event_kind: "marlin.conversation.turn.finished.v1".into(),
            causality_parent_event_id: Some(
                EventId::new("conversation:00000000000000000001:00-start").unwrap(),
            ),
            body: serde_json::to_vec(&recorded_completion("orphan")).unwrap(),
            created_at_unix_ms: 2,
        })
        .await
        .unwrap();
    assert!(matches!(
        ConversationJournal::open(storage, project, session, agent, |_| {}).await,
        Err(ConversationJournalError::WrongActiveTurn)
    ));
}
