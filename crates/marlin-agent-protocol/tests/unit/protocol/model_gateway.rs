use marlin_agent_protocol::{
    ModelEndpoint, ModelGateway, ModelGatewayCompletionChoice, ModelGatewayCompletionResponse,
    ModelGatewayFuture, ModelGatewayMessageRole, ModelGatewayRequest, ModelGatewayResult,
    ModelGatewayTransport, system_gateway_message, user_gateway_message,
};

struct StaticGateway;

impl ModelGateway for StaticGateway {
    fn complete(
        &self,
        _request: ModelGatewayRequest,
    ) -> ModelGatewayFuture<ModelGatewayResult<ModelGatewayCompletionResponse>> {
        Box::pin(async {
            Ok(ModelGatewayCompletionResponse::new(
                "completion-1",
                "openai/gpt-5-mini",
                vec![ModelGatewayCompletionChoice::new(
                    0,
                    system_gateway_message("ok"),
                    Some("stop".to_owned()),
                )],
            ))
        })
    }
}

#[test]
fn model_gateway_request_owns_runtime_visible_message_contract() {
    let request = ModelGatewayRequest::new(
        ModelEndpoint::new("openai", "gpt-5-mini"),
        vec![
            system_gateway_message("system"),
            user_gateway_message("hello"),
        ],
    )
    .with_transport(ModelGatewayTransport::Sse);

    assert_eq!(request.messages().len(), 2);
    assert_eq!(request.messages()[0].role, ModelGatewayMessageRole::System);
    assert_eq!(request.transport(), &ModelGatewayTransport::Sse);
}

#[test]
fn model_gateway_trait_compiles_without_provider_dependency() {
    let gateway = StaticGateway;
    let request = ModelGatewayRequest::new(ModelEndpoint::new("openai", "gpt-5-mini"), vec![]);
    let future = gateway.complete(request);

    drop(future);
}
