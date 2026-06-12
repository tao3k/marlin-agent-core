//! Compatibility re-exports for the stream-owned LiteLLM adapter.

pub use marlin_agent_stream::{
    ChatMessage, CompletionOptions, CompletionResponse, LiteLlmModelClient,
    LiteLlmModelClientError, LiteLlmModelClientResult, assistant_message, system_message,
    user_message,
};
