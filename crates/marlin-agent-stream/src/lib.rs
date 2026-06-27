//! Model stream protocol boundaries and LiteLLM gateway adapter.

mod chunk_gate;
mod event;
mod gateway;
mod litellm;
mod model_probe;
mod request;

pub use chunk_gate::{ChunkGate, ChunkGatePermit};
pub use event::{ModelStreamChunk, ModelStreamEvent, ModelStreamTransport};
pub use gateway::{LiteLlmStreamGateway, ModelStreamGateway};
pub use litellm::LiteLlmModelClient;
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
