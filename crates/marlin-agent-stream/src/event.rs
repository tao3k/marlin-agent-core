// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Serializable Marlin model stream event contracts.

use marlin_agent_protocol::ModelGatewayCompletionResponse;
pub use marlin_agent_protocol::ModelGatewayTransport as ModelStreamTransport;
use serde::{Deserialize, Serialize};

/// Maximum UTF-8 payload bytes emitted in one gateway stream chunk.
pub const MODEL_STREAM_MAX_CHUNK_BYTES: usize = 64 * 1024;

/// One text fragment emitted by a model stream.
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

/// Provider-returned thinking fragment. Its sequence is independent of answer
/// chunks and debug formatting never exposes the text. A fragment is an
/// observation; it does not contain or authenticate a provider continuation.
#[derive(Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct ModelThinkingChunk {
    pub sequence: u64,
    pub content: String,
}

impl ModelThinkingChunk {
    pub fn new(sequence: u64, content: impl Into<String>) -> Self {
        Self {
            sequence,
            content: content.into(),
        }
    }
}

impl std::fmt::Debug for ModelThinkingChunk {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ModelThinkingChunk")
            .field("sequence", &self.sequence)
            .field("bytes", &self.content.len())
            .finish()
    }
}

/// Marlin-owned stream event contract independent of the gateway backend.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ModelStreamEvent {
    Started {
        transport: ModelStreamTransport,
    },
    Chunk(ModelStreamChunk),
    Thinking(ModelThinkingChunk),
    Completed {
        response: ModelGatewayCompletionResponse,
    },
}
