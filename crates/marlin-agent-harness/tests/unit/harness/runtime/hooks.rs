use std::sync::Arc;

use marlin_agent_harness::{
    AgentHarness, HarnessEvidenceKind, HarnessRuntime, HarnessScenario, StaticHookRuntime,
};
use marlin_agent_protocol::{HookRunStatus, HookRunSummary};
use marlin_agent_runtime::TokioAgentRuntime;
use marlin_agent_test_support::{
    assert_custom_sub_agent_start_hook_summary, complex_gerbil_hook_policy_receipt_fixture,
    custom_hook_policy_receipt_fixture, custom_sub_agent_start_hook_summary_fixture,
    hook_dispatch_replay_evidence, sub_agent_hook_dispatch_selection_fixture,
};

#[tokio::test]
async fn static_hook_runtime_returns_configured_summary() {
    let summary = custom_sub_agent_start_hook_summary_fixture();
    let hook = Arc::new(StaticHookRuntime::<(), HookRunSummary>::new(
        summary.clone(),
    ));
    let (runtime, _events) = TokioAgentRuntime::new(4);

    let output = runtime
        .spawn_hook(hook, ())
        .join()
        .await
        .expect("hook task should finish");

    assert_eq!(output.status, HookRunStatus::Completed);
    assert_custom_sub_agent_start_hook_summary(&output);
    assert_eq!(output, summary);
}

#[tokio::test]
async fn static_hook_runtime_replay_receipts_feed_harness_runtime_evidence() {
    let summary = custom_sub_agent_start_hook_summary_fixture();
    let hook = Arc::new(StaticHookRuntime::<(), HookRunSummary>::new(
        summary.clone(),
    ));
    let (runtime, _events) = TokioAgentRuntime::new(4);

    let output = runtime
        .spawn_hook(hook, ())
        .join()
        .await
        .expect("hook task should finish");
    let selection = sub_agent_hook_dispatch_selection_fixture();
    let policy = custom_hook_policy_receipt_fixture();
    let scenario = HarnessScenario::new("hook-dispatch-replay")
        .expecting_evidence(HarnessEvidenceKind::Runtime);
    let mut harness = HarnessRuntime::new(4);
    harness.record_evidence(hook_dispatch_replay_evidence(&output, &selection, &policy));

    let report = AgentHarness::evaluate(&scenario, &[], harness.evidence());
    let detail = harness.evidence()[0]
        .detail
        .as_deref()
        .expect("hook replay detail");

    assert!(report.is_success());
    assert!(detail.contains("hook_id=custom-sub-agent-start"));
    assert!(detail.contains("matcher_strategy=AhoCorasickEventIndex"));
    assert!(detail.contains("selected_count=1"));
    assert!(detail.contains("selection_agent_scope=SubAgent"));
    assert!(detail.contains("policy_mode=EnforceTrusted"));
    assert!(detail.contains("policy_extension_kind=GerbilScheme"));
    assert!(detail.contains("rejected_decisions=1"));
    assert!(detail.contains("live_llm=false"));
}

#[tokio::test]
async fn static_hook_runtime_replay_records_complex_gerbil_policy_action_evidence() {
    let summary = custom_sub_agent_start_hook_summary_fixture();
    let hook = Arc::new(StaticHookRuntime::<(), HookRunSummary>::new(
        summary.clone(),
    ));
    let (runtime, _events) = TokioAgentRuntime::new(4);

    let output = runtime
        .spawn_hook(hook, ())
        .join()
        .await
        .expect("hook task should finish");
    let selection = sub_agent_hook_dispatch_selection_fixture();
    let policy = complex_gerbil_hook_policy_receipt_fixture();
    let scenario = HarnessScenario::new("complex-gerbil-hook-policy-replay")
        .expecting_evidence(HarnessEvidenceKind::Runtime);
    let mut harness = HarnessRuntime::new(4);
    harness.record_evidence(hook_dispatch_replay_evidence(&output, &selection, &policy));

    let report = AgentHarness::evaluate(&scenario, &[], harness.evidence());
    let detail = harness.evidence()[0]
        .detail
        .as_deref()
        .expect("hook replay detail");

    assert!(report.is_success());
    assert!(detail.contains("policy_agent_scope=CustomerAgent"));
    assert!(detail.contains("policy_action_count=4"));
    assert!(detail.contains("policy_action_kinds=Register|Defer|Deny|Rewrite"));
    assert!(detail.contains(
        "policy_action_targets=catalog:customer-agent-hook|session:release|dangerous-shell|command"
    ));
    assert!(detail.contains("policy_action_replacements=none|none|none|cargo test --locked"));
    assert!(detail.contains(
        "policy_action_reasons=customer agent session requires runtime catalog hook|release lineage waits for org memory review|dirty workspace blocks dangerous shell hook|session policy prefers locked tests"
    ));
    assert!(detail.contains("context_session_id=cheap-test-session"));
    assert!(detail.contains("context_agent_lineage=release"));
    assert!(detail.contains("context_workspace_state=dirty"));
    assert!(detail.contains("context_org_memory_hits=needs-human-review"));
    assert!(detail.contains("context_agent_class=customer-agent"));
    assert!(detail.contains("live_llm=false"));
}
