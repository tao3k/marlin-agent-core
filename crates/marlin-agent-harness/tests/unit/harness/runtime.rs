use std::sync::Arc;

use marlin_agent_harness::{
    AgentHarness, HarnessGraphBuilder, HarnessRuntime, StaticHookRuntime,
    runtime_environment_visibility_evidence,
};
use marlin_agent_kernel::{
    GraphLoopExecutionRequest, GraphLoopExecutionStatus, GraphNodeExecutionReceipt,
    GraphNodeExecutor, GraphNodeInvocation, TokioGraphLoopKernel,
};
use marlin_agent_protocol::{
    AgentScenario, AgentScenarioStep, HookEventName, HookHandlerType, HookRunStatus,
    HookRunSummary, LoopEvidence, LoopEvidenceKind, LoopPerformanceEvidence,
    PERFORMANCE_EVIDENCE_KEYS, RuntimeHome,
};
use marlin_agent_runtime::{
    HookRuntime, RuntimeContext, RuntimeEnvironment, RuntimeEvent, RuntimeFuture, SubAgentRuntime,
    TokioAgentRuntime, observability,
};
use marlin_gerbil_ir::{ReleaseGateSpec, ReleaseTopologySpec, ReleaseVisibilitySpec};
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
    let scenario =
        AgentScenario::new("environment").expecting_evidence(LoopEvidenceKind::Visibility);
    let parent_environment = RuntimeEnvironment::default()
        .with_home(RuntimeHome::custom("/tmp/marlin-home").with_profile("main"))
        .with_cwd("/tmp/workspace");
    let child_environment = RuntimeEnvironment::default()
        .with_home(RuntimeHome::custom("/tmp/marlin-home/sub/reviewer").with_profile("reviewer"))
        .with_cwd("/tmp/workspace/sub");
    let mut harness = HarnessRuntime::with_environment(4, parent_environment.clone());
    harness.record_environment_visibility();

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

    let evidence = harness
        .evidence()
        .iter()
        .find(|evidence| evidence.kind == LoopEvidenceKind::Visibility)
        .expect("expected runtime environment visibility evidence");
    assert_eq!(
        evidence,
        &runtime_environment_visibility_evidence(&parent_environment)
    );
    assert_eq!(evidence.subject, "runtime-environment");
    assert_eq!(
        evidence.detail.as_deref(),
        Some("home=true cwd=true config_layers=0 writable_roots=0 network_access=false")
    );

    let report = AgentHarness::evaluate(&scenario, &[], harness.evidence());
    assert!(report.is_success());
}

#[test]
fn harness_runtime_records_release_visibility_evidence() {
    let scenario =
        AgentScenario::new("release-visibility").expecting_evidence(LoopEvidenceKind::Visibility);
    let topology = ReleaseTopologySpec {
        topology_id: "release:gerbil".to_owned(),
        crate_name: "marlin-gerbil-scheme".to_owned(),
        publish_enabled: false,
        asset_audit_command: "cargo package -p marlin-gerbil-scheme --list".to_owned(),
        package_assets: vec!["README.md".to_owned()],
        runtime_dependency_chain: vec!["marlin-gerbil-ir".to_owned()],
        workflow_dependency_chain: vec!["marlin-org-workflow".to_owned()],
        gates: vec![ReleaseGateSpec {
            gate_id: "real-gxi".to_owned(),
            command: "cargo test -p marlin-gerbil-scheme command::real_gxi".to_owned(),
            requires_local_gerbil: true,
            required_artifacts: vec!["workspace_schema".to_owned()],
            visibility: vec![ReleaseVisibilitySpec {
                report_key: "real_gxi_release_gate".to_owned(),
                evidence_keys: vec!["workspace_schema".to_owned(), "package_asset".to_owned()],
                artifact_paths: vec!["fixtures/gerbil/command-adapter.ss".to_owned()],
            }],
        }],
    };
    let mut harness = HarnessRuntime::new(16);

    harness.record_release_topology_visibility(&topology);

    let evidence = harness
        .evidence()
        .iter()
        .find(|evidence| evidence.kind == LoopEvidenceKind::Visibility)
        .expect("expected release visibility evidence");
    let detail = evidence.detail.as_deref().expect("visibility detail");
    assert_eq!(
        evidence.subject,
        "release-visibility:release:gerbil:real-gxi:real_gxi_release_gate"
    );
    assert!(detail.contains("topology_id=release:gerbil"));
    assert!(detail.contains("crate_name=marlin-gerbil-scheme"));
    assert!(detail.contains("gate_id=real-gxi"));
    assert!(detail.contains("report_key=real_gxi_release_gate"));
    assert!(detail.contains("evidence_keys=[workspace_schema,package_asset]"));
    assert!(detail.contains("artifact_paths=[fixtures/gerbil/command-adapter.ss]"));

    let report = AgentHarness::evaluate(&scenario, &[], harness.evidence());
    assert!(report.is_success());
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
async fn harness_execution_report_carries_performance_benchmark_evidence() {
    let scenario = AgentScenario::new("bench").expecting_evidence(LoopEvidenceKind::Performance);
    let graph = HarnessGraphBuilder::new("graph")
        .node("node-1", "eventful")
        .build();
    let request = GraphLoopExecutionRequest::new("run", graph);
    let kernel =
        TokioGraphLoopKernel::new("run", "graph").with_executor("eventful", EventfulExecutor);
    let mut harness = HarnessRuntime::new(16);
    let performance_evidence: LoopEvidence = LoopPerformanceEvidence {
        subject: "src/runtime.rs".to_owned(),
        benchmark_command: "cargo bench -p marlin-agent-harness".to_owned(),
        baseline: "p95=10ms".to_owned(),
        regression_threshold: "5%".to_owned(),
        latency_or_throughput: "throughput=1000/s".to_owned(),
        allocation_profile: "allocations=steady".to_owned(),
        profile_artifact: "target/criterion/report/index.html".to_owned(),
    }
    .into();

    harness.record_evidence(performance_evidence);

    let report = harness.execute_graph(&scenario, &kernel, request).await;
    let evaluated = AgentHarness::evaluate_execution_report(&scenario, &report);
    let detail = report.evidence[0]
        .detail
        .as_deref()
        .expect("performance detail");

    assert!(report.assertion.is_none());
    assert_eq!(report.evidence[0].kind, LoopEvidenceKind::Performance);
    assert!(evaluated.is_success());
    for key in PERFORMANCE_EVIDENCE_KEYS {
        assert!(
            detail.contains(key),
            "missing performance evidence key {key}"
        );
    }
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

    let execution_span = report
        .find_span(&observability::harness_execution_span_name())
        .expect("expected harness execution trace span");
    assert_eq!(
        execution_span
            .fields
            .get(observability::FIELD_SCENARIO_ID)
            .map(String::as_str),
        Some("failing")
    );
    assert_eq!(
        execution_span
            .fields
            .get(observability::FIELD_RUN_ID)
            .map(String::as_str),
        Some("run-fail")
    );
    assert_eq!(
        execution_span
            .fields
            .get(observability::FIELD_GRAPH_ID)
            .map(String::as_str),
        Some("graph")
    );

    let result_span = report
        .find_span(&observability::harness_result_span_name())
        .expect("expected failed harness result trace span");
    assert_eq!(
        result_span
            .fields
            .get(observability::FIELD_RUN_ID)
            .map(String::as_str),
        Some("run-fail")
    );
    assert_eq!(
        result_span
            .fields
            .get(observability::FIELD_GRAPH_ID)
            .map(String::as_str),
        Some("graph")
    );
    assert!(
        report
            .find_span_with_field(
                &observability::harness_result_span_name(),
                observability::FIELD_RUN_ID,
                "run-fail",
            )
            .is_some()
    );
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

    let trace = report.execution_trace();
    assert_eq!(trace.run_id.as_str(), "run-fail");
    assert_eq!(trace.graph_id.as_str(), "graph");
    assert_eq!(trace.status, GraphLoopExecutionStatus::Failed);
    assert_eq!(trace.events, report.events);
    assert_eq!(trace.spans, report.trace_spans);
    assert_eq!(trace.diagnostics, report.result.diagnostics);

    let trace_summary = trace.summary();
    assert_eq!(trace_summary.status, report.summary.status);
    assert_eq!(trace_summary.event_count, report.summary.event_count);
    assert_eq!(trace_summary.span_count, report.summary.span_count);
    assert_eq!(
        trace_summary.diagnostic_count,
        report.summary.diagnostic_count
    );
}

#[tokio::test]
async fn harness_result_spans_correlate_many_runs_by_run_id() {
    let cases = [
        ("run-ok-0", false),
        ("run-fail-1", true),
        ("run-ok-2", false),
        ("run-fail-3", true),
        ("run-ok-4", false),
        ("run-fail-5", true),
        ("run-ok-6", false),
        ("run-fail-7", true),
    ];
    let mut failed_run_ids = Vec::new();

    for (run_id, should_fail) in cases {
        let scenario = AgentScenario::new(format!("scenario-{run_id}")).with_step(
            AgentScenarioStep::new("run")
                .expecting_span_name(observability::harness_result_span_name()),
        );
        let graph = HarnessGraphBuilder::new("graph")
            .node("node-1", "executor")
            .build();
        let request = GraphLoopExecutionRequest::new(run_id, graph);
        let kernel = if should_fail {
            TokioGraphLoopKernel::new(run_id, "graph").with_executor("executor", FailingExecutor)
        } else {
            TokioGraphLoopKernel::new(run_id, "graph").with_executor("executor", EventfulExecutor)
        };
        let mut harness = HarnessRuntime::new(16);

        let report = harness.execute_graph(&scenario, &kernel, request).await;
        let expected_status = if should_fail {
            GraphLoopExecutionStatus::Failed
        } else {
            GraphLoopExecutionStatus::Completed
        };
        let expected_status_field = if should_fail { "Failed" } else { "Completed" };

        assert_eq!(report.summary.status, expected_status);
        let result_span = report
            .find_span_with_field(
                &observability::harness_result_span_name(),
                observability::FIELD_RUN_ID,
                run_id,
            )
            .expect("expected result span correlated by run_id");
        assert_eq!(
            result_span
                .fields
                .get(observability::FIELD_GRAPH_ID)
                .map(String::as_str),
            Some("graph")
        );
        assert_eq!(
            result_span
                .fields
                .get(observability::FIELD_STATUS)
                .map(String::as_str),
            Some(expected_status_field)
        );

        if should_fail {
            failed_run_ids.push(run_id.to_owned());
        }
    }

    assert_eq!(
        failed_run_ids,
        vec!["run-fail-1", "run-fail-3", "run-fail-5", "run-fail-7"]
    );
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
