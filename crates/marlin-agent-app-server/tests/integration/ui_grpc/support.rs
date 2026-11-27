// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use std::{path::Path, process::Stdio, time::Duration};

use marlin_agent_app_server::v1::{
    SessionTurnOutcome, SessionTurnRef, StartSessionTurnRequest, WatchSessionTurnRequest,
    app_server_client::AppServerClient as V1AppServerClient, session_turn_event,
};
use marlin_agent_protocol::{
    ModelGateway, ModelGatewayCompletionChoice, ModelGatewayCompletionResponse,
    ModelGatewayFuture, ModelGatewayRequest, ModelGatewayResult, ModelGatewayTransport,
    assistant_gateway_message,
};
use marlin_agent_stream::{
    ModelGatewayEventStream, ModelStreamChunk, ModelStreamEvent, ModelStreamingGateway,
};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::{Child, Command},
};
use tonic::Request;

pub fn auth_req<T>(msg: T) -> Request<T> {
    let mut r = Request::new(msg);
    r.metadata_mut()
        .insert("authorization", "Bearer test-token".parse().unwrap());
    r
}

pub struct EchoGateway;

impl ModelStreamingGateway for EchoGateway {
    fn stream(
        &self,
        _request: ModelGatewayRequest,
    ) -> ModelGatewayFuture<ModelGatewayResult<ModelGatewayEventStream>> {
        Box::pin(async move {
            let stream = async_stream::try_stream! {
                yield ModelStreamEvent::Started { transport: ModelGatewayTransport::Sse };
                yield ModelStreamEvent::Chunk(ModelStreamChunk::new(1, "cohosted-ok"));
                yield ModelStreamEvent::Completed {
                    response: ModelGatewayCompletionResponse::new(
                        "cohost-fixture",
                        "gpt-fixture",
                        vec![ModelGatewayCompletionChoice::new(
                            0,
                            assistant_gateway_message("cohosted-ok"),
                            None,
                        )],
                    ),
                };
            };
            Ok(Box::pin(stream) as ModelGatewayEventStream)
        })
    }
}

impl ModelGateway for EchoGateway {
    fn complete(
        &self,
        request: ModelGatewayRequest,
    ) -> ModelGatewayFuture<ModelGatewayResult<ModelGatewayCompletionResponse>> {
        let prompt = request
            .messages()
            .last()
            .map(|m| m.content.clone())
            .unwrap_or_default();
        Box::pin(async move {
            Ok(ModelGatewayCompletionResponse::new(
                "cohost-atomic-fixture",
                "gpt-fixture",
                vec![ModelGatewayCompletionChoice::new(
                    0,
                    assistant_gateway_message(format!("atomic-echo:{prompt}")),
                    None,
                )],
            ))
        })
    }
}

pub async fn spawn_mygo_host_process(gate_id: &str, turso_path: Option<&Path>) -> (Child, String) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_marlin-mygo-host"));
    cmd.env("MARLIN_BRIDGE_ADDR", "127.0.0.1:0")
        .env("MARLIN_BRIDGE_TOKEN", "test-token")
        .env("MARLIN_BRIDGE_REAL_SESSION", "1")
        .env("MARLIN_BRIDGE_CHUNK_DELAY_MS", "1")
        .env("MARLIN_BRIDGE_GATE_ID", gate_id)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .kill_on_drop(true);
    if let Some(path) = turso_path {
        cmd.env("MARLIN_BRIDGE_TURSO_PATH", path);
    } else {
        cmd.env_remove("MARLIN_BRIDGE_TURSO_PATH");
    }
    let mut child = cmd.spawn().expect("spawn marlin-mygo-host binary");
    let stdout = child.stdout.take().expect("piped stdout");
    let mut lines = BufReader::new(stdout).lines();
    let ready_line = tokio::time::timeout(Duration::from_secs(10), lines.next_line())
        .await
        .expect("timed out waiting for marlin-mygo-host READY line")
        .expect("read READY line")
        .expect("marlin-mygo-host exited before printing READY");
    let addr = ready_line
        .strip_prefix("READY ")
        .unwrap_or_else(|| panic!("unexpected READY line: {ready_line}"))
        .trim()
        .to_owned();
    (child, format!("http://{addr}"))
}

pub async fn stop_mygo_host_process(mut child: Child) {
    #[cfg(unix)]
    {
        if let Some(pid) = child.id() {
            let status = std::process::Command::new("kill")
                .args(["-TERM", &pid.to_string()])
                .status()
                .expect("send SIGTERM to marlin-mygo-host");
            assert!(status.success(), "kill -TERM failed");
        }
        let exit = tokio::time::timeout(Duration::from_secs(10), child.wait())
            .await
            .expect("timed out waiting for graceful marlin-mygo-host exit")
            .expect("wait on marlin-mygo-host");
        assert!(
            exit.success(),
            "marlin-mygo-host should exit cleanly on SIGTERM, got {exit}"
        );
    }
    #[cfg(not(unix))]
    {
        let _ = child.kill().await;
        let _ = child.wait().await;
    }
}

pub async fn run_and_watch_session_turn(
    client: &mut V1AppServerClient<tonic::transport::Channel>,
    session_id: &str,
    user_text: &str,
    submission_id: &str,
) -> (SessionTurnRef, String) {
    let turn = client
        .start_session_turn(auth_req(StartSessionTurnRequest {
            session_id: session_id.to_owned(),
            user_text: user_text.to_owned(),
            after_turn: 0,
            submission_id: submission_id.to_owned(),
        }))
        .await
        .unwrap()
        .into_inner()
        .turn
        .expect("started turn ref");

    let mut events = client
        .watch_session_turn(auth_req(WatchSessionTurnRequest {
            turn: Some(turn.clone()),
            after_event_id: None,
        }))
        .await
        .unwrap()
        .into_inner();

    let mut collected_text = String::new();
    let mut completed = false;
    while let Some(ev) = events.message().await.unwrap() {
        match ev.event {
            Some(session_turn_event::Event::TextCheckpoint(cp)) => {
                collected_text.push_str(&cp.text);
            }
            Some(session_turn_event::Event::Outcome(code)) => {
                assert_eq!(code, SessionTurnOutcome::Completed as i32);
                completed = true;
                break;
            }
            _ => {}
        }
    }
    assert!(completed, "expected Completed terminal outcome");
    (turn, collected_text)
}
