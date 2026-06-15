use marlin_agent_harness_types::HarnessEvidenceKind;
use marlin_agent_protocol::SubAgentConfigSurface;
use marlin_agent_test_support::{
    DeterministicRoutedSubAgentExecutionReceipt,
    assert_deterministic_reviewer_applied_environment_activation_receipt,
    assert_deterministic_routed_sub_agent_execution,
    assert_deterministic_sub_agent_scenario_fixture,
    deterministic_reviewer_applied_environment_activation_receipt_fixture,
    deterministic_reviewer_routed_receipt_family_evidence,
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
    assert_eq!(
        fixture.session_fixture().config().surface,
        SubAgentConfigSurface::Toml,
    );
    assert_eq!(
        fixture
            .session_fixture()
            .config()
            .policy
            .context
            .max_history_items,
        Some(32),
    );
    assert!(
        fixture
            .session_fixture()
            .config()
            .environment_activation
            .is_some()
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

#[test]
fn deterministic_sub_agent_receipt_family_evidence_records_org_metadata_and_environment_delta() {
    let activation = deterministic_reviewer_applied_environment_activation_receipt_fixture();
    assert_deterministic_reviewer_applied_environment_activation_receipt(&activation);

    let evidence = deterministic_reviewer_routed_receipt_family_evidence();
    let detail = evidence.detail.as_deref().expect("receipt family detail");

    assert_eq!(evidence.kind, HarnessEvidenceKind::Runtime);
    assert_eq!(evidence.subject, "routed-sub-agent-receipt-family:reviewer");
    assert!(detail.contains("route_rule_id=reviewer-opus"));
    assert!(detail.contains("session_child_id=model-route/persistent/workspace:reviewer"));
    assert!(detail.contains("provider_model_id=anthropic/claude-opus-4-8"));
    assert!(detail.contains("environment_status=Applied"));
    assert!(detail.contains("environment_delta_added=[REVIEWER_ENV]"));
    assert!(detail.contains("environment_delta_changed=[PATH]"));
    assert!(detail.contains("environment_delta_removed=[REMOVE_ME]"));
    assert!(detail.contains("metadata_format=org"));
    assert!(detail.contains("live_llm=false"));
}
