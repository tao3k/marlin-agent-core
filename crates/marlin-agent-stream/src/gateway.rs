//! Pooled Weaver model access for the Marlin gateway boundary.

pub use marlin_agent_protocol::ModelGateway as ModelStreamGateway;
use marlin_agent_protocol::{
    ModelContinuation, ModelContinuationKind, ModelDestinationBinding, ModelEndpoint, ModelGateway,
    ModelGatewayCompletionChoice, ModelGatewayCompletionOptions, ModelGatewayCompletionResponse,
    ModelGatewayError, ModelGatewayFuture, ModelGatewayMessage, ModelGatewayMessageRole,
    ModelGatewayRequest, ModelGatewayResult, ModelGatewayTransport, ModelProviderId,
};
use tokio_stream::StreamExt;
use weaver_llm::{Client, Endpoint, Message, Options, Request, Role, TurnEvent, TurnStream};

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
                        if !choice.message.tool_calls.is_empty() {
                            return Err(ModelGatewayError::Completion(
                                "tool calls are not supported by this gateway".into(),
                            ));
                        }
                        let mut message = ModelGatewayMessage::new(
                            ModelGatewayMessageRole::Assistant,
                            choice.message.content,
                        );
                        message.name = choice.message.name;
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
                        } if tool_calls.is_empty() => {
                            let mut message =
                                ModelGatewayMessage::new(ModelGatewayMessageRole::Assistant, text);
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
        ModelGatewayMessageRole::Function => Role::Function,
    };
    Ok(Message {
        role,
        content: message.content,
        images: Vec::new(),
        name: message.name,
        tool_call_id: None,
        tool_calls: Vec::new(),
        responses_output,
        anthropic_output,
    })
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
