use std::sync::Arc;

use marlin_agent_harness::{AgentHarness, HarnessRuntime, StaticHookRuntime};
use marlin_agent_protocol::{AgentScenario, HookRunStatus, HookRunSummary, LoopEvidenceKind};
use marlin_agent_runtime::TokioAgentRuntime;
use marlin_agent_test_support::{
    assert_custom_sub_agent_start_hook_summary, custom_hook_policy_receipt_fixture,
    custom_sub_agent_start_hook_summary_fixture, hook_dispatch_replay_evidence,
    sub_agent_hook_dispatch_selection_fixture,
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
    let scenario =
        AgentScenario::new("hook-dispatch-replay").expecting_evidence(LoopEvidenceKind::Runtime);
    let mut harness = HarnessRuntime::new(4);
    harness.record_evidence(hook_dispatch_replay_evidence(&output, &selection, &policy));

    let report = AgentHarness::evaluate(&scenario, &[], harness.evidence());
    let detail = harness.evidence()[0]
        .detail
        .as_deref()
        .expect("hook replay detail");

    assert!(report.is_success());
    assert!(detail.contains("hook_id=custom-sub-agent-start"));
    assert!(detail.contains("selected_count=1"));
    assert!(detail.contains("rejected_decisions=1"));
}
