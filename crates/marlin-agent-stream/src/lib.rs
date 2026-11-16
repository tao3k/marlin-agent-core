// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Model stream protocol boundaries and Weaver gateway adapter.

mod chunk_gate;
mod event;
mod gateway;
mod model_probe;
mod request;
mod streaming;

pub use chunk_gate::{ChunkGate, ChunkGatePermit};
pub use event::{
    MODEL_STREAM_MAX_CHUNK_BYTES, ModelStreamChunk, ModelStreamEvent, ModelStreamTransport,
    ModelThinkingChunk,
};
pub use gateway::{ModelStreamGateway, WeaverModelGateway, matches_observed_anthropic_thinking};
pub use marlin_agent_protocol::{
    ModelGatewayCompletionChoice, ModelGatewayCompletionOptions, ModelGatewayCompletionResponse,
    ModelGatewayError, ModelGatewayMessage, ModelGatewayMessageRole, ModelGatewayResult,
    assistant_gateway_message, system_gateway_message, user_gateway_message,
};
pub use model_probe::{
    MODEL_NO_WRITE_PROBE_RECEIPT_SCHEMA_ID, ModelNoWriteProbeByteCount,
    ModelNoWriteProbeChoiceCount, ModelNoWriteProbeCompletionId, ModelNoWriteProbeContentDigest,
    ModelNoWriteProbeFailureMessage, ModelNoWriteProbeMarker, ModelNoWriteProbeReceipt,
    ModelNoWriteProbeRequest, ModelNoWriteProbeSchemaId, ModelNoWriteProbeStatus,
    run_model_no_write_probe,
};
pub use request::ModelStreamRequest;
pub use streaming::{ModelGatewayEventStream, ModelStreamOutputLimit, ModelStreamingGateway};
