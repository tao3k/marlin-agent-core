use std::time::Duration;

use super::journal_gateway::JournalGateway;
use marlin_agent_protocol::{
    ModelEndpoint, ModelGatewayRequest, ModelGatewayTransport, ModelStreamInterruption,
    user_gateway_message,
};
use marlin_agent_sessions::{
    ConversationJournal, ConversationTurnOutcome, ConversationUncertaintyReason,
};
use marlin_agent_storage::{AgentId, InMemoryAgentStorage, ProjectId, SessionId};
use marlin_agent_stream::{
    MODEL_STREAM_MAX_CHUNK_BYTES, ModelStreamEvent, ModelStreamOutputLimit, ModelStreamingGateway,
    WeaverModelGateway,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::oneshot,
    time::timeout,
};
use tokio_stream::StreamExt;

#[tokio::test]
async fn weaver_output_limit_fails_before_emitting_the_overflowing_chunk() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/v1/responses", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut bytes = Vec::new();
        while !bytes.ends_with(b"\r\n\r\n") {
            bytes.push(socket.read_u8().await.unwrap());
        }
        let headers = String::from_utf8(bytes).unwrap();
        let length: usize = headers
            .lines()
            .find_map(|line| {
                line.to_ascii_lowercase()
                    .strip_prefix("content-length:")
                    .and_then(|value| value.trim().parse().ok())
            })
            .unwrap();
        let mut body = vec![0; length];
        socket.read_exact(&mut body).await.unwrap();
        let events = ["中", "ab", "x"];
        let mut wire =
            b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n"
                .to_vec();
        for text in events {
            wire.extend_from_slice(
                format!(
                    "data: {{\"type\":\"response.output_text.delta\",\"delta\":\"{text}\"}}\n\n"
                )
                .as_bytes(),
            );
        }
        wire.extend_from_slice(b"data: {\"type\":\"response.completed\",\"response\":{\"id\":\"fixture\",\"model\":\"gpt-fixture\",\"status\":\"completed\",\"output\":[]}}\n\n");
        socket.write_all(&wire).await.unwrap();
    });
    let gateway = WeaverModelGateway::new(
        weaver_llm::Client::new(Duration::from_secs(3), 4096).unwrap(),
        "openai".into(),
        weaver_llm::Endpoint::new(weaver_llm::Protocol::OpenAiResponses, &url).unwrap(),
        "fixture",
    )
    .with_stream_output_limit(ModelStreamOutputLimit::new(
        std::num::NonZeroUsize::new(5).unwrap(),
    ));
    let request = ModelGatewayRequest::new(
        ModelEndpoint::new("openai", "gpt-fixture"),
        vec![user_gateway_message("hi")],
    );
    let storage = InMemoryAgentStorage::new();
    let project = ProjectId::new("project:stream-limit").unwrap();
    let session = SessionId::new("session:stream-limit").unwrap();
    let agent = AgentId::new("agent:stream-limit").unwrap();
    let journal = ConversationJournal::open(
        storage.clone(),
        project.clone(),
        session.clone(),
        agent.clone(),
        |_| {},
    )
    .await
    .unwrap();
    let gateway = JournalGateway::new(gateway, journal);
    let mut stream = gateway.stream(request).await.unwrap();
    assert!(matches!(
        stream.next().await.unwrap().unwrap(),
        ModelStreamEvent::Started { .. }
    ));
    assert!(
        matches!(stream.next().await.unwrap().unwrap(), ModelStreamEvent::Chunk(chunk) if chunk.content == "中")
    );
    assert!(
        matches!(stream.next().await.unwrap().unwrap(), ModelStreamEvent::Chunk(chunk) if chunk.content == "ab")
    );
    assert!(
        stream
            .next()
            .await
            .unwrap()
            .unwrap_err()
            .to_string()
            .contains("5 byte limit")
    );
    assert!(stream.next().await.is_none());
    let mut turns = Vec::new();
    ConversationJournal::open(storage, project, session, agent, |turn| {
        turns.push(turn.clone())
    })
    .await
    .unwrap();
    assert_eq!(turns[0].partial_text, "中ab");
    assert!(matches!(
        &turns[0].outcome,
        Some(ConversationTurnOutcome::Uncertain {
            reason: ConversationUncertaintyReason::StreamInterrupted {
                reason: ModelStreamInterruption::OutputLimit { limit_bytes: 5 }
            }
        })
    ));
    server.await.unwrap();
}

#[tokio::test]
async fn weaver_delivers_chunk_before_provider_finishes() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/v1/responses", listener.local_addr().unwrap());
    let (finish_tx, finish_rx) = oneshot::channel();
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut bytes = Vec::new();
        while !bytes.ends_with(b"\r\n\r\n") {
            bytes.push(socket.read_u8().await.unwrap());
        }
        let headers = String::from_utf8(bytes).unwrap();
        let length: usize = headers
            .lines()
            .find_map(|line| {
                line.to_ascii_lowercase()
                    .strip_prefix("content-length:")
                    .and_then(|value| value.trim().parse().ok())
            })
            .unwrap();
        let mut body = vec![0; length];
        socket.read_exact(&mut body).await.unwrap();
        assert!(String::from_utf8(body).unwrap().contains("\"stream\":true"));
        socket.write_all(concat!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n", "data: {\"type\":\"response.output_text.delta\",\"delta\":\"hello\"}\n\n").as_bytes()).await.unwrap();
        finish_rx.await.unwrap();
        socket.write_all(b"data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_fixture\",\"model\":\"gpt-fixture\",\"status\":\"completed\",\"output\":[{\"type\":\"message\",\"content\":[{\"type\":\"output_text\",\"text\":\"hello\"}]}]}}\n\n").await.unwrap();
    });
    let gateway = WeaverModelGateway::new(
        weaver_llm::Client::new(Duration::from_secs(3), 4096).unwrap(),
        "openai".into(),
        weaver_llm::Endpoint::new(weaver_llm::Protocol::OpenAiResponses, &url).unwrap(),
        "fixture",
    );
    let request = ModelGatewayRequest::new(
        ModelEndpoint::new("openai", "gpt-fixture"),
        vec![user_gateway_message("hi")],
    );
    let mut stream = gateway.stream(request).await.unwrap();
    assert!(matches!(
        stream.next().await.unwrap().unwrap(),
        ModelStreamEvent::Started {
            transport: ModelGatewayTransport::Sse
        }
    ));
    let chunk = timeout(Duration::from_secs(2), stream.next())
        .await
        .expect("chunk before provider completes")
        .unwrap()
        .unwrap();
    assert!(
        matches!(chunk, ModelStreamEvent::Chunk(chunk) if chunk.sequence == 1 && chunk.content == "hello")
    );
    finish_tx.send(()).unwrap();
    let event = stream.next().await.unwrap().unwrap();
    assert!(
        matches!(event, ModelStreamEvent::Completed { response } if response.id == "resp_fixture" && response.choices[0].message.content == "hello")
    );
    assert!(stream.next().await.is_none());
    server.await.unwrap();
}

#[tokio::test]
async fn weaver_reports_a_truncated_stream_after_its_partial_output() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/v1/responses", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut bytes = Vec::new();
        while !bytes.ends_with(b"\r\n\r\n") {
            bytes.push(socket.read_u8().await.unwrap());
        }
        let headers = String::from_utf8(bytes).unwrap();
        let length: usize = headers
            .lines()
            .find_map(|line| {
                line.to_ascii_lowercase()
                    .strip_prefix("content-length:")
                    .and_then(|value| value.trim().parse().ok())
            })
            .unwrap();
        let mut body = vec![0; length];
        socket.read_exact(&mut body).await.unwrap();
        socket.write_all(concat!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n",
            "data: {\"type\":\"response.output_text.delta\",\"delta\":\"partial\"}\n\n",
        ).as_bytes()).await.unwrap();
    });
    let gateway = WeaverModelGateway::new(
        weaver_llm::Client::new(Duration::from_secs(3), 4096).unwrap(),
        "openai".into(),
        weaver_llm::Endpoint::new(weaver_llm::Protocol::OpenAiResponses, &url).unwrap(),
        "fixture",
    );
    let request = ModelGatewayRequest::new(
        ModelEndpoint::new("openai", "gpt-fixture"),
        vec![user_gateway_message("hi")],
    );
    let mut stream = gateway.stream(request).await.unwrap();
    assert!(matches!(
        stream.next().await.unwrap().unwrap(),
        ModelStreamEvent::Started { .. }
    ));
    assert!(
        matches!(stream.next().await.unwrap().unwrap(), ModelStreamEvent::Chunk(chunk) if chunk.content == "partial")
    );
    assert!(stream.next().await.unwrap().is_err());
    assert!(stream.next().await.is_none());
    server.await.unwrap();
}

#[tokio::test]
async fn large_unicode_delta_is_split_without_losing_content() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/v1/responses", listener.local_addr().unwrap());
    let content = "中".repeat(50_000);
    let emitted = content.clone();
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut bytes = Vec::new();
        while !bytes.ends_with(b"\r\n\r\n") {
            bytes.push(socket.read_u8().await.unwrap());
        }
        let headers = String::from_utf8(bytes).unwrap();
        let length: usize = headers
            .lines()
            .find_map(|line| {
                line.to_ascii_lowercase()
                    .strip_prefix("content-length:")
                    .and_then(|value| value.trim().parse().ok())
            })
            .unwrap();
        let mut body = vec![0; length];
        socket.read_exact(&mut body).await.unwrap();
        socket
            .write_all(
                b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n",
            )
            .await
            .unwrap();
        socket
            .write_all(
                format!(
                    "data: {{\"type\":\"response.output_text.delta\",\"delta\":\"{emitted}\"}}\n\n"
                )
                .as_bytes(),
            )
            .await
            .unwrap();
        let terminal = serde_json::json!({"type":"response.completed", "response":{
            "id":"resp_large", "model":"gpt-fixture", "status":"completed",
            "output":[{"type":"message", "content":[{"type":"output_text", "text":emitted}]}]
        }});
        socket
            .write_all(format!("data: {terminal}\n\n").as_bytes())
            .await
            .unwrap();
    });
    let gateway = WeaverModelGateway::new(
        weaver_llm::Client::new(Duration::from_secs(5), 1024 * 1024).unwrap(),
        "openai".into(),
        weaver_llm::Endpoint::new(weaver_llm::Protocol::OpenAiResponses, &url).unwrap(),
        "fixture",
    );
    let request = ModelGatewayRequest::new(
        ModelEndpoint::new("openai", "gpt-fixture"),
        vec![user_gateway_message("hi")],
    );
    let mut stream = gateway.stream(request).await.unwrap();
    let mut observed = String::new();
    let mut count = 0u64;
    let mut completed = false;
    while let Some(event) = stream.next().await {
        match event.unwrap() {
            ModelStreamEvent::Started { .. } => {}
            ModelStreamEvent::Chunk(chunk) => {
                count += 1;
                assert_eq!(chunk.sequence, count);
                assert!(chunk.content.len() <= MODEL_STREAM_MAX_CHUNK_BYTES);
                observed.push_str(&chunk.content);
            }
            ModelStreamEvent::Thinking(_) => panic!("fixture does not emit thinking"),
            ModelStreamEvent::Completed { response } => {
                assert_eq!(response.choices[0].message.content, content);
                completed = true;
            }
        }
    }
    assert!(completed && count >= 3);
    assert_eq!(observed, content);
    server.await.unwrap();
}
