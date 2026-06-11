use std::sync::Arc;

use marlin_agent_harness::{AgentHarness, StaticHookRuntime};
use marlin_agent_protocol::{
    AgentEvent, AgentScenario, AgentScenarioStep, HookEventName, HookHandlerType, HookRunStatus,
    HookRunSummary, LoopEvidence, LoopEvidenceKind,
};
use marlin_agent_runtime::TokioAgentRuntime;

#[test]
fn harness_accepts_present_evidence_and_event_topics() {
    let scenario = AgentScenario::new("loop")
        .with_step(AgentScenarioStep::new("run").expecting_event_topic("kernel.execution"))
        .expecting_evidence(LoopEvidenceKind::Runtime);
    let events = vec![AgentEvent::new("kernel.execution", "run started")];
    let evidence = vec![LoopEvidence::present(LoopEvidenceKind::Runtime, "tokio")];

    let report = AgentHarness::evaluate(&scenario, &events, &evidence);

    assert!(report.is_success());
    assert_eq!(report.scenario_id, "loop");
}

#[test]
fn harness_reports_missing_evidence_and_event_topics() {
    let scenario = AgentScenario::new("loop")
        .with_step(AgentScenarioStep::new("run").expecting_event_topic("kernel.execution"))
        .expecting_evidence(LoopEvidenceKind::Runtime);

    let report = AgentHarness::evaluate(&scenario, &[], &[]);

    assert_eq!(
        report.diagnostics,
        vec![
            "missing expected evidence `Runtime`",
            "missing expected event topic `kernel.execution` for step run",
        ]
    );
}

#[tokio::test]
async fn static_hook_runtime_returns_configured_summary() {
    let summary = HookRunSummary::running(
        "hook-1",
        HookEventName::PreToolUse,
        HookHandlerType::Command,
    )
    .completed();
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
    assert_eq!(output, summary);
}
