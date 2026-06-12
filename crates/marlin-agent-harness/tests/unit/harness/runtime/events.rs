use std::time::Duration;

use marlin_agent_harness::{AgentHarness, HarnessGraphBuilder, HarnessRuntime};
use marlin_agent_kernel::{
    GraphLoopExecutionRequest, GraphLoopExecutionStatus, TokioGraphLoopKernel,
};
use marlin_agent_protocol::{AgentScenario, AgentScenarioStep};

use super::support::{EventfulExecutor, QuietExecutor};

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
async fn harness_execution_report_drain_has_no_idle_timeout_floor() {
    const RUN_COUNT: u32 = 128;
    const PER_RUN_DURATION_BUDGET: Duration = Duration::from_millis(2);

    let scenario = AgentScenario::new("event-drain-no-idle-timeout");
    let mut harness = HarnessRuntime::new(16);
    let mut total_duration = Duration::ZERO;

    for run_index in 0..RUN_COUNT {
        let run_id = format!("run-{run_index}");
        let graph = HarnessGraphBuilder::new("graph")
            .node("node-1", "quiet")
            .build();
        let request = GraphLoopExecutionRequest::new(run_id.clone(), graph);
        let kernel =
            TokioGraphLoopKernel::new(run_id, "graph").with_executor("quiet", QuietExecutor);

        let report = harness.execute_graph(&scenario, &kernel, request).await;

        assert_eq!(report.result.status, GraphLoopExecutionStatus::Completed);
        assert_eq!(report.summary.event_count, report.events.len());
        assert!(
            !report
                .events
                .iter()
                .any(|event| event.topic == "test.harness"),
            "quiet executor should not emit custom harness events"
        );
        total_duration += report.summary.duration;
    }

    let total_duration_budget = PER_RUN_DURATION_BUDGET * RUN_COUNT;
    assert!(
        total_duration <= total_duration_budget,
        "ready event drain should not pay an idle timeout floor: {:?} across {} runs, budget {:?}",
        total_duration,
        RUN_COUNT,
        total_duration_budget
    );
}
