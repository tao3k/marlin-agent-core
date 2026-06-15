use std::sync::{Arc, Mutex};
use std::time::Duration;

use marlin_agent_protocol::ModelEndpoint;
use marlin_agent_runtime::{
    ModelGateway, ModelGatewayCompletionChoice, ModelGatewayCompletionResponse, ModelGatewayError,
    ModelGatewayFuture, ModelGatewayMessageRole, ModelGatewayRequest, ModelGatewayResult,
    RuntimeEdgeLayer, RuntimeEdgeModelGateway, RuntimeEdgePolicy, assistant_gateway_message,
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

#[derive(Clone, Default)]
struct SlowGateway;

impl ModelGateway for SlowGateway {
    fn complete(
        &self,
        request: ModelGatewayRequest,
    ) -> ModelGatewayFuture<ModelGatewayResult<ModelGatewayCompletionResponse>> {
        Box::pin(async move {
            tokio::time::sleep(Duration::from_millis(20)).await;
            Ok(ModelGatewayCompletionResponse::new(
                "slow-runtime-test-completion",
                request.endpoint().litellm_model_id().as_str(),
                vec![ModelGatewayCompletionChoice::new(
                    0,
                    assistant_gateway_message("slow provider finished"),
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

#[tokio::test]
async fn runtime_edge_model_gateway_applies_policy_receipt_and_delegates_request() {
    let gateway = RecordingGateway::default();
    let observed_gateway = gateway.clone();
    let edge_gateway = RuntimeEdgeModelGateway::new(
        gateway,
        RuntimeEdgePolicy::new()
            .with_concurrency_limit(1)
            .with_load_shed(true)
            .with_timeout_ms(100),
    )
    .expect("runtime edge policy should compile");

    let receipt = edge_gateway.edge_policy_receipt();
    assert_eq!(receipt.concurrency_limit(), Some(1));
    assert!(receipt.load_shed());
    assert_eq!(receipt.timeout_ms(), Some(100));
    assert_eq!(
        receipt.layers(),
        &[
            RuntimeEdgeLayer::ConcurrencyLimit,
            RuntimeEdgeLayer::LoadShed,
            RuntimeEdgeLayer::Timeout,
        ]
    );

    let response = edge_gateway
        .complete(ModelGatewayRequest::new(
            ModelEndpoint::new("openai", "gpt-5-mini"),
            vec![user_gateway_message("route through the runtime edge")],
        ))
        .await
        .expect("edge gateway should delegate to wrapped gateway");

    assert_eq!(response.id, "runtime-test-completion");
    let seen = observed_gateway.seen_requests();
    assert_eq!(seen.len(), 1);
    assert_eq!(
        seen[0].endpoint().litellm_model_id().as_str(),
        "openai/gpt-5-mini"
    );
}

#[tokio::test]
async fn runtime_edge_model_gateway_times_out_slow_provider() {
    let edge_gateway = RuntimeEdgeModelGateway::new(
        SlowGateway,
        RuntimeEdgePolicy::new()
            .with_concurrency_limit(1)
            .with_timeout_ms(1),
    )
    .expect("runtime edge policy should compile");

    let result = edge_gateway
        .complete(ModelGatewayRequest::new(
            ModelEndpoint::new("openai", "gpt-5-mini"),
            vec![user_gateway_message("timeout slow provider")],
        ))
        .await;

    assert!(matches!(
        result,
        Err(ModelGatewayError::Completion(message))
            if message.contains("runtime edge policy rejected model gateway request")
    ));
}
