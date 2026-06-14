use marlin_agent_protocol::{ModelRouteAgentScope, ModelRouteRequest};
use marlin_gerbil_scheme::{
    GerbilDeckRuntimeContextMode, GerbilDeckRuntimeIsolationMode,
    GerbilDeckRuntimeModelRoutePolicy, GerbilDeckRuntimeModelRoutePolicyRequest,
    GerbilDeckRuntimeModelRouteSelectedPolicy, GerbilDeckRuntimeModelRouteSelectionReceipt,
    GerbilDeckRuntimeSelectedPolicyKind,
};

#[test]
fn gerbil_deck_runtime_policy_request_uses_typed_agent_scope() {
    let policy =
        GerbilDeckRuntimeModelRoutePolicy::new("cheap-test-runner", "openai", "gpt-5-mini")
            .with_command_prefix("cargo test")
            .with_agent_scope("sub-agent");
    let request = ModelRouteRequest::command(["cargo", "test"])
        .with_agent_scope(ModelRouteAgentScope::SubAgent)
        .with_sub_agent_role("tester");

    let scheme_request =
        GerbilDeckRuntimeModelRoutePolicyRequest::from_model_route_request([policy], &request);

    assert_eq!(scheme_request.command, "cargo test");
    assert_eq!(scheme_request.agent_scope, "sub-agent");
    assert_eq!(scheme_request.policies[0].provider, "openai");
    assert_eq!(scheme_request.policies[0].model, "gpt-5-mini");
}

#[test]
fn gerbil_deck_runtime_policy_receipt_preserves_provider_model_identity() {
    let receipt = GerbilDeckRuntimeModelRouteSelectionReceipt {
        schema_id: "marlin-deck-runtime.model-route-selection.v1".to_string(),
        command: "cargo test".to_string(),
        agent_scope: "sub-agent".to_string(),
        matched: true,
        policy: Some(GerbilDeckRuntimeModelRouteSelectedPolicy {
            kind: GerbilDeckRuntimeSelectedPolicyKind::new(
                "marlin-deck-runtime.model-route-policy.v1",
            ),
            name: "cheap-test-runner".to_string(),
            provider: "openai".to_string(),
            model: "gpt-5-mini".to_string(),
            command_prefixes: vec!["cargo test".to_string()],
            agent_scopes: vec!["sub-agent".to_string()],
            context_mode: GerbilDeckRuntimeContextMode::new("forked-context"),
            isolation_mode: GerbilDeckRuntimeIsolationMode::new("workspace-isolated"),
        }),
    };

    let policy = receipt.selected_policy().expect("selected policy");
    assert!(receipt.matched);
    assert_eq!(policy.provider, "openai");
    assert_eq!(policy.model, "gpt-5-mini");
    assert_eq!(policy.context_mode.as_str(), "forked-context");
}
