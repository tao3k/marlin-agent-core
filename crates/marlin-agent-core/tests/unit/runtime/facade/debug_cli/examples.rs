use std::path::{Path, PathBuf};

use marlin_agent_core::{
    GraphLoopExecutionResult, GraphLoopExecutionStatus, LoopRunReceipt, run_marlin_cli_from_args,
};

fn process_command_example_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join("debug-cli")
        .join("process-command")
}

#[test]
fn debug_cli_process_command_example_graph_run_is_executable() {
    let example_dir = process_command_example_dir();
    let graph = example_dir.join("graph.json");
    let catalog = example_dir.join("catalog.toml");

    let result = run_marlin_cli_from_args([
        "graph",
        "run",
        "--input",
        graph.to_str().expect("utf8 path"),
        "--run-id",
        "example-process-command",
        "--catalog",
        catalog.to_str().expect("utf8 path"),
    ]);

    assert_eq!(result.status, 0, "{}", result.stderr);
    let receipt: GraphLoopExecutionResult =
        serde_json::from_str(&result.stdout).expect("execution result");
    assert_eq!(receipt.status, GraphLoopExecutionStatus::Completed);
    assert_eq!(receipt.visited_nodes, vec!["plan"]);
    let diagnostics = &receipt.node_receipts[0].diagnostics;
    assert!(diagnostics.contains(&"process-command.exit_status:0".to_owned()));
    assert!(diagnostics.contains(&"process-command.stdout:marlin-debug-cli-example".to_owned()));
    assert!(diagnostics.contains(&"process-command.node:plan".to_owned()));
}

#[test]
fn debug_cli_process_command_example_loop_run_is_executable() {
    let example_dir = process_command_example_dir();
    let graph = example_dir.join("graph.json");
    let catalog = example_dir.join("catalog.toml");

    let result = run_marlin_cli_from_args([
        "loop",
        "run",
        "--input",
        graph.to_str().expect("utf8 path"),
        "--max-iterations",
        "1",
        "--no-store",
        "--catalog",
        catalog.to_str().expect("utf8 path"),
    ]);

    assert_eq!(result.status, 0, "{}", result.stderr);
    let receipt: LoopRunReceipt = serde_json::from_str(&result.stdout).expect("loop run receipt");
    assert_eq!(
        receipt.terminal_status,
        Some(GraphLoopExecutionStatus::Completed)
    );
    assert_eq!(
        receipt.reports[0].execution_result.visited_nodes,
        vec!["plan"]
    );
}
