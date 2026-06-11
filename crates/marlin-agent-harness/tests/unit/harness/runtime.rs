use std::sync::Arc;

use marlin_agent_harness::{AgentHarness, HarnessGraphBuilder, HarnessRuntime, StaticHookRuntime};
use marlin_agent_kernel::{
    GraphLoopExecutionRequest, GraphLoopExecutionStatus, GraphNodeExecutionReceipt,
    GraphNodeExecutor, GraphNodeInvocation, TokioGraphLoopKernel,
};
use marlin_agent_protocol::{
    AgentScenario, AgentScenarioStep, HookEventName, HookHandlerType, HookRunStatus,
    HookRunSummary, RuntimeHome,
};
use marlin_agent_runtime::{
    HookRuntime, RuntimeContext, RuntimeEnvironment, RuntimeEvent, RuntimeFuture, SubAgentRuntime,
    TokioAgentRuntime, observability,
};

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

#[tokio::test]
async fn harness_execution_report_captures_runtime_events() {
    let scenario = AgentScenario::new("eventful")
        .with_step(AgentScenarioStep::new("run").expecting_event_topic("test.harness"));
    let graph = HarnessGraphBuilder::new("graph")
        .node("node-1", "eventful")
        .build();
    let request = GraphLoopExecutionRequest::new("run", graph);
    let kernel =
        TokioGraphLoopKernel::new("run", "graph").with_executor("eventful", EventfulExecutor);
    let mut harness = HarnessRuntime::new(16);

    let report = harness.execute_graph(&scenario, &kernel, request).await;
    let evaluated = AgentHarness::evaluate_execution_report(&scenario, &report);

    assert_eq!(report.result.status, GraphLoopExecutionStatus::Completed);
    assert!(report.assertion.is_none());
    assert!(
        report
            .events
            .iter()
            .any(|event| event.topic == "test.harness" && event.message == "node node-1 observed")
    );
    assert!(evaluated.is_success());
}

#[tokio::test]
async fn harness_execution_report_captures_failed_result_observability() {
    let scenario = AgentScenario::new("failing").with_step(
        AgentScenarioStep::new("run")
            .expecting_event_topic(observability::TOPIC_KERNEL_EXECUTION)
            .expecting_span_name(observability::runtime_task_span_name())
            .expecting_span_name(observability::harness_result_span_name()),
    );
    let graph = HarnessGraphBuilder::new("graph")
        .node("node-1", "failing")
        .build();
    let request = GraphLoopExecutionRequest::new("run-fail", graph);
    let kernel =
        TokioGraphLoopKernel::new("run", "graph").with_executor("failing", FailingExecutor);
    let mut harness = HarnessRuntime::new(16);

    let report = harness.execute_graph(&scenario, &kernel, request).await;
    let evaluated = AgentHarness::evaluate_execution_report(&scenario, &report);

    assert_eq!(report.result.status, GraphLoopExecutionStatus::Failed);
    assert_eq!(report.summary.status, GraphLoopExecutionStatus::Failed);
    assert_eq!(report.summary.event_count, report.events.len());
    assert_eq!(report.summary.span_count, report.trace_spans.len());
    assert_eq!(report.summary.diagnostic_count, 1);
    assert_eq!(
        report.result.diagnostics,
        vec!["node node-1 failed intentionally"]
    );
    assert!(report.assertion.is_none());
    assert!(evaluated.is_success());
    assert!(
        report
            .events
            .iter()
            .any(|event| event.topic == observability::TOPIC_KERNEL_EXECUTION
                && event.message == "run run-fail failed graph graph")
    );

    let result_span = report
        .find_span(&observability::harness_result_span_name())
        .expect("expected failed harness result trace span");
    assert_eq!(
        result_span
            .fields
            .get(observability::FIELD_STATUS)
            .map(String::as_str),
        Some("Failed")
    );
    assert_eq!(
        result_span
            .fields
            .get(observability::FIELD_DIAGNOSTIC_COUNT)
            .map(String::as_str),
        Some("1")
    );
    let event_count = report.summary.event_count.to_string();
    assert_eq!(
        result_span.fields.get(observability::FIELD_EVENT_COUNT),
        Some(&event_count)
    );
    result_span
        .fields
        .get(observability::FIELD_DURATION_MS)
        .expect("expected duration_ms field")
        .parse::<u64>()
        .expect("duration_ms field should be numeric");
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

#[derive(Clone, Debug)]
struct EventfulExecutor;

impl GraphNodeExecutor for EventfulExecutor {
    fn execute_node(
        &self,
        invocation: GraphNodeInvocation,
        context: RuntimeContext,
    ) -> RuntimeFuture<GraphNodeExecutionReceipt> {
        Box::pin(async move {
            let node_id = invocation.node_id;
            let executor = invocation.executor;
            context
                .emit(RuntimeEvent::new(
                    "test.harness",
                    format!("node {} observed", node_id.as_str()),
                ))
                .await
                .expect("harness event should be emitted");
            GraphNodeExecutionReceipt::completed(node_id, executor)
        })
    }
}

#[derive(Clone, Debug)]
struct FailingExecutor;

impl GraphNodeExecutor for FailingExecutor {
    fn execute_node(
        &self,
        invocation: GraphNodeInvocation,
        _context: RuntimeContext,
    ) -> RuntimeFuture<GraphNodeExecutionReceipt> {
        Box::pin(async move {
            GraphNodeExecutionReceipt::failed(
                invocation.node_id,
                invocation.executor,
                vec!["node node-1 failed intentionally".to_owned()],
            )
        })
    }
}
