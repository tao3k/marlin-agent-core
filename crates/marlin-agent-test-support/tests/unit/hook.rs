use marlin_agent_harness_types::AgentHarnessEvidenceKind;
use marlin_agent_test_support::{
    assert_complex_gerbil_hook_policy_receipt, assert_custom_hook_policy_receipt,
    assert_custom_sub_agent_start_hook_summary, assert_sub_agent_hook_dispatch_selection,
    complex_gerbil_hook_policy_receipt_fixture, custom_hook_policy_receipt_fixture,
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
fn complex_gerbil_hook_policy_fixture_records_dynamic_actions() {
    let receipt = complex_gerbil_hook_policy_receipt_fixture();

    assert_complex_gerbil_hook_policy_receipt(&receipt);
}

#[test]
fn hook_dispatch_replay_fixture_projects_runtime_evidence() {
    let summary = custom_sub_agent_start_hook_summary_fixture();
    let selection = sub_agent_hook_dispatch_selection_fixture();
    let policy = custom_hook_policy_receipt_fixture();

    let evidence = hook_dispatch_replay_evidence(&summary, &selection, &policy);
    let detail = evidence.detail.as_deref().expect("hook replay detail");

    assert!(evidence.present);
    assert_eq!(evidence.kind, AgentHarnessEvidenceKind::Runtime);
    assert_eq!(
        evidence.subject,
        "hook-dispatch-replay:custom-sub-agent-start",
    );
    assert!(detail.contains("selected_count=1"));
    assert!(detail.contains("candidate_count=2"));
    assert!(detail.contains("matcher_strategy=AhoCorasickEventIndex"));
    assert!(detail.contains("matched_token_count=1"));
    assert!(detail.contains("policy_decisions=2"));
    assert!(detail.contains("policy_mode=EnforceTrusted"));
    assert!(detail.contains("policy_extension_kind=GerbilScheme"));
    assert!(detail.contains("allowed_decisions=1"));
    assert!(detail.contains("rejected_decisions=1"));
    assert!(detail.contains("policy_action_count=0"));
    assert!(detail.contains("summary_agent_scope=SubAgent"));
    assert!(detail.contains("selection_agent_scope=SubAgent"));
    assert!(detail.contains("policy_agent_scope=CustomAgent"));
    assert!(detail.contains("live_llm=false"));
}

#[test]
fn complex_gerbil_hook_policy_replay_projects_dynamic_action_evidence() {
    let summary = custom_sub_agent_start_hook_summary_fixture();
    let selection = sub_agent_hook_dispatch_selection_fixture();
    let policy = complex_gerbil_hook_policy_receipt_fixture();

    let evidence = hook_dispatch_replay_evidence(&summary, &selection, &policy);
    let detail = evidence.detail.as_deref().expect("hook replay detail");

    assert!(evidence.present);
    assert_eq!(evidence.kind, AgentHarnessEvidenceKind::Runtime);
    assert!(detail.contains("policy_decisions=1"));
    assert!(detail.contains("policy_mode=ObserveOnly"));
    assert!(detail.contains("policy_agent_scope=CustomerAgent"));
    assert!(detail.contains("policy_action_count=4"));
    assert!(detail.contains("policy_action_kinds=Register|Defer|Deny|Rewrite"));
    assert!(detail.contains("live_llm=false"));
}
