//! Request envelope for Marlin-owned model stream calls.

use marlin_agent_protocol::ModelEndpoint;

use crate::{ChatMessage, CompletionOptions, ModelStreamTransport};

/// Marlin-owned model stream request envelope.
pub struct ModelStreamRequest {
    endpoint: ModelEndpoint,
    messages: Vec<ChatMessage>,
    options: Option<CompletionOptions>,
    transport: ModelStreamTransport,
}

impl ModelStreamRequest {
    /// Creates a stream request that lets the gateway choose transport.
    pub fn new(endpoint: ModelEndpoint, messages: Vec<ChatMessage>) -> Self {
        Self {
            endpoint,
            messages,
            options: None,
            transport: ModelStreamTransport::Auto,
        }
    }

    /// Attaches LiteLLM completion options to this request.
    pub fn with_options(mut self, options: CompletionOptions) -> Self {
        self.options = Some(options);
        self
    }

    /// Selects the requested transport policy.
    pub fn with_transport(mut self, transport: ModelStreamTransport) -> Self {
        self.transport = transport;
        self
    }

    /// Returns the model endpoint.
    pub fn endpoint(&self) -> &ModelEndpoint {
        &self.endpoint
    }

    /// Returns the chat messages.
    pub fn messages(&self) -> &[ChatMessage] {
        &self.messages
    }

    /// Returns the completion options.
    pub fn options(&self) -> Option<&CompletionOptions> {
        self.options.as_ref()
    }

    /// Returns the requested transport policy.
    pub fn transport(&self) -> &ModelStreamTransport {
        &self.transport
    }

    /// Consumes the request into gateway-ready parts.
    pub fn into_parts(
        self,
    ) -> (
        ModelEndpoint,
        Vec<ChatMessage>,
        Option<CompletionOptions>,
        ModelStreamTransport,
    ) {
        (self.endpoint, self.messages, self.options, self.transport)
    }
}
