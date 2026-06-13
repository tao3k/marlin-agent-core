use std::sync::{Arc, Mutex};

use marlin_agent_protocol::ModelEndpoint;
use marlin_agent_runtime::{
    ModelGateway, ModelGatewayCompletionChoice, ModelGatewayCompletionResponse, ModelGatewayFuture,
    ModelGatewayMessageRole, ModelGatewayRequest, ModelGatewayResult, assistant_gateway_message,
    system_gateway_message, user_gateway_message,
};

#[derive(Clone, Default)]
struct RecordingGateway {
    seen: Arc<Mutex<Vec<ModelGatewayRequest>>>,
}

impl RecordingGateway {
    fn seen_requests(&self) -> Vec<ModelGatewayRequest> {
        self.seen.lock().expect("recording gateway lock").clone()
    }
}

impl ModelGateway for RecordingGateway {
    fn complete(
        &self,
        request: ModelGatewayRequest,
    ) -> ModelGatewayFuture<ModelGatewayResult<ModelGatewayCompletionResponse>> {
        let seen = Arc::clone(&self.seen);
        Box::pin(async move {
            seen.lock()
                .expect("recording gateway lock")
                .push(request.clone());

            Ok(ModelGatewayCompletionResponse::new(
                "runtime-test-completion",
                request.endpoint().litellm_model_id().as_str(),
                vec![ModelGatewayCompletionChoice::new(
                    0,
                    assistant_gateway_message("runtime used the protocol gateway"),
                    Some("stop".to_string()),
                )],
            ))
        })
    }
}

#[tokio::test]
async fn runtime_completes_through_provider_neutral_model_gateway() {
    let gateway = RecordingGateway::default();
    let endpoint = ModelEndpoint::new("openai", "gpt-5-mini");
    let request = ModelGatewayRequest::new(
        endpoint,
        vec![
            system_gateway_message("keep the runtime provider-neutral"),
            user_gateway_message("prove the gateway contract works"),
        ],
    );

    let response = gateway
        .complete(request)
        .await
        .expect("recording gateway should complete");

    assert_eq!(response.id, "runtime-test-completion");
    assert_eq!(response.model, "openai/gpt-5-mini");
    assert_eq!(response.choices.len(), 1);
    assert_eq!(
        response.choices[0].message.role,
        ModelGatewayMessageRole::Assistant
    );
    assert_eq!(
        response.choices[0].message.content,
        "runtime used the protocol gateway"
    );

    let seen = gateway.seen_requests();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].endpoint().provider.as_str(), "openai");
    assert_eq!(seen[0].endpoint().model.as_str(), "gpt-5-mini");
    assert_eq!(seen[0].messages().len(), 2);
}
