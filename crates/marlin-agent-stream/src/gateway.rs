//! Gateway traits and the default `LiteLLM` stream adapter.

pub use marlin_agent_protocol::ModelGateway as ModelStreamGateway;
use marlin_agent_protocol::{
    ModelGateway, ModelGatewayCompletionResponse, ModelGatewayFuture, ModelGatewayRequest,
    ModelGatewayResult,
};

use crate::LiteLlmModelClient;

/// Default stream gateway backed by `litellm-rs`.
#[derive(Clone, Debug, Default)]
pub struct LiteLlmStreamGateway {
    client: LiteLlmModelClient,
}

impl LiteLlmStreamGateway {
    /// Creates a LiteLLM-backed stream gateway.
    pub fn new() -> Self {
        Self {
            client: LiteLlmModelClient::new(),
        }
    }

    /// Returns the underlying LiteLLM client adapter.
    pub fn client(&self) -> &LiteLlmModelClient {
        &self.client
    }
}

impl ModelGateway for LiteLlmStreamGateway {
    fn complete(
        &self,
        request: ModelGatewayRequest,
    ) -> ModelGatewayFuture<ModelGatewayResult<ModelGatewayCompletionResponse>> {
        let client = self.client.clone();
        Box::pin(async move {
            let (endpoint, messages, options, _transport) = request.into_parts();
            client.complete(&endpoint, messages, options).await
        })
    }
}
