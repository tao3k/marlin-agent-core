//! Gateway traits and the default `LiteLLM` stream adapter.

use async_trait::async_trait;

use crate::{CompletionResponse, LiteLlmModelClient, LiteLlmModelClientResult, ModelStreamRequest};

/// Gateway abstraction for Marlin-owned model stream requests.
#[async_trait]
pub trait ModelStreamGateway: Send + Sync {
    /// Completes a request through the gateway implementation.
    async fn complete(
        &self,
        request: ModelStreamRequest,
    ) -> LiteLlmModelClientResult<CompletionResponse>;
}

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

#[async_trait]
impl ModelStreamGateway for LiteLlmStreamGateway {
    async fn complete(
        &self,
        request: ModelStreamRequest,
    ) -> LiteLlmModelClientResult<CompletionResponse> {
        let (endpoint, messages, options, _transport) = request.into_parts();
        self.client.complete(&endpoint, messages, options).await
    }
}
