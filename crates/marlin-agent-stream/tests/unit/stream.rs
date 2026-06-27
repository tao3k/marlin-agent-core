use std::{
    env,
    time::{Duration, Instant},
};

use marlin_agent_protocol::{ModelEndpoint, ModelEndpointContractError, ModelGatewayError};
use marlin_agent_stream::{
    ChunkGate, LiteLlmModelClient, LiteLlmStreamGateway, ModelGatewayCompletionChoice,
    ModelGatewayCompletionOptions, ModelGatewayCompletionResponse, ModelNoWriteProbeRequest,
    ModelNoWriteProbeStatus, ModelStreamChunk, ModelStreamEvent, ModelStreamGateway,
    ModelStreamRequest, ModelStreamTransport, assistant_gateway_message, run_model_no_write_probe,
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
async fn litellm_client_validates_endpoint_contract_before_network_call() {
    let client = LiteLlmModelClient::new();
    let endpoint = ModelEndpoint::new("openai", "codex");
    let result = client.complete(&endpoint, vec![], None).await;

    assert!(matches!(
        result,
        Err(ModelGatewayError::EndpointContract(
            ModelEndpointContractError::CodexIsNotModelName { .. }
        ))
    ));
}

#[tokio::test]
async fn litellm_stream_gateway_validates_endpoint_contract_before_network_call() {
    let gateway = LiteLlmStreamGateway::new();
    let request = ModelStreamRequest::new(ModelEndpoint::new("openai", "codex"), vec![]);
    let result = gateway.complete(request).await;

    assert!(matches!(
        result,
        Err(ModelGatewayError::EndpointContract(
            ModelEndpointContractError::CodexIsNotModelName { .. }
        ))
    ));
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
#[ignore = "requires MARLIN_LIVE_LLM_GATE=1 and live LiteLLM provider credentials"]
async fn live_litellm_stream_gateway_completes_provider_neutral_request() {
    if !live_llm_gate_enabled() {
        eprintln!("skipping live LLM gate: set {LIVE_LLM_GATE_ENV}=1 to enable");
        return;
    }

    let provider = required_live_llm_env(LIVE_LLM_PROVIDER_ENV);
    let model = required_live_llm_env(LIVE_LLM_MODEL_ENV);
    require_live_provider_key(&provider);
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
        run_model_no_write_probe(&LiteLlmStreamGateway::new(), request),
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

fn require_live_provider_key(provider: &str) {
    if let Ok(env_name) = env::var(LIVE_LLM_PROVIDER_API_KEY_ENV) {
        if env::var(&env_name).is_ok_and(|value| !value.trim().is_empty()) {
            return;
        }
        panic!(
            "{env_name} must be set when {LIVE_LLM_PROVIDER_API_KEY_ENV}={env_name} and {LIVE_LLM_GATE_ENV}=1"
        );
    }

    let expected_env_names: &[&str] = match provider {
        "anthropic" => &["ANTHROPIC_API_KEY"],
        "deepseek" => &["DEEPSEEK_API_KEY"],
        "openai" => &["OPENAI_API_KEY"],
        "openrouter" => &["OPENROUTER_API_KEY"],
        _ => return,
    };

    if expected_env_names
        .iter()
        .any(|name| env::var(name).is_ok_and(|value| !value.trim().is_empty()))
    {
        return;
    }

    panic!(
        "{provider} live LLM gate requires one of {:?}, or set {LIVE_LLM_PROVIDER_API_KEY_ENV} to a provider-specific key env name",
        expected_env_names
    );
}

fn live_llm_timeout() -> Duration {
    env::var(LIVE_LLM_TIMEOUT_MS_ENV)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|millis| *millis > 0)
        .map(Duration::from_millis)
        .unwrap_or(Duration::from_millis(DEFAULT_LIVE_LLM_TIMEOUT_MS))
}
