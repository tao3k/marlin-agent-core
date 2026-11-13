// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use std::{
    env,
    time::{Duration, Instant},
};

use marlin_agent_protocol::{ModelEndpoint, ModelEndpointContractError, ModelGatewayError};
use marlin_agent_stream::{
    ChunkGate, ModelGatewayCompletionChoice, ModelGatewayCompletionOptions,
    ModelGatewayCompletionResponse, ModelNoWriteProbeRequest, ModelNoWriteProbeStatus,
    ModelStreamChunk, ModelStreamEvent, ModelStreamGateway, ModelStreamRequest,
    ModelStreamTransport, ModelStreamingGateway, WeaverModelGateway, assistant_gateway_message,
    run_model_no_write_probe, system_gateway_message, user_gateway_message,
};
use marlin_agent_test_support::{
    NO_LIVE_LLM_GATE_DENIAL_MESSAGE, NoLiveHttpModelGatewayFixture, NoLiveLlmModelGateway,
    ScriptedModelGateway,
};
use tokio::time::timeout;

const LIVE_LLM_GATE_ENV: &str = "MARLIN_LIVE_LLM_GATE";
const LIVE_LLM_PROVIDER_ENV: &str = "MARLIN_LIVE_LLM_PROVIDER";
const LIVE_LLM_MODEL_ENV: &str = "MARLIN_LIVE_LLM_MODEL";
const LIVE_LLM_PROVIDER_API_KEY_ENV: &str = "MARLIN_LIVE_LLM_PROVIDER_API_KEY_ENV";
const LIVE_LLM_TIMEOUT_MS_ENV: &str = "MARLIN_LIVE_LLM_TIMEOUT_MS";
const DEFAULT_LIVE_LLM_TIMEOUT_MS: u64 = 60_000;

#[test]
fn anthropic_thinking_requires_matching_valid_continuation() {
    use marlin_agent_protocol::{ModelContinuation, ModelContinuationKind};
    use marlin_agent_stream::matches_observed_anthropic_thinking;
    let endpoint = ModelEndpoint::new("anthropic", "fixture");
    let mut message = assistant_gateway_message("answer");
    message.continuation = Some(ModelContinuation {
        endpoint,
        kind: ModelContinuationKind::AnthropicMessages,
        destination: None,
        payload: r#"[{"type":"thinking","thinking":"thought","signature":"sig"},{"type":"text","text":"answer"}]"#.into(),
    });
    assert!(matches_observed_anthropic_thinking(&message, "thought"));
    assert!(!matches_observed_anthropic_thinking(&message, "thought!"));
    message.content = "changed".into();
    assert!(!matches_observed_anthropic_thinking(&message, "thought"));
    message.content = "answer".into();
    message.continuation.as_mut().unwrap().payload = "[]".into();
    assert!(!matches_observed_anthropic_thinking(&message, "thought"));
    message.continuation.as_mut().unwrap().payload = "not-json".into();
    assert!(!matches_observed_anthropic_thinking(&message, "thought"));
}

#[tokio::test]
async fn invalid_request_override_is_rejected_before_stream_dispatch() {
    let defaults = weaver_llm::AnthropicSettings {
        thinking: Some(weaver_llm::AnthropicThinking::Enabled {
            budget_tokens: 2048,
        }),
        effort: None,
        max_tokens: Some(4096),
    }
    .options(weaver_llm::Protocol::Anthropic)
    .unwrap();
    let gateway = WeaverModelGateway::new(
        weaver_llm::Client::new(Duration::from_secs(1), 4096).unwrap(),
        "fixture".into(),
        weaver_llm::Endpoint::new(
            weaver_llm::Protocol::Anthropic,
            "http://127.0.0.1:9/v1/messages",
        )
        .unwrap(),
        "fixture",
    )
    .with_default_options(defaults);
    let request = marlin_agent_protocol::ModelGatewayRequest::new(
        ModelEndpoint::new("fixture", "fixture"),
        vec![user_gateway_message("hello")],
    )
    .with_options(ModelGatewayCompletionOptions {
        max_tokens: Some(1024),
        ..Default::default()
    });
    assert!(matches!(
        gateway.stream(request).await,
        Err(ModelGatewayError::RequestNotDispatched(_))
    ));
}

#[tokio::test]
async fn continuation_cannot_cross_destination_credential_or_legacy_boundary() {
    use marlin_agent_protocol::{
        ModelContinuation, ModelContinuationKind, ModelDestinationBinding, ModelGatewayRequest,
    };
    let identity = ModelEndpoint::new("fixture", "fixture");
    let original = weaver_llm::Endpoint::new(
        weaver_llm::Protocol::Anthropic,
        "http://127.0.0.1:9/v1/messages",
    )
    .unwrap();
    let binding = ModelDestinationBinding::new(original.continuation_binding("first-key"));
    let mut assistant = assistant_gateway_message("answer");
    assistant.continuation = Some(ModelContinuation {
        endpoint: identity.clone(),
        kind: ModelContinuationKind::AnthropicMessages,
        destination: Some(binding),
        payload: r#"[{"type":"text","text":"answer"}]"#.into(),
    });
    let request = ModelGatewayRequest::new(identity, vec![assistant, user_gateway_message("next")]);
    let client = weaver_llm::Client::new(Duration::from_secs(1), 4096).unwrap();
    let new_gateway = |endpoint, key: &'static str| {
        WeaverModelGateway::new(client.clone(), "fixture".into(), endpoint, key)
    };
    let changed_url = weaver_llm::Endpoint::new(
        weaver_llm::Protocol::Anthropic,
        "http://127.0.0.1:10/v1/messages",
    )
    .unwrap();
    for gateway in [
        new_gateway(changed_url, "first-key"),
        new_gateway(original.clone(), "second-key"),
        new_gateway(
            original
                .clone()
                .with_authentication(weaver_llm::Authentication::Bearer),
            "first-key",
        ),
    ] {
        assert!(matches!(
            gateway.stream(request.clone()).await,
            Err(ModelGatewayError::RequestNotDispatched(_))
        ));
    }
    let mut legacy = request.messages().to_vec();
    let mut stored = serde_json::to_value(&legacy[0]).unwrap();
    stored["continuation"]
        .as_object_mut()
        .unwrap()
        .remove("destination");
    legacy[0] = serde_json::from_value(stored).unwrap();
    assert!(
        legacy[0]
            .continuation
            .as_ref()
            .unwrap()
            .destination
            .is_none()
    );
    let legacy = ModelGatewayRequest::new(request.endpoint().clone(), legacy);
    assert!(matches!(
        new_gateway(original, "first-key").stream(legacy).await,
        Err(ModelGatewayError::RequestNotDispatched(_))
    ));
}

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
async fn weaver_gateway_validates_endpoint_contract_before_network_call() {
    let gateway = WeaverModelGateway::new(
        weaver_llm::Client::new(Duration::from_secs(1), 4096).expect("client"),
        "openai".into(),
        weaver_llm::Endpoint::new(
            weaver_llm::Protocol::OpenAiResponses,
            "https://api.openai.com/v1/responses",
        )
        .expect("endpoint"),
        "fixture",
    );
    let request = ModelStreamRequest::new(ModelEndpoint::new("openai", "model-latest"), vec![]);
    let result = gateway.complete(request).await;
    assert!(matches!(
        result,
        Err(ModelGatewayError::EndpointContract(
            ModelEndpointContractError::OpenAiModelMustBeGpt { .. }
        ))
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn weaver_gateway_runs_marlin_probe_through_responses_sse() {
    use tokio::io::AsyncWriteExt;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener");
    let url = format!(
        "http://{}/v1/responses",
        listener.local_addr().expect("address")
    );
    let upstream = tokio::spawn(async move {
        let (mut socket, payload) = responses_fixture_request(&listener).await;
        assert_eq!(payload["stream"], true);
        assert_eq!(payload["model"], "gpt-fixture");
        assert_eq!(payload["store"], false);
        socket.write_all(concat!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n",
            "data: {\"type\":\"response.output_text.delta\",\"delta\":\"marker\"}\n\n",
            "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_fixture\",\"model\":\"gpt-fixture\",\"status\":\"completed\",\"output\":[{\"type\":\"message\",\"content\":[{\"type\":\"output_text\",\"text\":\"marker\"}]}]}}\n\n",
        ).as_bytes()).await.expect("reply");
    });
    let gateway = WeaverModelGateway::new(
        weaver_llm::Client::new(Duration::from_secs(2), 4096).expect("client"),
        "openai".into(),
        weaver_llm::Endpoint::new(weaver_llm::Protocol::OpenAiResponses, &url).expect("endpoint"),
        "fixture",
    );
    let response = gateway
        .complete(
            ModelStreamRequest::new(
                ModelEndpoint::new("openai", "gpt-fixture"),
                vec![user_gateway_message("return marker")],
            )
            .with_transport(ModelStreamTransport::Sse),
        )
        .await
        .expect("completion");
    assert_eq!(response.id, "resp_fixture");
    assert_eq!(response.choices[0].message.content, "marker");
    upstream.await.expect("upstream fixture");
}

async fn responses_fixture_request(
    listener: &tokio::net::TcpListener,
) -> (tokio::net::TcpStream, serde_json::Value) {
    use tokio::io::AsyncReadExt;
    let (mut socket, _) = listener.accept().await.expect("accept");
    let mut bytes = Vec::new();
    loop {
        bytes.push(socket.read_u8().await.expect("request headers"));
        if bytes.ends_with(b"\r\n\r\n") {
            break;
        }
    }
    let headers = String::from_utf8(bytes).expect("UTF-8 headers");
    assert!(headers.starts_with("POST /v1/responses "));
    let length = headers
        .lines()
        .find_map(|line| {
            line.to_ascii_lowercase()
                .strip_prefix("content-length:")
                .and_then(|value| value.trim().parse::<usize>().ok())
        })
        .expect("content length");
    let mut body = vec![0; length];
    socket.read_exact(&mut body).await.expect("body");
    let payload: serde_json::Value = serde_json::from_slice(&body).expect("request JSON");
    (socket, payload)
}

#[tokio::test]
async fn weaver_stream_open_transport_failure_is_typed_as_uncertain() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/v1/responses", listener.local_addr().unwrap());
    let fixture = tokio::spawn(async move {
        let (socket, payload) = responses_fixture_request(&listener).await;
        assert_eq!(payload["input"][0]["content"], "hello");
        drop(socket);
    });
    let gateway = WeaverModelGateway::new(
        weaver_llm::Client::new(Duration::from_secs(2), 4096).unwrap(),
        "openai".into(),
        weaver_llm::Endpoint::new(weaver_llm::Protocol::OpenAiResponses, &url).unwrap(),
        "fixture",
    );
    let request = marlin_agent_protocol::ModelGatewayRequest::new(
        ModelEndpoint::new("openai", "gpt-fixture"),
        vec![user_gateway_message("hello")],
    );
    assert!(matches!(
        gateway.stream(request).await,
        Err(ModelGatewayError::StreamOpenTransport)
    ));
    fixture.await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn weaver_gateway_replays_a_tool_turn_through_marlin() {
    use serde_json::json;
    use tokio::io::AsyncWriteExt;

    const SSE: &str =
        "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n";
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener");
    let url = format!(
        "http://{}/v1/responses",
        listener.local_addr().expect("address")
    );
    let fixture = tokio::spawn(async move {
        let (mut socket, first) = responses_fixture_request(&listener).await;
        assert_eq!(first["store"], false);
        assert_eq!(first["include"][0], "reasoning.encrypted_content");
        assert_eq!(first["tools"][0]["name"], "read_marker");
        let completed = json!({"type":"response.completed","response":{
            "status":"completed","id":"resp_first","model":"gpt-fixture",
            "output":[
                {"type":"reasoning","id":"rs_1","encrypted_content":"sealed-fixture","summary":[]},
                {"type":"function_call","id":"fc_1","call_id":"call_1","name":"read_marker",
                    "arguments":"{}","status":"completed"}
            ]
        }});
        socket
            .write_all(format!("{SSE}data: {completed}\n\n").as_bytes())
            .await
            .expect("first turn");
        drop(socket);

        let (mut socket, second) = responses_fixture_request(&listener).await;
        assert_eq!(second["store"], false);
        assert!(second.get("previous_response_id").is_none());
        assert_eq!(second["input"][1]["encrypted_content"], "sealed-fixture");
        assert_eq!(second["input"][2]["call_id"], "call_1");
        assert_eq!(
            second["input"][3],
            json!({
                "type":"function_call_output","call_id":"call_1","output":"marlin-live-tool-ok"
            })
        );
        let delta = json!({"type":"response.output_text.delta","delta":"marlin-live-tool-ok"});
        let completed = json!({"type":"response.completed","response":{
            "status":"completed","id":"resp_second","model":"gpt-fixture","output":[{"type":"message","content":[{"type":"output_text","text":"marlin-live-tool-ok"}]}]
        }});
        socket
            .write_all(format!("{SSE}data: {delta}\n\ndata: {completed}\n\n").as_bytes())
            .await
            .expect("second turn");
    });

    let gateway = WeaverModelGateway::new(
        weaver_llm::Client::new(Duration::from_secs(2), 4096).expect("client"),
        "openai".into(),
        weaver_llm::Endpoint::new(weaver_llm::Protocol::OpenAiResponses, &url).expect("endpoint"),
        "fixture",
    );
    let endpoint = ModelEndpoint::new("openai", "gpt-fixture");
    assert_eq!(replay_marker_tool(&gateway, &endpoint).await, "resp_second");
    fixture.await.expect("fixture verified");
}

async fn replay_marker_tool(gateway: &WeaverModelGateway, endpoint: &ModelEndpoint) -> String {
    use marlin_agent_protocol::{
        ModelGatewayRequest, ModelGatewayTransport, ModelToolDefinition, ModelToolName,
        tool_gateway_message,
    };
    use serde_json::json;
    let mut history = vec![user_gateway_message(concat!(
        "Call read_marker exactly once with an empty object before answering. ",
        "After receiving the tool result, reply with its exact text."
    ))];
    let options = ModelGatewayCompletionOptions {
        max_tokens: Some(512),
        tools: Some(vec![ModelToolDefinition {
            name: ModelToolName::new("read_marker").unwrap(),
            description: "Return a fixture marker".into(),
            parameters: json!({"type":"object"}),
        }]),
        ..Default::default()
    };
    let first = gateway
        .complete(
            ModelGatewayRequest::new(endpoint.clone(), history.clone())
                .with_options(options.clone())
                .with_transport(ModelGatewayTransport::Sse),
        )
        .await
        .expect("first completion");
    let assistant = first
        .choices
        .into_iter()
        .next()
        .expect("assistant choice")
        .message;
    let output: serde_json::Value = serde_json::from_str(
        &assistant
            .continuation
            .as_ref()
            .expect("Responses continuation")
            .payload,
    )
    .expect("serializable replay items");
    assert!(
        output
            .as_array()
            .expect("ordered replay items")
            .iter()
            .any(|item| {
                item["type"] == "reasoning"
                    && item["encrypted_content"]
                        .as_str()
                        .is_some_and(|value| !value.is_empty())
            }),
        "missing encrypted reasoning in Responses output"
    );
    assert_eq!(assistant.tool_calls.len(), 1);
    assert_eq!(assistant.tool_calls[0].name.as_str(), "read_marker");
    assert_eq!(
        assistant.tool_calls[0].arguments,
        json!({}).as_object().unwrap().clone()
    );
    let id = assistant.tool_calls[0].id.clone();
    history.extend([assistant, tool_gateway_message(id, "marlin-live-tool-ok")]);
    let second = gateway
        .complete(
            ModelGatewayRequest::new(endpoint.clone(), history)
                .with_options(options)
                .with_transport(ModelGatewayTransport::Sse),
        )
        .await
        .expect("second completion");
    assert_eq!(
        second.choices[0].message.content.trim(),
        "marlin-live-tool-ok"
    );
    assert!(second.choices[0].message.tool_calls.is_empty());
    second.id
}

#[tokio::test]
async fn streaming_tool_request_has_a_distinct_terminal_event() {
    use marlin_agent_protocol::{ModelGatewayRequest, ModelToolDefinition, ModelToolName};
    use serde_json::json;
    use tokio::{io::AsyncWriteExt, net::TcpListener};
    use tokio_stream::StreamExt;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/v1/responses", listener.local_addr().unwrap());
    let fixture = tokio::spawn(async move {
        let (mut socket, request) = responses_fixture_request(&listener).await;
        assert_eq!(request["tools"][0]["name"], "read_marker");
        let finished = json!({"type":"response.completed","response":{
            "status":"completed","id":"resp_tool","model":"gpt-fixture",
            "output":[{"type":"function_call","id":"fc_1","call_id":"call_1",
                "name":"read_marker","arguments":"{}","status":"completed"}]
        }});
        socket.write_all(format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\ndata: {finished}\n\n"
        ).as_bytes()).await.unwrap();
    });
    let gateway = WeaverModelGateway::new(
        weaver_llm::Client::new(Duration::from_secs(2), 4096).unwrap(),
        "openai".into(),
        weaver_llm::Endpoint::new(weaver_llm::Protocol::OpenAiResponses, &url).unwrap(),
        "fixture",
    );
    let request = ModelGatewayRequest::new(
        ModelEndpoint::new("openai", "gpt-fixture"),
        vec![user_gateway_message("call read_marker")],
    )
    .with_options(ModelGatewayCompletionOptions {
        tools: Some(vec![ModelToolDefinition {
            name: ModelToolName::new("read_marker").unwrap(),
            description: "Return a marker".into(),
            parameters: json!({"type":"object"}),
        }]),
        ..Default::default()
    });
    let mut stream = gateway.stream(request).await.unwrap();
    assert!(matches!(
        stream.next().await,
        Some(Ok(ModelStreamEvent::Started { .. }))
    ));
    let event = stream.next().await.unwrap().unwrap();
    let ModelStreamEvent::ToolRequested { response } = event else {
        panic!("tool request must remain distinct from a completed answer");
    };
    assert_eq!(
        response.choices[0].message.tool_calls[0].id.as_str(),
        "call_1"
    );
    assert_eq!(
        response.choices[0].message.tool_calls[0].name.as_str(),
        "read_marker"
    );
    assert!(stream.next().await.is_none());
    fixture.await.unwrap();
}

#[tokio::test]
async fn model_no_write_probe_records_marker_without_tools_or_file_writes() {
    let gateway = ScriptedModelGateway::new([Ok(ModelGatewayCompletionResponse::new(
        "probe-marker-ok",
        "gpt-test-probe",
        vec![ModelGatewayCompletionChoice::new(
            0,
            assistant_gateway_message("marlin-no-write-ok"),
            Some("stop".to_owned()),
        )],
    ))]);
    let receipt = run_model_no_write_probe(
        &gateway,
        ModelNoWriteProbeRequest::new(
            ModelEndpoint::new("openai", "gpt-test-probe"),
            "marlin-no-write-ok",
        ),
    )
    .await;

    assert_eq!(receipt.status, ModelNoWriteProbeStatus::Completed);
    assert!(receipt.marker_present);
    assert_eq!(
        receipt.completion_id.as_ref().map(|id| id.as_str()),
        Some("probe-marker-ok")
    );
    assert_eq!(
        receipt
            .completion_model
            .as_ref()
            .map(|model| model.as_str()),
        Some("gpt-test-probe")
    );
    assert_eq!(receipt.choice_count.get(), 1);
    assert!(
        receipt
            .assistant_content_digest
            .as_ref()
            .is_some_and(|digest| digest.as_str().starts_with("fnv1a64:"))
    );
    assert_eq!(
        receipt.assistant_content_bytes.get(),
        "marlin-no-write-ok".len()
    );
    assert_eq!(receipt.finish_reason.as_deref(), Some("stop"));
    assert!(!receipt.tool_handoff_allowed);
    assert!(!receipt.filesystem_write_allowed);
    assert_eq!(gateway.requests().len(), 1);
}

#[tokio::test]
async fn model_no_write_probe_reports_marker_missing() {
    let gateway = ScriptedModelGateway::new([Ok(ModelGatewayCompletionResponse::new(
        "probe-marker-missing",
        "gpt-test-probe",
        vec![ModelGatewayCompletionChoice::new(
            0,
            assistant_gateway_message("different-marker"),
            Some("stop".to_owned()),
        )],
    ))]);
    let receipt = run_model_no_write_probe(
        &gateway,
        ModelNoWriteProbeRequest::new(
            ModelEndpoint::new("openai", "gpt-test-probe"),
            "marlin-no-write-ok",
        ),
    )
    .await;

    assert_eq!(receipt.status, ModelNoWriteProbeStatus::MarkerMissing);
    assert!(!receipt.marker_present);
    assert!(receipt.failure_message.is_none());
    assert!(!receipt.tool_handoff_allowed);
    assert!(!receipt.filesystem_write_allowed);
}

#[tokio::test]
async fn model_no_write_probe_records_no_live_denial_receipt() {
    let gateway = NoLiveLlmModelGateway::new();
    let receipt = run_model_no_write_probe(
        &gateway,
        ModelNoWriteProbeRequest::new(
            ModelEndpoint::new("openai", "gpt-test-probe"),
            "marlin-no-write-ok",
        ),
    )
    .await;

    assert_eq!(receipt.status, ModelNoWriteProbeStatus::Failed);
    assert!(!receipt.marker_present);
    assert_eq!(receipt.choice_count.get(), 0);
    assert!(
        receipt
            .failure_message
            .as_ref()
            .is_some_and(|message| message.as_str().contains(NO_LIVE_LLM_GATE_DENIAL_MESSAGE))
    );
    assert!(!receipt.tool_handoff_allowed);
    assert!(!receipt.filesystem_write_allowed);
    assert_eq!(gateway.denied_requests().len(), 1);
}

#[tokio::test]
async fn no_live_http_fixture_denies_stream_provider_posts() {
    let fixture = NoLiveHttpModelGatewayFixture::start().await;
    let response = reqwest::Client::new()
        .post(fixture.chat_completions_url())
        .json(&serde_json::json!({
            "model": "anthropic/claude-opus-4-8",
            "stream": true,
            "messages": [
                {
                    "role": "user",
                    "content": "stream crate must not cross live provider boundary"
                }
            ]
        }))
        .send()
        .await
        .expect("no-live fixture should deny stream provider request");

    assert_eq!(response.status().as_u16(), fixture.denial_status());
    assert_eq!(
        response
            .text()
            .await
            .expect("no-live denial body should be readable"),
        NO_LIVE_LLM_GATE_DENIAL_MESSAGE
    );
}

#[tokio::test]
#[ignore = "requires MARLIN_LIVE_LLM_GATE=1 and live Weaver provider credentials"]
async fn live_weaver_stream_gateway_completes_provider_neutral_request() {
    if !live_llm_gate_enabled() {
        eprintln!("skipping live LLM gate: set {LIVE_LLM_GATE_ENV}=1 to enable");
        return;
    }

    let provider = required_live_llm_env(LIVE_LLM_PROVIDER_ENV);
    let model = required_live_llm_env(LIVE_LLM_MODEL_ENV);
    let gateway = live_weaver_gateway(&provider, live_llm_timeout());
    let endpoint = ModelEndpoint::new(provider, model);
    let request = ModelNoWriteProbeRequest::new(endpoint, "marlin-live-llm-ok")
        .with_system_prompt("Reply only with the requested marker.")
        .with_user_prompt("Return exactly: marlin-live-llm-ok")
        .with_options(ModelGatewayCompletionOptions {
            max_tokens: Some(24),
            ..Default::default()
        });

    let timeout_duration = live_llm_timeout();
    let started_at = Instant::now();
    let receipt = timeout(
        timeout_duration,
        run_model_no_write_probe(&gateway, request),
    )
    .await
    .unwrap_or_else(|_| {
        panic!(
            "live LLM completion exceeded {} ms",
            timeout_duration.as_millis()
        )
    });
    let elapsed = started_at.elapsed();

    assert_eq!(receipt.status, ModelNoWriteProbeStatus::Completed);
    assert!(receipt.marker_present);
    assert!(
        receipt
            .completion_id
            .as_ref()
            .is_some_and(|id| !id.as_str().trim().is_empty()),
        "completion id is empty"
    );
    assert!(
        receipt
            .completion_model
            .as_ref()
            .is_some_and(|model| !model.as_str().trim().is_empty()),
        "completion model is empty"
    );
    assert!(!receipt.tool_handoff_allowed);
    assert!(!receipt.filesystem_write_allowed);

    eprintln!(
        "live LLM gate completed provider-neutral response in {} ms via model {}",
        elapsed.as_millis(),
        receipt
            .completion_model
            .as_ref()
            .map(|model| model.as_str())
            .unwrap_or("unknown")
    );
}

fn live_llm_gate_enabled() -> bool {
    matches!(
        env::var(LIVE_LLM_GATE_ENV).as_deref(),
        Ok("1" | "true" | "TRUE" | "yes" | "YES" | "on" | "ON")
    )
}

fn required_live_llm_env(name: &str) -> String {
    env::var(name).unwrap_or_else(|_| panic!("{name} must be set when {LIVE_LLM_GATE_ENV}=1"))
}

pub(super) fn live_weaver_gateway(provider: &str, timeout: Duration) -> WeaverModelGateway {
    let (protocol, url, default_key) = match provider {
        "openai" => (
            weaver_llm::Protocol::OpenAiResponses,
            "https://api.openai.com/v1/responses",
            "OPENAI_API_KEY",
        ),
        "anthropic" => (
            weaver_llm::Protocol::Anthropic,
            "https://api.anthropic.com/v1/messages",
            "ANTHROPIC_API_KEY",
        ),
        "deepseek" => (
            weaver_llm::Protocol::OpenAi,
            "https://api.deepseek.com/chat/completions",
            "DEEPSEEK_API_KEY",
        ),
        "openrouter" => (
            weaver_llm::Protocol::OpenAi,
            "https://openrouter.ai/api/v1/chat/completions",
            "OPENROUTER_API_KEY",
        ),
        _ => panic!("no live Weaver provider binding for {provider}"),
    };
    let key_name =
        env::var(LIVE_LLM_PROVIDER_API_KEY_ENV).unwrap_or_else(|_| default_key.to_owned());
    let key = required_live_llm_env(&key_name);
    WeaverModelGateway::new(
        weaver_llm::Client::new(timeout, 1024 * 1024).expect("Weaver client"),
        provider.into(),
        weaver_llm::Endpoint::new(protocol, url).expect("provider endpoint"),
        key,
    )
}

fn live_llm_timeout() -> Duration {
    env::var(LIVE_LLM_TIMEOUT_MS_ENV)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|millis| *millis > 0)
        .map(Duration::from_millis)
        .unwrap_or(Duration::from_millis(DEFAULT_LIVE_LLM_TIMEOUT_MS))
}
