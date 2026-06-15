use marlin_agent_harness::{AgentHarnessGraphBuilder, AgentHarnessRuntime, AgentHarnessScenario};
use marlin_agent_kernel::{
    GraphLoopExecutionRequest, GraphLoopExecutionStatus, TokioGraphLoopKernel,
};
use marlin_agent_protocol::AgentScenarioStep;
use marlin_agent_runtime::observability;

use super::support::{EventfulExecutor, FailingExecutor};

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
        let scenario = AgentHarnessScenario::new(format!("scenario-{run_id}")).with_step(
            AgentScenarioStep::new("run")
                .expecting_span_name(observability::harness_result_span_name()),
        );
        let graph = AgentHarnessGraphBuilder::new("graph")
            .node("node-1", "executor")
            .build();
        let request = GraphLoopExecutionRequest::new(run_id, graph);
        let kernel = if should_fail {
            TokioGraphLoopKernel::new(run_id, "graph").with_executor("executor", FailingExecutor)
        } else {
            TokioGraphLoopKernel::new(run_id, "graph").with_executor("executor", EventfulExecutor)
        };
        let mut harness = AgentHarnessRuntime::new(16);

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
