use marlin_agent_protocol::{
    ModelCommandMatcher, ModelEndpoint, ModelRouteRequest, ModelRouteRule,
};
use marlin_agent_runtime::ModelRouteSelectionProjectionSource;
use marlin_agent_runtime::{CompiledModelRouteResolver, ModelRouteSelectionProjectionError};

#[test]
fn native_projection_uses_original_policy_index_after_priority_sort() {
    let resolver = CompiledModelRouteResolver::new(vec![
        ModelRouteRule::new(
            "native-cheap",
            1,
            ModelCommandMatcher::new().with_argv_glob("cargo test*"),
            ModelEndpoint::new("openai", "gpt-5.4-mini"),
        ),
        ModelRouteRule::new(
            "native-expensive",
            100,
            ModelCommandMatcher::new().with_argv_glob("cargo test*"),
            ModelEndpoint::new("anthropic", "claude-opus-4-8"),
        ),
    ])
    .expect("route rules compile");
    let request = ModelRouteRequest::command(["cargo", "test", "-p", "marlin-agent-runtime"]);

    let priority_decision = resolver
        .resolve(&request)
        .expect("priority resolver still resolves");
    assert_eq!(
        priority_decision.receipt.rule_id.as_str(),
        "native-expensive"
    );

    let native_decision = resolver
        .resolve_selected_policy_index(&request, 0)
        .expect("native policy index projects");
    assert_eq!(native_decision.receipt.rule_id.as_str(), "native-cheap");
    assert_eq!(
        native_decision.receipt.litellm_model_id.as_str(),
        "openai/gpt-5.4-mini"
    );
    assert!(
        native_decision
            .receipt
            .matched_globs
            .contains(&"argv:cargo test*".to_owned())
    );
}

#[test]
fn native_projection_emits_typed_projection_receipt() {
    let resolver = CompiledModelRouteResolver::new(vec![ModelRouteRule::new(
        "native-cheap",
        1,
        ModelCommandMatcher::new().with_argv_glob("cargo test*"),
        ModelEndpoint::new("openai", "gpt-5.4-mini"),
    )])
    .expect("route rules compile");
    let request = ModelRouteRequest::command(["cargo", "test", "-p", "marlin-agent-runtime"]);

    let projected = resolver
        .resolve_selected_policy_index_with_source(
            &request,
            0,
            ModelRouteSelectionProjectionSource::NativeAbiPolicyIndex,
        )
        .expect("native policy index projects with receipt");

    assert_eq!(
        projected.projection.source,
        ModelRouteSelectionProjectionSource::NativeAbiPolicyIndex
    );
    assert_eq!(projected.projection.policy_index, 0);
    assert_eq!(projected.projection.rule_id.as_str(), "native-cheap");
    assert_eq!(
        projected.projection.litellm_model_id.as_str(),
        "openai/gpt-5.4-mini"
    );
    assert_eq!(
        projected.decision.receipt.rule_id,
        projected.projection.rule_id
    );
}

#[test]
fn native_projection_rejects_unknown_policy_index() {
    let resolver = CompiledModelRouteResolver::new(vec![ModelRouteRule::new(
        "only-route",
        1,
        ModelCommandMatcher::new().with_argv_glob("cargo test*"),
        ModelEndpoint::new("openai", "gpt-5.4-mini"),
    )])
    .expect("route rule compiles");

    let error = resolver
        .resolve_selected_policy_index(&ModelRouteRequest::command(["cargo", "test"]), 7)
        .expect_err("unknown policy index should fail");

    assert_eq!(
        error,
        ModelRouteSelectionProjectionError::UnknownPolicyIndex { policy_index: 7 }
    );
}

#[test]
fn native_projection_revalidates_selected_policy_against_request() {
    let resolver = CompiledModelRouteResolver::new(vec![ModelRouteRule::new(
        "cargo-tests",
        1,
        ModelCommandMatcher::new().with_executable_glob("cargo"),
        ModelEndpoint::new("openai", "gpt-5.4-mini"),
    )])
    .expect("route rule compiles");

    let error = resolver
        .resolve_selected_policy_index(&ModelRouteRequest::command(["just", "test"]), 0)
        .expect_err("stale selected policy should fail");

    assert_eq!(
        error,
        ModelRouteSelectionProjectionError::SelectedRuleDidNotMatch { policy_index: 0 }
    );
}
