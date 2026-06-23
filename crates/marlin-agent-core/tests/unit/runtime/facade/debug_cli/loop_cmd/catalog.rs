use marlin_agent_core::{GraphLoopExecutionStatus, LoopRunReceipt, run_marlin_cli_from_args};
use std::fs;
use tempfile::tempdir;

use crate::runtime::facade::debug_cli::fixtures::{
    catalog_toml, process_command_catalog_toml, single_node_graph_with_executor,
};

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
