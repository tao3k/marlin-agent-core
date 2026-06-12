//! LiteLLM-backed model client adapter.

use std::error::Error;
use std::fmt::{self, Display, Formatter};

pub use litellm_rs::core::types::chat::ChatMessage;
pub use litellm_rs::{
    CompletionOptions, CompletionResponse, assistant_message, system_message, user_message,
};
use marlin_agent_protocol::{ModelEndpoint, ModelEndpointContractError};

/// Failure returned before or during a LiteLLM model completion.
#[derive(Debug, Eq, PartialEq)]
pub enum LiteLlmModelClientError {
    EndpointContract(ModelEndpointContractError),
    Completion(String),
}

impl Display for LiteLlmModelClientError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::EndpointContract(source) => {
                write!(formatter, "invalid model route endpoint: {source}")
            }
            Self::Completion(message) => write!(formatter, "litellm completion failed: {message}"),
        }
    }
}

impl Error for LiteLlmModelClientError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::EndpointContract(source) => Some(source),
            Self::Completion(_) => None,
        }
    }
}

/// Result returned by the LiteLLM model client adapter.
pub type LiteLlmModelClientResult<T> = Result<T, LiteLlmModelClientError>;

/// Thin adapter over `litellm-rs` chat completions.
#[derive(Clone, Debug, Default)]
pub struct LiteLlmModelClient;

impl LiteLlmModelClient {
    /// Creates a LiteLLM-backed model client adapter.
    pub fn new() -> Self {
        Self
    }

    /// Sends a chat completion request after validating the endpoint contract.
    pub async fn complete(
        &self,
        endpoint: &ModelEndpoint,
        messages: Vec<ChatMessage>,
        options: Option<CompletionOptions>,
    ) -> LiteLlmModelClientResult<CompletionResponse> {
        endpoint
            .validate_contract()
            .map_err(LiteLlmModelClientError::EndpointContract)?;
        let model_id = endpoint.litellm_model_id();
        litellm_rs::completion(model_id.as_str(), messages, options)
            .await
            .map_err(|source| LiteLlmModelClientError::Completion(source.to_string()))
    }
}
