use marlin_agent_protocol::{
    ModelCommandMatcher, ModelContextForkMode, ModelEndpoint, ModelRouteRequest, ModelRouteRule,
    ModelSessionLifecycle, ModelSessionPolicy, RuntimeEnvironmentActivation,
    RuntimeEnvironmentActivationReceipt, RuntimeEnvironmentActivationStatus, RuntimeEnvrcPolicy,
    RuntimeShellIsolationPolicy, SubAgentSpawnConfigSet, SubAgentSpawnProfileId,
};
use marlin_agent_runtime::{CompiledModelRouteResolver, SessionKind, TokioAgentRuntime};

const MODEL_ROUTE_SUB_AGENT_PROFILE_TOML: &str = r#"
[[profiles]]
profile_id = "builder"
agent_type = "builder"
role = "builder"

[profiles.environment_activation.activation.Direnv]
envrc = "Project"
capture_delta = true

[profiles.environment_activation.shell]
isolate_host_environment = true
allowlist = ["PATH", "HOME"]
denylist = ["AWS_SECRET_ACCESS_KEY"]
"#;

#[test]
fn model_route_session_binding_covers_create_reuse_reject_and_environment_receipts() {
    let profile_config = SubAgentSpawnConfigSet::from_toml_str(MODEL_ROUTE_SUB_AGENT_PROFILE_TOML)
        .expect("sub-agent profile TOML compiles");
    let builder_profile = profile_config
        .profile(&SubAgentSpawnProfileId::from("builder"))
        .expect("builder profile exists");
    let builder_environment = builder_profile
        .environment_activation
        .as_ref()
        .expect("builder profile carries environment activation");

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
    let create_binding = create_binding.with_environment_activation_receipt(
        RuntimeEnvironmentActivationReceipt::planned(builder_environment),
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
    let expected_shell = RuntimeShellIsolationPolicy::isolated()
        .with_allowed("PATH")
        .with_allowed("HOME")
        .with_denied("AWS_SECRET_ACCESS_KEY");
    let binding_environment = create_binding
        .environment_activation_receipt()
        .expect("create route carries environment receipt");
    assert_eq!(
        binding_environment.status,
        RuntimeEnvironmentActivationStatus::Planned
    );
    assert!(matches!(
        &binding_environment.activation,
        RuntimeEnvironmentActivation::Direnv {
            envrc: RuntimeEnvrcPolicy::Project,
            capture_delta: true,
        }
    ));
    assert_eq!(binding_environment.shell, expected_shell);

    let route_environment = create_binding
        .route_receipt()
        .environment_activation
        .as_ref()
        .expect("route receipt projects environment receipt");
    assert_eq!(
        route_environment.status,
        RuntimeEnvironmentActivationStatus::Planned
    );
    assert!(matches!(
        &route_environment.activation,
        RuntimeEnvironmentActivation::Direnv {
            envrc: RuntimeEnvrcPolicy::Project,
            capture_delta: true,
        }
    ));
    assert_eq!(route_environment.shell, expected_shell);

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
