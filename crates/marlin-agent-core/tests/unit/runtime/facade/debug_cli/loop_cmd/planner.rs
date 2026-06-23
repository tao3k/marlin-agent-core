use marlin_agent_core::{
    GraphLoopExecutionBudget, GraphLoopExecutionRequest, GraphLoopExecutionStatus,
    GraphLoopNextAction, GraphLoopRunRequest, GraphLoopStopPolicy, LoopRunReceipt,
    run_marlin_cli_from_args,
};
use std::fs;
use tempfile::tempdir;

use crate::runtime::facade::debug_cli::fixtures::{single_node_graph, two_step_graph};

#[test]
fn debug_cli_loop_run_repeat_graph_planner_continues_until_iteration_budget() {
    let dir = tempdir().expect("tempdir");
    let input = dir.path().join("graph.json");
    fs::write(
        &input,
        serde_json::to_string(&single_node_graph()).expect("graph JSON"),
    )
    .expect("write graph");

    let run = run_marlin_cli_from_args([
        "loop",
        "run",
        "--input",
        input.to_str().expect("utf8 path"),
        "--max-iterations",
        "3",
        "--continuation-planner",
        "repeat-graph",
        "--no-store",
    ]);

    assert_eq!(run.status, 0, "{}", run.stderr);
    let receipt: LoopRunReceipt = serde_json::from_str(&run.stdout).expect("loop run receipt");
    assert_eq!(receipt.iteration_count, 3);
    assert_eq!(
        receipt.terminal_status,
        Some(GraphLoopExecutionStatus::Completed)
    );
    assert!(matches!(
        receipt.reports[0].next_action,
        GraphLoopNextAction::ContinueWithGraph(_)
    ));
    assert!(matches!(
        receipt.reports[1].next_action,
        GraphLoopNextAction::ContinueWithGraph(_)
    ));
    assert_eq!(
        receipt.reports[2].next_action,
        GraphLoopNextAction::StopCompleted
    );
    assert!(
        receipt.reports[0]
            .continuation_receipt
            .as_ref()
            .expect("continuation receipt")
            .diagnostics
            .contains(&"continuation_planner=repeat-graph".to_owned())
    );
}

#[test]
fn debug_cli_loop_run_retry_on_failure_planner_continues_failed_iterations_until_budget() {
    let dir = tempdir().expect("tempdir");
    let input = dir.path().join("graph.json");
    let request = GraphLoopRunRequest::new(GraphLoopExecutionRequest::new(
        "marlin-loop-run",
        two_step_graph(),
    ))
    .with_iteration_budget(GraphLoopExecutionBudget::max_node_executions(1))
    .with_stop_policy(GraphLoopStopPolicy::max_iterations(3));
    fs::write(
        &input,
        serde_json::to_string(&request).expect("graph loop request JSON"),
    )
    .expect("write graph loop request");

    let run = run_marlin_cli_from_args([
        "loop",
        "run",
        "--input",
        input.to_str().expect("utf8 path"),
        "--continuation-planner",
        "retry-on-failure",
        "--no-store",
    ]);

    assert_eq!(run.status, 0, "{}", run.stderr);
    let receipt: LoopRunReceipt = serde_json::from_str(&run.stdout).expect("loop run receipt");
    assert_eq!(receipt.iteration_count, 3);
    assert_eq!(
        receipt.terminal_status,
        Some(GraphLoopExecutionStatus::Failed)
    );
    assert!(matches!(
        receipt.reports[0].next_action,
        GraphLoopNextAction::ContinueWithGraph(_)
    ));
    assert!(matches!(
        receipt.reports[1].next_action,
        GraphLoopNextAction::ContinueWithGraph(_)
    ));
    assert_eq!(
        receipt.reports[2].next_action,
        GraphLoopNextAction::StopFailed
    );
    let continuation = receipt.reports[0]
        .continuation_receipt
        .as_ref()
        .expect("continuation receipt");
    assert!(
        continuation
            .diagnostics
            .contains(&"continuation_planner=retry-on-failure".to_owned())
    );
    assert!(
        receipt.reports[0]
            .failure_classification_receipt
            .as_ref()
            .expect("failure classification receipt")
            .suggested_recovery_graph
            .is_some()
    );
}
