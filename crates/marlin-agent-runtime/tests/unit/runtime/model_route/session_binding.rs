use marlin_agent_protocol::{
    ModelCommandMatcher, ModelContextForkMode, ModelEndpoint, ModelRouteRequest, ModelRouteRule,
    ModelSessionPolicy, RuntimeEnvironmentActivationPolicy, RuntimeEnvironmentActivationReceipt,
    RuntimeEnvironmentActivationStatus,
};
use marlin_agent_runtime::{
    AgentSessionContext, CancellationToken, CompiledModelRouteResolver, ContextNamespace,
    ContextVisibility, RuntimeEnvironment, SessionKind, TokioAgentRuntime,
};
use marlin_agent_stream::{
    LiteLlmModelClientError, ModelStreamGateway, ModelStreamRequest, user_message,
};
use marlin_agent_test_support::ScriptedModelGateway;

#[test]
fn model_route_session_binding_reuses_persistent_key_as_child_session_id() {
    let decision = reviewer_decision(ModelContextForkMode::ForkSnapshot);
    let parent_session = AgentSessionContext::root(
        "session/root",
        ContextVisibility::from_namespaces([
            ContextNamespace::System,
            ContextNamespace::Workspace,
            ContextNamespace::Memory,
        ])
        .with_max_history_items(Some(16)),
    );
    let (runtime, _events) = TokioAgentRuntime::with_session(
        4,
        CancellationToken::new(),
        RuntimeEnvironment::default(),
        parent_session,
    );

    let (child_runtime, binding) =
        runtime.child_runtime_for_model_route(&decision, SessionKind::SubAgent);

    assert_eq!(
        binding.child_session_id().as_str(),
        "model-route/persistent/workspace:reviewer"
    );
    assert_eq!(
        child_runtime
            .session()
            .parent_session_id()
            .expect("child should keep parent")
            .as_str(),
        "session/root"
    );
    assert!(
        child_runtime
            .session()
            .visibility()
            .contains(&ContextNamespace::Memory)
    );
    assert_eq!(
        binding.route_receipt().litellm_model_id.as_str(),
        "anthropic/claude-opus-4-8"
    );
    assert!(binding.isolation_receipt().denied_namespaces().is_empty());
}

#[tokio::test]
async fn model_route_session_binding_uses_scripted_gateway_without_live_llm() {
    let decision = reviewer_decision(ModelContextForkMode::ForkSnapshot);
    let parent_session = AgentSessionContext::root(
        "session/root",
        ContextVisibility::from_namespaces([
            ContextNamespace::System,
            ContextNamespace::Workspace,
            ContextNamespace::Memory,
        ])
        .with_max_history_items(Some(16)),
    );
    let (runtime, _events) = TokioAgentRuntime::with_session(
        4,
        CancellationToken::new(),
        RuntimeEnvironment::default(),
        parent_session,
    );

    let (child_runtime, binding) =
        runtime.child_runtime_for_model_route(&decision, SessionKind::SubAgent);
    let gateway = ScriptedModelGateway::completion_failure("scripted no-live-llm");
    let request = ModelStreamRequest::new(
        ModelEndpoint::new("anthropic", "claude-opus-4-8"),
        vec![user_message("review this patch")],
    );

    let result = gateway.complete(request).await;

    assert!(matches!(
        result,
        Err(LiteLlmModelClientError::Completion(message)) if message == "scripted no-live-llm"
    ));
    assert_eq!(
        child_runtime
            .session()
            .parent_session_id()
            .expect("child should keep parent")
            .as_str(),
        "session/root"
    );

    let requests = gateway.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(
        requests[0].litellm_model_id,
        binding.route_receipt().litellm_model_id.as_str()
    );
    assert_eq!(requests[0].message_count, 1);
}

#[test]
fn model_route_session_binding_isolated_context_requests_system_only() {
    let decision = reviewer_decision(ModelContextForkMode::Isolated);
    let parent_session = AgentSessionContext::root(
        "session/root",
        ContextVisibility::from_namespaces([
            ContextNamespace::System,
            ContextNamespace::Workspace,
            ContextNamespace::Memory,
        ])
        .with_max_history_items(Some(16)),
    );
    let (runtime, _events) = TokioAgentRuntime::with_session(
        4,
        CancellationToken::new(),
        RuntimeEnvironment::default(),
        parent_session,
    );

    let (child_runtime, binding) =
        runtime.child_runtime_for_model_route(&decision, SessionKind::SubAgent);

    assert!(
        child_runtime
            .session()
            .visibility()
            .contains(&ContextNamespace::System)
    );
    assert!(
        !child_runtime
            .session()
            .visibility()
            .contains(&ContextNamespace::Workspace)
    );
    assert!(
        !child_runtime
            .session()
            .visibility()
            .contains(&ContextNamespace::Memory)
    );
    assert_eq!(
        child_runtime.session().visibility().max_history_items(),
        Some(0)
    );
    assert_eq!(
        binding
            .isolation_receipt()
            .granted_visibility()
            .max_history_items(),
        Some(0)
    );
}

#[test]
fn model_route_session_binding_honors_requested_session_id() {
    let resolver = CompiledModelRouteResolver::new(vec![
        ModelRouteRule::new(
            "tester",
            100,
            ModelCommandMatcher::new().with_sub_agent_role_glob("tester"),
            ModelEndpoint::new("openai", "gpt-5-mini"),
        )
        .with_session(
            ModelSessionPolicy::persistent("workspace:tester", ModelContextForkMode::ForkSnapshot)
                .with_requested_session_id("session:tester"),
        ),
    ])
    .expect("route rule compiles");
    let decision = resolver
        .resolve(&ModelRouteRequest::command(["cargo", "test"]).with_sub_agent_role("tester"))
        .expect("tester route resolves");
    let (runtime, _events) = TokioAgentRuntime::new(4);

    let (_child_runtime, binding) =
        runtime.child_runtime_for_model_route(&decision, SessionKind::SubAgent);

    assert_eq!(binding.child_session_id().as_str(), "session:tester");
    assert_eq!(
        binding
            .route_receipt()
            .requested_session_id
            .as_ref()
            .expect("requested session id should be recorded")
            .as_str(),
        "session:tester"
    );
}

#[test]
fn model_route_session_binding_can_attach_environment_activation_receipt() {
    let decision = reviewer_decision(ModelContextForkMode::ForkSnapshot);
    let (runtime, _events) = TokioAgentRuntime::new(4);
    let (_child_runtime, binding) =
        runtime.child_runtime_for_model_route(&decision, SessionKind::SubAgent);
    let binding =
        binding.with_environment_activation_receipt(RuntimeEnvironmentActivationReceipt::planned(
            &RuntimeEnvironmentActivationPolicy::default(),
        ));

    assert_eq!(
        binding
            .environment_activation_receipt()
            .expect("environment receipt")
            .status,
        RuntimeEnvironmentActivationStatus::Planned
    );
    assert_eq!(
        binding
            .route_receipt()
            .environment_activation
            .as_ref()
            .expect("route receipt environment")
            .status,
        RuntimeEnvironmentActivationStatus::Planned
    );
}

fn reviewer_decision(context: ModelContextForkMode) -> marlin_agent_protocol::ModelRouteDecision {
    let resolver = CompiledModelRouteResolver::new(vec![
        ModelRouteRule::new(
            "reviewer-opus",
            100,
            ModelCommandMatcher::new().with_sub_agent_role_glob("reviewer"),
            ModelEndpoint::new("anthropic", "claude-opus-4-8"),
        )
        .with_session(ModelSessionPolicy::persistent(
            "workspace:reviewer",
            context,
        )),
    ])
    .expect("route rule compiles");

    resolver
        .resolve(
            &ModelRouteRequest::command(["codex", "sub-agent", "review"])
                .with_sub_agent_role("reviewer"),
        )
        .expect("sub-agent route resolves")
}
