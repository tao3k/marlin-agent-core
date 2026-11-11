// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Incremental, provider-neutral model gateway events.

use std::{num::NonZeroUsize, pin::Pin};

use marlin_agent_protocol::{
    ModelGatewayCompletionChoice, ModelGatewayCompletionResponse, ModelGatewayError,
    ModelGatewayFuture, ModelGatewayMessage, ModelGatewayMessageRole, ModelGatewayRequest,
    ModelGatewayResult, ModelGatewayTransport, ModelStreamInterruption,
};
use tokio_stream::{Stream, StreamExt};
use weaver_llm::{Request, TurnEvent};

use crate::{
    MODEL_STREAM_MAX_CHUNK_BYTES, ModelStreamChunk, ModelStreamEvent, ModelThinkingChunk,
    WeaverModelGateway,
    gateway::{provider_continuation, request_message},
};

/// A pull-driven model stream. Dropping it cancels the underlying exchange.
pub type ModelGatewayEventStream =
    Pin<Box<dyn Stream<Item = ModelGatewayResult<ModelStreamEvent>> + Send + 'static>>;

/// Maximum cumulative UTF-8 bytes retained for an incremental text response.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModelStreamOutputLimit(NonZeroUsize);

impl ModelStreamOutputLimit {
    /// Creates a positive per-turn text limit.
    pub fn new(bytes: NonZeroUsize) -> Self {
        Self(bytes)
    }

    /// Returns the maximum accumulated text size in bytes.
    pub fn bytes(self) -> usize {
        self.0.get()
    }
}

/// Opens a typed incremental model turn without exposing provider wire events.
/// Implementations must emit UTF-8 chunks no larger than
/// [`MODEL_STREAM_MAX_CHUNK_BYTES`], in order and with consecutive sequences.
pub trait ModelStreamingGateway: Send + Sync + 'static {
    /// Open the request. Stream failures are returned as stream items.
    fn stream(
        &self,
        request: ModelGatewayRequest,
    ) -> ModelGatewayFuture<ModelGatewayResult<ModelGatewayEventStream>>;
}

impl ModelStreamingGateway for WeaverModelGateway {
    fn stream(
        &self,
        request: ModelGatewayRequest,
    ) -> ModelGatewayFuture<ModelGatewayResult<ModelGatewayEventStream>> {
        let gateway = self.clone();
        Box::pin(async move {
            let (endpoint, messages, options, transport) = request.into_parts();
            if transport == ModelGatewayTransport::WebSocket {
                return Err(ModelGatewayError::RequestNotDispatched(
                    "the configured gateway uses HTTP or SSE".into(),
                ));
            }
            let messages: Vec<_> = messages
                .into_iter()
                .map(|message| request_message(message, &endpoint, gateway.destination()))
                .collect::<ModelGatewayResult<_>>()?;
            let options = gateway.options(options);
            let request = Request {
                model: endpoint.model.as_str(),
                messages: &messages,
                options: &options,
            };
            let turn = gateway.stream_turn(&endpoint, &request).await?;
            let output_limit = gateway.stream_output_limit();
            let output = async_stream::try_stream! {
                yield ModelStreamEvent::Started { transport: ModelGatewayTransport::Sse };
                let mut events = std::pin::pin!(turn.into_stream());
                let mut text = String::new();
                let mut sequence = 0u64;
                let mut thinking_sequence = 0u64;
                let mut thinking_bytes = 0usize;
                let mut completed = false;
                while let Some(event) = events.next().await {
                    match event.map_err(stream_event_error)? {
                        TurnEvent::Text(delta) => {
                            let mut start = 0;
                            while start < delta.len() {
                                let mut end = start.saturating_add(MODEL_STREAM_MAX_CHUNK_BYTES).min(delta.len());
                                while !delta.is_char_boundary(end) { end -= 1; }
                                let part = &delta[start..end];
                                if let Some(limit) = output_limit
                                    && text.len().checked_add(thinking_bytes).and_then(|size| size.checked_add(part.len())).is_none_or(|size| size > limit.bytes()) {
                                    Err(ModelGatewayError::StreamInterrupted(ModelStreamInterruption::OutputLimit { limit_bytes: limit.bytes() }))?;
                                }
                                text.push_str(part);
                                sequence = sequence.checked_add(1).ok_or(ModelGatewayError::StreamInterrupted(ModelStreamInterruption::SequenceExhausted))?;
                                yield ModelStreamEvent::Chunk(ModelStreamChunk::new(sequence, part));
                                start = end;
                            }
                        }
                        TurnEvent::Thinking(delta) => {
                            let delta = delta.as_str();
                            let mut start = 0;
                            while start < delta.len() {
                                let mut end = start.saturating_add(MODEL_STREAM_MAX_CHUNK_BYTES).min(delta.len());
                                while !delta.is_char_boundary(end) { end -= 1; }
                                let part = &delta[start..end];
                                if let Some(limit) = output_limit
                                    && text.len().checked_add(thinking_bytes).and_then(|size| size.checked_add(part.len())).is_none_or(|size| size > limit.bytes()) {
                                    Err(ModelGatewayError::StreamInterrupted(ModelStreamInterruption::OutputLimit { limit_bytes: limit.bytes() }))?;
                                }
                                thinking_bytes += part.len();
                                thinking_sequence = thinking_sequence.checked_add(1).ok_or(ModelGatewayError::StreamInterrupted(ModelStreamInterruption::SequenceExhausted))?;
                                yield ModelStreamEvent::Thinking(ModelThinkingChunk::new(thinking_sequence, part));
                                start = end;
                            }
                        }
                        TurnEvent::Finished { reason, identity: Some(identity), tool_calls, responses_output, anthropic_output, .. }
                            if tool_calls.is_empty() => {
                                let mut message = ModelGatewayMessage::new(ModelGatewayMessageRole::Assistant, std::mem::take(&mut text));
                                message.continuation = provider_continuation(&endpoint, gateway.destination(), responses_output, anthropic_output)?;
                                let choice = ModelGatewayCompletionChoice::new(0, message, Some(reason));
                                yield ModelStreamEvent::Completed {
                                    response: ModelGatewayCompletionResponse::new(identity.id, identity.model, vec![choice]),
                                };
                                completed = true;
                                break;
                            }
                        TurnEvent::Finished { .. } => {
                            Err(ModelGatewayError::StreamInterrupted(ModelStreamInterruption::InvalidCompletion))?;
                        }
                    }
                }
                if !completed {
                    Err(ModelGatewayError::StreamInterrupted(ModelStreamInterruption::EndedWithoutTerminal))?;
                }
            };
            Ok(Box::pin(output) as ModelGatewayEventStream)
        })
    }
}

fn stream_event_error(error: weaver_llm::Error) -> ModelGatewayError {
    match error {
        weaver_llm::Error::Transport => {
            ModelGatewayError::StreamInterrupted(ModelStreamInterruption::Transport)
        }
        weaver_llm::Error::Timeout => {
            ModelGatewayError::StreamInterrupted(ModelStreamInterruption::Timeout)
        }
        weaver_llm::Error::Response => {
            ModelGatewayError::StreamInterrupted(ModelStreamInterruption::InvalidResponse)
        }
        weaver_llm::Error::Limit => {
            ModelGatewayError::StreamInterrupted(ModelStreamInterruption::ProviderByteLimit)
        }
        other => ModelGatewayError::Completion(other.to_string()),
    }
}
