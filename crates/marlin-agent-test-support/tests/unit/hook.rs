use marlin_agent_protocol::LoopEvidenceKind;
use marlin_agent_test_support::{
    assert_custom_hook_policy_receipt, assert_custom_sub_agent_start_hook_summary,
    assert_sub_agent_hook_dispatch_selection, custom_hook_policy_receipt_fixture,
    custom_sub_agent_start_hook_summary_fixture, hook_dispatch_replay_evidence,
    sub_agent_hook_dispatch_selection_fixture,
};

#[test]
fn custom_sub_agent_start_hook_fixture_records_runtime_scope() {
    let summary = custom_sub_agent_start_hook_summary_fixture();

    assert_custom_sub_agent_start_hook_summary(&summary);
}

#[test]
fn sub_agent_hook_selection_fixture_records_candidates_and_fast_match() {
    let receipt = sub_agent_hook_dispatch_selection_fixture();

    assert_sub_agent_hook_dispatch_selection(&receipt);
}

#[test]
fn custom_hook_policy_fixture_records_enforced_extension_decisions() {
    let receipt = custom_hook_policy_receipt_fixture();

    assert_custom_hook_policy_receipt(&receipt);
}

#[test]
fn hook_dispatch_replay_fixture_projects_runtime_evidence() {
    let summary = custom_sub_agent_start_hook_summary_fixture();
    let selection = sub_agent_hook_dispatch_selection_fixture();
    let policy = custom_hook_policy_receipt_fixture();

    let evidence = hook_dispatch_replay_evidence(&summary, &selection, &policy);
    let detail = evidence.detail.as_deref().expect("hook replay detail");

    assert!(evidence.present);
    assert_eq!(evidence.kind, LoopEvidenceKind::Runtime);
    assert_eq!(
        evidence.subject,
        "hook-dispatch-replay:custom-sub-agent-start",
    );
    assert!(detail.contains("selected_count=1"));
    assert!(detail.contains("candidate_count=2"));
    assert!(detail.contains("policy_decisions=2"));
    assert!(detail.contains("rejected_decisions=1"));
}
