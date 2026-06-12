//! Serializable Marlin model stream event contracts.

use serde::{Deserialize, Serialize};

/// Stream transport selected by a gateway implementation.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum ModelStreamTransport {
    /// Let the gateway choose its best transport.
    #[default]
    Auto,
    /// Server-sent events transport.
    Sse,
    /// WebSocket transport.
    WebSocket,
}

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
