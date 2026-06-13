//! LiteLLM-backed model client adapter.

use litellm_rs::core::types::{
    chat::ChatMessage as LiteLlmChatMessage,
    message::{MessageContent as LiteLlmMessageContent, MessageRole as LiteLlmMessageRole},
};
use litellm_rs::{
    CompletionOptions as LiteLlmCompletionOptions, CompletionResponse as LiteLlmCompletionResponse,
    assistant_message as litellm_assistant_message, system_message as litellm_system_message,
    user_message as litellm_user_message,
};
use marlin_agent_protocol::{
    ModelEndpoint, ModelGatewayCompletionChoice, ModelGatewayCompletionOptions,
    ModelGatewayCompletionResponse, ModelGatewayError, ModelGatewayMessage,
    ModelGatewayMessageRole, ModelGatewayResult,
};

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
        messages: Vec<ModelGatewayMessage>,
        options: Option<ModelGatewayCompletionOptions>,
    ) -> ModelGatewayResult<ModelGatewayCompletionResponse> {
        endpoint
            .validate_contract()
            .map_err(ModelGatewayError::EndpointContract)?;
        let model_id = endpoint.litellm_model_id();
        let messages = messages
            .into_iter()
            .map(litellm_message_from_gateway)
            .collect();
        let options = options.map(litellm_options_from_gateway);
        let response = litellm_rs::completion(model_id.as_str(), messages, options)
            .await
            .map_err(|source| ModelGatewayError::Completion(source.to_string()))?;
        Ok(gateway_response_from_litellm(response))
    }
}

fn litellm_message_from_gateway(message: ModelGatewayMessage) -> LiteLlmChatMessage {
    let ModelGatewayMessage {
        role,
        content,
        name,
    } = message;
    let mut message = match role {
        ModelGatewayMessageRole::System => litellm_system_message(content),
        ModelGatewayMessageRole::User => litellm_user_message(content),
        ModelGatewayMessageRole::Assistant => litellm_assistant_message(content),
        ModelGatewayMessageRole::Developer => {
            litellm_text_message(LiteLlmMessageRole::Developer, content)
        }
        ModelGatewayMessageRole::Tool => litellm_text_message(LiteLlmMessageRole::Tool, content),
        ModelGatewayMessageRole::Function => {
            litellm_text_message(LiteLlmMessageRole::Function, content)
        }
    };
    message.name = name;
    message
}

fn litellm_text_message(role: LiteLlmMessageRole, content: String) -> LiteLlmChatMessage {
    LiteLlmChatMessage {
        role,
        content: Some(LiteLlmMessageContent::Text(content)),
        ..Default::default()
    }
}

fn litellm_options_from_gateway(
    options: ModelGatewayCompletionOptions,
) -> LiteLlmCompletionOptions {
    LiteLlmCompletionOptions {
        temperature: options.temperature,
        max_tokens: options.max_tokens,
        top_p: options.top_p,
        stop: options.stop,
        user: options.user,
        seed: options.seed,
        ..Default::default()
    }
}

fn gateway_response_from_litellm(
    response: LiteLlmCompletionResponse,
) -> ModelGatewayCompletionResponse {
    let choices = response
        .choices
        .into_iter()
        .map(|choice| {
            ModelGatewayCompletionChoice::new(
                choice.index,
                gateway_message_from_litellm(choice.message),
                choice.finish_reason.map(|reason| format!("{reason:?}")),
            )
        })
        .collect();

    ModelGatewayCompletionResponse::new(response.id, response.model, choices)
}

fn gateway_message_from_litellm(message: LiteLlmChatMessage) -> ModelGatewayMessage {
    let role = match message.role {
        LiteLlmMessageRole::System => ModelGatewayMessageRole::System,
        LiteLlmMessageRole::Developer => ModelGatewayMessageRole::Developer,
        LiteLlmMessageRole::User => ModelGatewayMessageRole::User,
        LiteLlmMessageRole::Assistant => ModelGatewayMessageRole::Assistant,
        LiteLlmMessageRole::Tool => ModelGatewayMessageRole::Tool,
        LiteLlmMessageRole::Function => ModelGatewayMessageRole::Function,
    };
    let content = message
        .content
        .map(|content| content.to_string())
        .unwrap_or_default();
    let gateway_message = ModelGatewayMessage::new(role, content);
    if let Some(name) = message.name {
        gateway_message.with_name(name)
    } else {
        gateway_message
    }
}
