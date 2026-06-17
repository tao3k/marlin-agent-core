use std::fs;

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode, header::CONTENT_TYPE},
};
use marlin_agent_protocol::{
    MODEL_ROUTE_ADMISSION_SCHEMA_ID, ModelCommandMatcher, ModelEndpoint, ModelRouteAdmissionMode,
    ModelRouteAdmissionResponse, ModelRouteRule,
};
use marlin_agent_route::{
    MODEL_ROUTE_CHAT_PATH, ModelRouteHttpErrorBody, ModelRouteHttpState, model_route_router,
    model_route_router_from_toml_path, model_route_router_from_toml_str,
};
use marlin_agent_runtime::CompiledModelRouteResolver;
use serde_json::{Value, json};
use tempfile::tempdir;
use tower::ServiceExt;

#[tokio::test]
async fn chat_route_admission_returns_typed_deterministic_response() {
    let app = chat_router(vec![ModelRouteRule::new(
        "chat-default",
        100,
        ModelCommandMatcher::new().with_command_kind_glob("chat"),
        ModelEndpoint::new("openai", "gpt-5-mini"),
    )]);

    let response = app
        .oneshot(json_request(json!({
            "precisionTier": "",
            "privacyTier": " ",
            "latencyBudgetMs": 45_000,
            "evidenceProfile": "\t",
            "artifactRefs": ["artifact://evidence-pack/001"]
        })))
        .await
        .expect("route service responds");

    assert_eq!(response.status(), StatusCode::OK);
    let admission: ModelRouteAdmissionResponse =
        serde_json::from_slice(&body_bytes(response).await).expect("admission response json");

    assert_eq!(admission.schema_id, MODEL_ROUTE_ADMISSION_SCHEMA_ID);
    assert_eq!(
        admission.model_routing_mode,
        ModelRouteAdmissionMode::Deterministic
    );
    assert_eq!(admission.intent.task_kind.as_str(), "chat");
    assert_eq!(admission.intent.modality.as_str(), "text");
    assert_eq!(admission.intent.precision_tier.as_str(), "high");
    assert_eq!(admission.intent.privacy_tier.as_str(), "private");
    assert_eq!(admission.intent.latency_budget_ms, 45_000);
    assert_eq!(
        admission.intent.evidence_profile.as_str(),
        "local-knowledge-chat"
    );
    assert_eq!(
        admission.intent.artifact_refs[0].as_str(),
        "artifact://evidence-pack/001"
    );
    assert_eq!(admission.decision.endpoint.provider.as_str(), "openai");
    assert_eq!(admission.decision.endpoint.model.as_str(), "gpt-5-mini");
    assert_eq!(admission.decision.receipt.rule_id.as_str(), "chat-default");
}

#[tokio::test]
async fn chat_route_admission_maps_missing_route_to_http_error() {
    let app = chat_router(vec![ModelRouteRule::new(
        "review-only",
        100,
        ModelCommandMatcher::new().with_command_kind_glob("review"),
        ModelEndpoint::new("openai", "gpt-5-mini"),
    )]);

    let response = app
        .oneshot(json_request(json!({})))
        .await
        .expect("route service responds");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let body: Value = serde_json::from_slice(&body_bytes(response).await).expect("error json");
    assert_eq!(body["code"], "MODEL_ROUTE_NOT_FOUND");
    assert!(
        body["message"]
            .as_str()
            .expect("message")
            .contains("no deterministic route for task `chat`")
    );
}

#[tokio::test]
async fn chat_route_admission_maps_invalid_request_json_to_http_error() {
    let app = chat_router(vec![ModelRouteRule::new(
        "chat-default",
        100,
        ModelCommandMatcher::new().with_command_kind_glob("chat"),
        ModelEndpoint::new("openai", "gpt-5-mini"),
    )]);

    let response = app
        .oneshot(json_request(json!({
            "unexpectedField": true
        })))
        .await
        .expect("route service responds");

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body: ModelRouteHttpErrorBody =
        serde_json::from_slice(&body_bytes(response).await).expect("typed error json");
    assert_eq!(body.code, "MODEL_ROUTE_INVALID_REQUEST");
    assert!(body.message.contains("unknown field"));
}

#[tokio::test]
async fn chat_route_admission_can_be_built_from_toml_config() {
    let app = model_route_router_from_toml_str(
        r#"
[[rules]]
rule_id = "chat-from-config"
priority = 100

[rules.matcher]
command_kind_globs = ["chat"]

[rules.endpoint]
provider = "openai"
model = "gpt-5-mini"
"#,
    )
    .expect("TOML route config builds router");

    let response = app
        .oneshot(json_request(json!({})))
        .await
        .expect("route service responds");

    assert_eq!(response.status(), StatusCode::OK);
    let admission: ModelRouteAdmissionResponse =
        serde_json::from_slice(&body_bytes(response).await).expect("admission response json");

    assert_eq!(
        admission.decision.receipt.rule_id.as_str(),
        "chat-from-config"
    );
    assert_eq!(admission.decision.endpoint.provider.as_str(), "openai");
    assert_eq!(admission.decision.endpoint.model.as_str(), "gpt-5-mini");
}

#[tokio::test]
async fn chat_route_admission_can_be_built_from_toml_config_file() {
    let directory = tempdir().expect("temp dir");
    let config_path = directory.path().join("model-route.toml");
    fs::write(
        &config_path,
        r#"
[[rules]]
rule_id = "chat-from-file"
priority = 100

[rules.matcher]
command_kind_globs = ["chat"]

[rules.endpoint]
provider = "openai"
model = "gpt-5-mini"
"#,
    )
    .expect("write model route config");

    let app = model_route_router_from_toml_path(&config_path)
        .expect("TOML route config path builds router");

    let response = app
        .oneshot(json_request(json!({})))
        .await
        .expect("route service responds");

    assert_eq!(response.status(), StatusCode::OK);
    let admission: ModelRouteAdmissionResponse =
        serde_json::from_slice(&body_bytes(response).await).expect("admission response json");

    assert_eq!(
        admission.decision.receipt.rule_id.as_str(),
        "chat-from-file"
    );
    assert_eq!(admission.decision.endpoint.provider.as_str(), "openai");
    assert_eq!(admission.decision.endpoint.model.as_str(), "gpt-5-mini");
}

fn chat_router(rules: Vec<ModelRouteRule>) -> Router {
    let resolver = CompiledModelRouteResolver::new(rules).expect("route rules compile");
    model_route_router(ModelRouteHttpState::new(resolver))
}

fn json_request(body: Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(MODEL_ROUTE_CHAT_PATH)
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .expect("request builds")
}

async fn body_bytes(response: axum::response::Response) -> Vec<u8> {
    to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body bytes")
        .to_vec()
}
