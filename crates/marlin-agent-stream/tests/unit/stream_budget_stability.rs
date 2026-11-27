// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Adversarial stability tests for SM17 (`StreamingOutputBudgetEnvelope`):
//! 1. `WeaverModelGateway::complete` over `ModelGatewayTransport::Sse` must enforce
//!    `stream_output_limit` on cumulative `Text` deltas.
//! 2. `WeaverModelGateway::complete` over `ModelGatewayTransport::Sse` must account
//!    hidden `Thinking` deltas against `stream_output_limit`.
//! 3. `OpenAiCompatibleHttpClient::complete_text` must fail fast on oversized
//!    `Content-Length` headers without waiting for or allocating the response body.
//! 4. `OpenAiCompatibleHttpClient::complete_text` must abort chunked HTTP floods
//!    at the 4 MiB boundary via incremental `.chunk().await` instead of buffering
//!    the entire stream via `.bytes().await`.

use std::{
    num::NonZeroUsize,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

use marlin_agent_protocol::{
    ModelEndpoint, ModelGatewayError, ModelStreamInterruption,
};
use marlin_agent_stream::{
    ModelStreamGateway, ModelStreamOutputLimit, ModelStreamRequest, ModelStreamTransport,
    OpenAiCompatibleHttpClient, OpenAiHttpConfig, WeaverModelGateway, user_gateway_message,
};
use serde_json::json;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

async fn drain_http_request(socket: &mut tokio::net::TcpStream) {
    let mut bytes = Vec::new();
    loop {
        bytes.push(socket.read_u8().await.expect("request header byte"));
        if bytes.ends_with(b"\r\n\r\n") {
            break;
        }
    }
    let headers = String::from_utf8(bytes).expect("utf8 headers");
    let length = headers
        .lines()
        .find_map(|line| {
            line.to_ascii_lowercase()
                .strip_prefix("content-length:")
                .and_then(|value| value.trim().parse::<usize>().ok())
        })
        .unwrap_or(0);
    if length > 0 {
        let mut body = vec![0u8; length];
        socket.read_exact(&mut body).await.expect("request body");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sm17_sse_complete_enforces_configured_stream_output_limit_on_text_flood() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/v1/responses", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        drain_http_request(&mut socket).await;
        let mut wire =
            b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n"
                .to_vec();
        for _ in 0..8 {
            wire.extend_from_slice(
                b"data: {\"type\":\"response.output_text.delta\",\"delta\":\"0123456789abcdef\"}\n\n",
            );
        }
        let full_text = "0123456789abcdef".repeat(8);
        let completed = json!({
            "type": "response.completed",
            "response": {
                "id": "resp_flood",
                "model": "gpt-fixture",
                "status": "completed",
                "output": [{"type": "message", "content": [{"type": "output_text", "text": full_text}]}]
            }
        });
        wire.extend_from_slice(format!("data: {completed}\n\n").as_bytes());
        let _ = socket.write_all(&wire).await;
    });

    let gateway = WeaverModelGateway::new(
        weaver_llm::Client::new(Duration::from_secs(2), 64 * 1024).unwrap(),
        "openai".into(),
        weaver_llm::Endpoint::new(weaver_llm::Protocol::OpenAiResponses, &url).unwrap(),
        "fixture",
    )
    .with_stream_output_limit(ModelStreamOutputLimit::new(NonZeroUsize::new(32).unwrap()));

    let result = gateway
        .complete(
            ModelStreamRequest::new(
                ModelEndpoint::new("openai", "gpt-fixture"),
                vec![user_gateway_message("flood text")],
            )
            .with_transport(ModelStreamTransport::Sse),
        )
        .await;

    assert!(
        matches!(
            result,
            Err(ModelGatewayError::StreamInterrupted(
                ModelStreamInterruption::OutputLimit { limit_bytes: 32 }
            ))
        ),
        "expected OutputLimit(32) on SSE complete text flood, got: {result:?}"
    );
    let _ = server.await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sm17_sse_complete_accounts_hidden_thinking_deltas_against_stream_output_limit() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/v1/messages", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        drain_http_request(&mut socket).await;
        let big_thought = "T".repeat(96);
        let events = [
            json!({"type":"message_start","message":{"id":"msg_flood","model":"claude-fixture","usage":{"input_tokens":1,"output_tokens":0}}}),
            json!({"type":"content_block_start","index":0,"content_block":{"type":"thinking","thinking":""}}),
            json!({"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":big_thought}}),
            json!({"type":"content_block_delta","index":0,"delta":{"type":"signature_delta","signature":"sig-1"}}),
            json!({"type":"content_block_stop","index":0}),
            json!({"type":"content_block_start","index":1,"content_block":{"type":"text","text":""}}),
            json!({"type":"content_block_delta","index":1,"delta":{"type":"text_delta","text":"ok"}}),
            json!({"type":"content_block_stop","index":1}),
            json!({"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":1}}),
            json!({"type":"message_stop"}),
        ];
        let mut wire =
            b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n"
                .to_vec();
        for ev in events {
            wire.extend_from_slice(format!("data: {ev}\n\n").as_bytes());
        }
        let _ = socket.write_all(&wire).await;
    });

    let gateway = WeaverModelGateway::new(
        weaver_llm::Client::new(Duration::from_secs(2), 64 * 1024).unwrap(),
        "anthropic".into(),
        weaver_llm::Endpoint::new(weaver_llm::Protocol::Anthropic, &url).unwrap(),
        "fixture",
    )
    .with_stream_output_limit(ModelStreamOutputLimit::new(NonZeroUsize::new(32).unwrap()));

    let result = gateway
        .complete(
            ModelStreamRequest::new(
                ModelEndpoint::new("anthropic", "claude-fixture"),
                vec![user_gateway_message("flood thinking")],
            )
            .with_transport(ModelStreamTransport::Sse),
        )
        .await;

    assert!(
        matches!(
            result,
            Err(ModelGatewayError::StreamInterrupted(
                ModelStreamInterruption::OutputLimit { limit_bytes: 32 }
            ))
        ),
        "expected OutputLimit(32) on SSE complete thinking flood, got: {result:?}"
    );
    let _ = server.await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sm17_openai_http_rejects_oversized_content_length_before_reading_body() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (done_tx, done_rx) = tokio::sync::oneshot::channel::<()>();
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        drain_http_request(&mut socket).await;
        // Declare 100 MiB Content-Length and intentionally stall sending the body.
        // A pre-allocation gate MUST fail fast on the header without waiting for body bytes.
        let headers = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 104857600\r\nConnection: close\r\n\r\n";
        let _ = socket.write_all(headers.as_bytes()).await;
        let _ = done_rx.await;
    });

    let client = OpenAiCompatibleHttpClient::new(OpenAiHttpConfig {
        base_url: format!("http://{addr}"),
        model_override: Some("gpt-sm17".to_owned()),
        api_key: None,
        timeout: Duration::from_secs(2),
    })
    .unwrap();
    let req = ModelStreamRequest::new(
        ModelEndpoint::new("openai", "gpt-sm17"),
        vec![user_gateway_message("oversized header")],
    );

    let start = Instant::now();
    let err = client
        .complete_text(&req)
        .await
        .expect_err("oversized Content-Length must fail closed");
    let elapsed = start.elapsed();
    let _ = done_tx.send(());
    let _ = server.await;

    assert!(
        err.contains("exceeded 4 MiB safety limit"),
        "expected 4 MiB safety-limit refusal before timeout, got: {err} (elapsed={elapsed:?})"
    );
    assert!(
        elapsed < Duration::from_secs(1),
        "expected fast header rejection (<1s), took {elapsed:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sm17_openai_http_aborts_chunked_flood_at_4mib_boundary() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let sent_payload_bytes = Arc::new(AtomicUsize::new(0));
    let sent_clone = Arc::clone(&sent_payload_bytes);
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        drain_http_request(&mut socket).await;
        let headers = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n";
        if socket.write_all(headers.as_bytes()).await.is_err() {
            return;
        }
        // Stream up to 24 MiB in 64 KiB chunks. A pre-allocation reader must
        // refuse and close the socket once 4 MiB is crossed instead of draining all 24 MiB.
        let chunk_payload = vec![b'A'; 64 * 1024];
        let chunk_header = format!("{:x}\r\n", chunk_payload.len());
        for _ in 0..384 {
            if socket.write_all(chunk_header.as_bytes()).await.is_err() {
                break;
            }
            if socket.write_all(&chunk_payload).await.is_err() {
                break;
            }
            if socket.write_all(b"\r\n").await.is_err() {
                break;
            }
            sent_clone.fetch_add(chunk_payload.len(), Ordering::SeqCst);
            tokio::task::yield_now().await;
        }
        let _ = socket.write_all(b"0\r\n\r\n").await;
    });

    let client = OpenAiCompatibleHttpClient::new(OpenAiHttpConfig {
        base_url: format!("http://{addr}"),
        model_override: Some("gpt-sm17".to_owned()),
        api_key: None,
        timeout: Duration::from_secs(5),
    })
    .unwrap();
    let req = ModelStreamRequest::new(
        ModelEndpoint::new("openai", "gpt-sm17"),
        vec![user_gateway_message("chunked flood")],
    );

    let err = client
        .complete_text(&req)
        .await
        .expect_err("24 MiB chunked flood must fail closed");
    let _ = server.await;
    let transferred = sent_payload_bytes.load(Ordering::SeqCst);

    assert!(
        err.contains("exceeded 4 MiB safety limit"),
        "expected 4 MiB safety limit error, got: {err}"
    );
    assert!(
        transferred < 12 * 1024 * 1024,
        "client drained {transferred} bytes (check-after-buffering!), expected early abort near 4 MiB"
    );
}
