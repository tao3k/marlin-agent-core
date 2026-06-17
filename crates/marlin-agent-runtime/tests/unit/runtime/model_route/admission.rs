use marlin_agent_protocol::{
    MODEL_ROUTE_ADMISSION_SCHEMA_ID, ModelCommandMatcher, ModelEndpoint, ModelRouteAdmissionMode,
    ModelRouteAdmissionRequest, ModelRouteRequest, ModelRouteRule,
};
use marlin_agent_runtime::{
    CompiledModelRouteResolver, ModelRouteAdmissionError, admit_model_route_with_resolver,
};

#[test]
fn admission_normalizes_chat_request_and_returns_deterministic_decision() {
    let resolver = CompiledModelRouteResolver::new(vec![ModelRouteRule::new(
        "chat-default",
        100,
        ModelCommandMatcher::new().with_command_kind_glob("chat"),
        ModelEndpoint::new("openai", "gpt-5-mini"),
    )])
    .expect("route rules compile");
    let request = ModelRouteAdmissionRequest::chat(
        ModelRouteRequest::command(["marlin", "chat"]).with_command_kind("chat"),
    )
    .with_precision_tier(" ")
    .with_privacy_tier("")
    .with_latency_budget_ms(45_000)
    .with_evidence_profile("\n")
    .with_artifact_ref("artifact://evidence-pack/001");

    let response =
        admit_model_route_with_resolver(&resolver, request).expect("chat admission resolves");

    assert_eq!(response.schema_id, MODEL_ROUTE_ADMISSION_SCHEMA_ID);
    assert_eq!(
        response.model_routing_mode,
        ModelRouteAdmissionMode::Deterministic
    );
    assert_eq!(response.intent.task_kind.as_str(), "chat");
    assert_eq!(response.intent.modality.as_str(), "text");
    assert_eq!(response.intent.precision_tier.as_str(), "high");
    assert_eq!(response.intent.privacy_tier.as_str(), "private");
    assert_eq!(response.intent.latency_budget_ms, 45_000);
    assert_eq!(
        response.intent.evidence_profile.as_str(),
        "local-knowledge-chat"
    );
    assert_eq!(
        response.intent.artifact_refs[0].as_str(),
        "artifact://evidence-pack/001"
    );
    assert_eq!(response.decision.endpoint.provider.as_str(), "openai");
    assert_eq!(response.decision.endpoint.model.as_str(), "gpt-5-mini");
    assert_eq!(response.decision.receipt.rule_id.as_str(), "chat-default");
    assert!(
        response
            .decision
            .receipt
            .matched_globs
            .contains(&"command_kind:chat".to_owned())
    );
}

#[test]
fn admission_returns_typed_error_when_no_route_matches() {
    let resolver = CompiledModelRouteResolver::new(vec![ModelRouteRule::new(
        "non-chat",
        100,
        ModelCommandMatcher::new().with_command_kind_glob("review"),
        ModelEndpoint::new("openai", "gpt-5-mini"),
    )])
    .expect("route rules compile");
    let request = ModelRouteAdmissionRequest::chat(
        ModelRouteRequest::command(["marlin", "chat"]).with_command_kind("chat"),
    );

    let error = admit_model_route_with_resolver(&resolver, request)
        .expect_err("chat admission should not match review route");

    assert!(matches!(
        error,
        ModelRouteAdmissionError::NoMatchingRoute { .. }
    ));
    assert!(
        error
            .to_string()
            .contains("no deterministic route for task `chat`")
    );
}
