//! Asynchronous admission and terminal observations for a streaming turn.

use std::{
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use marlin_agent_protocol::{
    ModelGatewayError, ModelGatewayFuture, ModelGatewayMessageRole, ModelGatewayRequest,
    ModelGatewayResult, ModelStreamInterruption,
};
use marlin_agent_sessions::{
    ConversationJournal, ConversationTerminalObservation, ConversationTurnId,
    ConversationTurnStart, ConversationUncertaintyReason,
};
use marlin_agent_storage::SessionEventStore;
use marlin_agent_stream::{
    MODEL_STREAM_MAX_CHUNK_BYTES, ModelGatewayEventStream, ModelStreamEvent, ModelStreamingGateway,
};
use tokio::sync::{Mutex, OwnedMutexGuard};
use tokio_stream::StreamExt;

use super::output_limit::{MAX_RESPONSE_BYTES, completion_heap_limit};
const CHECKPOINT_INTERVAL: Duration = Duration::from_secs(1);

/// Serializes admission and records the observed outcome before publishing it.
/// Dropping an in-flight stream leaves an unfinished turn; it never implies
/// that the remote provider retired the request.
pub(crate) struct JournalGateway<G, S> {
    gateway: Arc<G>,
    journal: Arc<Mutex<ConversationJournal<S>>>,
}

impl<G, S> JournalGateway<G, S> {
    pub(crate) fn new(gateway: G, journal: ConversationJournal<S>) -> Self {
        Self {
            gateway: Arc::new(gateway),
            journal: Arc::new(Mutex::new(journal)),
        }
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

fn failure(message: impl std::fmt::Display) -> ModelGatewayError {
    ModelGatewayError::Completion(message.to_string())
}

async fn record_failure<S: SessionEventStore>(
    journal: &mut OwnedMutexGuard<ConversationJournal<S>>,
    id: ConversationTurnId,
    error: ModelGatewayError,
) -> ModelGatewayError {
    let diagnostic = error.to_string();
    match journal
        .finish(
            id,
            ConversationTerminalObservation::RejectedBeforeDispatch {
                diagnostic: diagnostic.clone(),
            },
            now_ms(),
        )
        .await
    {
        Ok(()) => error,
        Err(error) => failure(format!(
            "{diagnostic}; terminal observation could not be persisted: {error}"
        )),
    }
}

async fn record_stream_uncertainty<S: SessionEventStore>(
    journal: &mut OwnedMutexGuard<ConversationJournal<S>>,
    id: ConversationTurnId,
    reason: ModelStreamInterruption,
    partial: &str,
    persisted: usize,
) -> ModelGatewayError {
    if persisted < partial.len()
        && let Err(error) = journal
            .checkpoint(id, &partial[persisted..], now_ms())
            .await
    {
        return failure(format!(
            "{reason}; observed prefix could not be persisted: {error}"
        ));
    }
    match journal
        .mark_uncertain(
            id,
            ConversationUncertaintyReason::StreamInterrupted { reason },
            now_ms(),
        )
        .await
    {
        Ok(()) => ModelGatewayError::StreamInterrupted(reason),
        Err(error) => failure(format!(
            "{reason}; uncertainty could not be persisted: {error}"
        )),
    }
}

impl<G: ModelStreamingGateway, S: SessionEventStore + 'static> ModelStreamingGateway
    for JournalGateway<G, S>
{
    fn stream(
        &self,
        request: ModelGatewayRequest,
    ) -> ModelGatewayFuture<ModelGatewayResult<ModelGatewayEventStream>> {
        let gateway = self.gateway.clone();
        let journal = self.journal.clone();
        Box::pin(async move {
            let user_text = request
                .messages()
                .last()
                .filter(|m| m.role == ModelGatewayMessageRole::User)
                .ok_or_else(|| failure("conversation request must end with a user message"))?
                .content
                .clone();
            let start = ConversationTurnStart {
                endpoint: request.endpoint().clone(),
                user_text,
            };
            let mut journal = journal.lock_owned().await;
            let id = journal.start(start, now_ms()).await.map_err(failure)?;
            let mut source = match gateway.stream(request).await {
                Ok(source) => source,
                Err(ModelGatewayError::StreamOpenTimeout) => {
                    return Err(
                        match journal
                            .mark_uncertain(
                                id,
                                ConversationUncertaintyReason::StreamOpenTimeout,
                                now_ms(),
                            )
                            .await
                        {
                            Ok(()) => ModelGatewayError::StreamOpenTimeout,
                            Err(error) => failure(format!(
                                "stream-open deadline elapsed; uncertainty could not be persisted: {error}"
                            )),
                        },
                    );
                }
                Err(ModelGatewayError::StreamOpenTransport) => {
                    return Err(
                        match journal
                            .mark_uncertain(
                                id,
                                ConversationUncertaintyReason::StreamOpenTransport,
                                now_ms(),
                            )
                            .await
                        {
                            Ok(()) => ModelGatewayError::StreamOpenTransport,
                            Err(error) => failure(format!(
                                "stream-open transport failed; uncertainty could not be persisted: {error}"
                            )),
                        },
                    );
                }
                Err(error @ ModelGatewayError::RequestNotDispatched(_)) => {
                    return Err(record_failure(&mut journal, id, error).await);
                }
                Err(_) => {
                    return Err(
                        match journal
                            .mark_uncertain(
                                id,
                                ConversationUncertaintyReason::StreamOpenUnclassified,
                                now_ms(),
                            )
                            .await
                        {
                            Ok(()) => ModelGatewayError::StreamOpenUnclassified,
                            Err(error) => failure(format!(
                                "stream-open failed; uncertainty could not be persisted: {error}"
                            )),
                        },
                    );
                }
            };
            let output = async_stream::stream! {
                let mut partial = String::new();
                let mut persisted = 0usize;
                let mut next_sequence = 1u64;
                let mut next_thinking_sequence = 1u64;
                let mut thinking_bytes = 0usize;
                let mut had_chunks = false;
                let mut deadline: Option<tokio::time::Instant> = None;
                loop {
                    let event = tokio::select! {
                        biased;
                        _ = async {
                            match deadline {
                                Some(at) => tokio::time::sleep_until(at).await,
                                None => std::future::pending::<()>().await,
                            }
                        } => {
                            if let Err(error) = journal.checkpoint(id, &partial[persisted..], now_ms()).await {
                                yield Err(failure(format!("model output checkpoint could not be persisted: {error}")));
                                return;
                            }
                            persisted = partial.len();
                            deadline = None;
                            continue;
                        }
                        event = source.next() => event,
                    };
                    let Some(event) = event else {
                        yield Err(record_stream_uncertainty(&mut journal, id, ModelStreamInterruption::EndedWithoutTerminal, &partial, persisted).await);
                        return;
                    };
                    match event {
                        Ok(ModelStreamEvent::Chunk(chunk)) => {
                            let rejected = if chunk.sequence != next_sequence {
                                Some(ModelStreamInterruption::InvalidSequence { expected: next_sequence, actual: chunk.sequence })
                            } else if chunk.content.len() > MODEL_STREAM_MAX_CHUNK_BYTES {
                                Some(ModelStreamInterruption::ChunkTooLarge { limit_bytes: MODEL_STREAM_MAX_CHUNK_BYTES, actual_bytes: chunk.content.len() })
                            } else if partial.len().checked_add(thinking_bytes).and_then(|size| size.checked_add(chunk.content.len())).is_none_or(|size| size > MAX_RESPONSE_BYTES) {
                                Some(ModelStreamInterruption::OutputLimit { limit_bytes: MAX_RESPONSE_BYTES })
                            } else { None };
                            if let Some(reason) = rejected {
                                yield Err(record_stream_uncertainty(&mut journal, id, reason, &partial, persisted).await);
                                return;
                            }
                            next_sequence = match next_sequence.checked_add(1) {
                                Some(next) => next,
                                None => { yield Err(record_stream_uncertainty(&mut journal, id, ModelStreamInterruption::SequenceExhausted, &partial, persisted).await); return; }
                            };
                            had_chunks = true;
                            partial.push_str(&chunk.content);
                            let unsaved = partial.len() - persisted;
                            if unsaved > 0 && (persisted == 0 || unsaved >= MODEL_STREAM_MAX_CHUNK_BYTES) {
                                if let Err(error) = journal.checkpoint(id, &partial[persisted..], now_ms()).await {
                                    yield Err(failure(format!("model output checkpoint could not be persisted: {error}")));
                                    return;
                                }
                                persisted = partial.len();
                                deadline = None;
                            } else if unsaved > 0 && deadline.is_none() {
                                deadline = Some(tokio::time::Instant::now() + CHECKPOINT_INTERVAL);
                            }
                            yield Ok(ModelStreamEvent::Chunk(chunk));
                        }
                        Ok(ModelStreamEvent::Thinking(chunk)) => {
                            let rejected = if chunk.sequence != next_thinking_sequence {
                                Some(ModelStreamInterruption::InvalidSequence { expected: next_thinking_sequence, actual: chunk.sequence })
                            } else if chunk.content.is_empty() {
                                Some(ModelStreamInterruption::InvalidCompletion)
                            } else if chunk.content.len() > MODEL_STREAM_MAX_CHUNK_BYTES {
                                Some(ModelStreamInterruption::ChunkTooLarge { limit_bytes: MODEL_STREAM_MAX_CHUNK_BYTES, actual_bytes: chunk.content.len() })
                            } else if partial.len().checked_add(thinking_bytes).and_then(|size| size.checked_add(chunk.content.len())).is_none_or(|size| size > MAX_RESPONSE_BYTES) {
                                Some(ModelStreamInterruption::OutputLimit { limit_bytes: MAX_RESPONSE_BYTES })
                            } else { None };
                            if let Some(reason) = rejected {
                                yield Err(record_stream_uncertainty(&mut journal, id, reason, &partial, persisted).await);
                                return;
                            }
                            next_thinking_sequence = match next_thinking_sequence.checked_add(1) {
                                Some(next) => next,
                                None => { yield Err(record_stream_uncertainty(&mut journal, id, ModelStreamInterruption::SequenceExhausted, &partial, persisted).await); return; }
                            };
                            if persisted < partial.len() {
                                if let Err(error) = journal.checkpoint(id, &partial[persisted..], now_ms()).await {
                                    yield Err(failure(format!("model output checkpoint could not be persisted: {error}")));
                                    return;
                                }
                                persisted = partial.len();
                                deadline = None;
                            }
                            if let Err(error) = journal.checkpoint_thinking(id, &chunk.content, now_ms()).await {
                                yield Err(failure(format!("thinking checkpoint could not be persisted: {error}")));
                                return;
                            }
                            thinking_bytes += chunk.content.len();
                            yield Ok(ModelStreamEvent::Thinking(chunk));
                        }
                        Ok(ModelStreamEvent::Completed { response }) => {
                            let assistant = response.choices.iter().find(|choice| choice.message.role == ModelGatewayMessageRole::Assistant);
                            let any_oversized = response.choices.iter().filter(|choice| choice.message.role == ModelGatewayMessageRole::Assistant).any(|choice| {
                                choice.message.content.len() > MAX_RESPONSE_BYTES || (!had_chunks && thinking_bytes.checked_add(choice.message.content.len()).is_none_or(|size| size > MAX_RESPONSE_BYTES))
                            });
                            let heap_rejection = completion_heap_limit(&response);
                            let rejected = match assistant {
                                _ if heap_rejection.is_some() => heap_rejection,
                                _ if any_oversized => Some(ModelStreamInterruption::OutputLimit { limit_bytes: MAX_RESPONSE_BYTES }),
                                Some(choice) if had_chunks && choice.message.content != partial => Some(ModelStreamInterruption::InvalidCompletion),
                                None => Some(ModelStreamInterruption::InvalidCompletion),
                                _ => None,
                            };
                            if let Some(reason) = rejected {
                                yield Err(record_stream_uncertainty(&mut journal, id, reason, &partial, persisted).await);
                                return;
                            }
                            match journal.finish(id, ConversationTerminalObservation::Completed { response: response.clone() }, now_ms()).await {
                                Ok(()) => yield Ok(ModelStreamEvent::Completed { response }),
                                Err(error) => yield Err(failure(format!("model completion could not be persisted: {error}"))),
                            }
                            return;
                        }
                        Ok(event) => yield Ok(event),
                        Err(ModelGatewayError::StreamInterrupted(reason)) => { yield Err(record_stream_uncertainty(&mut journal, id, reason, &partial, persisted).await); return; }
                        Err(_) => { yield Err(record_stream_uncertainty(&mut journal, id, ModelStreamInterruption::UnclassifiedGatewayFailure, &partial, persisted).await); return; }
                    }
                }
            };
            Ok(Box::pin(output) as ModelGatewayEventStream)
        })
    }
}
