use std::{
    env,
    time::{Duration, Instant},
};

use marlin_agent_protocol::{ModelEndpoint, ModelEndpointContractError, ModelGatewayError};
use marlin_agent_stream::{
    ChunkGate, ModelGatewayCompletionChoice, ModelGatewayCompletionOptions,
    ModelGatewayCompletionResponse, ModelNoWriteProbeRequest, ModelNoWriteProbeStatus,
    ModelStreamChunk, ModelStreamEvent, ModelStreamGateway, ModelStreamRequest,
    ModelStreamTransport, WeaverModelGateway, assistant_gateway_message, run_model_no_write_probe,
    system_gateway_message, user_gateway_message,
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
            "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_fixture\",\"model\":\"gpt-fixture\",\"status\":\"completed\",\"output\":[]}}\n\n",
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
            "status":"completed","id":"resp_second","model":"gpt-fixture","output":[]
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
    use serde_json::json;
    use tokio_stream::StreamExt;
    use weaver_llm::{Message, Options, Request, Role, Tool, TurnEvent};

    let mut history = vec![Message {
        role: Role::User,
        content: concat!(
            "Call read_marker exactly once with an empty object before answering. ",
            "After receiving the tool result, reply with its exact text."
        )
        .into(),
        images: Vec::new(),
        name: None,
        tool_call_id: None,
        tool_calls: vec![],
        responses_output: None,
    }];
    let options = Options {
        max_tokens: Some(512),
        tools: vec![Tool {
            name: "read_marker".into(),
            description: "Return a fixture marker".into(),
            parameters: json!({"type":"object"}),
        }],
        ..Options::default()
    };
    let mut events = Box::pin(
        gateway
            .stream_turn(
                endpoint,
                &Request {
                    model: endpoint.model.as_str(),
                    messages: &history,
                    options: &options,
                },
            )
            .await
            .expect("first stream")
            .into_stream(),
    );
    let (tool_calls, responses_output) = loop {
        match events
            .next()
            .await
            .expect("first terminal event")
            .expect("first event")
        {
            TurnEvent::Text(_) => {}
            TurnEvent::Finished {
                tool_calls,
                responses_output,
                ..
            } => {
                break (tool_calls, responses_output);
            }
        }
    };
    let output = serde_json::to_value(responses_output.as_ref().expect("Responses output"))
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
    assert_eq!(tool_calls.len(), 1);
    assert_eq!(tool_calls[0].name, "read_marker");
    assert_eq!(tool_calls[0].arguments, json!({}));
    let id = tool_calls[0].id.clone();
    assert!(events.next().await.is_none());
    history.extend([
        Message {
            role: Role::Assistant,
            content: String::new(),
            images: Vec::new(),
            name: None,
            tool_call_id: None,
            tool_calls,
            responses_output,
        },
        Message {
            role: Role::Tool,
            content: "marlin-live-tool-ok".into(),
            images: Vec::new(),
            name: None,
            tool_call_id: Some(id),
            tool_calls: vec![],
            responses_output: None,
        },
    ]);
    let mut events = Box::pin(
        gateway
            .stream_turn(
                endpoint,
                &Request {
                    model: endpoint.model.as_str(),
                    messages: &history,
                    options: &options,
                },
            )
            .await
            .expect("second stream")
            .into_stream(),
    );
    let mut text = String::new();
    let id = loop {
        match events
            .next()
            .await
            .expect("second terminal event")
            .expect("second event")
        {
            TurnEvent::Text(delta) => text.push_str(&delta),
            TurnEvent::Finished {
                identity,
                tool_calls,
                ..
            } => {
                assert!(tool_calls.is_empty());
                break identity.expect("provider identity").id;
            }
        }
    };
    assert_eq!(text.trim(), "marlin-live-tool-ok");
    assert!(events.next().await.is_none());
    id
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
