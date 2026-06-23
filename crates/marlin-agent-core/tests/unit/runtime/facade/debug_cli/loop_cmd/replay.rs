use marlin_agent_core::{
    GraphLoopExecutionStatus, LoopReplayReceipt, RunId, run_marlin_cli_from_args,
};
use std::fs;
use tempfile::tempdir;

use crate::runtime::facade::debug_cli::fixtures::single_node_graph;

#[test]
fn debug_cli_loop_replay_accepts_loop_run_receipt_file() {
    let dir = tempdir().expect("tempdir");
    let input = dir.path().join("graph.json");
    let receipt_path = dir.path().join("run-receipt.json");
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
    fs::write(&receipt_path, &run.stdout).expect("write run receipt");

    let replay =
        run_marlin_cli_from_args(["loop", "replay", receipt_path.to_str().expect("utf8 path")]);
    assert_eq!(replay.status, 0, "{}", replay.stderr);
    let receipt: LoopReplayReceipt = serde_json::from_str(&replay.stdout).expect("replay receipt");
    assert_eq!(receipt.source, receipt_path);
    assert_eq!(receipt.iteration_count, 1);
    assert!(receipt.replayable);
    assert_eq!(receipt.missing_trace_count, 0);
    assert_eq!(receipt.statuses, vec![GraphLoopExecutionStatus::Completed]);
    assert_eq!(receipt.run_ids, vec![RunId::new("marlin-loop-run")]);
}
