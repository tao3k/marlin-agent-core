use std::sync::Arc;

use marlin_agent_harness::{AgentHarness, HarnessRuntime, StaticHookRuntime};
use marlin_agent_protocol::{
    AgentEvent, AgentScenario, AgentScenarioStep, HookEventName, HookHandlerType, HookRunStatus,
    HookRunSummary, LoopEvidence, LoopEvidenceKind, RuntimeHome,
};
use marlin_agent_runtime::{
    HookRuntime, RuntimeContext, RuntimeEnvironment, RuntimeFuture, SubAgentRuntime,
    TokioAgentRuntime,
};

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

#[tokio::test]
async fn harness_runtime_preserves_custom_environment_for_hooks_and_sub_agents() {
    let parent_environment = RuntimeEnvironment::default()
        .with_home(RuntimeHome::custom("/tmp/marlin-home").with_profile("main"))
        .with_cwd("/tmp/workspace");
    let child_environment = RuntimeEnvironment::default()
        .with_home(RuntimeHome::custom("/tmp/marlin-home/sub/reviewer").with_profile("reviewer"))
        .with_cwd("/tmp/workspace/sub");
    let harness = HarnessRuntime::with_environment(4, parent_environment.clone());

    let hook_environment = harness
        .runtime()
        .spawn_hook(Arc::new(EnvironmentEchoHook), "pre-tool".to_owned())
        .join()
        .await
        .expect("hook task should finish");
    let sub_agent_environment = harness
        .runtime()
        .spawn_sub_agent_with_environment(
            Arc::new(EnvironmentEchoSubAgent),
            (),
            child_environment.clone(),
        )
        .join()
        .await
        .expect("sub-agent task should finish");

    assert_eq!(harness.environment(), &parent_environment);
    assert_eq!(hook_environment, parent_environment);
    assert_eq!(sub_agent_environment, child_environment);
}

#[derive(Clone, Debug)]
struct EnvironmentEchoHook;

impl HookRuntime for EnvironmentEchoHook {
    type Request = String;
    type Output = RuntimeEnvironment;

    fn run_hook(
        &self,
        _request: Self::Request,
        context: RuntimeContext,
    ) -> RuntimeFuture<Self::Output> {
        let environment = context.environment().clone();
        Box::pin(async move { environment })
    }
}

#[derive(Clone, Debug)]
struct EnvironmentEchoSubAgent;

impl SubAgentRuntime for EnvironmentEchoSubAgent {
    type Input = ();
    type Output = RuntimeEnvironment;

    fn run_sub_agent(
        &self,
        _input: Self::Input,
        context: RuntimeContext,
    ) -> RuntimeFuture<Self::Output> {
        let environment = context.environment().clone();
        Box::pin(async move { environment })
    }
}
