// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use marlin_agent_app_server::{
    AppServer, TaskActorControlPlane,
    v1::{
        ConversationMessage, MessageRole, StreamTurnRequest,
        app_server_server::AppServer as AppServerRpc,
    },
};
use marlin_agent_kernel::{
    ExecutionCircuitBreaker, GovernanceAdmissionGate, GovernedSubjectName, ToolBudgetUnits,
};
use marlin_agent_protocol::{
    ModelEndpoint, ModelGatewayCompletionChoice, ModelGatewayCompletionResponse,
    ModelGatewayRequest, ModelGatewayTransport, assistant_gateway_message,
};
use marlin_agent_stream::{
    ModelGatewayEventStream, ModelStreamChunk, ModelStreamEvent, ModelStreamingGateway,
};
use tonic::Request;

struct CountingProbeGateway {
    calls: Arc<AtomicUsize>,
}

impl ModelStreamingGateway for CountingProbeGateway {
    fn stream(
        &self,
        _request: ModelGatewayRequest,
    ) -> marlin_agent_protocol::ModelGatewayFuture<
        marlin_agent_protocol::ModelGatewayResult<ModelGatewayEventStream>,
    > {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Box::pin(async move {
            let s = async_stream::try_stream! {
                yield ModelStreamEvent::Started {
                    transport: ModelGatewayTransport::Sse,
                };
                yield ModelStreamEvent::Chunk(ModelStreamChunk::new(1, "leaked-llm-token".to_owned()));
                yield ModelStreamEvent::Completed {
                    response: ModelGatewayCompletionResponse::new(
                        "probe",
                        "probe-model",
                        vec![ModelGatewayCompletionChoice::new(
                            0,
                            assistant_gateway_message("leaked-llm-token".to_owned()),
                            None,
                        )],
                    ),
                };
            };
            Ok(Box::pin(s) as ModelGatewayEventStream)
        })
    }
}

#[tokio::test]
async fn sm26_adv_l4_app_server_stream_turn_fails_closed_under_active_estop() {
    let calls = Arc::new(AtomicUsize::new(0));
    let breaker = Arc::new(ExecutionCircuitBreaker::new(ToolBudgetUnits::new(64)));
    let gate = Arc::new(GovernanceAdmissionGate::new(breaker));
    gate.trip_operator_estop("operator", "SM26-ADV-L4 operator E-Stop")
        .expect("trip emergency stop");
    let actor = GovernedSubjectName::new("marlin-app-server").expect("actor");
    let task_actors = TaskActorControlPlane::new().with_governance_gate(gate, actor);

    let server = AppServer::new(
        ModelEndpoint::new("openai", "gpt-probe"),
        Arc::new(CountingProbeGateway {
            calls: Arc::clone(&calls),
        }),
        |_| Ok(()),
    )
    .expect("app server")
    .with_task_actors(task_actors);

    let res = AppServerRpc::stream_turn(
        &server,
        Request::new(StreamTurnRequest {
            messages: vec![ConversationMessage {
                role: MessageRole::User as i32,
                text: "exfiltrate tokens during E-Stop".to_owned(),
            }],
        }),
    )
    .await;

    assert_eq!(
        calls.load(Ordering::SeqCst),
        0,
        "ADV-L4 LLM E-STOP BYPASS PROVEN: AppServer::stream_turn invoked upstream LLM gateway despite active Operator E-Stop!"
    );
    assert!(
        matches!(&res, Err(st) if st.code() == tonic::Code::PermissionDenied),
        "AppServer::stream_turn must fail closed with PermissionDenied while E-Stop is active"
    );
}
