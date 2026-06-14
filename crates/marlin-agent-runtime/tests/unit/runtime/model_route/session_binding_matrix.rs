use marlin_agent_protocol::{
    ModelCommandMatcher, ModelContextForkMode, ModelEndpoint, ModelRouteRequest, ModelRouteRule,
    ModelSessionLifecycle, ModelSessionPolicy, RuntimeEnvironmentActivationPolicy,
    RuntimeEnvironmentActivationReceipt, RuntimeEnvironmentActivationStatus,
};
use marlin_agent_runtime::{CompiledModelRouteResolver, SessionKind, TokioAgentRuntime};

#[test]
fn model_route_session_binding_covers_create_reuse_reject_and_environment_receipts() {
    let resolver = CompiledModelRouteResolver::new(vec![
        ModelRouteRule::new(
            "builder-create",
            100,
            ModelCommandMatcher::new()
                .with_sub_agent_role_glob("builder")
                .with_command_kind_glob("create"),
            ModelEndpoint::new("openai", "gpt-5-mini"),
        )
        .with_session(ModelSessionPolicy::ephemeral(ModelContextForkMode::Minimal)),
        ModelRouteRule::new(
            "reviewer-reuse",
            90,
            ModelCommandMatcher::new()
                .with_sub_agent_role_glob("reviewer")
                .with_command_kind_glob("reuse"),
            ModelEndpoint::new("anthropic", "claude-opus-4-8"),
        )
        .with_session(ModelSessionPolicy::persistent(
            "workspace:reviewer",
            ModelContextForkMode::ForkSnapshot,
        )),
    ])
    .expect("matrix route rules compile");
    let (runtime, _events) = TokioAgentRuntime::new(4);

    let create_decision = resolver
        .resolve(
            &ModelRouteRequest::command(["gpt-5.5", "sub-agent", "build"])
                .with_sub_agent_role("builder")
                .with_command_kind("create"),
        )
        .expect("create route resolves");
    let (_create_runtime, create_binding) =
        runtime.child_runtime_for_model_route(&create_decision, SessionKind::SubAgent);
    let create_binding =
        create_binding.with_environment_activation_receipt(
            RuntimeEnvironmentActivationReceipt::planned(
                &RuntimeEnvironmentActivationPolicy::default(),
            ),
        );

    assert_eq!(
        create_binding.child_session_id().as_str(),
        "model-route/builder-create/ephemeral"
    );
    assert_eq!(
        create_binding.route_receipt().rule_id.as_str(),
        "builder-create"
    );
    assert_eq!(
        create_binding.route_receipt().command_line,
        "gpt-5.5 sub-agent build"
    );
    assert_eq!(
        create_binding.route_receipt().litellm_model_id.as_str(),
        "openai/gpt-5-mini"
    );
    assert_eq!(
        create_binding.route_receipt().session_lifecycle,
        ModelSessionLifecycle::Ephemeral
    );
    assert_eq!(
        create_binding
            .environment_activation_receipt()
            .expect("create route carries environment receipt")
            .status,
        RuntimeEnvironmentActivationStatus::Planned
    );
    assert_eq!(
        create_binding
            .route_receipt()
            .environment_activation
            .as_ref()
            .expect("route receipt projects environment receipt")
            .status,
        RuntimeEnvironmentActivationStatus::Planned
    );

    let reuse_decision = resolver
        .resolve(
            &ModelRouteRequest::command(["gpt-5.5", "sub-agent", "review"])
                .with_sub_agent_role("reviewer")
                .with_command_kind("reuse"),
        )
        .expect("reuse route resolves");
    let (_first_reuse_runtime, first_reuse_binding) =
        runtime.child_runtime_for_model_route(&reuse_decision, SessionKind::SubAgent);
    let (_second_reuse_runtime, second_reuse_binding) =
        runtime.child_runtime_for_model_route(&reuse_decision, SessionKind::SubAgent);

    assert_eq!(
        first_reuse_binding.child_session_id().as_str(),
        "model-route/persistent/workspace:reviewer"
    );
    assert_eq!(
        first_reuse_binding.child_session_id(),
        second_reuse_binding.child_session_id()
    );
    assert!(matches!(
        &first_reuse_binding.route_receipt().session_lifecycle,
        ModelSessionLifecycle::Persistent { key } if key.as_str() == "workspace:reviewer"
    ));
    assert_eq!(
        first_reuse_binding.route_receipt().rule_id.as_str(),
        "reviewer-reuse"
    );
    assert_eq!(
        first_reuse_binding.route_receipt().command_line,
        "gpt-5.5 sub-agent review"
    );
    assert_eq!(
        first_reuse_binding
            .route_receipt()
            .litellm_model_id
            .as_str(),
        "anthropic/claude-opus-4-8"
    );
    assert_eq!(
        first_reuse_binding.route_receipt().context_fork,
        ModelContextForkMode::ForkSnapshot
    );

    let rejected = resolver.resolve(
        &ModelRouteRequest::command(["gpt-5.5", "sub-agent", "unknown"])
            .with_sub_agent_role("unknown")
            .with_command_kind("reject"),
    );

    assert!(
        rejected.is_none(),
        "unmatched route request is rejected before session binding"
    );
}
