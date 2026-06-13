use marlin_agent_sessions::SessionKind;
use marlin_agent_test_support::{
    accepted_gerbil_ir_graph_policy_proposal_fixture, accepted_graph_policy_proposal_fixture,
    assert_accepted_gerbil_ir_graph_policy_proposal_fixture,
    assert_accepted_graph_policy_proposal_fixture, assert_budgeted_graph_policy_execution_request,
    assert_deterministic_sub_agent_scenario_fixture, assert_rejected_graph_policy_proposal_fixture,
    assert_sub_agent_memory_session_fixture, budgeted_graph_policy_execution_request_fixture,
    deterministic_reviewer_sub_agent_scenario_fixture, rejected_graph_policy_proposal_fixture,
};

#[test]
fn graph_policy_fixture_accepts_native_scheme_proposal_without_live_llm() {
    let fixture = accepted_graph_policy_proposal_fixture();

    assert_accepted_graph_policy_proposal_fixture(&fixture);
    assert_eq!(
        fixture.expected_run_id(),
        "test-support/graph-policy/accepted"
    );
    assert_eq!(
        fixture.proposal().strategy.strategy_id.as_str(),
        "test-support-scheme-loop-ranker"
    );
}

#[test]
fn graph_policy_fixture_accepts_gerbil_ir_proposal_without_live_llm() {
    let fixture = accepted_gerbil_ir_graph_policy_proposal_fixture();

    assert_accepted_gerbil_ir_graph_policy_proposal_fixture(&fixture);
    assert_eq!(
        fixture.expected_run_id(),
        "test-support/graph-policy/gerbil-ir"
    );
    assert_eq!(
        fixture.proposal().strategy.strategy_id.as_str(),
        "test-support-gerbil-ir-loop-ranker"
    );
}

#[test]
fn graph_policy_fixture_rejects_invalid_native_gerbil_proposal_without_live_llm() {
    let fixture = rejected_graph_policy_proposal_fixture();

    assert_rejected_graph_policy_proposal_fixture(&fixture);
    assert_eq!(
        fixture.expected_run_id(),
        "test-support/graph-policy/rejected"
    );
    assert_eq!(
        fixture.proposal().strategy.strategy_id.as_str(),
        "test-support-gerbil-loop-ranker"
    );
}

#[test]
fn graph_policy_fixture_records_budget_without_live_llm() {
    let graph_policy = accepted_gerbil_ir_graph_policy_proposal_fixture();
    let request = budgeted_graph_policy_execution_request_fixture(&graph_policy, 2);

    assert_accepted_gerbil_ir_graph_policy_proposal_fixture(&graph_policy);
    assert_budgeted_graph_policy_execution_request(&request, 2);
    assert_eq!(request.graph.graph_id, "test-support-gerbil-ir-graph");
}

#[test]
fn graph_policy_fixture_combines_with_sub_agent_session_memory_and_hooks_without_live_llm() {
    let graph_policy = accepted_graph_policy_proposal_fixture();
    let gerbil_graph_policy = accepted_gerbil_ir_graph_policy_proposal_fixture();
    let sub_agent = deterministic_reviewer_sub_agent_scenario_fixture();
    let session = sub_agent.session_fixture();
    let (child_session, isolation_receipt) = session.parent_session().child_session(
        SessionKind::SubAgent,
        session.config().child_session_id(),
        session.requested_visibility(),
    );
    let graph_evidence = graph_policy.visibility_evidence();

    assert_accepted_graph_policy_proposal_fixture(&graph_policy);
    assert_accepted_gerbil_ir_graph_policy_proposal_fixture(&gerbil_graph_policy);
    assert_deterministic_sub_agent_scenario_fixture(&sub_agent);
    assert_sub_agent_memory_session_fixture(
        session,
        &child_session,
        session.config(),
        &isolation_receipt,
    );
    assert_eq!(graph_policy.compilation().receipt.diagnostics.len(), 0);
    assert!(graph_evidence.present);
    assert!(
        graph_evidence
            .detail
            .as_deref()
            .is_some_and(|detail| detail.contains("status=Accepted"))
    );
}
