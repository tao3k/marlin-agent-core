use marlin_agent_protocol::{
    ModelEndpoint, ModelGatewayCompletionOptions, ModelGatewayMessageRole, ModelGatewayTransport,
};
use marlin_agent_runtime::{ModelGatewayRequest, system_gateway_message, user_gateway_message};

#[test]
fn runtime_model_gateway_contract_exposes_provider_neutral_request() {
    let endpoint = ModelEndpoint::new("anthropic", "claude-opus-4-8");
    let request = ModelGatewayRequest::new(
        endpoint,
        vec![
            system_gateway_message("You are a focused code reviewer."),
            user_gateway_message("Review the workspace status."),
        ],
    )
    .with_options(ModelGatewayCompletionOptions {
        temperature: Some(0.2),
        ..Default::default()
    })
    .with_transport(ModelGatewayTransport::Sse);

    assert_eq!(
        request.endpoint().litellm_model_id().as_str(),
        "anthropic/claude-opus-4-8"
    );
    assert_eq!(request.messages()[0].role, ModelGatewayMessageRole::System);
    assert!(request.options().is_some());
    assert_eq!(request.transport(), &ModelGatewayTransport::Sse);
}
