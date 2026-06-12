//! Model stream protocol boundaries and LiteLLM gateway adapter.

mod chunk_gate;
mod event;
mod gateway;
mod litellm;
mod request;

pub use chunk_gate::{ChunkGate, ChunkGatePermit};
pub use event::{ModelStreamChunk, ModelStreamEvent, ModelStreamTransport};
pub use gateway::{LiteLlmStreamGateway, ModelStreamGateway};
pub use litellm::{
    ChatMessage, CompletionOptions, CompletionResponse, LiteLlmModelClient,
    LiteLlmModelClientError, LiteLlmModelClientResult, assistant_message, system_message,
    user_message,
};
pub use request::ModelStreamRequest;
