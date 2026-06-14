use marlin_agent_protocol::{
    LoopEvidenceKind, ModelCommandMatcher, ModelContextForkMode, ModelEndpoint, ModelGateway,
    ModelGatewayError, ModelRouteDecision, ModelRouteRequest, ModelRouteRule, ModelSessionPolicy,
    RuntimeEnvironmentActivationPolicy, RuntimeEnvironmentActivationReceipt,
    RuntimeEnvironmentActivationStatus,
};
use marlin_agent_runtime::{
    AgentSessionContext, CancellationToken, CompiledModelRouteResolver, ContextNamespace,
    ContextVisibility, RuntimeEnvironment, SessionKind, TokioAgentRuntime,
};
use marlin_agent_test_support::{
    DeterministicSubAgentScenarioFixture, ScriptedModelGateway,
    assert_deterministic_routed_sub_agent_session, assert_deterministic_sub_agent_gateway_request,
    assert_deterministic_sub_agent_route_decision,
    deterministic_reviewer_sub_agent_scenario_fixture, sub_agent_memory_session_replay_evidence,
    sub_agent_memory_session_visibility_evidence,
};

#[test]
fn model_route_session_binding_reuses_persistent_key_as_child_session_id() {
    let fixture = deterministic_reviewer_sub_agent_scenario_fixture();
    let decision = reviewer_decision_from_fixture(&fixture);
    let parent_session = fixture.session_fixture().parent_session().clone();
    let (runtime, _events) = TokioAgentRuntime::with_session(
        4,
        CancellationToken::new(),
        RuntimeEnvironment::default(),
        parent_session,
    );

    let (child_runtime, binding) =
        runtime.child_runtime_for_model_route(&decision, SessionKind::SubAgent);

    assert_deterministic_routed_sub_agent_session(
        &fixture,
        child_runtime.session(),
        binding.isolation_receipt(),
    );
    assert_eq!(
        binding.child_session_id().as_str(),
        fixture.expected_route_child_session_id(),
    );
    assert_deterministic_sub_agent_route_decision(&fixture, &decision);
}

#[test]
fn model_route_session_binding_projects_memory_visibility_evidence_without_live_llm() {
    let fixture = deterministic_reviewer_sub_agent_scenario_fixture();
    let decision = reviewer_decision_from_fixture(&fixture);
    let parent_session = fixture.session_fixture().parent_session().clone();
    let (runtime, _events) = TokioAgentRuntime::with_session(
        4,
        CancellationToken::new(),
        RuntimeEnvironment::default(),
        parent_session,
    );

    let (child_runtime, binding) =
        runtime.child_runtime_for_model_route(&decision, SessionKind::SubAgent);
    assert_deterministic_routed_sub_agent_session(
        &fixture,
        child_runtime.session(),
        binding.isolation_receipt(),
    );

    let evidence = sub_agent_memory_session_visibility_evidence(
        child_runtime.session(),
        binding.isolation_receipt(),
    );
    let detail = evidence.detail.as_deref().expect("visibility detail");

    assert!(evidence.present);
    assert_eq!(evidence.kind, LoopEvidenceKind::Visibility);
    assert_eq!(
        evidence.subject,
        format!(
            "sub-agent-memory-session:{}",
            fixture.expected_route_child_session_id()
        )
    );
    assert!(detail.contains("memory_visible=true"));
    assert!(detail.contains("denied_memory=false"));
    assert!(detail.contains(&format!(
        "history_limit_applied={}",
        binding.isolation_receipt().history_limit_applied()
    )));
}

#[test]
fn model_route_session_binding_projects_replay_visibility_evidence_without_live_llm() {
    let fixture = deterministic_reviewer_sub_agent_scenario_fixture();
    let decision = reviewer_decision_from_fixture(&fixture);
    let parent_session = fixture.session_fixture().parent_session().clone();
    let (runtime, _events) = TokioAgentRuntime::with_session(
        4,
        CancellationToken::new(),
        RuntimeEnvironment::default(),
        parent_session,
    );

    let (child_runtime, binding) =
        runtime.child_runtime_for_model_route(&decision, SessionKind::SubAgent);
    let evidence = sub_agent_memory_session_replay_evidence(
        child_runtime.session(),
        binding.isolation_receipt(),
    );
    let detail = evidence.detail.as_deref().expect("replay detail");

    assert!(evidence.present);
    assert_eq!(evidence.kind, LoopEvidenceKind::Visibility);
    assert_eq!(
        evidence.subject,
        format!(
            "sub-agent-session-replay:{}",
            fixture.expected_route_child_session_id()
        )
    );
    assert!(detail.contains("parent_session_id=session/root"));
    assert!(detail.contains("root_session_id=session/root"));
    assert!(detail.contains("requested_namespaces=[System,User,Workspace,Memory]"));
    assert!(detail.contains("granted_namespaces=[System,User,Workspace,Memory]"));
    assert!(detail.contains("denied_namespaces=[]"));
    assert!(detail.contains("visibility_contracted=false"));
    assert!(detail.contains("live_llm=false"));
}

#[tokio::test]
async fn model_route_session_binding_uses_scripted_gateway_without_live_llm() {
    let fixture = deterministic_reviewer_sub_agent_scenario_fixture();
    let decision = reviewer_decision_from_fixture(&fixture);
    let parent_session = fixture.session_fixture().parent_session().clone();
    let (runtime, _events) = TokioAgentRuntime::with_session(
        4,
        CancellationToken::new(),
        RuntimeEnvironment::default(),
        parent_session,
    );

    let (child_runtime, binding) =
        runtime.child_runtime_for_model_route(&decision, SessionKind::SubAgent);
    let gateway = ScriptedModelGateway::completion_failure("scripted no-live-llm");
    let request = fixture.model_request("review this patch");

    let result = gateway.complete(request).await;

    assert!(matches!(
        result,
        Err(ModelGatewayError::Completion(message)) if message == "scripted no-live-llm"
    ));
    assert_eq!(
        child_runtime
            .session()
            .parent_session_id()
            .expect("child should keep parent")
            .as_str(),
        "session/root"
    );
    assert_deterministic_routed_sub_agent_session(
        &fixture,
        child_runtime.session(),
        binding.isolation_receipt(),
    );

    let requests = gateway.requests();
    assert_eq!(requests.len(), 1);
    assert_deterministic_sub_agent_gateway_request(&fixture, &requests[0]);
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

    let evidence = sub_agent_memory_session_visibility_evidence(
        child_runtime.session(),
        binding.isolation_receipt(),
    );
    let detail = evidence.detail.as_deref().expect("visibility detail");

    assert!(evidence.present);
    assert_eq!(evidence.kind, LoopEvidenceKind::Visibility);
    assert!(detail.contains("memory_visible=false"));
    assert!(detail.contains("max_history_items=Some(0)"));
    assert!(detail.contains(&format!(
        "history_limit_applied={}",
        binding.isolation_receipt().history_limit_applied()
    )));
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

    let (child_runtime, binding) =
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

    let evidence = sub_agent_memory_session_visibility_evidence(
        child_runtime.session(),
        binding.isolation_receipt(),
    );
    let detail = evidence.detail.as_deref().expect("visibility detail");
    let expected_parent_session_id = child_runtime
        .session()
        .parent_session_id()
        .map(|session_id| session_id.as_str())
        .unwrap_or("none");

    assert_eq!(evidence.subject, "sub-agent-memory-session:session:tester");
    assert!(detail.contains("session_id=session:tester"));
    assert!(detail.contains(&format!("parent_session_id={expected_parent_session_id}",)));
    assert!(detail.contains(&format!(
        "root_session_id={}",
        child_runtime.session().root_session_id().as_str()
    )));
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
            &ModelRouteRequest::command(["gpt-5.5", "sub-agent", "review"])
                .with_sub_agent_role("reviewer"),
        )
        .expect("sub-agent route resolves")
}

fn reviewer_decision_from_fixture(
    fixture: &DeterministicSubAgentScenarioFixture,
) -> ModelRouteDecision {
    let resolver = CompiledModelRouteResolver::new(vec![fixture.route_rule().clone()])
        .expect("fixture route rule compiles");
    let decision = resolver
        .resolve(fixture.route_request())
        .expect("fixture sub-agent route resolves");

    assert_deterministic_sub_agent_route_decision(fixture, &decision);
    decision
}
