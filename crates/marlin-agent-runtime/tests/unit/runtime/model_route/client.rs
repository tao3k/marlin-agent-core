use marlin_agent_protocol::{ModelEndpoint, ModelEndpointContractError};
use marlin_agent_runtime::{
    LiteLlmModelClient, LiteLlmModelClientError, system_message, user_message,
};

#[test]
fn litellm_client_exposes_chat_completion_messages_without_network_call() {
    let _client = LiteLlmModelClient::new();
    let endpoint = ModelEndpoint::new("anthropic", "claude-opus-4-8");
    let messages = [
        system_message("You are a focused code reviewer."),
        user_message("Review the workspace status."),
    ];

    assert_eq!(
        endpoint.litellm_model_id().as_str(),
        "anthropic/claude-opus-4-8"
    );
    assert_eq!(messages.len(), 2);
}

#[tokio::test]
async fn litellm_client_validates_endpoint_contract_before_network_call() {
    let client = LiteLlmModelClient::new();
    let endpoint = ModelEndpoint::new("openai", "codex");
    let result = client.complete(&endpoint, vec![], None).await;

    assert!(matches!(
        result,
        Err(LiteLlmModelClientError::EndpointContract(
            ModelEndpointContractError::CodexIsNotModelName { .. }
        ))
    ));
}
