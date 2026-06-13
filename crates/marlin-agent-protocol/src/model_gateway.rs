//! Model gateway protocol contracts independent of concrete LLM providers.

use std::{error::Error, fmt, future::Future, pin::Pin};

use serde::{Deserialize, Serialize};

use crate::{ModelEndpoint, ModelEndpointContractError};

/// Boxed asynchronous model gateway call.
pub type ModelGatewayFuture<T> = Pin<Box<dyn Future<Output = T> + Send + 'static>>;

/// Result returned by a model gateway implementation.
pub type ModelGatewayResult<T> = Result<T, ModelGatewayError>;

/// Runtime-visible model gateway abstraction.
pub trait ModelGateway: Send + Sync + 'static {
    /// Completes a request through a concrete gateway implementation.
    fn complete(
        &self,
        request: ModelGatewayRequest,
    ) -> ModelGatewayFuture<ModelGatewayResult<ModelGatewayCompletionResponse>>;
}

/// Transport selected by a gateway implementation.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum ModelGatewayTransport {
    /// Let the gateway choose its best transport.
    #[default]
    Auto,
    /// Server-sent events transport.
    Sse,
    /// WebSocket transport.
    WebSocket,
}

/// Marlin-owned model message role.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ModelGatewayMessageRole {
    System,
    Developer,
    User,
    Assistant,
    Tool,
    Function,
}

/// Marlin-owned model message.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ModelGatewayMessage {
    pub role: ModelGatewayMessageRole,
    pub content: String,
    pub name: Option<String>,
}

impl ModelGatewayMessage {
    /// Creates a model gateway message.
    pub fn new(role: ModelGatewayMessageRole, content: impl Into<String>) -> Self {
        Self {
            role,
            content: content.into(),
            name: None,
        }
    }

    /// Attaches a sender name to this message.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }
}

/// Creates a system message.
pub fn system_gateway_message(content: impl Into<String>) -> ModelGatewayMessage {
    ModelGatewayMessage::new(ModelGatewayMessageRole::System, content)
}

/// Creates a user message.
pub fn user_gateway_message(content: impl Into<String>) -> ModelGatewayMessage {
    ModelGatewayMessage::new(ModelGatewayMessageRole::User, content)
}

/// Creates an assistant message.
pub fn assistant_gateway_message(content: impl Into<String>) -> ModelGatewayMessage {
    ModelGatewayMessage::new(ModelGatewayMessageRole::Assistant, content)
}

/// Provider-neutral completion options used by runtime-facing gateway calls.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ModelGatewayCompletionOptions {
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub top_p: Option<f32>,
    pub stop: Option<Vec<String>>,
    pub user: Option<String>,
    pub seed: Option<i32>,
}

/// Runtime-facing model gateway request envelope.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ModelGatewayRequest {
    endpoint: ModelEndpoint,
    messages: Vec<ModelGatewayMessage>,
    options: Option<ModelGatewayCompletionOptions>,
    transport: ModelGatewayTransport,
}

impl ModelGatewayRequest {
    /// Creates a gateway request that lets the implementation choose transport.
    pub fn new(endpoint: ModelEndpoint, messages: Vec<ModelGatewayMessage>) -> Self {
        Self {
            endpoint,
            messages,
            options: None,
            transport: ModelGatewayTransport::Auto,
        }
    }

    /// Attaches completion options to this request.
    pub fn with_options(mut self, options: ModelGatewayCompletionOptions) -> Self {
        self.options = Some(options);
        self
    }

    /// Selects the requested transport policy.
    pub fn with_transport(mut self, transport: ModelGatewayTransport) -> Self {
        self.transport = transport;
        self
    }

    /// Returns the model endpoint.
    pub fn endpoint(&self) -> &ModelEndpoint {
        &self.endpoint
    }

    /// Returns the gateway messages.
    pub fn messages(&self) -> &[ModelGatewayMessage] {
        &self.messages
    }

    /// Returns the completion options.
    pub fn options(&self) -> Option<&ModelGatewayCompletionOptions> {
        self.options.as_ref()
    }

    /// Returns the requested transport policy.
    pub fn transport(&self) -> &ModelGatewayTransport {
        &self.transport
    }

    /// Consumes the request into gateway-ready parts.
    pub fn into_parts(
        self,
    ) -> (
        ModelEndpoint,
        Vec<ModelGatewayMessage>,
        Option<ModelGatewayCompletionOptions>,
        ModelGatewayTransport,
    ) {
        (self.endpoint, self.messages, self.options, self.transport)
    }
}

/// One provider-neutral completion choice.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ModelGatewayCompletionChoice {
    pub index: u32,
    pub message: ModelGatewayMessage,
    pub finish_reason: Option<String>,
}

impl ModelGatewayCompletionChoice {
    /// Creates a provider-neutral completion choice.
    pub fn new(index: u32, message: ModelGatewayMessage, finish_reason: Option<String>) -> Self {
        Self {
            index,
            message,
            finish_reason,
        }
    }
}

/// Provider-neutral completion response.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ModelGatewayCompletionResponse {
    pub id: String,
    pub model: String,
    pub choices: Vec<ModelGatewayCompletionChoice>,
}

impl ModelGatewayCompletionResponse {
    /// Creates a provider-neutral completion response.
    pub fn new(
        id: impl Into<String>,
        model: impl Into<String>,
        choices: Vec<ModelGatewayCompletionChoice>,
    ) -> Self {
        Self {
            id: id.into(),
            model: model.into(),
            choices,
        }
    }
}

/// Failure returned by runtime-facing model gateway calls.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModelGatewayError {
    EndpointContract(ModelEndpointContractError),
    Completion(String),
}

impl fmt::Display for ModelGatewayError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EndpointContract(source) => {
                write!(formatter, "invalid model route endpoint: {source}")
            }
            Self::Completion(message) => {
                write!(formatter, "model gateway completion failed: {message}")
            }
        }
    }
}

impl Error for ModelGatewayError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::EndpointContract(source) => Some(source),
            Self::Completion(_) => None,
        }
    }
}
