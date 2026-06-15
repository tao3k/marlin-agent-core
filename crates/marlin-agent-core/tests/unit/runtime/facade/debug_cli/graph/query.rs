use marlin_agent_core::{GraphLoopExecutionStatus, LoopQuerySummary, run_marlin_cli_from_args};
use std::fs;
use tempfile::tempdir;

use crate::runtime::facade::debug_cli::fixtures::single_node_graph;

#[test]
fn debug_cli_graph_query_reads_loop_run_receipt_facts() {
    let dir = tempdir().expect("tempdir");
    let input = dir.path().join("graph.json");
    let receipt_path = dir.path().join("loop-run-receipt.json");
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
        "--no-store",
    ]);
    assert_eq!(run.status, 0, "{}", run.stderr);
    fs::write(&receipt_path, &run.stdout).expect("write loop run receipt");

    let query = run_marlin_cli_from_args([
        "graph",
        "query",
        "--input",
        receipt_path.to_str().expect("utf8 path"),
    ]);

    assert_eq!(query.status, 0, "{}", query.stderr);
    let summary: LoopQuerySummary =
        serde_json::from_str(&query.stdout).expect("loop query summary");
    assert_eq!(summary.iteration_count, 1);
    assert_eq!(
        summary.terminal_status,
        Some(GraphLoopExecutionStatus::Completed)
    );
    assert!(summary.replayable);
    assert_eq!(summary.missing_trace_count, 0);
    assert_eq!(summary.statuses, vec![GraphLoopExecutionStatus::Completed]);
    assert_eq!(summary.visited_nodes_by_iteration, vec![vec!["plan"]]);
    assert_eq!(summary.node_receipt_count, 1);
    assert!(summary.trace_event_count > 0);
}
