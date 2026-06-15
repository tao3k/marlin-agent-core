//! Deterministic scripted model gateway and stream support for tests.

use std::collections::VecDeque;
use std::fmt::{self, Formatter};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use marlin_agent_harness_types::{AgentHarnessEvidence, AgentHarnessEvidenceKind};
use marlin_agent_protocol::{
    ModelGateway, ModelGatewayCompletionResponse, ModelGatewayError, ModelGatewayFuture,
    ModelGatewayRequest, ModelGatewayResult, ModelGatewayTransport,
};
use tokio::sync::Semaphore;

/// Stable denial returned when no-LLM tests attempt to cross the live gateway boundary.
pub const NO_LIVE_LLM_GATE_DENIAL_MESSAGE: &str =
    "live LLM gateway disabled by no-LLM runtime replay policy";

/// Deterministic test gate for scripted stream chunk delivery.
#[derive(Clone, Debug)]
pub struct ScriptedChunkGate {
    admitted_chunks: Arc<AtomicU64>,
    permits: Arc<Semaphore>,
}

impl Default for ScriptedChunkGate {
    fn default() -> Self {
        Self {
            admitted_chunks: Arc::new(AtomicU64::new(0)),
            permits: Arc::new(Semaphore::new(0)),
        }
    }
}

impl ScriptedChunkGate {
    /// Creates a gate with no chunk permits.
    pub fn closed() -> Self {
        Self::default()
    }

    /// Releases one pending chunk.
    pub fn release_next(&self) {
        self.permits.add_permits(1);
    }

    /// Releases several pending chunks.
    pub fn release_many(&self, permits: usize) {
        self.permits.add_permits(permits);
    }

    /// Waits until this gate admits the next chunk.
    pub async fn wait_for_next(&self) -> ScriptedChunkGatePermit {
        let permit = self
            .permits
            .acquire()
            .await
            .expect("scripted chunk gate semaphore is not externally closed");
        permit.forget();
        let sequence = self.admitted_chunks.fetch_add(1, Ordering::SeqCst) + 1;
        ScriptedChunkGatePermit { sequence }
    }

    /// Returns how many chunks have passed through this gate.
    pub fn admitted_chunks(&self) -> u64 {
        self.admitted_chunks.load(Ordering::SeqCst)
    }
}

/// Receipt returned when a chunk is admitted by a scripted gate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScriptedChunkGatePermit {
    sequence: u64,
}

impl ScriptedChunkGatePermit {
    /// Sequence number of the admitted chunk, starting at one.
    pub fn sequence(self) -> u64 {
        self.sequence
    }
}

/// One text or tool-call payload fragment emitted by a scripted model stream.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScriptedModelStreamChunk {
    pub sequence: u64,
    pub content: String,
}

impl ScriptedModelStreamChunk {
    /// Creates a scripted model stream chunk.
    pub fn new(sequence: u64, content: impl Into<String>) -> Self {
        Self {
            sequence,
            content: content.into(),
        }
    }
}

/// Provider-neutral scripted stream event.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ScriptedModelStreamEvent {
    Started { transport: ModelGatewayTransport },
    Chunk(ScriptedModelStreamChunk),
    Completed { stop_reason: Option<String> },
    Failed { message: String },
}

/// Scripted stream that emits `ScriptedModelStreamEvent` values in order.
#[derive(Clone, Debug)]
pub struct ScriptedModelStream {
    events: Vec<ScriptedModelStreamEvent>,
    chunk_gate: Option<ScriptedChunkGate>,
}

impl ScriptedModelStream {
    /// Creates a scripted stream from an explicit event list.
    pub fn new(events: impl IntoIterator<Item = ScriptedModelStreamEvent>) -> Self {
        Self {
            events: events.into_iter().collect(),
            chunk_gate: None,
        }
    }

    /// Creates a one-chunk text stream with `Auto` transport.
    pub fn single_text_delta(content: impl Into<String>) -> Self {
        Self::new([
            ScriptedModelStreamEvent::Started {
                transport: ModelGatewayTransport::Auto,
            },
            ScriptedModelStreamEvent::Chunk(ScriptedModelStreamChunk::new(1, content)),
            ScriptedModelStreamEvent::Completed { stop_reason: None },
        ])
    }

    /// Gates every scripted chunk event through the provided gate.
    pub fn with_chunk_gate(mut self, chunk_gate: ScriptedChunkGate) -> Self {
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
            if matches!(event, ScriptedModelStreamEvent::Chunk(_)) {
                if let Some(chunk_gate) = &self.chunk_gate {
                    gate_sequences.push(chunk_gate.wait_for_next().await.sequence());
                }
                chunk_count += 1;
            }
            completed |= matches!(event, ScriptedModelStreamEvent::Completed { .. });
            failed |= matches!(event, ScriptedModelStreamEvent::Failed { .. });
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
    pub events: Vec<ScriptedModelStreamEvent>,
    /// Number of chunk events emitted by the scripted stream.
    pub chunk_count: usize,
    /// Gate permit sequences observed while releasing chunk events.
    pub gate_sequences: Vec<u64>,
    /// Whether the script emitted a completion event.
    pub completed: bool,
    /// Whether the script emitted a failure event.
    pub failed: bool,
}

/// Project a scripted stream gate receipt into runtime evidence for harness tests.
pub fn scripted_stream_gate_evidence(
    stream_id: impl Into<String>,
    receipt: &ScriptedStreamReceipt,
    gate: &ScriptedChunkGate,
) -> AgentHarnessEvidence {
    let stream_id = stream_id.into();
    let gate_sequences = receipt
        .gate_sequences
        .iter()
        .map(u64::to_string)
        .collect::<Vec<_>>()
        .join(",");
    let detail = format!(
        "stream_id={} chunk_count={} gate_sequences=[{}] admitted_chunks={} completed={} failed={} live_llm=false",
        stream_id,
        receipt.chunk_count,
        gate_sequences,
        gate.admitted_chunks(),
        receipt.completed,
        receipt.failed,
    );

    AgentHarnessEvidence::present(
        AgentHarnessEvidenceKind::Runtime,
        format!("scripted-stream-gate:{stream_id}"),
    )
    .with_detail(detail)
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
    pub transport: ModelGatewayTransport,
}

impl ScriptedGatewayRequestReceipt {
    fn from_request(request: &ModelGatewayRequest) -> Self {
        Self {
            litellm_model_id: request.endpoint().litellm_model_id().as_str().to_owned(),
            message_count: request.messages().len(),
            has_options: request.options().is_some(),
            transport: request.transport().clone(),
        }
    }
}

/// Explicit no-LLM `ModelGateway` guard for replay and harness tests.
#[derive(Clone, Default)]
pub struct NoLiveLlmModelGateway {
    denied_requests: Arc<Mutex<Vec<ScriptedGatewayRequestReceipt>>>,
}

impl fmt::Debug for NoLiveLlmModelGateway {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NoLiveLlmModelGateway")
            .field("denied_request_count", &self.denied_requests().len())
            .finish()
    }
}

impl NoLiveLlmModelGateway {
    /// Creates a no-LLM gateway guard with no denied requests yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns captured request receipts denied by this guard.
    pub fn denied_requests(&self) -> Vec<ScriptedGatewayRequestReceipt> {
        self.denied_requests
            .lock()
            .expect("no-live-LLM gateway request lock poisoned")
            .clone()
    }
}

impl ModelGateway for NoLiveLlmModelGateway {
    fn complete(
        &self,
        request: ModelGatewayRequest,
    ) -> ModelGatewayFuture<ModelGatewayResult<ModelGatewayCompletionResponse>> {
        self.denied_requests
            .lock()
            .expect("no-live-LLM gateway request lock poisoned")
            .push(ScriptedGatewayRequestReceipt::from_request(&request));

        Box::pin(async {
            Err(ModelGatewayError::Completion(
                NO_LIVE_LLM_GATE_DENIAL_MESSAGE.to_owned(),
            ))
        })
    }
}

/// Deterministic `ModelGateway` double that records requests and returns queued outcomes.
#[derive(Clone)]
pub struct ScriptedModelGateway {
    outcomes: Arc<Mutex<VecDeque<ModelGatewayResult<ModelGatewayCompletionResponse>>>>,
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
        outcomes: impl IntoIterator<Item = ModelGatewayResult<ModelGatewayCompletionResponse>>,
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
        Self::new([Err(ModelGatewayError::Completion(message.into()))])
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

impl ModelGateway for ScriptedModelGateway {
    fn complete(
        &self,
        request: ModelGatewayRequest,
    ) -> ModelGatewayFuture<ModelGatewayResult<ModelGatewayCompletionResponse>> {
        self.requests
            .lock()
            .expect("scripted gateway request lock poisoned")
            .push(ScriptedGatewayRequestReceipt::from_request(&request));

        let outcome = self
            .outcomes
            .lock()
            .expect("scripted gateway outcome lock poisoned")
            .pop_front()
            .unwrap_or_else(|| {
                Err(ModelGatewayError::Completion(
                    "scripted model gateway has no queued completion outcome".to_owned(),
                ))
            });
        Box::pin(async move { outcome })
    }
}
