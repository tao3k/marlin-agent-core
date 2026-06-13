use marlin_agent_protocol::{ModelEndpoint, ModelEndpointContractError, ModelGatewayError};
use marlin_agent_stream::{
    ChunkGate, LiteLlmModelClient, LiteLlmStreamGateway, ModelStreamChunk, ModelStreamEvent,
    ModelStreamGateway, ModelStreamRequest, ModelStreamTransport, system_gateway_message,
    user_gateway_message,
};

#[test]
fn model_stream_request_preserves_gateway_parts() {
    let endpoint = ModelEndpoint::new("anthropic", "claude-opus-4-8");
    let request = ModelStreamRequest::new(
        endpoint,
        vec![
            system_gateway_message("system"),
            user_gateway_message("hello"),
        ],
    )
    .with_transport(ModelStreamTransport::Sse);

    assert_eq!(
        request.endpoint().litellm_model_id().as_str(),
        "anthropic/claude-opus-4-8"
    );
    assert_eq!(request.messages().len(), 2);
    assert_eq!(request.transport(), &ModelStreamTransport::Sse);
}

#[test]
fn model_stream_events_are_gateway_independent_json() {
    let event = ModelStreamEvent::Chunk(ModelStreamChunk::new(7, "delta"));
    let serialized = serde_json::to_string(&event).expect("stream event serializes");

    assert!(serialized.contains("delta"));
    assert!(serialized.contains('7'));
}

#[tokio::test]
async fn chunk_gate_releases_chunks_in_order() {
    let gate = ChunkGate::closed();
    gate.release_many(2);

    let first = gate.wait_for_next().await;
    let second = gate.wait_for_next().await;

    assert_eq!(first.sequence(), 1);
    assert_eq!(second.sequence(), 2);
    assert_eq!(gate.admitted_chunks(), 2);
}

#[tokio::test]
async fn litellm_client_validates_endpoint_contract_before_network_call() {
    let client = LiteLlmModelClient::new();
    let endpoint = ModelEndpoint::new("openai", "codex");
    let result = client.complete(&endpoint, vec![], None).await;

    assert!(matches!(
        result,
        Err(ModelGatewayError::EndpointContract(
            ModelEndpointContractError::CodexIsNotModelName { .. }
        ))
    ));
}

#[tokio::test]
async fn litellm_stream_gateway_validates_endpoint_contract_before_network_call() {
    let gateway = LiteLlmStreamGateway::new();
    let request = ModelStreamRequest::new(ModelEndpoint::new("openai", "codex"), vec![]);
    let result = gateway.complete(request).await;

    assert!(matches!(
        result,
        Err(ModelGatewayError::EndpointContract(
            ModelEndpointContractError::CodexIsNotModelName { .. }
        ))
    ));
}
