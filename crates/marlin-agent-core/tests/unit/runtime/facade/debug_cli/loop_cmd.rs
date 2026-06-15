use marlin_agent_core::{
    GraphLoopExecutionStatus, LoopInspectReceipt, LoopReplayReceipt, LoopRunReceipt, RunId,
    run_marlin_cli_from_args,
};
use std::fs;
use tempfile::tempdir;

use super::fixtures::{
    catalog_toml, process_command_catalog_toml, single_node_graph, single_node_graph_with_executor,
};

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
fn debug_cli_loop_run_accepts_catalog_config() {
    let dir = tempdir().expect("tempdir");
    let input = dir.path().join("graph.json");
    let catalog = dir.path().join("catalog.toml");
    fs::write(
        &input,
        serde_json::to_string(&single_node_graph_with_executor("custom.subagent.echo"))
            .expect("graph JSON"),
    )
    .expect("write graph");
    fs::write(&catalog, catalog_toml("custom.subagent.echo", "sub-agent")).expect("write catalog");

    let run = run_marlin_cli_from_args([
        "loop",
        "run",
        "--input",
        input.to_str().expect("utf8 path"),
        "--max-iterations",
        "1",
        "--no-store",
        "--catalog",
        catalog.to_str().expect("utf8 path"),
    ]);

    assert_eq!(run.status, 0, "{}", run.stderr);
    let receipt: LoopRunReceipt = serde_json::from_str(&run.stdout).expect("loop run receipt");
    assert_eq!(
        receipt.terminal_status,
        Some(GraphLoopExecutionStatus::Completed)
    );
    assert_eq!(
        receipt.reports[0].execution_result.visited_nodes,
        vec!["plan"]
    );
}

#[test]
fn debug_cli_loop_run_accepts_process_command_runtime_binding() {
    let dir = tempdir().expect("tempdir");
    let input = dir.path().join("graph.json");
    let catalog = dir.path().join("catalog.toml");
    fs::write(
        &input,
        serde_json::to_string(&single_node_graph_with_executor("custom.process.echo"))
            .expect("graph JSON"),
    )
    .expect("write graph");
    fs::write(
        &catalog,
        process_command_catalog_toml("custom.process.echo", "/bin/echo", &["marlin-loop"]),
    )
    .expect("write catalog");

    let run = run_marlin_cli_from_args([
        "loop",
        "run",
        "--input",
        input.to_str().expect("utf8 path"),
        "--max-iterations",
        "1",
        "--no-store",
        "--catalog",
        catalog.to_str().expect("utf8 path"),
    ]);

    assert_eq!(run.status, 0, "{}", run.stderr);
    let receipt: LoopRunReceipt = serde_json::from_str(&run.stdout).expect("loop run receipt");
    assert_eq!(
        receipt.terminal_status,
        Some(GraphLoopExecutionStatus::Completed)
    );
    let diagnostics = &receipt.reports[0].execution_result.node_receipts[0].diagnostics;
    assert!(diagnostics.contains(&"process-command.exit_status:0".to_owned()));
    assert!(diagnostics.contains(&"process-command.stdout:marlin-loop".to_owned()));
    assert!(diagnostics.contains(&"process-command.node:plan".to_owned()));
}

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
