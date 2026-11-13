// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Pooled Weaver model access for the Marlin gateway boundary.

pub use marlin_agent_protocol::ModelGateway as ModelStreamGateway;
use marlin_agent_protocol::{
    ModelContinuation, ModelContinuationKind, ModelDestinationBinding, ModelEndpoint, ModelGateway,
    ModelGatewayCompletionChoice, ModelGatewayCompletionOptions, ModelGatewayCompletionResponse,
    ModelGatewayError, ModelGatewayFuture, ModelGatewayMessage, ModelGatewayMessageRole,
    ModelGatewayRequest, ModelGatewayResult, ModelGatewayTransport, ModelProviderId, ModelToolCall,
    ModelToolCallId, ModelToolChoice, ModelToolName, assistant_gateway_message,
};
use tokio_stream::StreamExt;
use weaver_llm::{
    Client, Endpoint, Message, Options, Request, Role, Tool, ToolChoice, TurnEvent, TurnStream,
};

/// Direct model gateway bound to one configured provider and credential.
#[derive(Clone)]
pub struct WeaverModelGateway {
    client: Client,
    provider: ModelProviderId,
    endpoint: Endpoint,
    credential: std::sync::Arc<str>,
    destination: ModelDestinationBinding,
    stream_output_limit: Option<crate::ModelStreamOutputLimit>,
    default_options: Options,
}

impl WeaverModelGateway {
    /// Bind a shared HTTP client to a trusted provider destination.
    pub fn new(
        client: Client,
        provider: ModelProviderId,
        endpoint: Endpoint,
        credential: impl Into<std::sync::Arc<str>>,
    ) -> Self {
        let credential = credential.into();
        let destination = ModelDestinationBinding::new(endpoint.continuation_binding(&credential));
        Self {
            client,
            provider,
            endpoint,
            credential,
            destination,
            stream_output_limit: None,
            default_options: Options::default(),
        }
    }

    /// Apply a cumulative text limit to incremental model-stream responses.
    pub fn with_stream_output_limit(mut self, limit: crate::ModelStreamOutputLimit) -> Self {
        self.stream_output_limit = Some(limit);
        self
    }

    /// Set provider-owned defaults for this route. Explicit request fields win.
    pub fn with_default_options(mut self, options: Options) -> Self {
        self.default_options = options;
        self
    }

    pub(crate) fn options(&self, requested: Option<ModelGatewayCompletionOptions>) -> Options {
        let mut options = self.default_options.clone();
        if let Some(requested) = requested {
            if let Some(tools) = requested.tools {
                options.tools = tools
                    .into_iter()
                    .map(|tool| Tool {
                        name: tool.name.as_str().to_owned(),
                        description: tool.description,
                        parameters: tool.parameters,
                    })
                    .collect();
            }
            if let Some(choice) = requested.tool_choice {
                options.tool_choice = Some(match choice {
                    ModelToolChoice::Auto => ToolChoice::Auto,
                    ModelToolChoice::None => ToolChoice::None,
                    ModelToolChoice::Required => ToolChoice::Required,
                    ModelToolChoice::Function(name) => {
                        ToolChoice::Function(name.as_str().to_owned())
                    }
                });
            }
            options.temperature = requested.temperature.or(options.temperature);
            options.max_tokens = requested.max_tokens.or(options.max_tokens);
            options.top_p = requested.top_p.or(options.top_p);
            options.stop = requested.stop.or(options.stop);
            options.user = requested.user.or(options.user);
            options.seed = requested.seed.or(options.seed);
        }
        options
    }

    pub(crate) fn stream_output_limit(&self) -> Option<crate::ModelStreamOutputLimit> {
        self.stream_output_limit
    }

    pub(crate) fn destination(&self) -> &ModelDestinationBinding {
        &self.destination
    }

    /// Open a typed turn for text, tools and provider continuation.
    pub async fn stream_turn(
        &self,
        endpoint: &ModelEndpoint,
        request: &Request<'_>,
    ) -> ModelGatewayResult<TurnStream> {
        self.validate(endpoint)?;
        if request.model != endpoint.model.as_str() {
            return Err(ModelGatewayError::RequestNotDispatched(
                "request model differs from the selected model".into(),
            ));
        }
        self.client
            .stream_turn(&self.endpoint, &self.credential, request)
            .await
            .map_err(stream_open_error)
    }

    fn validate(&self, endpoint: &ModelEndpoint) -> ModelGatewayResult<()> {
        endpoint
            .validate_contract()
            .map_err(ModelGatewayError::EndpointContract)?;
        if endpoint.provider != self.provider {
            return Err(ModelGatewayError::RequestNotDispatched(
                "selected provider differs from the gateway binding".into(),
            ));
        }
        Ok(())
    }

    async fn complete_request(
        &self,
        request: ModelGatewayRequest,
    ) -> ModelGatewayResult<ModelGatewayCompletionResponse> {
        let (endpoint, messages, options, transport) = request.into_parts();
        self.validate(&endpoint)?;
        let messages: Vec<_> = messages
            .into_iter()
            .map(|message| request_message(message, &endpoint, &self.destination))
            .collect::<ModelGatewayResult<_>>()?;
        let options = self.options(options);
        let request = Request {
            model: endpoint.model.as_str(),
            messages: &messages,
            options: &options,
        };
        match transport {
            ModelGatewayTransport::Auto => {
                let response = self
                    .client
                    .complete(&self.endpoint, &self.credential, &request)
                    .await
                    .map_err(gateway_error)?;
                let choices = response
                    .choices
                    .into_iter()
                    .map(|choice| {
                        let mut message = assistant_gateway_message(choice.message.content);
                        message.name = choice.message.name;
                        message.tool_calls = provider_tool_calls(choice.message.tool_calls)?;
                        message.continuation = provider_continuation(
                            &endpoint,
                            &self.destination,
                            choice.message.responses_output,
                            choice.message.anthropic_output,
                        )?;
                        Ok(ModelGatewayCompletionChoice::new(
                            choice.index,
                            message,
                            choice.finish_reason,
                        ))
                    })
                    .collect::<ModelGatewayResult<_>>()?;
                Ok(ModelGatewayCompletionResponse::new(
                    response.id,
                    response.model,
                    choices,
                ))
            }
            ModelGatewayTransport::Sse => {
                let stream = self.stream_turn(&endpoint, &request).await?;
                let mut stream = std::pin::pin!(stream.into_stream());
                let mut text = String::new();
                while let Some(event) = stream.next().await.transpose().map_err(gateway_error)? {
                    match event {
                        TurnEvent::Text(delta) => text.push_str(&delta),
                        TurnEvent::Thinking(_) => {}
                        TurnEvent::Finished {
                            reason,
                            identity: Some(identity),
                            tool_calls,
                            responses_output,
                            anthropic_output,
                            ..
                        } => {
                            let mut message = assistant_gateway_message(text);
                            message.tool_calls = provider_tool_calls(tool_calls)?;
                            message.continuation = provider_continuation(
                                &endpoint,
                                &self.destination,
                                responses_output,
                                anthropic_output,
                            )?;
                            let choice =
                                ModelGatewayCompletionChoice::new(0, message, Some(reason));
                            return Ok(ModelGatewayCompletionResponse::new(
                                identity.id,
                                identity.model,
                                vec![choice],
                            ));
                        }
                        TurnEvent::Finished { .. } => break,
                    }
                }
                Err(ModelGatewayError::Completion(
                    "expected a text completion with provider identity".into(),
                ))
            }
            ModelGatewayTransport::WebSocket => Err(ModelGatewayError::Completion(
                "the configured gateway uses HTTP or SSE".into(),
            )),
        }
    }
}

impl ModelGateway for WeaverModelGateway {
    fn complete(
        &self,
        request: ModelGatewayRequest,
    ) -> ModelGatewayFuture<ModelGatewayResult<ModelGatewayCompletionResponse>> {
        let gateway = self.clone();
        Box::pin(async move { gateway.complete_request(request).await })
    }
}

pub(crate) fn provider_continuation(
    endpoint: &ModelEndpoint,
    destination: &ModelDestinationBinding,
    responses: Option<weaver_llm::ResponsesOutput>,
    anthropic: Option<weaver_llm::AnthropicOutput>,
) -> ModelGatewayResult<Option<ModelContinuation>> {
    let (kind, payload) = match (responses, anthropic) {
        (None, None) => return Ok(None),
        (Some(output), None) => (
            ModelContinuationKind::OpenAiResponses,
            serde_json::to_string(&output),
        ),
        (None, Some(output)) => (
            ModelContinuationKind::AnthropicMessages,
            serde_json::to_string(&output),
        ),
        (Some(_), Some(_)) => {
            return Err(ModelGatewayError::Completion(
                "conflicting provider continuations".into(),
            ));
        }
    };
    Ok(Some(ModelContinuation {
        endpoint: endpoint.clone(),
        kind,
        destination: Some(destination.clone()),
        payload: payload.map_err(|_| {
            ModelGatewayError::Completion("cannot retain provider continuation".into())
        })?,
    }))
}

/// Check that observed thinking and answer are represented in an Anthropic
/// continuation. This is consistency checking, not signature authentication.
pub fn matches_observed_anthropic_thinking(message: &ModelGatewayMessage, thinking: &str) -> bool {
    let Some(state) = &message.continuation else {
        return false;
    };
    if message.role != ModelGatewayMessageRole::Assistant
        || state.kind != ModelContinuationKind::AnthropicMessages
    {
        return false;
    }
    serde_json::from_str::<weaver_llm::AnthropicOutput>(&state.payload)
        .is_ok_and(|output| output.matches_observed(&message.content, thinking))
}

pub(crate) fn request_message(
    message: ModelGatewayMessage,
    endpoint: &ModelEndpoint,
    destination: &ModelDestinationBinding,
) -> ModelGatewayResult<Message> {
    message
        .validate_tool_contract()
        .map_err(|diagnostic| ModelGatewayError::RequestNotDispatched(diagnostic.into()))?;
    let (responses_output, anthropic_output) = match message.continuation {
        None => (None, None),
        Some(state)
            if message.role == ModelGatewayMessageRole::Assistant
                && state.endpoint == *endpoint
                && state.destination.as_ref() == Some(destination) =>
        {
            match state.kind {
                ModelContinuationKind::OpenAiResponses => (
                    Some(serde_json::from_str(&state.payload).map_err(|_| {
                        ModelGatewayError::RequestNotDispatched(
                            "invalid provider continuation".into(),
                        )
                    })?),
                    None,
                ),
                ModelContinuationKind::AnthropicMessages => (
                    None,
                    Some(serde_json::from_str(&state.payload).map_err(|_| {
                        ModelGatewayError::RequestNotDispatched(
                            "invalid provider continuation".into(),
                        )
                    })?),
                ),
            }
        }
        Some(_) => {
            return Err(ModelGatewayError::RequestNotDispatched(
                "provider continuation has no matching destination and credential binding; restore its original route or reconcile the historical session".into(),
            ));
        }
    };
    let role = match message.role {
        ModelGatewayMessageRole::System => Role::System,
        ModelGatewayMessageRole::Developer => Role::Developer,
        ModelGatewayMessageRole::User => Role::User,
        ModelGatewayMessageRole::Assistant => Role::Assistant,
        ModelGatewayMessageRole::Tool => Role::Tool,
    };
    Ok(Message {
        role,
        content: message.content,
        images: Vec::new(),
        name: message.name,
        tool_call_id: message.tool_call_id.map(String::from),
        tool_calls: message
            .tool_calls
            .into_iter()
            .map(|call| weaver_llm::ToolCall {
                id: call.id.as_str().to_owned(),
                name: call.name.as_str().to_owned(),
                arguments: serde_json::Value::Object(call.arguments),
            })
            .collect(),
        responses_output,
        anthropic_output,
    })
}

pub(crate) fn provider_tool_calls(
    calls: Vec<weaver_llm::ToolCall>,
) -> ModelGatewayResult<Vec<ModelToolCall>> {
    if calls.len() > 32 {
        return Err(ModelGatewayError::Completion(
            "too many provider tool calls".into(),
        ));
    }
    let mut result = Vec::with_capacity(calls.len());
    for call in calls {
        let id = ModelToolCallId::new(call.id)
            .map_err(|error| ModelGatewayError::Completion(error.into()))?;
        let name = ModelToolName::new(call.name)
            .map_err(|error| ModelGatewayError::Completion(error.into()))?;
        let serde_json::Value::Object(arguments) = call.arguments else {
            return Err(ModelGatewayError::Completion(
                "model tool arguments must be an object".into(),
            ));
        };
        if result
            .iter()
            .any(|previous: &ModelToolCall| previous.id == id)
        {
            return Err(ModelGatewayError::Completion(
                "duplicate provider tool call ID".into(),
            ));
        }
        result.push(ModelToolCall {
            id,
            name,
            arguments,
        });
    }
    Ok(result)
}

fn gateway_error(error: weaver_llm::Error) -> ModelGatewayError {
    ModelGatewayError::Completion(error.to_string())
}

fn stream_open_error(error: weaver_llm::Error) -> ModelGatewayError {
    match error {
        weaver_llm::Error::Timeout => ModelGatewayError::StreamOpenTimeout,
        weaver_llm::Error::Transport => ModelGatewayError::StreamOpenTransport,
        weaver_llm::Error::Request(message) => {
            ModelGatewayError::RequestNotDispatched(message.into())
        }
        other => gateway_error(other),
    }
}
