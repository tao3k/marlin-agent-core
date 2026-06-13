use marlin_agent_harness::{AgentHarness, HarnessRuntime};
use marlin_agent_protocol::{AgentScenario, LoopEvidenceKind};
use marlin_agent_runtime::{
    CancellationToken, CompiledModelRouteResolver, RuntimeEnvironment, TokioAgentRuntime,
};
use marlin_agent_sessions::SessionKind;
use marlin_agent_test_support::{
    SubAgentMemoryExpectation, assert_deterministic_routed_sub_agent_session,
    assert_deterministic_sub_agent_route_decision, assert_sub_agent_memory_session_fixture,
    deterministic_reviewer_sub_agent_scenario_fixture, sub_agent_memory_allowed_fixture,
    sub_agent_memory_denied_fixture, sub_agent_memory_session_visibility_evidence,
};

#[test]
fn harness_consumes_sub_agent_memory_session_visibility_without_live_llm() {
    for fixture in [
        sub_agent_memory_allowed_fixture(),
        sub_agent_memory_denied_fixture(),
    ] {
        let (child_session, isolation_receipt) = fixture.parent_session().child_session(
            SessionKind::SubAgent,
            fixture.config().child_session_id(),
            fixture.requested_visibility(),
        );
        assert_sub_agent_memory_session_fixture(
            &fixture,
            &child_session,
            fixture.config(),
            &isolation_receipt,
        );

        let scenario = AgentScenario::new("sub-agent-memory-session")
            .expecting_evidence(LoopEvidenceKind::Visibility);
        let mut harness = HarnessRuntime::new(8);
        harness.record_evidence(sub_agent_memory_session_visibility_evidence(
            &child_session,
            &isolation_receipt,
        ));

        let report = AgentHarness::evaluate(&scenario, &[], harness.evidence());
        let detail = harness.evidence()[0]
            .detail
            .as_deref()
            .expect("sub-agent memory visibility detail");

        assert!(report.is_success());
        match fixture.expectation() {
            SubAgentMemoryExpectation::Granted => {
                assert!(detail.contains("memory_visible=true"));
                assert!(detail.contains("denied_memory=false"));
                assert!(detail.contains("denied_namespace_count=0"));
                assert!(detail.contains("max_history_items=Some(16)"));
                assert!(detail.contains("history_limit_applied=true"));
            }
            SubAgentMemoryExpectation::Denied => {
                assert!(detail.contains("memory_visible=false"));
                assert!(detail.contains("denied_memory=true"));
                assert!(detail.contains("denied_namespace_count=1"));
                assert!(detail.contains("max_history_items=Some(32)"));
                assert!(detail.contains("history_limit_applied=false"));
            }
        }
    }
}

#[test]
fn harness_consumes_model_route_sub_agent_memory_visibility_without_live_llm() {
    let fixture = deterministic_reviewer_sub_agent_scenario_fixture();
    let resolver = CompiledModelRouteResolver::new(vec![fixture.route_rule().clone()])
        .expect("fixture route rule compiles");
    let decision = resolver
        .resolve(fixture.route_request())
        .expect("fixture route request resolves");
    assert_deterministic_sub_agent_route_decision(&fixture, &decision);
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

    let scenario = AgentScenario::new("model-route-sub-agent-memory-session")
        .expecting_evidence(LoopEvidenceKind::Visibility);
    let mut harness = HarnessRuntime::new(8);
    harness.record_evidence(sub_agent_memory_session_visibility_evidence(
        child_runtime.session(),
        binding.isolation_receipt(),
    ));

    let report = AgentHarness::evaluate(&scenario, &[], harness.evidence());
    let detail = harness.evidence()[0]
        .detail
        .as_deref()
        .expect("sub-agent memory visibility detail");

    assert!(report.is_success());
    assert!(detail.contains(fixture.expected_route_child_session_id()));
    assert!(detail.contains("root_session_id=session/root"));
    assert!(detail.contains("memory_visible=true"));
    assert!(detail.contains("denied_memory=false"));
    assert!(detail.contains("max_history_items=Some(16)"));
}
