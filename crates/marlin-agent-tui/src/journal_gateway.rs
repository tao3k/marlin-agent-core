// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Asynchronous admission and terminal observations for a streaming turn.

use std::{
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use marlin_agent_protocol::{
    ModelGatewayError, ModelGatewayFuture, ModelGatewayMessageRole, ModelGatewayRequest,
    ModelGatewayResult, ModelStreamInterruption,
};
use marlin_agent_runtime::{ModelToolOffer, ModelToolOutcome, RuntimeContext};
use marlin_agent_sessions::{
    ConversationJournal, ConversationPendingToolRequest, ConversationTerminalObservation,
    ConversationToolCallResult, ConversationToolContinuationStart, ConversationToolResultOutcome,
    ConversationToolResultsReceipt, ConversationToolReviewDecision, ConversationToolReviewReceipt,
    ConversationToolReviewRequest, ConversationTurnId, ConversationTurnStart,
    ConversationUncertaintyReason,
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
    tool_offer: Option<Arc<ModelToolOffer>>,
}

/// A live Ask gate bound to the same journal and immutable offer as a request.
pub(crate) struct JournalToolReview<S> {
    journal: Arc<Mutex<ConversationJournal<S>>>,
    offer: Arc<ModelToolOffer>,
}

impl<S> Clone for JournalToolReview<S> {
    fn clone(&self) -> Self {
        Self {
            journal: Arc::clone(&self.journal),
            offer: Arc::clone(&self.offer),
        }
    }
}

pub(crate) enum JournalToolReviewOutcome {
    Rejected(ConversationToolReviewReceipt),
    Results(ConversationToolResultsReceipt),
}

impl<G, S> JournalGateway<G, S> {
    pub(crate) fn new(gateway: G, journal: ConversationJournal<S>) -> Self {
        Self {
            gateway: Arc::new(gateway),
            journal: Arc::new(Mutex::new(journal)),
            tool_offer: None,
        }
    }

    pub(crate) fn with_tool_offer(mut self, offer: ModelToolOffer) -> Self {
        self.tool_offer = Some(Arc::new(offer));
        self
    }

    pub(crate) fn tool_review_handle(&self) -> Option<JournalToolReview<S>> {
        Some(JournalToolReview {
            journal: Arc::clone(&self.journal),
            offer: Arc::clone(self.tool_offer.as_ref()?),
        })
    }
}

impl<S: SessionEventStore> JournalToolReview<S> {
    pub(crate) fn try_pending(&self) -> Option<ConversationPendingToolRequest> {
        self.journal
            .try_lock()
            .ok()?
            .pending_tool_request()
            .cloned()
    }

    pub(crate) async fn decide(
        &self,
        request: ConversationToolReviewRequest,
        context: RuntimeContext,
    ) -> Result<JournalToolReviewOutcome, ModelGatewayError> {
        let mut journal = self.journal.lock().await;
        let approval = journal
            .review_tool_request(request, now_ms())
            .await
            .map_err(failure)?;
        if approval.decision() == ConversationToolReviewDecision::Reject {
            return Ok(JournalToolReviewOutcome::Rejected(approval));
        }
        let claim = journal
            .claim_tool_execution(&approval, now_ms())
            .await
            .map_err(failure)?;
        let key = claim.key();
        drop(journal);
        let outcomes = self
            .offer
            .execute_claim(claim, context)
            .await
            .map_err(|error| {
                failure(format!("tool offer rejected the durable claim: {error:?}"))
            })?;
        let results = outcomes
            .into_iter()
            .map(|(id, outcome)| ConversationToolCallResult {
                id,
                outcome: match outcome {
                    ModelToolOutcome::Completed(output) => {
                        ConversationToolResultOutcome::Completed(output)
                    }
                    ModelToolOutcome::Failed(error) => ConversationToolResultOutcome::Failed(error),
                },
            })
            .collect();
        let receipt = self
            .journal
            .lock()
            .await
            .record_tool_results(key, results, now_ms())
            .await
            .map_err(failure)?;
        Ok(JournalToolReviewOutcome::Results(receipt))
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
        let tool_offer = self.tool_offer.clone();
        Box::pin(async move {
            let offered_tools = request
                .options()
                .and_then(|options| options.tools.as_ref())
                .map(Vec::as_slice);
            match (tool_offer.as_ref(), offered_tools) {
                (None, None) => {}
                (Some(offer), expected) if offer.matches_definitions(expected) => {}
                _ => {
                    return Err(ModelGatewayError::RequestNotDispatched(
                        "model tool offer does not match the runtime handler catalog".into(),
                    ));
                }
            }
            let mut journal = journal.lock_owned().await;
            let id = match request.messages().last().map(|message| &message.role) {
                Some(ModelGatewayMessageRole::User) => {
                    let start = ConversationTurnStart {
                        endpoint: request.endpoint().clone(),
                        user_text: request
                            .messages()
                            .last()
                            .expect("matched user")
                            .content
                            .clone(),
                    };
                    journal.start(start, now_ms()).await.map_err(failure)?
                }
                Some(ModelGatewayMessageRole::Tool) => {
                    let pending = journal.pending_tool_request().ok_or_else(|| {
                        ModelGatewayError::RequestNotDispatched("no committed tool results".into())
                    })?;
                    let continuation_messages =
                        pending.continuation_messages().ok_or_else(|| {
                            ModelGatewayError::RequestNotDispatched(
                                "tool review has no committed continuation".into(),
                            )
                        })?;
                    let preceding = request
                        .messages()
                        .strip_suffix(continuation_messages.as_slice())
                        .ok_or_else(|| {
                            ModelGatewayError::RequestNotDispatched(
                                "tool continuation does not match the committed review".into(),
                            )
                        })?;
                    if request.endpoint() != pending.endpoint()
                        || preceding.last() != Some(pending.assistant())
                    {
                        return Err(ModelGatewayError::RequestNotDispatched(
                            "tool continuation route or assistant request differs from the journal"
                                .into(),
                        ));
                    }
                    let start = ConversationToolContinuationStart {
                        endpoint: request.endpoint().clone(),
                        source: pending.key(),
                    };
                    journal
                        .start_tool_continuation(start, now_ms())
                        .await
                        .map_err(failure)?
                }
                _ => {
                    return Err(ModelGatewayError::RequestNotDispatched(
                        "conversation request must end with user input or committed tool results"
                            .into(),
                    ));
                }
            };
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
                        Ok(event @ (ModelStreamEvent::Completed { .. } | ModelStreamEvent::ToolRequested { .. })) => {
                            let (response, tool_requested) = match &event {
                                ModelStreamEvent::Completed { response } => (response, false),
                                ModelStreamEvent::ToolRequested { response } => (response, true),
                                _ => unreachable!("terminal event pattern"),
                            };
                            let assistant = response.choices.iter().find(|choice| choice.message.role == ModelGatewayMessageRole::Assistant);
                            let any_oversized = response.choices.iter().filter(|choice| choice.message.role == ModelGatewayMessageRole::Assistant).any(|choice| {
                                choice.message.content.len() > MAX_RESPONSE_BYTES || (!had_chunks && thinking_bytes.checked_add(choice.message.content.len()).is_none_or(|size| size > MAX_RESPONSE_BYTES))
                            });
                            let heap_rejection = completion_heap_limit(response);
                            let rejected = match assistant {
                                _ if heap_rejection.is_some() => heap_rejection,
                                _ if any_oversized => Some(ModelStreamInterruption::OutputLimit { limit_bytes: MAX_RESPONSE_BYTES }),
                                Some(choice) if tool_requested && (response.choices.len() != 1 || choice.message.tool_calls.is_empty()) => Some(ModelStreamInterruption::InvalidCompletion),
                                Some(choice) if tool_requested && tool_offer.as_ref().is_none_or(|offer| offer.admit_calls(&choice.message.tool_calls).is_err()) => Some(ModelStreamInterruption::InvalidCompletion),
                                Some(choice) if !tool_requested && !choice.message.tool_calls.is_empty() => Some(ModelStreamInterruption::InvalidCompletion),
                                Some(choice) if had_chunks && choice.message.content != partial => Some(ModelStreamInterruption::InvalidCompletion),
                                None => Some(ModelStreamInterruption::InvalidCompletion),
                                _ => None,
                            };
                            if let Some(reason) = rejected {
                                yield Err(record_stream_uncertainty(&mut journal, id, reason, &partial, persisted).await);
                                return;
                            }
                            let observation = if tool_requested {
                                ConversationTerminalObservation::ToolRequested { response: response.clone() }
                            } else {
                                ConversationTerminalObservation::Completed { response: response.clone() }
                            };
                            match journal.finish(id, observation, now_ms()).await {
                                Ok(()) => yield Ok(event),
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
