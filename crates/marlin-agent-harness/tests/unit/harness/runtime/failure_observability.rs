use marlin_agent_harness::{
    AgentHarness, AgentHarnessGraphBuilder, AgentHarnessRuntime, AgentHarnessScenario,
};
use marlin_agent_kernel::{
    GraphLoopExecutionRequest, GraphLoopExecutionStatus, TokioGraphLoopKernel,
};
use marlin_agent_protocol::AgentScenarioStep;
use marlin_agent_runtime::observability;

use super::support::FailingExecutor;

#[tokio::test]
async fn harness_execution_report_captures_failed_result_observability() {
    let scenario = AgentHarnessScenario::new("failing").with_step(
        AgentScenarioStep::new("run")
            .expecting_event_topic(observability::TOPIC_KERNEL_EXECUTION)
            .expecting_span_name(observability::runtime_task_span_name())
            .expecting_span_name(observability::harness_result_span_name()),
    );
    let graph = AgentHarnessGraphBuilder::new("graph")
        .node("node-1", "failing")
        .build();
    let request = GraphLoopExecutionRequest::new("run-fail", graph);
    let kernel =
        TokioGraphLoopKernel::new("run", "graph").with_executor("failing", FailingExecutor);
    let mut harness = AgentHarnessRuntime::new(16);

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
