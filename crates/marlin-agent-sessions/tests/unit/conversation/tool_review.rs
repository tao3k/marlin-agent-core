// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use marlin_agent_protocol::{ModelEndpoint, ModelToolCallId};
use marlin_agent_sessions::{
    ConversationEvent, ConversationEventReader, ConversationJournal, ConversationJournalError,
    ConversationTerminalObservation, ConversationToolCallResult, ConversationToolContinuationStart,
    ConversationToolResultOutcome, ConversationToolReviewDecision, ConversationToolReviewRequest,
    ConversationToolReviewStatus, ConversationTurnOutcome, ConversationTurnSource,
};
use marlin_agent_storage::SessionEventStore;
use marlin_agent_storage::{
    EventId, InMemoryAgentStorage, SessionEventRecord, SessionId, StoragePageLimit, TurnId,
};

use super::fixtures::{completion, ids, start, tool_request};

#[tokio::test]
async fn tool_request_is_durable_and_blocks_an_unreviewed_followup() {
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
    let id = journal.start(start("read marker"), 1).await.unwrap();
    assert!(journal.pending_tool_request().is_none());
    journal
        .checkpoint(id, "I need the requested marker.", 2)
        .await
        .unwrap();
    assert!(matches!(
        journal
            .finish(
                id,
                ConversationTerminalObservation::Completed {
                    response: tool_request()
                },
                3
            )
            .await,
        Err(ConversationJournalError::InvalidEvent(_))
    ));
    assert!(journal.pending_tool_request().is_none());
    journal
        .finish(
            id,
            ConversationTerminalObservation::ToolRequested {
                response: tool_request(),
            },
            3,
        )
        .await
        .unwrap();
    let pending = journal.pending_tool_request().unwrap();
    assert_eq!(pending.session(), &session);
    assert_eq!(pending.turn(), id);
    assert_eq!(pending.calls().len(), 1);
    assert_eq!(pending.calls()[0].id.as_str(), "call_1");
    assert_eq!(pending.calls()[0].name.as_str(), "read_marker");
    assert!(matches!(
        journal.start(start("next"), 4).await,
        Err(ConversationJournalError::ToolRequestPending)
    ));
    let mut replayed = Vec::new();
    let mut reopened =
        ConversationJournal::open(storage, project, session.clone(), agent, |turn| {
            replayed.push(turn.clone());
        })
        .await
        .unwrap();
    assert!(matches!(
        &replayed[0].outcome,
        Some(ConversationTurnOutcome::ToolRequested { response })
            if response.choices[0].message.tool_calls[0].id.as_str() == "call_1"
    ));
    let pending = reopened.pending_tool_request().unwrap();
    assert_eq!(pending.session(), &session);
    assert_eq!(pending.turn(), id);
    assert_eq!(pending.calls()[0].id.as_str(), "call_1");
    assert!(matches!(
        reopened.start(start("next"), 5).await,
        Err(ConversationJournalError::ToolRequestPending)
    ));
}

#[tokio::test]
async fn tool_execution_requires_a_matching_review_and_one_durable_claim() {
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
    let id = journal.start(start("read marker"), 1).await.unwrap();
    journal
        .finish(
            id,
            ConversationTerminalObservation::ToolRequested {
                response: tool_request(),
            },
            2,
        )
        .await
        .unwrap();
    let key = journal.pending_tool_request().unwrap().key();
    let other_session = SessionId::new("session:other").unwrap();
    let mut other = ConversationJournal::open(
        storage.clone(),
        project.clone(),
        other_session,
        agent.clone(),
        |_| {},
    )
    .await
    .unwrap();
    let other_id = other.start(start("read marker"), 1).await.unwrap();
    other
        .finish(
            other_id,
            ConversationTerminalObservation::ToolRequested {
                response: tool_request(),
            },
            2,
        )
        .await
        .unwrap();
    assert!(
        journal
            .review_tool_request(
                ConversationToolReviewRequest::new(
                    other.pending_tool_request().unwrap().key(),
                    ConversationToolReviewDecision::ApproveOnce,
                ),
                3,
            )
            .await
            .is_err()
    );
    let receipt = journal
        .review_tool_request(
            ConversationToolReviewRequest::new(
                key.clone(),
                ConversationToolReviewDecision::ApproveOnce,
            ),
            3,
        )
        .await
        .unwrap();
    assert_eq!(receipt.key(), &key);
    assert_eq!(
        journal.pending_tool_request().unwrap().status(),
        ConversationToolReviewStatus::Approved
    );
    drop(journal);
    let mut reopened = ConversationJournal::open(
        storage.clone(),
        project.clone(),
        session.clone(),
        agent.clone(),
        |_| {},
    )
    .await
    .unwrap();
    assert_eq!(
        reopened.pending_tool_request().unwrap().status(),
        ConversationToolReviewStatus::Approved
    );
    let claim = reopened.claim_tool_execution(&receipt, 4).await.unwrap();
    assert_eq!(claim.session(), &session);
    assert_eq!(claim.turn(), id);
    assert_eq!(claim.into_calls()[0].id.as_str(), "call_1");
    assert!(reopened.claim_tool_execution(&receipt, 5).await.is_err());
    let reader = ConversationEventReader::new(
        storage.clone(),
        project.clone(),
        session.clone(),
        agent.clone(),
    );
    let tail = reader.latest_turn().await.unwrap().unwrap();
    assert_eq!(tail.turn, id);
    assert_eq!(
        tail.state,
        marlin_agent_sessions::ConversationTailState::Uncertain
    );
    assert!(matches!(
        reader.committed_outcome(id).await.unwrap(),
        Some(ConversationTurnOutcome::ToolRequested { .. })
    ));
    let entries = reader
        .page(id, None, StoragePageLimit::new(4).unwrap())
        .await
        .unwrap();
    assert_eq!(entries.len(), 4);
    assert_eq!(&tail.event_id, entries[3].cursor.event_id());
    assert!(matches!(
        &entries[2].event,
        ConversationEvent::ToolReview(_)
    ));
    assert!(matches!(
        &entries[3].event,
        ConversationEvent::ToolExecutionClaim(_)
    ));
    assert!(
        reader
            .resolve_cursor(id, entries[3].cursor.event_id().clone())
            .await
            .unwrap()
            .is_some()
    );
    let reopened = ConversationJournal::open(storage, project, session, agent, |_| {})
        .await
        .unwrap();
    assert_eq!(
        reopened.pending_tool_request().unwrap().status(),
        ConversationToolReviewStatus::ExecutionUncertain
    );
}

#[tokio::test]
async fn tool_results_require_the_live_claim_and_survive_replay() {
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
    let id = journal.start(start("read marker"), 1).await.unwrap();
    journal
        .finish(
            id,
            ConversationTerminalObservation::ToolRequested {
                response: tool_request(),
            },
            2,
        )
        .await
        .unwrap();
    let key = journal.pending_tool_request().unwrap().key();
    let result = ConversationToolCallResult {
        id: ModelToolCallId::new("call_1").unwrap(),
        outcome: ConversationToolResultOutcome::Completed("marker".into()),
    };
    assert!(
        journal
            .record_tool_results(key.clone(), vec![result.clone()], 3)
            .await
            .is_err()
    );
    let approval = journal
        .review_tool_request(
            ConversationToolReviewRequest::new(
                key.clone(),
                ConversationToolReviewDecision::ApproveOnce,
            ),
            3,
        )
        .await
        .unwrap();
    let claim = journal.claim_tool_execution(&approval, 4).await.unwrap();
    assert_eq!(claim.key(), key);
    assert!(
        journal
            .record_tool_results(
                key.clone(),
                vec![ConversationToolCallResult {
                    id: ModelToolCallId::new("foreign").unwrap(),
                    outcome: ConversationToolResultOutcome::Completed("marker".into()),
                }],
                5,
            )
            .await
            .is_err()
    );
    let receipt = journal
        .record_tool_results(key.clone(), vec![result.clone()], 5)
        .await
        .unwrap();
    assert_eq!(receipt.results(), &[result]);
    assert!(journal.record_tool_results(key, vec![], 6).await.is_err());
    drop(journal);
    let reopened = ConversationJournal::open(
        storage.clone(),
        project.clone(),
        session.clone(),
        agent.clone(),
        |_| {},
    )
    .await
    .unwrap();
    let pending = reopened.pending_tool_request().unwrap();
    assert_eq!(
        pending.status(),
        ConversationToolReviewStatus::ResultsCommitted
    );
    assert_eq!(pending.results_receipt(), Some(&receipt));
    let reader = ConversationEventReader::new(storage, project, session, agent);
    let tail = reader.latest_turn().await.unwrap().unwrap();
    assert_eq!(
        tail.state,
        marlin_agent_sessions::ConversationTailState::ToolContinuationPending
    );
    assert!(matches!(
        reader
            .resolve_cursor(id, tail.event_id)
            .await
            .unwrap()
            .unwrap()
            .event,
        ConversationEvent::ToolResults(_)
    ));
}

#[tokio::test]
async fn committed_tool_results_admit_a_typed_continuation_without_reexecuting() {
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
    let request = journal.start(start("read marker"), 1).await.unwrap();
    journal
        .finish(
            request,
            ConversationTerminalObservation::ToolRequested {
                response: tool_request(),
            },
            2,
        )
        .await
        .unwrap();
    let key = journal.pending_tool_request().unwrap().key();
    let approval = journal
        .review_tool_request(
            ConversationToolReviewRequest::new(
                key.clone(),
                ConversationToolReviewDecision::ApproveOnce,
            ),
            3,
        )
        .await
        .unwrap();
    let claim = journal.claim_tool_execution(&approval, 4).await.unwrap();
    let receipt = journal
        .record_tool_results(
            claim.key(),
            vec![ConversationToolCallResult {
                id: ModelToolCallId::new("call_1").unwrap(),
                outcome: ConversationToolResultOutcome::Completed("marker".into()),
            }],
            5,
        )
        .await
        .unwrap();
    assert_eq!(
        receipt.gateway_messages()[0]
            .tool_call_id
            .as_ref()
            .unwrap()
            .as_str(),
        "call_1"
    );
    assert!(matches!(
        journal.start(start("next user"), 6).await,
        Err(ConversationJournalError::ToolRequestPending)
    ));
    assert!(
        journal
            .start_tool_continuation(
                ConversationToolContinuationStart {
                    endpoint: ModelEndpoint::new("openai", "wrong-model"),
                    source: key.clone(),
                },
                6
            )
            .await
            .is_err()
    );
    let continuation = ConversationToolContinuationStart {
        endpoint: ModelEndpoint::new("openai", "gpt-fixture"),
        source: key,
    };
    let first = journal
        .start_tool_continuation(continuation.clone(), 7)
        .await
        .unwrap();
    assert!(
        journal
            .start_tool_continuation(continuation.clone(), 8)
            .await
            .is_err()
    );
    journal
        .finish(
            first,
            ConversationTerminalObservation::RejectedBeforeDispatch {
                diagnostic: "local route unavailable".into(),
            },
            8,
        )
        .await
        .unwrap();
    assert_eq!(
        journal.pending_tool_request().unwrap().status(),
        ConversationToolReviewStatus::ResultsCommitted
    );
    let second = journal
        .start_tool_continuation(continuation, 9)
        .await
        .unwrap();
    journal
        .finish(second, completion("marker read"), 10)
        .await
        .unwrap();
    assert!(journal.pending_tool_request().is_none());
    let mut turns = Vec::new();
    let reopened = ConversationJournal::open(
        storage.clone(),
        project.clone(),
        session.clone(),
        agent.clone(),
        |turn| turns.push(turn.clone()),
    )
    .await
    .unwrap();
    assert!(reopened.pending_tool_request().is_none());
    assert_eq!(turns.len(), 3);
    assert_eq!(turns[0].tool_results.as_ref(), Some(&receipt));
    assert!(matches!(
        &turns[1].start,
        ConversationTurnSource::ToolResults(_)
    ));
    assert!(matches!(
        &turns[2].start,
        ConversationTurnSource::ToolResults(_)
    ));
    assert_eq!(turns[1].start.user_text(), None);
    assert_eq!(turns[2].start.user_text(), None);
    let reader = ConversationEventReader::new(storage, project, session, agent);
    assert_eq!(
        reader.latest_turn().await.unwrap().unwrap().state,
        marlin_agent_sessions::ConversationTailState::Finished
    );
}

#[tokio::test]
async fn rejected_tool_request_continues_without_claiming_execution() {
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
    let id = journal.start(start("read marker"), 1).await.unwrap();
    journal
        .finish(
            id,
            ConversationTerminalObservation::ToolRequested {
                response: tool_request(),
            },
            2,
        )
        .await
        .unwrap();
    let key = journal.pending_tool_request().unwrap().key();
    let receipt = journal
        .review_tool_request(
            ConversationToolReviewRequest::new(key, ConversationToolReviewDecision::Reject),
            3,
        )
        .await
        .unwrap();
    assert!(journal.claim_tool_execution(&receipt, 4).await.is_err());
    let denial = receipt.denial_messages().unwrap();
    assert_eq!(denial.len(), 1);
    assert_eq!(denial[0].tool_call_id.as_ref().unwrap().as_str(), "call_1");
    assert_eq!(denial[0].content, "Tool execution denied by the user.");
    let reader = ConversationEventReader::new(
        storage.clone(),
        project.clone(),
        session.clone(),
        agent.clone(),
    );
    assert_eq!(
        reader.latest_turn().await.unwrap().unwrap().state,
        marlin_agent_sessions::ConversationTailState::ToolContinuationPending
    );
    let mut turns = Vec::new();
    let mut reopened = ConversationJournal::open(
        storage.clone(),
        project.clone(),
        session.clone(),
        agent.clone(),
        |turn| turns.push(turn.clone()),
    )
    .await
    .unwrap();
    assert_eq!(
        reopened.pending_tool_request().unwrap().status(),
        ConversationToolReviewStatus::Rejected
    );
    assert!(reopened.claim_tool_execution(&receipt, 5).await.is_err());
    assert_eq!(turns[0].tool_denial.as_ref(), Some(&receipt));
    assert_eq!(turns[0].continuation_messages().unwrap(), denial);
    let next = reopened
        .start_tool_continuation(
            ConversationToolContinuationStart {
                endpoint: start("ignored").endpoint,
                source: receipt.key().clone(),
            },
            6,
        )
        .await
        .unwrap();
    reopened
        .finish(next, completion("understood"), 7)
        .await
        .unwrap();
    assert!(reopened.pending_tool_request().is_none());
    let mut recovered = Vec::new();
    let reopened = ConversationJournal::open(storage, project, session, agent, |turn| {
        recovered.push(turn.clone())
    })
    .await
    .unwrap();
    assert!(reopened.pending_tool_request().is_none());
    assert_eq!(recovered.len(), 2);
    assert_eq!(recovered[0].tool_denial.as_ref(), Some(&receipt));
    assert!(matches!(
        recovered[1].start,
        ConversationTurnSource::ToolResults(_)
    ));
}

#[tokio::test]
async fn competing_claim_write_relinquishes_local_execution_authority() {
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
    let id = journal.start(start("read marker"), 1).await.unwrap();
    journal
        .finish(
            id,
            ConversationTerminalObservation::ToolRequested {
                response: tool_request(),
            },
            2,
        )
        .await
        .unwrap();
    let receipt = journal
        .review_tool_request(
            ConversationToolReviewRequest::new(
                journal.pending_tool_request().unwrap().key(),
                ConversationToolReviewDecision::ApproveOnce,
            ),
            3,
        )
        .await
        .unwrap();
    storage
        .append_session_event(SessionEventRecord {
            project_id: project.clone(),
            session_id: session.clone(),
            agent_id: agent.clone(),
            turn_id: TurnId::new("conversation:00000000000000000001").unwrap(),
            event_id: EventId::new("conversation:00000000000000000001:03-tool-claim").unwrap(),
            event_kind: "marlin.conversation.tool.claim.v1".into(),
            causality_parent_event_id: Some(
                EventId::new("conversation:00000000000000000001:02-tool-review").unwrap(),
            ),
            body: serde_json::to_vec(&receipt).unwrap(),
            created_at_unix_ms: 4,
        })
        .await
        .unwrap();
    assert!(journal.claim_tool_execution(&receipt, 4).await.is_err());
    assert!(matches!(
        journal.claim_tool_execution(&receipt, 5).await,
        Err(ConversationJournalError::UnownedTurn)
    ));
    let reopened = ConversationJournal::open(storage, project, session, agent, |_| {})
        .await
        .unwrap();
    assert_eq!(
        reopened.pending_tool_request().unwrap().status(),
        ConversationToolReviewStatus::ExecutionUncertain
    );
}
