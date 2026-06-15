use marlin_agent_kernel::{
    GraphLoopController, GraphLoopExecutionBudget, GraphLoopExecutionRequest,
    GraphLoopExecutionStatus, GraphLoopNextAction, GraphLoopRunRequest, GraphLoopStopPolicy,
    LoopGraph,
};
use marlin_agent_runtime::{GraphLoopRunStatus, TokioAgentRuntime};

use super::{controller, edge, node};

#[tokio::test]
async fn controller_runs_initial_graph_and_reports_terminal_action() {
    let request = GraphLoopRunRequest::new(GraphLoopExecutionRequest::new(
        "run",
        LoopGraph {
            graph_id: "graph".to_owned(),
            nodes: vec![node("plan")],
            edges: Vec::new(),
        },
    ))
    .with_stop_policy(GraphLoopStopPolicy::max_iterations(1));
    let (runtime, _events) = TokioAgentRuntime::new(8);

    let reports = controller()
        .spawn_loop(request, &runtime)
        .join()
        .await
        .expect("controller task should join");

    assert_eq!(reports.len(), 1);
    let report = &reports[0];
    assert_eq!(report.iteration, 0);
    assert_eq!(
        report.execution_result.status,
        GraphLoopExecutionStatus::Completed
    );
    assert_eq!(report.execution_result.visited_nodes, vec!["plan"]);
    assert!(report.execution_result.node_receipts.is_empty());
    assert!(report.trace.is_none());
    assert_eq!(report.next_action, GraphLoopNextAction::StopCompleted);
}

#[tokio::test]
async fn controller_records_loop_run_lifecycle_in_runtime_registry() {
    let request = GraphLoopRunRequest::new(GraphLoopExecutionRequest::new(
        "run",
        LoopGraph {
            graph_id: "graph".to_owned(),
            nodes: vec![node("plan")],
            edges: Vec::new(),
        },
    ))
    .with_stop_policy(GraphLoopStopPolicy::max_iterations(1));
    let (runtime, _events) = TokioAgentRuntime::new(8);

    let reports = controller()
        .spawn_loop(request, &runtime)
        .join()
        .await
        .expect("controller task should join");

    assert_eq!(reports.len(), 1);
    let snapshot = runtime
        .graph_loop_runs()
        .read_registry(|registry| registry.snapshot(1));

    assert_eq!(snapshot.run_count, 1);
    assert_eq!(snapshot.active_count, 0);
    let observation = snapshot.runs.first().expect("run observation");
    assert_eq!(observation.run_id.as_str(), "run");
    assert_eq!(observation.graph_id.as_str(), "graph");
    assert_eq!(observation.status, GraphLoopRunStatus::Completed);
    assert_eq!(
        observation.terminal_status,
        Some(GraphLoopExecutionStatus::Completed)
    );
    assert_eq!(
        observation
            .current_iteration_id
            .expect("current iteration id")
            .get(),
        0
    );
}

#[tokio::test]
async fn controller_honors_zero_iteration_stop_policy_without_execution() {
    let request = GraphLoopRunRequest::new(GraphLoopExecutionRequest::new(
        "run",
        LoopGraph {
            graph_id: "graph".to_owned(),
            nodes: vec![node("plan")],
            edges: Vec::new(),
        },
    ))
    .with_stop_policy(GraphLoopStopPolicy::max_iterations(0));
    let (runtime, _events) = TokioAgentRuntime::new(8);

    let reports = controller()
        .spawn_loop(request, &runtime)
        .join()
        .await
        .expect("controller task should join");

    assert!(reports.is_empty());
}

#[tokio::test]
async fn controller_applies_iteration_budget_to_kernel_execution() {
    let request = GraphLoopRunRequest::new(GraphLoopExecutionRequest::new(
        "run",
        LoopGraph {
            graph_id: "graph".to_owned(),
            nodes: vec![node("plan"), node("apply")],
            edges: vec![edge("plan", "apply")],
        },
    ))
    .with_iteration_budget(GraphLoopExecutionBudget::max_node_executions(1));
    let (runtime, _events) = TokioAgentRuntime::new(8);

    let reports = controller()
        .spawn_loop(request, &runtime)
        .join()
        .await
        .expect("controller task should join");

    assert_eq!(reports.len(), 1);
    let report = &reports[0];
    assert_eq!(
        report.execution_result.status,
        GraphLoopExecutionStatus::Failed
    );
    assert_eq!(report.next_action, GraphLoopNextAction::StopFailed);
    assert_eq!(
        report.execution_result.diagnostics,
        vec!["graph execution budget exceeded: planned node executions 2 > max 1"]
    );
}
