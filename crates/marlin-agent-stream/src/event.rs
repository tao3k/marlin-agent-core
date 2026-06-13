//! Serializable Marlin model stream event contracts.

pub use marlin_agent_protocol::ModelGatewayTransport as ModelStreamTransport;
use serde::{Deserialize, Serialize};

/// One text or tool-call payload fragment emitted by a model stream.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ModelStreamChunk {
    pub sequence: u64,
    pub content: String,
}

impl ModelStreamChunk {
    /// Creates a model stream chunk.
    pub fn new(sequence: u64, content: impl Into<String>) -> Self {
        Self {
            sequence,
            content: content.into(),
        }
    }
}

/// Marlin-owned stream event contract independent of the gateway backend.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ModelStreamEvent {
    Started { transport: ModelStreamTransport },
    Chunk(ModelStreamChunk),
    Completed { stop_reason: Option<String> },
    Failed { message: String },
}
