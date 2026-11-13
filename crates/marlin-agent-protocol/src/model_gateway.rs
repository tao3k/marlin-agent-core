// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Model gateway protocol contracts independent of concrete LLM providers.

use std::{error::Error, fmt, future::Future, pin::Pin};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

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
}

/// Provider correlation identifier, distinct from a graph node identity.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ModelToolCallId(String);

impl TryFrom<String> for ModelToolCallId {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.is_empty() || value.len() > 256 || value.chars().any(char::is_control) {
            return Err("invalid model tool call ID");
        }
        Ok(Self(value))
    }
}

impl From<ModelToolCallId> for String {
    fn from(value: ModelToolCallId) -> Self {
        value.0
    }
}

impl ModelToolCallId {
    pub fn new(value: impl Into<String>) -> Result<Self, &'static str> {
        Self::try_from(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Provider-visible tool name.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ModelToolName(String);

impl TryFrom<String> for ModelToolName {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.is_empty()
            || value.len() > 64
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
        {
            return Err("invalid model tool name");
        }
        Ok(Self(value))
    }
}

impl From<ModelToolName> for String {
    fn from(value: ModelToolName) -> Self {
        value.0
    }
}

impl ModelToolName {
    pub fn new(value: impl Into<String>) -> Result<Self, &'static str> {
        Self::try_from(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// An assistant request for a named tool. Arguments must be a JSON object.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ModelToolCall {
    pub id: ModelToolCallId,
    pub name: ModelToolName,
    pub arguments: Map<String, Value>,
}

/// Tool schema offered by the runtime; this does not grant execution authority.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ModelToolDefinition {
    pub name: ModelToolName,
    pub description: String,
    pub parameters: Value,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ModelToolChoice {
    Auto,
    None,
    Required,
    Function(ModelToolName),
}

/// Provider-owned continuation data. The payload is opaque to Marlin and must
/// only be replayed by its originating provider protocol and model route.
#[derive(Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct ModelContinuation {
    pub endpoint: ModelEndpoint,
    pub kind: ModelContinuationKind,
    /// Missing on historical records; such records cannot be automatically rebound.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub destination: Option<ModelDestinationBinding>,
    pub payload: String,
}

/// Sensitive, opaque provider destination and credential fingerprint.
#[derive(Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct ModelDestinationBinding([u8; 32]);

impl ModelDestinationBinding {
    pub fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

/// Identifies the library-owned format of an opaque continuation payload.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ModelContinuationKind {
    OpenAiResponses,
    AnthropicMessages,
}

impl ModelContinuation {
    /// Account payload and route bytes for admission without cloning.
    pub fn accounted_bytes(&self) -> usize {
        self.payload
            .len()
            .saturating_add(self.endpoint.provider.as_str().len())
            .saturating_add(self.endpoint.model.as_str().len())
            .saturating_add(
                self.endpoint
                    .alias
                    .as_ref()
                    .map_or(0, |alias| alias.as_str().len()),
            )
    }

    fn heap_capacity(&self) -> usize {
        self.payload
            .capacity()
            .saturating_add(self.endpoint.heap_capacity())
    }
}

impl fmt::Debug for ModelContinuation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ModelContinuation")
            .field("endpoint", &self.endpoint)
            .field("kind", &self.kind)
            .field("payload_bytes", &self.payload.len())
            .finish()
    }
}

/// Marlin-owned model message.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "ModelGatewayMessageWire")]
pub struct ModelGatewayMessage {
    pub role: ModelGatewayMessageRole,
    pub content: String,
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<ModelToolCallId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ModelToolCall>,
    /// Opaque provider output required to continue this assistant turn.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub continuation: Option<ModelContinuation>,
}

#[derive(Deserialize)]
struct ModelGatewayMessageWire {
    role: ModelGatewayMessageRole,
    content: String,
    name: Option<String>,
    #[serde(default)]
    tool_call_id: Option<ModelToolCallId>,
    #[serde(default)]
    tool_calls: Vec<ModelToolCall>,
    #[serde(default)]
    continuation: Option<ModelContinuation>,
}

impl TryFrom<ModelGatewayMessageWire> for ModelGatewayMessage {
    type Error = &'static str;

    fn try_from(wire: ModelGatewayMessageWire) -> Result<Self, Self::Error> {
        let message = Self {
            role: wire.role,
            content: wire.content,
            name: wire.name,
            tool_call_id: wire.tool_call_id,
            tool_calls: wire.tool_calls,
            continuation: wire.continuation,
        };
        message.validate_tool_contract()?;
        Ok(message)
    }
}

impl ModelGatewayMessage {
    fn plain(role: ModelGatewayMessageRole, content: impl Into<String>) -> Self {
        Self {
            role,
            content: content.into(),
            name: None,
            tool_call_id: None,
            tool_calls: Vec::new(),
            continuation: None,
        }
    }

    /// Attaches a sender name to this message.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn validate_tool_contract(&self) -> Result<(), &'static str> {
        if self.tool_calls.len() > 32
            || self.tool_calls.iter().enumerate().any(|(index, call)| {
                self.tool_calls[..index]
                    .iter()
                    .any(|previous| previous.id == call.id)
            })
        {
            return Err("invalid model tool call set");
        }
        match self.role {
            ModelGatewayMessageRole::Tool
                if self.tool_call_id.is_some()
                    && self.tool_calls.is_empty()
                    && self.continuation.is_none() =>
            {
                Ok(())
            }
            ModelGatewayMessageRole::Assistant if self.tool_call_id.is_none() => Ok(()),
            ModelGatewayMessageRole::System
            | ModelGatewayMessageRole::Developer
            | ModelGatewayMessageRole::User
                if self.tool_call_id.is_none()
                    && self.tool_calls.is_empty()
                    && self.continuation.is_none() =>
            {
                Ok(())
            }
            _ => Err("invalid model tool message correlation"),
        }
    }
}

/// Creates a system message.
pub fn system_gateway_message(content: impl Into<String>) -> ModelGatewayMessage {
    ModelGatewayMessage::plain(ModelGatewayMessageRole::System, content)
}

/// Creates a developer instruction message.
pub fn developer_gateway_message(content: impl Into<String>) -> ModelGatewayMessage {
    ModelGatewayMessage::plain(ModelGatewayMessageRole::Developer, content)
}

/// Creates a user message.
pub fn user_gateway_message(content: impl Into<String>) -> ModelGatewayMessage {
    ModelGatewayMessage::plain(ModelGatewayMessageRole::User, content)
}

/// Creates an assistant message.
pub fn assistant_gateway_message(content: impl Into<String>) -> ModelGatewayMessage {
    ModelGatewayMessage::plain(ModelGatewayMessageRole::Assistant, content)
}

/// Creates a correlated tool result for a preceding assistant tool call.
pub fn tool_gateway_message(
    id: ModelToolCallId,
    content: impl Into<String>,
) -> ModelGatewayMessage {
    let mut message = ModelGatewayMessage::plain(ModelGatewayMessageRole::Tool, content);
    message.tool_call_id = Some(id);
    message
}

/// Provider-neutral completion options used by runtime-facing gateway calls.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ModelGatewayCompletionOptions {
    pub tools: Option<Vec<ModelToolDefinition>>,
    pub tool_choice: Option<ModelToolChoice>,
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

    /// Owned heap capacity retained by the response and all choices.
    pub fn heap_capacity(&self) -> usize {
        self.id
            .capacity()
            .saturating_add(self.model.capacity())
            .saturating_add(
                self.choices
                    .capacity()
                    .saturating_mul(std::mem::size_of::<ModelGatewayCompletionChoice>()),
            )
            .saturating_add(self.choices.iter().fold(0usize, |sum, choice| {
                sum.saturating_add(choice.message.content.capacity())
                    .saturating_add(choice.message.name.as_ref().map_or(0, String::capacity))
                    .saturating_add(
                        choice
                            .message
                            .tool_call_id
                            .as_ref()
                            .map_or(0, |id| id.0.capacity()),
                    )
                    .saturating_add(choice.message.tool_calls.iter().fold(0usize, |size, call| {
                        size.saturating_add(std::mem::size_of::<ModelToolCall>())
                            .saturating_add(call.id.0.capacity())
                            .saturating_add(call.name.0.capacity())
                            .saturating_add(json_object_heap_bytes(&call.arguments))
                    }))
                    .saturating_add(
                        choice
                            .message
                            .continuation
                            .as_ref()
                            .map_or(0, ModelContinuation::heap_capacity),
                    )
                    .saturating_add(choice.finish_reason.as_ref().map_or(0, String::capacity))
            }))
    }
}

fn json_heap_bytes(value: &Value) -> usize {
    match value {
        Value::Null | Value::Bool(_) | Value::Number(_) => 0,
        Value::String(text) => text.capacity(),
        Value::Array(items) => items
            .capacity()
            .saturating_mul(std::mem::size_of::<Value>())
            .saturating_add(items.iter().fold(0usize, |sum, item| {
                sum.saturating_add(json_heap_bytes(item))
            })),
        Value::Object(items) => json_object_heap_bytes(items),
    }
}

fn json_object_heap_bytes(items: &Map<String, Value>) -> usize {
    items.iter().fold(0usize, |sum, (key, item)| {
        sum.saturating_add(std::mem::size_of::<(String, Value)>() * 2)
            .saturating_add(key.capacity())
            .saturating_add(json_heap_bytes(item))
    })
}

/// An observed stream interruption without remote retirement evidence.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ModelStreamInterruption {
    CancelledLocally,
    Transport,
    Timeout,
    InvalidResponse,
    EndedWithoutTerminal,
    InvalidSequence {
        expected: u64,
        actual: u64,
    },
    SequenceExhausted,
    ChunkTooLarge {
        limit_bytes: usize,
        actual_bytes: usize,
    },
    OutputLimit {
        limit_bytes: usize,
    },
    CompletionHeapLimit {
        limit_bytes: usize,
        actual_bytes: usize,
    },
    ProviderByteLimit,
    InvalidCompletion,
    /// The gateway returned an error without evidence of remote retirement.
    UnclassifiedGatewayFailure,
}

impl fmt::Display for ModelStreamInterruption {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CancelledLocally => formatter.write_str("model stream cancelled locally"),
            Self::Transport => formatter.write_str("stream transport failed"),
            Self::Timeout => formatter.write_str("stream transport timed out"),
            Self::InvalidResponse => {
                formatter.write_str("provider stream response was invalid or incomplete")
            }
            Self::EndedWithoutTerminal => {
                formatter.write_str("stream ended without a terminal event")
            }
            Self::InvalidSequence { expected, actual } => write!(
                formatter,
                "invalid stream sequence: expected {expected}, received {actual}"
            ),
            Self::SequenceExhausted => formatter.write_str("stream sequence exhausted"),
            Self::ChunkTooLarge {
                limit_bytes,
                actual_bytes,
            } => write!(
                formatter,
                "stream chunk exceeded the {limit_bytes} byte limit: {actual_bytes} bytes"
            ),
            Self::OutputLimit { limit_bytes } => write!(
                formatter,
                "model output exceeded the {limit_bytes} byte limit"
            ),
            Self::CompletionHeapLimit {
                limit_bytes,
                actual_bytes,
            } => write!(
                formatter,
                "model completion retained {actual_bytes} bytes, exceeding the {limit_bytes} byte limit"
            ),
            Self::ProviderByteLimit => {
                formatter.write_str("provider response exceeded its byte limit")
            }
            Self::InvalidCompletion => {
                formatter.write_str("model completion does not match its stream")
            }
            Self::UnclassifiedGatewayFailure => {
                formatter.write_str("model stream failed without a retirement receipt")
            }
        }
    }
}

/// Failure returned by runtime-facing model gateway calls.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModelGatewayError {
    EndpointContract(ModelEndpointContractError),
    /// The request was rejected before invoking any downstream gateway.
    RequestNotDispatched(String),
    /// The local stream-open deadline elapsed; remote request retirement is unknown.
    StreamOpenTimeout,
    /// The stream-open transport failed without proving remote retirement.
    StreamOpenTransport,
    /// Stream opening failed without evidence that the request was not dispatched.
    StreamOpenUnclassified,
    /// Stream consumption stopped without evidence of remote retirement.
    StreamInterrupted(ModelStreamInterruption),
    Completion(String),
}

impl ModelGatewayError {
    /// Owned heap capacity retained by the error, for bounded event admission.
    pub fn heap_capacity(&self) -> usize {
        match self {
            Self::EndpointContract(error) => error.heap_capacity(),
            Self::Completion(message) | Self::RequestNotDispatched(message) => message.capacity(),
            Self::StreamOpenTimeout
            | Self::StreamOpenTransport
            | Self::StreamOpenUnclassified
            | Self::StreamInterrupted(_) => 0,
        }
    }
}

impl fmt::Display for ModelGatewayError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EndpointContract(source) => {
                write!(formatter, "invalid model route endpoint: {source}")
            }
            Self::RequestNotDispatched(message) => {
                write!(formatter, "request not dispatched: {message}")
            }
            Self::StreamOpenTimeout => formatter
                .write_str("runtime stream-open deadline elapsed; remote outcome is unknown"),
            Self::StreamOpenTransport => {
                formatter.write_str("stream-open transport failed; remote outcome is unknown")
            }
            Self::StreamOpenUnclassified => {
                formatter.write_str("stream-open failed; remote outcome is unknown")
            }
            Self::StreamInterrupted(reason) => {
                write!(formatter, "{reason}; remote outcome is unknown")
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
            Self::RequestNotDispatched(_) => None,
            Self::StreamOpenTimeout => None,
            Self::StreamOpenTransport => None,
            Self::StreamOpenUnclassified => None,
            Self::StreamInterrupted(_) => None,
            Self::Completion(_) => None,
        }
    }
}
