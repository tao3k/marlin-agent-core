use marlin_agent_test_support::{
    DeterministicRoutedSubAgentExecutionReceipt, assert_deterministic_routed_sub_agent_execution,
    assert_deterministic_sub_agent_scenario_fixture,
    deterministic_reviewer_sub_agent_scenario_fixture,
};

#[test]
fn deterministic_sub_agent_scenario_fixture_combines_route_session_and_hooks() {
    let fixture = deterministic_reviewer_sub_agent_scenario_fixture();

    assert_deterministic_sub_agent_scenario_fixture(&fixture);
    assert_eq!(
        fixture.expected_route_child_session_id(),
        "model-route/persistent/workspace:reviewer",
    );
    assert_eq!(
        fixture.expected_litellm_model_id(),
        "anthropic/claude-opus-4-8"
    );
}

#[test]
fn deterministic_sub_agent_execution_receipt_records_routed_session_visibility() {
    let fixture = deterministic_reviewer_sub_agent_scenario_fixture();
    let receipt = DeterministicRoutedSubAgentExecutionReceipt {
        session_id: fixture.expected_route_child_session_id().to_owned(),
        parent_session_id: Some("session/root".to_owned()),
        system_visible: true,
        workspace_visible: true,
        memory_visible: true,
    };

    assert_deterministic_routed_sub_agent_execution(&fixture, &receipt);
}
