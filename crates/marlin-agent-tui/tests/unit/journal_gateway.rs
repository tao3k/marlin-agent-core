// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use marlin_agent_storage::SessionEventStore;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use super::journal_gateway::JournalGateway;
use marlin_agent_protocol::{
    ModelEndpoint, ModelGatewayCompletionChoice, ModelGatewayCompletionResponse, ModelGatewayError,
    ModelGatewayFuture, ModelGatewayRequest, ModelGatewayResult, ModelStreamInterruption,
    assistant_gateway_message, user_gateway_message,
};
use marlin_agent_sessions::{
    ConversationJournal, ConversationTurnOutcome, ConversationUncertaintyReason,
};
use marlin_agent_storage::{
    AgentId, EventId, InMemoryAgentStorage, ProjectId, SessionEventRecord, SessionId, TurnId,
};
use marlin_agent_stream::{
    ModelGatewayEventStream, ModelStreamChunk, ModelStreamEvent, ModelStreamingGateway,
    ModelThinkingChunk,
};
use tokio_stream::StreamExt;

#[path = "journal_gateway_tool_review.rs"]
mod tool_review;

struct Fixture {
    calls: Arc<AtomicUsize>,
    completion: &'static str,
}
impl ModelStreamingGateway for Fixture {
    fn stream(
        &self,
        _: ModelGatewayRequest,
    ) -> ModelGatewayFuture<ModelGatewayResult<ModelGatewayEventStream>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let response = ModelGatewayCompletionResponse::new(
            "id",
            "model",
            vec![ModelGatewayCompletionChoice::new(
                0,
                assistant_gateway_message(self.completion),
                None,
            )],
        );
        Box::pin(async move {
            Ok(Box::pin(tokio_stream::iter(vec![
                Ok(ModelStreamEvent::Chunk(ModelStreamChunk::new(1, "hello"))),
                Ok(ModelStreamEvent::Completed { response }),
            ])) as ModelGatewayEventStream)
        })
    }
}

fn ids() -> (ProjectId, SessionId, AgentId) {
    (
        ProjectId::new("project:test").unwrap(),
        SessionId::new("session:test").unwrap(),
        AgentId::new("agent:test").unwrap(),
    )
}
fn request() -> ModelGatewayRequest {
    ModelGatewayRequest::new(
        ModelEndpoint::new("provider", "model"),
        vec![user_gateway_message("hello")],
    )
}
async fn open(storage: InMemoryAgentStorage) -> ConversationJournal<InMemoryAgentStorage> {
    let (p, s, a) = ids();
    ConversationJournal::open(storage, p, s, a, |_| {})
        .await
        .unwrap()
}

struct BatchFixture;

#[tokio::test]
async fn rejected_request_with_unrecordable_terminal_is_not_reported_as_safe() {
    struct ConflictingRejection(InMemoryAgentStorage);
    impl ModelStreamingGateway for ConflictingRejection {
        fn stream(
            &self,
            _: ModelGatewayRequest,
        ) -> ModelGatewayFuture<ModelGatewayResult<ModelGatewayEventStream>> {
            let storage = self.0.clone();
            Box::pin(async move {
                let (project_id, session_id, agent_id) = ids();
                storage
                    .append_session_event(SessionEventRecord {
                        project_id,
                        session_id,
                        agent_id,
                        turn_id: TurnId::new("conversation:00000000000000000001").unwrap(),
                        event_id: EventId::new("conversation:00000000000000000001:01-finish")
                            .unwrap(),
                        event_kind: "fixture.conflicting.event".into(),
                        causality_parent_event_id: None,
                        body: Vec::new(),
                        created_at_unix_ms: 0,
                    })
                    .await
                    .unwrap();
                Err(ModelGatewayError::RequestNotDispatched(
                    "fixture rejection".into(),
                ))
            })
        }
    }
    let storage = InMemoryAgentStorage::new();
    let gateway = JournalGateway::new(ConflictingRejection(storage.clone()), open(storage).await);
    assert!(
        matches!(gateway.stream(request()).await, Err(ModelGatewayError::Completion(message)) if message.contains("terminal observation could not be persisted"))
    );
}

struct ThinkingFixture;
impl ModelStreamingGateway for ThinkingFixture {
    fn stream(
        &self,
        _: ModelGatewayRequest,
    ) -> ModelGatewayFuture<ModelGatewayResult<ModelGatewayEventStream>> {
        Box::pin(async {
            Ok(Box::pin(tokio_stream::iter(vec![
                Ok(ModelStreamEvent::Chunk(ModelStreamChunk::new(1, "a"))),
                Ok(ModelStreamEvent::Chunk(ModelStreamChunk::new(2, "b"))),
                Ok(ModelStreamEvent::Thinking(ModelThinkingChunk::new(
                    1, "thought",
                ))),
                Ok(ModelStreamEvent::Chunk(ModelStreamChunk::new(3, "c"))),
            ])) as ModelGatewayEventStream)
        })
    }
}

#[tokio::test]
async fn failed_thinking_checkpoint_is_not_delivered() {
    let storage = InMemoryAgentStorage::new();
    let gateway = JournalGateway::new(ThinkingFixture, open(storage.clone()).await);
    let mut stream = gateway.stream(request()).await.unwrap();
    stream.next().await.unwrap().unwrap();
    stream.next().await.unwrap().unwrap();
    let (project_id, session_id, agent_id) = ids();
    storage
        .append_session_event(SessionEventRecord {
            project_id,
            session_id,
            agent_id,
            turn_id: TurnId::new("conversation:00000000000000000001").unwrap(),
            event_id: EventId::new(
                "conversation:00000000000000000001:00-start:chunk:00000000000000000003",
            )
            .unwrap(),
            event_kind: "fixture.conflicting.event".into(),
            causality_parent_event_id: None,
            body: Vec::new(),
            created_at_unix_ms: 0,
        })
        .await
        .unwrap();
    assert!(
        matches!(stream.next().await.unwrap(), Err(error) if error.to_string().contains("thinking checkpoint could not be persisted"))
    );
}

#[tokio::test]
async fn thinking_is_checkpointed_in_order_before_delivery() {
    use marlin_agent_storage::{SessionEventPageRequest, StoragePageLimit};
    let storage = InMemoryAgentStorage::new();
    let gateway = JournalGateway::new(ThinkingFixture, open(storage.clone()).await);
    let mut stream = gateway.stream(request()).await.unwrap();
    assert!(matches!(
        stream.next().await.unwrap().unwrap(),
        ModelStreamEvent::Chunk(_)
    ));
    assert!(matches!(
        stream.next().await.unwrap().unwrap(),
        ModelStreamEvent::Chunk(_)
    ));
    assert!(matches!(
        stream.next().await.unwrap().unwrap(),
        ModelStreamEvent::Thinking(_)
    ));
    let turn = open(storage.clone()).await.unfinished().unwrap().clone();
    assert_eq!(turn.partial_text, "ab");
    assert_eq!(turn.partial_thinking, "thought");
    assert!(matches!(
        stream.next().await.unwrap().unwrap(),
        ModelStreamEvent::Chunk(_)
    ));
    assert!(matches!(
        stream.next().await.unwrap(),
        Err(ModelGatewayError::StreamInterrupted(
            ModelStreamInterruption::EndedWithoutTerminal
        ))
    ));
    drop(stream);
    let turn = open(storage.clone()).await.unfinished().unwrap().clone();
    assert_eq!(turn.partial_text, "abc");
    assert_eq!(turn.partial_thinking, "thought");
    let (project, session, _) = ids();
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
            "marlin.conversation.turn.checkpoint.v1",
            "marlin.conversation.turn.checkpoint.v1",
            "marlin.conversation.turn.thinking-checkpoint.v1",
            "marlin.conversation.turn.checkpoint.v1",
            "marlin.conversation.turn.uncertain.v1"
        ]
    );
}

struct OversizedCompletionFixture;
impl ModelStreamingGateway for OversizedCompletionFixture {
    fn stream(
        &self,
        _: ModelGatewayRequest,
    ) -> ModelGatewayFuture<ModelGatewayResult<ModelGatewayEventStream>> {
        Box::pin(async {
            let mut id = String::with_capacity(super::output_limit::MAX_COMPLETION_HEAP_BYTES + 1);
            id.push_str("id");
            let response = ModelGatewayCompletionResponse::new(
                id,
                "model",
                vec![ModelGatewayCompletionChoice::new(
                    0,
                    assistant_gateway_message("first-tail"),
                    None,
                )],
            );
            Ok(Box::pin(tokio_stream::iter(vec![
                Ok(ModelStreamEvent::Chunk(ModelStreamChunk::new(1, "first"))),
                Ok(ModelStreamEvent::Chunk(ModelStreamChunk::new(2, "-tail"))),
                Ok(ModelStreamEvent::Completed { response }),
            ])) as ModelGatewayEventStream)
        })
    }
}

#[tokio::test]
async fn oversized_completion_is_not_committed_and_preserves_the_prefix() {
    let storage = InMemoryAgentStorage::new();
    let gateway = JournalGateway::new(OversizedCompletionFixture, open(storage.clone()).await);
    let mut stream = gateway.stream(request()).await.unwrap();
    stream.next().await.unwrap().unwrap();
    stream.next().await.unwrap().unwrap();
    let reason = match stream.next().await.unwrap() {
        Err(ModelGatewayError::StreamInterrupted(
            reason @ ModelStreamInterruption::CompletionHeapLimit { .. },
        )) => reason,
        _ => panic!("expected typed completion heap limit"),
    };
    drop(stream);
    let (p, s, a) = ids();
    let mut turns = Vec::new();
    let reopened = ConversationJournal::open(storage, p, s, a, |turn| turns.push(turn.clone()))
        .await
        .unwrap();
    assert_eq!(turns[0].partial_text, "first-tail");
    assert!(
        matches!(turns[0].outcome, Some(ConversationTurnOutcome::Uncertain {
        reason: ConversationUncertaintyReason::StreamInterrupted { reason: observed }
    }) if observed == reason)
    );
    assert!(reopened.unfinished().is_some());
    assert!(gateway.stream(request()).await.is_err());
}

struct InterruptedFixture(Option<ModelStreamInterruption>);
impl ModelStreamingGateway for InterruptedFixture {
    fn stream(
        &self,
        _: ModelGatewayRequest,
    ) -> ModelGatewayFuture<ModelGatewayResult<ModelGatewayEventStream>> {
        let mut events = vec![
            Ok(ModelStreamEvent::Chunk(ModelStreamChunk::new(1, "first"))),
            Ok(ModelStreamEvent::Chunk(ModelStreamChunk::new(2, "-tail"))),
        ];
        if let Some(reason) = self.0 {
            events.push(Err(ModelGatewayError::StreamInterrupted(reason)));
        }
        Box::pin(async move { Ok(Box::pin(tokio_stream::iter(events)) as ModelGatewayEventStream) })
    }
}

struct UnclassifiedFailureFixture(Arc<AtomicUsize>);
impl ModelStreamingGateway for UnclassifiedFailureFixture {
    fn stream(
        &self,
        _: ModelGatewayRequest,
    ) -> ModelGatewayFuture<ModelGatewayResult<ModelGatewayEventStream>> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Box::pin(async {
            Ok(Box::pin(tokio_stream::iter(vec![
                Ok(ModelStreamEvent::Chunk(ModelStreamChunk::new(1, "first"))),
                Ok(ModelStreamEvent::Chunk(ModelStreamChunk::new(2, "-tail"))),
                Err(ModelGatewayError::Completion(
                    "unspecified gateway failure".into(),
                )),
            ])) as ModelGatewayEventStream)
        })
    }
}

#[tokio::test]
async fn unclassified_stream_failure_never_retires_or_redispatches_the_turn() {
    let storage = InMemoryAgentStorage::new();
    let calls = Arc::new(AtomicUsize::new(0));
    let gateway = JournalGateway::new(
        UnclassifiedFailureFixture(calls.clone()),
        open(storage.clone()).await,
    );
    let mut stream = gateway.stream(request()).await.unwrap();
    stream.next().await.unwrap().unwrap();
    stream.next().await.unwrap().unwrap();
    assert!(matches!(
        stream.next().await.unwrap(),
        Err(ModelGatewayError::StreamInterrupted(
            ModelStreamInterruption::UnclassifiedGatewayFailure
        ))
    ));
    drop(stream);
    assert!(gateway.stream(request()).await.is_err());
    let (p, s, a) = ids();
    let mut turns = Vec::new();
    let reopened = ConversationJournal::open(storage, p, s, a, |turn| turns.push(turn.clone()))
        .await
        .unwrap();
    assert_eq!(turns[0].partial_text, "first-tail");
    assert!(matches!(
        turns[0].outcome,
        Some(ConversationTurnOutcome::Uncertain {
            reason: ConversationUncertaintyReason::StreamInterrupted {
                reason: ModelStreamInterruption::UnclassifiedGatewayFailure
            }
        })
    ));
    let resumed = JournalGateway::new(UnclassifiedFailureFixture(calls.clone()), reopened);
    assert!(resumed.stream(request()).await.is_err());
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

struct InvalidChunkFixture {
    bad_sequence: bool,
}
impl ModelStreamingGateway for InvalidChunkFixture {
    fn stream(
        &self,
        _: ModelGatewayRequest,
    ) -> ModelGatewayFuture<ModelGatewayResult<ModelGatewayEventStream>> {
        let invalid = if self.bad_sequence {
            ModelStreamChunk::new(9, "rejected")
        } else {
            ModelStreamChunk::new(
                3,
                "x".repeat(marlin_agent_stream::MODEL_STREAM_MAX_CHUNK_BYTES + 1),
            )
        };
        Box::pin(async move {
            Ok(Box::pin(tokio_stream::iter(vec![
                Ok(ModelStreamEvent::Chunk(ModelStreamChunk::new(1, "first"))),
                Ok(ModelStreamEvent::Chunk(ModelStreamChunk::new(2, "-tail"))),
                Ok(ModelStreamEvent::Chunk(invalid)),
            ])) as ModelGatewayEventStream)
        })
    }
}

#[tokio::test]
async fn invalid_chunks_preserve_only_accepted_prefix_and_block_replay() {
    for bad_sequence in [true, false] {
        let storage = InMemoryAgentStorage::new();
        let gateway = JournalGateway::new(
            InvalidChunkFixture { bad_sequence },
            open(storage.clone()).await,
        );
        let mut stream = gateway.stream(request()).await.unwrap();
        stream.next().await.unwrap().unwrap();
        stream.next().await.unwrap().unwrap();
        let expected = if bad_sequence {
            ModelStreamInterruption::InvalidSequence {
                expected: 3,
                actual: 9,
            }
        } else {
            ModelStreamInterruption::ChunkTooLarge {
                limit_bytes: marlin_agent_stream::MODEL_STREAM_MAX_CHUNK_BYTES,
                actual_bytes: marlin_agent_stream::MODEL_STREAM_MAX_CHUNK_BYTES + 1,
            }
        };
        assert!(
            matches!(stream.next().await.unwrap(), Err(ModelGatewayError::StreamInterrupted(reason)) if reason == expected)
        );
        drop(stream);
        let (p, s, a) = ids();
        let mut turns = Vec::new();
        let reopened = ConversationJournal::open(storage, p, s, a, |turn| turns.push(turn.clone()))
            .await
            .unwrap();
        assert_eq!(turns[0].partial_text, "first-tail");
        assert!(
            matches!(turns[0].outcome, Some(ConversationTurnOutcome::Uncertain { reason: ConversationUncertaintyReason::StreamInterrupted { reason } }) if reason == expected)
        );
        assert!(reopened.unfinished().is_some());
    }
}

#[tokio::test]
async fn interrupted_stream_checkpoints_the_observed_tail_and_remains_unresolved() {
    for supplied in [
        Some(ModelStreamInterruption::Transport),
        Some(ModelStreamInterruption::Timeout),
        None,
    ] {
        let storage = InMemoryAgentStorage::new();
        let gateway =
            JournalGateway::new(InterruptedFixture(supplied), open(storage.clone()).await);
        let mut stream = gateway.stream(request()).await.unwrap();
        stream.next().await.unwrap().unwrap();
        stream.next().await.unwrap().unwrap();
        let expected = supplied.unwrap_or(ModelStreamInterruption::EndedWithoutTerminal);
        assert!(
            matches!(stream.next().await.unwrap(), Err(ModelGatewayError::StreamInterrupted(reason)) if reason == expected)
        );
        drop(stream);
        assert!(gateway.stream(request()).await.is_err());
        let (p, s, a) = ids();
        let mut turns = Vec::new();
        let reopened = ConversationJournal::open(storage, p, s, a, |turn| turns.push(turn.clone()))
            .await
            .unwrap();
        assert_eq!(turns[0].partial_text, "first-tail");
        assert!(
            matches!(turns[0].outcome, Some(ConversationTurnOutcome::Uncertain { reason: ConversationUncertaintyReason::StreamInterrupted { reason } }) if reason == expected)
        );
        assert!(reopened.unfinished().is_some());
    }
}
impl ModelStreamingGateway for BatchFixture {
    fn stream(
        &self,
        _: ModelGatewayRequest,
    ) -> ModelGatewayFuture<ModelGatewayResult<ModelGatewayEventStream>> {
        Box::pin(async {
            Ok(Box::pin(tokio_stream::iter(vec![
                Ok(ModelStreamEvent::Chunk(ModelStreamChunk::new(1, "a"))),
                Ok(ModelStreamEvent::Chunk(ModelStreamChunk::new(2, "b"))),
                Ok(ModelStreamEvent::Chunk(ModelStreamChunk::new(
                    3,
                    "x".repeat(65_535),
                ))),
            ])) as ModelGatewayEventStream)
        })
    }
}

struct StalledFixture;
impl ModelStreamingGateway for StalledFixture {
    fn stream(
        &self,
        _: ModelGatewayRequest,
    ) -> ModelGatewayFuture<ModelGatewayResult<ModelGatewayEventStream>> {
        Box::pin(async {
            let initial = tokio_stream::iter(vec![
                Ok(ModelStreamEvent::Chunk(ModelStreamChunk::new(1, "first"))),
                Ok(ModelStreamEvent::Chunk(ModelStreamChunk::new(2, "-tail"))),
            ]);
            Ok(Box::pin(initial.chain(tokio_stream::pending())) as ModelGatewayEventStream)
        })
    }
}

#[tokio::test]
async fn idle_stream_checkpoints_its_small_tail_while_polled() {
    let storage = InMemoryAgentStorage::new();
    let gateway = JournalGateway::new(StalledFixture, open(storage.clone()).await);
    let mut stream = gateway.stream(request()).await.unwrap();
    stream.next().await.unwrap().unwrap();
    stream.next().await.unwrap().unwrap();
    assert_eq!(
        open(storage.clone())
            .await
            .unfinished()
            .unwrap()
            .partial_text,
        "first"
    );
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(1500), stream.next())
            .await
            .is_err()
    );
    assert_eq!(
        open(storage).await.unfinished().unwrap().partial_text,
        "first-tail"
    );
}

#[tokio::test]
async fn idle_checkpoint_failure_is_reported_while_stream_is_open() {
    let storage = InMemoryAgentStorage::new();
    let gateway = JournalGateway::new(StalledFixture, open(storage.clone()).await);
    let mut stream = gateway.stream(request()).await.unwrap();
    stream.next().await.unwrap().unwrap();
    stream.next().await.unwrap().unwrap();
    let (project_id, session_id, agent_id) = ids();
    storage
        .append_session_event(SessionEventRecord {
            project_id,
            session_id,
            agent_id,
            turn_id: TurnId::new("conversation:00000000000000000001").unwrap(),
            event_id: EventId::new(
                "conversation:00000000000000000001:00-start:chunk:00000000000000000002",
            )
            .unwrap(),
            event_kind: "fixture.conflicting.event".into(),
            causality_parent_event_id: None,
            body: Vec::new(),
            created_at_unix_ms: 0,
        })
        .await
        .unwrap();
    let result = tokio::time::timeout(std::time::Duration::from_secs(3), stream.next())
        .await
        .unwrap()
        .unwrap();
    assert!(
        matches!(result, Err(error) if error.to_string().contains("checkpoint could not be persisted"))
    );
}

#[tokio::test]
async fn first_chunk_and_full_batch_are_checkpointed_but_tail_is_not_claimed() {
    let storage = InMemoryAgentStorage::new();
    let gateway = JournalGateway::new(BatchFixture, open(storage.clone()).await);
    let mut stream = gateway.stream(request()).await.unwrap();
    stream.next().await.unwrap().unwrap();
    assert_eq!(
        open(storage.clone())
            .await
            .unfinished()
            .unwrap()
            .partial_text,
        "a"
    );
    stream.next().await.unwrap().unwrap();
    assert_eq!(
        open(storage.clone())
            .await
            .unfinished()
            .unwrap()
            .partial_text,
        "a"
    );
    stream.next().await.unwrap().unwrap();
    assert_eq!(
        open(storage).await.unfinished().unwrap().partial_text,
        format!("ab{}", "x".repeat(65_535))
    );
}

#[tokio::test]
async fn failed_checkpoint_never_publishes_its_chunk() {
    let storage = InMemoryAgentStorage::new();
    let gateway = JournalGateway::new(BatchFixture, open(storage.clone()).await);
    let mut stream = gateway.stream(request()).await.unwrap();
    let (project_id, session_id, agent_id) = ids();
    storage
        .append_session_event(SessionEventRecord {
            project_id,
            session_id,
            agent_id,
            turn_id: TurnId::new("conversation:00000000000000000001").unwrap(),
            event_id: EventId::new(
                "conversation:00000000000000000001:00-start:chunk:00000000000000000001",
            )
            .unwrap(),
            event_kind: "fixture.conflicting.event".into(),
            causality_parent_event_id: None,
            body: Vec::new(),
            created_at_unix_ms: 0,
        })
        .await
        .unwrap();
    let result = stream.next().await.unwrap();
    assert!(
        matches!(result, Err(error) if error.to_string().contains("checkpoint could not be persisted"))
    );
}

#[tokio::test]
async fn admission_precedes_gateway_and_completion_precedes_delivery() {
    let storage = InMemoryAgentStorage::new();
    let calls = Arc::new(AtomicUsize::new(0));
    let gateway = JournalGateway::new(
        Fixture {
            calls: calls.clone(),
            completion: "hello",
        },
        open(storage.clone()).await,
    );
    let mut stream = gateway.stream(request()).await.unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(open(storage.clone()).await.unfinished().is_some());
    assert!(matches!(
        stream.next().await.unwrap().unwrap(),
        ModelStreamEvent::Chunk(_)
    ));
    assert!(matches!(
        stream.next().await.unwrap().unwrap(),
        ModelStreamEvent::Completed { .. }
    ));
    let (p, s, a) = ids();
    let mut turns = Vec::new();
    ConversationJournal::open(storage, p, s, a, |turn| turns.push(turn.clone()))
        .await
        .unwrap();
    assert!(matches!(
        turns[0].outcome,
        Some(ConversationTurnOutcome::Completed { .. })
    ));
}

#[tokio::test]
async fn dropped_stream_blocks_redispatch_and_replayed_owner() {
    let storage = InMemoryAgentStorage::new();
    let calls = Arc::new(AtomicUsize::new(0));
    let gateway = JournalGateway::new(
        Fixture {
            calls: calls.clone(),
            completion: "hello",
        },
        open(storage.clone()).await,
    );
    let mut stream = gateway.stream(request()).await.unwrap();
    stream.next().await.unwrap().unwrap();
    drop(stream);
    assert!(gateway.stream(request()).await.is_err());
    assert!(open(storage).await.unfinished().is_some());
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn stale_journal_rejection_happens_before_provider_dispatch() {
    let storage = InMemoryAgentStorage::new();
    let calls = Arc::new(AtomicUsize::new(0));
    let first = JournalGateway::new(
        Fixture {
            calls: calls.clone(),
            completion: "hello",
        },
        open(storage.clone()).await,
    );
    let stale = JournalGateway::new(
        Fixture {
            calls: calls.clone(),
            completion: "hello",
        },
        open(storage).await,
    );
    let _in_flight = first.stream(request()).await.unwrap();
    assert!(stale.stream(request()).await.is_err());
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn mismatched_completion_is_recorded_as_uncertain() {
    let storage = InMemoryAgentStorage::new();
    let gateway = JournalGateway::new(
        Fixture {
            calls: Arc::new(AtomicUsize::new(0)),
            completion: "different",
        },
        open(storage.clone()).await,
    );
    let mut stream = gateway.stream(request()).await.unwrap();
    stream.next().await.unwrap().unwrap();
    assert!(stream.next().await.unwrap().is_err());
    let (p, s, a) = ids();
    let mut turns = Vec::new();
    let reopened = ConversationJournal::open(storage, p, s, a, |turn| turns.push(turn.clone()))
        .await
        .unwrap();
    assert_eq!(turns[0].partial_text, "hello");
    assert!(reopened.unfinished().is_some());
    assert!(matches!(
        &turns[0].outcome,
        Some(ConversationTurnOutcome::Uncertain {
            reason: ConversationUncertaintyReason::StreamInterrupted {
                reason: ModelStreamInterruption::InvalidCompletion
            }
        })
    ));
}

#[tokio::test]
async fn terminal_storage_conflict_is_not_published_as_a_completion() {
    let storage = InMemoryAgentStorage::new();
    let gateway = JournalGateway::new(
        Fixture {
            calls: Arc::new(AtomicUsize::new(0)),
            completion: "hello",
        },
        open(storage.clone()).await,
    );
    let mut stream = gateway.stream(request()).await.unwrap();
    stream.next().await.unwrap().unwrap();
    let (project_id, session_id, agent_id) = ids();
    storage
        .append_session_event(SessionEventRecord {
            project_id,
            session_id,
            agent_id,
            turn_id: TurnId::new("conversation:00000000000000000001").unwrap(),
            event_id: EventId::new("conversation:00000000000000000001:01-finish").unwrap(),
            event_kind: "fixture.conflicting.event".into(),
            causality_parent_event_id: None,
            body: Vec::new(),
            created_at_unix_ms: 0,
        })
        .await
        .unwrap();
    let error = stream.next().await.unwrap().unwrap_err().to_string();
    assert!(
        error.contains("completion could not be persisted"),
        "{error}"
    );
    drop(stream);
    assert!(gateway.stream(request()).await.is_err());
}
