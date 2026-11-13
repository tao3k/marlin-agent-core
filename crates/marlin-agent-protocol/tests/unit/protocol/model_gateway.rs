// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use marlin_agent_protocol::{
    ModelEndpoint, ModelGateway, ModelGatewayCompletionChoice, ModelGatewayCompletionResponse,
    ModelGatewayFuture, ModelGatewayMessageRole, ModelGatewayRequest, ModelGatewayResult,
    ModelGatewayTransport, ModelToolCall, ModelToolCallId, ModelToolName,
    assistant_gateway_message, system_gateway_message, tool_gateway_message, user_gateway_message,
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
fn uncorrelated_tool_roles_are_rejected_during_decode() {
    for role in ["tool", "function"] {
        let message = serde_json::json!({"role": role, "content": "result", "name": null});
        assert!(
            serde_json::from_value::<marlin_agent_protocol::ModelGatewayMessage>(message).is_err()
        );
    }
}

#[test]
fn correlated_tool_turn_round_trips_without_losing_arguments() {
    let id = ModelToolCallId::new("call_1").unwrap();
    let mut assistant = assistant_gateway_message("");
    assistant.tool_calls.push(ModelToolCall {
        id: id.clone(),
        name: ModelToolName::new("read_marker").unwrap(),
        arguments: serde_json::json!({"path":"src/lib.rs"})
            .as_object()
            .unwrap()
            .clone(),
    });
    let result = tool_gateway_message(id, "marker");
    let mut duplicate = assistant.clone();
    duplicate.tool_calls.push(duplicate.tool_calls[0].clone());
    assert!(duplicate.validate_tool_contract().is_err());
    for message in [assistant, result] {
        let encoded = serde_json::to_vec(&message).unwrap();
        let decoded: marlin_agent_protocol::ModelGatewayMessage =
            serde_json::from_slice(&encoded).unwrap();
        assert_eq!(decoded, message);
    }
    assert!(
        serde_json::from_value::<marlin_agent_protocol::ModelGatewayMessage>(
            serde_json::json!({"role":"tool","content":"marker","tool_call_id":""})
        )
        .is_err()
    );
    assert!(serde_json::from_value::<marlin_agent_protocol::ModelGatewayMessage>(
        serde_json::json!({"role":"user","content":"hello","tool_calls":[{"id":"call_1","name":"read_marker","arguments":{}}]})
    ).is_err());
}

#[test]
fn model_gateway_trait_compiles_without_provider_dependency() {
    let gateway = StaticGateway;
    let request = ModelGatewayRequest::new(ModelEndpoint::new("openai", "gpt-5-mini"), vec![]);
    let future = gateway.complete(request);

    drop(future);
}
