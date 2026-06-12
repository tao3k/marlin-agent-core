//! Deterministic scripted model stream support for tests.

use std::collections::VecDeque;
use std::fmt::{self, Formatter};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use marlin_agent_stream::{
    ChunkGate, CompletionResponse, LiteLlmModelClientError, LiteLlmModelClientResult,
    ModelStreamChunk, ModelStreamEvent, ModelStreamGateway, ModelStreamRequest,
    ModelStreamTransport,
};

/// Scripted stream that emits `ModelStreamEvent` values in order.
#[derive(Clone, Debug)]
pub struct ScriptedModelStream {
    events: Vec<ModelStreamEvent>,
    chunk_gate: Option<ChunkGate>,
}

impl ScriptedModelStream {
    /// Creates a scripted stream from an explicit event list.
    pub fn new(events: impl IntoIterator<Item = ModelStreamEvent>) -> Self {
        Self {
            events: events.into_iter().collect(),
            chunk_gate: None,
        }
    }

    /// Creates a one-chunk text stream with `Auto` transport.
    pub fn single_text_delta(content: impl Into<String>) -> Self {
        Self::new([
            ModelStreamEvent::Started {
                transport: ModelStreamTransport::Auto,
            },
            ModelStreamEvent::Chunk(ModelStreamChunk::new(1, content)),
            ModelStreamEvent::Completed { stop_reason: None },
        ])
    }

    /// Gates every scripted chunk event through the provided gate.
    pub fn with_chunk_gate(mut self, chunk_gate: ChunkGate) -> Self {
        self.chunk_gate = Some(chunk_gate);
        self
    }

    /// Collects this script into a receipt, waiting at each gated chunk.
    pub async fn collect(self) -> ScriptedStreamReceipt {
        let mut emitted_events = Vec::with_capacity(self.events.len());
        let mut chunk_count = 0;
        let mut gate_sequences = Vec::new();
        let mut completed = false;
        let mut failed = false;

        for event in self.events {
            if matches!(event, ModelStreamEvent::Chunk(_)) {
                if let Some(chunk_gate) = &self.chunk_gate {
                    gate_sequences.push(chunk_gate.wait_for_next().await.sequence());
                }
                chunk_count += 1;
            }
            completed |= matches!(event, ModelStreamEvent::Completed { .. });
            failed |= matches!(event, ModelStreamEvent::Failed { .. });
            emitted_events.push(event);
        }

        ScriptedStreamReceipt {
            events: emitted_events,
            chunk_count,
            gate_sequences,
            completed,
            failed,
        }
    }
}

/// Receipt produced after collecting a scripted stream.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScriptedStreamReceipt {
    /// Events emitted by the scripted stream.
    pub events: Vec<ModelStreamEvent>,
    /// Number of chunk events emitted by the scripted stream.
    pub chunk_count: usize,
    /// Gate permit sequences observed while releasing chunk events.
    pub gate_sequences: Vec<u64>,
    /// Whether the script emitted a completion event.
    pub completed: bool,
    /// Whether the script emitted a failure event.
    pub failed: bool,
}

/// Request receipt captured by `ScriptedModelGateway`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScriptedGatewayRequestReceipt {
    /// LiteLLM model id projected from the requested endpoint.
    pub litellm_model_id: String,
    /// Number of messages sent to the gateway.
    pub message_count: usize,
    /// Whether the request supplied completion options.
    pub has_options: bool,
    /// Requested transport policy.
    pub transport: ModelStreamTransport,
}

impl ScriptedGatewayRequestReceipt {
    fn from_request(request: &ModelStreamRequest) -> Self {
        Self {
            litellm_model_id: request.endpoint().litellm_model_id().as_str().to_owned(),
            message_count: request.messages().len(),
            has_options: request.options().is_some(),
            transport: request.transport().clone(),
        }
    }
}

/// Deterministic `ModelStreamGateway` double that records requests and returns queued outcomes.
#[derive(Clone)]
pub struct ScriptedModelGateway {
    outcomes: Arc<Mutex<VecDeque<LiteLlmModelClientResult<CompletionResponse>>>>,
    requests: Arc<Mutex<Vec<ScriptedGatewayRequestReceipt>>>,
}

impl fmt::Debug for ScriptedModelGateway {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ScriptedModelGateway")
            .field("queued_outcomes", &self.queued_outcomes())
            .field("request_count", &self.requests().len())
            .finish()
    }
}

impl ScriptedModelGateway {
    /// Creates a scripted gateway from explicit completion outcomes.
    pub fn new(
        outcomes: impl IntoIterator<Item = LiteLlmModelClientResult<CompletionResponse>>,
    ) -> Self {
        Self {
            outcomes: Arc::new(Mutex::new(outcomes.into_iter().collect())),
            requests: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Creates a gateway with no queued completion outcomes.
    pub fn empty() -> Self {
        Self::new(Vec::new())
    }

    /// Creates a gateway that returns one deterministic completion failure.
    pub fn completion_failure(message: impl Into<String>) -> Self {
        Self::new([Err(LiteLlmModelClientError::Completion(message.into()))])
    }

    /// Returns captured request receipts.
    pub fn requests(&self) -> Vec<ScriptedGatewayRequestReceipt> {
        self.requests
            .lock()
            .expect("scripted gateway request lock poisoned")
            .clone()
    }

    /// Returns the number of queued outcomes not yet consumed.
    pub fn queued_outcomes(&self) -> usize {
        self.outcomes
            .lock()
            .expect("scripted gateway outcome lock poisoned")
            .len()
    }
}

#[async_trait]
impl ModelStreamGateway for ScriptedModelGateway {
    async fn complete(
        &self,
        request: ModelStreamRequest,
    ) -> LiteLlmModelClientResult<CompletionResponse> {
        self.requests
            .lock()
            .expect("scripted gateway request lock poisoned")
            .push(ScriptedGatewayRequestReceipt::from_request(&request));

        self.outcomes
            .lock()
            .expect("scripted gateway outcome lock poisoned")
            .pop_front()
            .unwrap_or_else(|| {
                Err(LiteLlmModelClientError::Completion(
                    "scripted model gateway has no queued completion outcome".to_owned(),
                ))
            })
    }
}
