use marlin_agent_core::{
    GraphLoopExecutionStatus, LoopInspectReceipt, LoopRunReceipt, run_marlin_cli_from_args,
    runtime::GraphLoopRunStatus,
};
use std::fs;
use tempfile::tempdir;

use crate::runtime::facade::debug_cli::fixtures::single_node_graph;

#[test]
fn debug_cli_loop_run_writes_store_and_inspect_reads_run_id() {
    let dir = tempdir().expect("tempdir");
    let input = dir.path().join("graph.json");
    let store = dir.path().join("runs");
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
        "1",
        "--store",
        store.to_str().expect("utf8 path"),
    ]);
    assert_eq!(run.status, 0, "{}", run.stderr);
    let run_receipt: LoopRunReceipt = serde_json::from_str(&run.stdout).expect("loop run receipt");
    assert_eq!(run_receipt.iteration_count, 1);
    assert_eq!(
        run_receipt.terminal_status,
        Some(GraphLoopExecutionStatus::Completed)
    );
    let runtime_observation = run_receipt
        .runtime_observation
        .as_ref()
        .expect("loop run receipt should include runtime observation");
    assert_eq!(runtime_observation.run_id.as_str(), "marlin-loop-run");
    assert_eq!(runtime_observation.graph_id.as_str(), "graph");
    assert_eq!(runtime_observation.status, GraphLoopRunStatus::Completed);
    assert_eq!(
        runtime_observation.terminal_status,
        Some(GraphLoopExecutionStatus::Completed)
    );
    assert_eq!(
        runtime_observation
            .current_iteration_id
            .expect("current iteration id")
            .get(),
        0
    );
    let report_path = run_receipt.report_path.expect("stored report path");
    assert!(
        report_path.exists(),
        "stored report should exist at {}",
        report_path.display()
    );

    let inspect = run_marlin_cli_from_args([
        "loop",
        "inspect",
        "marlin-loop-run",
        "--store",
        store.to_str().expect("utf8 path"),
    ]);
    assert_eq!(inspect.status, 0, "{}", inspect.stderr);
    let receipt: LoopInspectReceipt =
        serde_json::from_str(&inspect.stdout).expect("inspect receipt");
    assert_eq!(receipt.report_path, report_path);
    assert_eq!(receipt.iteration_count, 1);
    assert_eq!(
        receipt.terminal_status,
        Some(GraphLoopExecutionStatus::Completed)
    );
    assert!(receipt.replayable);
    assert_eq!(receipt.missing_trace_count, 0);
}

#[test]
fn debug_cli_loop_run_defaults_to_runtime_state_home_receipt_path() {
    let dir = tempdir().expect("tempdir");
    let input = dir.path().join("graph.json");
    let home = dir.path().join("runtime-home");
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
        "1",
        "--home",
        home.to_str().expect("utf8 runtime home"),
    ]);

    assert_eq!(run.status, 0, "{}", run.stderr);
    let run_receipt: LoopRunReceipt = serde_json::from_str(&run.stdout).expect("loop run receipt");
    let report_path = run_receipt.report_path.expect("state-home report path");
    assert_eq!(
        report_path,
        home.join("receipts").join("marlin-loop-run.json")
    );
    assert!(
        report_path.exists(),
        "state-home receipt should exist at {}",
        report_path.display()
    );

    let inspect = run_marlin_cli_from_args([
        "loop",
        "inspect",
        "marlin-loop-run",
        "--home",
        home.to_str().expect("utf8 runtime home"),
    ]);
    assert_eq!(inspect.status, 0, "{}", inspect.stderr);
    let receipt: LoopInspectReceipt =
        serde_json::from_str(&inspect.stdout).expect("inspect receipt");
    assert_eq!(receipt.report_path, report_path);
    assert_eq!(receipt.iteration_count, 1);
    assert_eq!(
        receipt.terminal_status,
        Some(GraphLoopExecutionStatus::Completed)
    );
}
