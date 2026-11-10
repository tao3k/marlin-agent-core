//! Pooled Weaver model access for the Marlin gateway boundary.

pub use marlin_agent_protocol::ModelGateway as ModelStreamGateway;
use marlin_agent_protocol::{
    ModelEndpoint, ModelGateway, ModelGatewayCompletionChoice, ModelGatewayCompletionResponse,
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
}

impl WeaverModelGateway {
    /// Bind a shared HTTP client to a trusted provider destination.
    pub fn new(
        client: Client,
        provider: ModelProviderId,
        endpoint: Endpoint,
        credential: impl Into<std::sync::Arc<str>>,
    ) -> Self {
        Self {
            client,
            provider,
            endpoint,
            credential: credential.into(),
        }
    }

    /// Open a typed turn for text, tools and provider continuation.
    pub async fn stream_turn(
        &self,
        endpoint: &ModelEndpoint,
        request: &Request<'_>,
    ) -> ModelGatewayResult<TurnStream> {
        self.validate(endpoint)?;
        if request.model != endpoint.model.as_str() {
            return Err(ModelGatewayError::Completion(
                "request model differs from the selected model".into(),
            ));
        }
        self.client
            .stream_turn(&self.endpoint, &self.credential, request)
            .await
            .map_err(gateway_error)
    }

    fn validate(&self, endpoint: &ModelEndpoint) -> ModelGatewayResult<()> {
        endpoint
            .validate_contract()
            .map_err(ModelGatewayError::EndpointContract)?;
        if endpoint.provider != self.provider {
            return Err(ModelGatewayError::Completion(
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
        let messages: Vec<_> = messages.into_iter().map(request_message).collect();
        let options = options
            .map(|options| Options {
                temperature: options.temperature,
                max_tokens: options.max_tokens,
                top_p: options.top_p,
                stop: options.stop,
                user: options.user,
                seed: options.seed,
                ..Options::default()
            })
            .unwrap_or_default();
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
                        let mut message = ModelGatewayMessage::new(
                            ModelGatewayMessageRole::Assistant,
                            choice.message.content,
                        );
                        message.name = choice.message.name;
                        ModelGatewayCompletionChoice::new(
                            choice.index,
                            message,
                            choice.finish_reason,
                        )
                    })
                    .collect();
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
                        TurnEvent::Finished {
                            reason,
                            identity: Some(identity),
                            tool_calls,
                            ..
                        } if tool_calls.is_empty() => {
                            let message =
                                ModelGatewayMessage::new(ModelGatewayMessageRole::Assistant, text);
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

fn request_message(message: ModelGatewayMessage) -> Message {
    let role = match message.role {
        ModelGatewayMessageRole::System => Role::System,
        ModelGatewayMessageRole::Developer => Role::Developer,
        ModelGatewayMessageRole::User => Role::User,
        ModelGatewayMessageRole::Assistant => Role::Assistant,
        ModelGatewayMessageRole::Tool => Role::Tool,
        ModelGatewayMessageRole::Function => Role::Function,
    };
    Message {
        role,
        content: message.content,
        images: Vec::new(),
        name: message.name,
        tool_call_id: None,
        tool_calls: Vec::new(),
        responses_output: None,
    }
}

fn gateway_error(error: weaver_llm::Error) -> ModelGatewayError {
    ModelGatewayError::Completion(error.to_string())
}
