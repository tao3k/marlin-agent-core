// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use marlin_agent_protocol::ModelGatewayCompletionResponse;
use marlin_agent_sessions::{
    ConversationJournal, ConversationJournalError, ConversationTerminalObservation,
    ConversationTurnOutcome, ConversationUncertaintyReason,
};
use marlin_agent_storage::SessionEventStore;
use marlin_agent_storage::{
    EventId, InMemoryAgentStorage, SessionEventRecord, StorageError, TurnId,
};

use super::fixtures::{completion, ids, recorded_completion, start};

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
