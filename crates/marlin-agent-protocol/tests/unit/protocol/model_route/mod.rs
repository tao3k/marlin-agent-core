mod admission;
mod artifact;

use marlin_agent_protocol::{
    MODEL_ROUTE_ADMISSION_SCHEMA_ID, ModelCommandMatcher, ModelContextForkMode, ModelEndpoint,
    ModelEndpointContractError, ModelRouteAdmissionMode, ModelRouteAdmissionRequest,
    ModelRouteAdmissionResponse, ModelRouteAgentScope, ModelRouteDecision, ModelRouteReceipt,
    ModelRouteRequest, ModelRouteRule, ModelSessionLifecycle, ModelSessionPolicy,
    RuntimeEnvironmentActivationPolicy, RuntimeEnvironmentActivationReceipt,
    RuntimeEnvironmentActivationStatus,
};

#[test]
fn model_endpoint_keeps_real_provider_and_model_names() {
    let endpoint = ModelEndpoint::new("anthropic", "claude-opus-4-8").with_alias("deep-review");

    assert_eq!(endpoint.provider.as_str(), "anthropic");
    assert_eq!(endpoint.model.as_str(), "claude-opus-4-8");
    assert_eq!(
        endpoint.alias.as_ref().expect("alias").as_str(),
        "deep-review"
    );
    assert_eq!(
        endpoint.litellm_model_id().as_str(),
        "anthropic/claude-opus-4-8"
    );
}

#[test]
fn model_endpoint_rejects_provider_and_model_identity_confusion() {
    assert_eq!(
        ModelEndpoint::new("openai", "model-latest").validate_contract(),
        Err(ModelEndpointContractError::OpenAiModelMustBeGpt {
            model: "model-latest".into()
        })
    );
    assert_eq!(
        ModelEndpoint::new("anthropic", "anthropic").validate_contract(),
        Err(ModelEndpointContractError::ModelLooksLikeProvider {
            provider: "anthropic".into(),
            model: "anthropic".into()
        })
    );
    assert_eq!(
        ModelEndpoint::new("claude-opus-4-8", "claude-opus-4-8").validate_contract(),
        Err(ModelEndpointContractError::ProviderLooksLikeModel {
            provider: "claude-opus-4-8".into()
        })
    );
    assert_eq!(
        ModelEndpoint::new("openai", "gpt-5-mini").validate_contract(),
        Ok(())
    );
    assert_eq!(
        ModelEndpoint::new("anthropic", "claude-opus-4-8").validate_contract(),
        Ok(())
    );
}

#[test]
fn model_route_rule_serializes_command_globs_and_session_lifecycle() {
    let rule = ModelRouteRule::new(
        "cargo-test-cheap",
        10,
        ModelCommandMatcher::new()
            .with_executable_glob("cargo")
            .with_argv_glob("cargo test*")
            .with_sub_agent_role_glob("tester")
            .with_agent_scope_glob("SubAgent")
            .with_cwd_glob("*/marlin-agent-core")
            .with_workspace_glob("*/marlin-agent-core")
            .with_command_kind_glob("test"),
        ModelEndpoint::new("openai", "gpt-5-mini"),
    )
    .with_session(ModelSessionPolicy::pooled(
        "workspace:testers",
        ModelContextForkMode::ForkSnapshot,
    ));

    let value = serde_json::to_value(&rule).expect("rule serializes");
    assert_eq!(value["rule_id"], "cargo-test-cheap");
    assert_eq!(value["endpoint"]["provider"], "openai");
    assert_eq!(value["endpoint"]["model"], "gpt-5-mini");
    assert_eq!(value["matcher"]["argv_globs"][0], "cargo test*");
    assert_eq!(value["matcher"]["agent_scope_globs"][0], "SubAgent");
    assert_eq!(value["matcher"]["cwd_globs"][0], "*/marlin-agent-core");
    assert_eq!(
        value["matcher"]["workspace_globs"][0],
        "*/marlin-agent-core"
    );
    assert_eq!(value["matcher"]["command_kind_globs"][0], "test");
    assert_eq!(value["session"]["context"], "ForkSnapshot");
    assert_eq!(
        value["session"]["lifecycle"]["Pooled"]["pool"],
        "workspace:testers"
    );

    let decoded: ModelRouteRule = serde_json::from_value(value).expect("rule deserializes");
    assert_eq!(decoded, rule);
}

#[test]
fn model_route_rule_defaults_missing_matcher_dimensions_and_session() {
    let value = serde_json::json!({
        "rule_id": "cargo-check-defaults",
        "priority": 5,
        "matcher": {
            "executable_globs": ["cargo"]
        },
        "endpoint": {
            "provider": "openai",
            "model": "gpt-5-mini"
        }
    });

    let decoded: ModelRouteRule = serde_json::from_value(value).expect("rule deserializes");

    assert_eq!(decoded.matcher.executable_globs, vec!["cargo"]);
    assert!(decoded.matcher.argv_globs.is_empty());
    assert!(decoded.matcher.cwd_globs.is_empty());
    assert!(decoded.matcher.workspace_globs.is_empty());
    assert!(decoded.matcher.sub_agent_role_globs.is_empty());
    assert!(decoded.matcher.agent_scope_globs.is_empty());
    assert!(decoded.matcher.command_kind_globs.is_empty());
    assert_eq!(decoded.session, ModelSessionPolicy::default());
}

#[test]
fn model_route_rejects_removed_hook_agent_scope_keys() {
    let matcher_error = serde_json::from_value::<ModelRouteRule>(serde_json::json!({
        "rule_id": "removed-hook-scope-glob",
        "priority": 5,
        "matcher": {
            "hook_agent_scope_globs": ["SubAgent"]
        },
        "endpoint": {
            "provider": "openai",
            "model": "gpt-5-mini"
        }
    }))
    .expect_err("removed matcher key must not be accepted");
    assert!(matcher_error.to_string().contains("hook_agent_scope_globs"));

    let request_error = serde_json::from_value::<ModelRouteRequest>(serde_json::json!({
        "argv": ["gpt-5.5", "sub-agent", "run"],
        "hook_agent_scope": "CustomerAgent"
    }))
    .expect_err("removed request key must not be accepted");
    assert!(request_error.to_string().contains("hook_agent_scope"));

    let receipt_error = serde_json::from_value::<ModelRouteReceipt>(serde_json::json!({
        "rule_id": "removed-hook-scope",
        "matched_globs": ["agent_scope:CustomerAgent"],
        "command_line": "gpt-5.5 sub-agent run",
        "litellm_model_id": "openai/gpt-5-mini",
        "session_lifecycle": "Ephemeral",
        "context_fork": "Minimal",
        "requested_session_id": null,
        "hook_agent_scope": "CustomerAgent",
        "fallback_reason": null
    }))
    .expect_err("removed receipt key must not be accepted");
    assert!(receipt_error.to_string().contains("hook_agent_scope"));
}

#[test]
fn model_route_decision_receipt_records_context_and_lifecycle() {
    let request = ModelRouteRequest::command(["gpt-5.5", "exec", "review"])
        .with_workspace("/tmp/workspace")
        .with_sub_agent_role("reviewer")
        .with_agent_scope(ModelRouteAgentScope::SubAgent);
    let receipt = ModelRouteReceipt {
        rule_id: "reviewer-opus".into(),
        matched_globs: vec!["sub_agent_role:reviewer".to_owned()],
        command_line: request.command_line(),
        litellm_model_id: "anthropic/claude-opus-4-8".into(),
        session_lifecycle: ModelSessionLifecycle::Persistent {
            key: "workspace:reviewer".into(),
        },
        context_fork: ModelContextForkMode::ForkSnapshot,
        requested_session_id: Some("reviewer-session".into()),
        agent_scope: Some(ModelRouteAgentScope::SubAgent),
        environment_activation: Some(RuntimeEnvironmentActivationReceipt::planned(
            &RuntimeEnvironmentActivationPolicy::default(),
        )),
        fallback_reason: None,
    };
    let decision = ModelRouteDecision {
        endpoint: ModelEndpoint::new("anthropic", "claude-opus-4-8"),
        session: ModelSessionPolicy::persistent(
            "workspace:reviewer",
            ModelContextForkMode::ForkSnapshot,
        )
        .with_requested_session_id("reviewer-session"),
        receipt,
    };

    assert_eq!(
        decision.receipt.session_lifecycle,
        ModelSessionLifecycle::Persistent {
            key: "workspace:reviewer".into()
        }
    );
    assert_eq!(
        decision.receipt.context_fork,
        ModelContextForkMode::ForkSnapshot
    );
    assert_eq!(
        decision
            .receipt
            .environment_activation
            .as_ref()
            .expect("environment receipt")
            .status,
        RuntimeEnvironmentActivationStatus::Planned
    );
    assert_eq!(decision.receipt.command_line, "gpt-5.5 exec review");
    assert_eq!(
        decision.receipt.agent_scope,
        Some(ModelRouteAgentScope::SubAgent)
    );
}

#[test]
fn model_route_admission_request_normalizes_chat_intent_defaults() {
    let request = ModelRouteAdmissionRequest::chat(
        ModelRouteRequest::command(["marlin", "chat"]).with_command_kind("chat"),
    )
    .with_precision_tier(" ")
    .with_privacy_tier("")
    .with_evidence_profile("\t")
    .with_latency_budget_ms(45_000)
    .with_artifact_ref("artifact://evidence-pack/001");

    let intent = request.intent();

    assert_eq!(intent.task_kind.as_str(), "chat");
    assert_eq!(intent.modality.as_str(), "text");
    assert_eq!(intent.precision_tier.as_str(), "high");
    assert_eq!(intent.privacy_tier.as_str(), "private");
    assert_eq!(intent.latency_budget_ms, 45_000);
    assert_eq!(intent.evidence_profile.as_str(), "local-knowledge-chat");
    assert_eq!(
        intent.artifact_refs[0].as_str(),
        "artifact://evidence-pack/001"
    );
}

#[test]
fn model_route_admission_response_serializes_schema_and_mode() {
    let request = ModelRouteAdmissionRequest::chat(ModelRouteRequest::command(["marlin", "chat"]));
    let receipt = ModelRouteReceipt {
        rule_id: "chat-default".into(),
        matched_globs: vec!["command_kind:chat".to_owned()],
        command_line: "marlin chat".to_owned(),
        litellm_model_id: "openai/gpt-5-mini".into(),
        session_lifecycle: ModelSessionLifecycle::Ephemeral,
        context_fork: ModelContextForkMode::ForkSnapshot,
        requested_session_id: None,
        agent_scope: None,
        environment_activation: None,
        fallback_reason: None,
    };
    let decision = ModelRouteDecision {
        endpoint: ModelEndpoint::new("openai", "gpt-5-mini"),
        session: ModelSessionPolicy::default(),
        receipt,
    };
    let response = ModelRouteAdmissionResponse::deterministic(request.intent(), decision);
    let value = serde_json::to_value(&response).expect("response serializes");

    assert_eq!(value["schema_id"], MODEL_ROUTE_ADMISSION_SCHEMA_ID);
    assert_eq!(value["model_routing_mode"], "Deterministic");

    let decoded: ModelRouteAdmissionResponse =
        serde_json::from_value(value).expect("response deserializes");
    assert_eq!(
        decoded.model_routing_mode,
        ModelRouteAdmissionMode::Deterministic
    );
    assert_eq!(decoded.schema_id, MODEL_ROUTE_ADMISSION_SCHEMA_ID);
}
