use marlin_agent_core::{
    GraphLoopExecutionResult, GraphLoopExecutionStatus, run_marlin_cli_from_args,
};
use std::fs;
use tempfile::tempdir;

use crate::runtime::facade::debug_cli::fixtures::{
    adapter_registration_graph, catalog_toml, process_command_catalog_toml, single_node_graph,
    single_node_graph_with_executor,
};

#[test]
fn debug_cli_graph_run_executes_debug_executor_catalog() {
    let dir = tempdir().expect("tempdir");
    let input = dir.path().join("graph.json");
    fs::write(
        &input,
        serde_json::to_string(&single_node_graph()).expect("graph JSON"),
    )
    .expect("write graph");

    let result = run_marlin_cli_from_args([
        "graph",
        "run",
        "--input",
        input.to_str().expect("utf8 path"),
        "--run-id",
        "debug-run",
    ]);

    assert_eq!(result.status, 0, "{}", result.stderr);
    let receipt: GraphLoopExecutionResult =
        serde_json::from_str(&result.stdout).expect("execution result");
    assert_eq!(receipt.status, GraphLoopExecutionStatus::Completed);
    assert_eq!(receipt.visited_nodes, vec!["plan"]);
}

#[test]
fn debug_cli_graph_run_rejects_unknown_debug_executor() {
    let dir = tempdir().expect("tempdir");
    let input = dir.path().join("graph.json");
    let mut graph = single_node_graph();
    graph.nodes[0].executor = "debug.unknown".to_owned();
    fs::write(&input, serde_json::to_string(&graph).expect("graph JSON")).expect("write graph");

    let result = run_marlin_cli_from_args([
        "graph",
        "run",
        "--input",
        input.to_str().expect("utf8 path"),
        "--run-id",
        "debug-run",
    ]);

    assert_eq!(result.status, 0, "{}", result.stderr);
    let receipt: GraphLoopExecutionResult =
        serde_json::from_str(&result.stdout).expect("execution result");
    assert_eq!(receipt.status, GraphLoopExecutionStatus::Failed);
    assert!(receipt.visited_nodes.is_empty());
    assert!(
        receipt
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("missing graph node executor `debug.unknown`"))
    );
}

#[test]
fn debug_cli_graph_run_executes_builtin_adapter_registrations() {
    let dir = tempdir().expect("tempdir");
    let input = dir.path().join("graph.json");
    fs::write(
        &input,
        serde_json::to_string(&adapter_registration_graph()).expect("graph JSON"),
    )
    .expect("write graph");

    let result = run_marlin_cli_from_args([
        "graph",
        "run",
        "--input",
        input.to_str().expect("utf8 path"),
        "--run-id",
        "adapter-run",
    ]);

    assert_eq!(result.status, 0, "{}", result.stderr);
    let receipt: GraphLoopExecutionResult =
        serde_json::from_str(&result.stdout).expect("execution result");
    assert_eq!(receipt.status, GraphLoopExecutionStatus::Completed);
    assert_eq!(receipt.visited_nodes, vec!["tool", "provider", "subagent"]);
}

#[test]
fn debug_cli_graph_run_accepts_catalog_config() {
    let dir = tempdir().expect("tempdir");
    let input = dir.path().join("graph.json");
    let catalog = dir.path().join("catalog.toml");
    fs::write(
        &input,
        serde_json::to_string(&single_node_graph_with_executor("custom.provider.echo"))
            .expect("graph JSON"),
    )
    .expect("write graph");
    fs::write(&catalog, catalog_toml("custom.provider.echo", "provider")).expect("write catalog");

    let result = run_marlin_cli_from_args([
        "graph",
        "run",
        "--input",
        input.to_str().expect("utf8 path"),
        "--run-id",
        "custom-catalog-run",
        "--catalog",
        catalog.to_str().expect("utf8 path"),
    ]);

    assert_eq!(result.status, 0, "{}", result.stderr);
    let receipt: GraphLoopExecutionResult =
        serde_json::from_str(&result.stdout).expect("execution result");
    assert_eq!(receipt.status, GraphLoopExecutionStatus::Completed);
    assert_eq!(receipt.visited_nodes, vec!["plan"]);
}

#[test]
fn debug_cli_graph_run_accepts_process_command_runtime_binding() {
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
        process_command_catalog_toml("custom.process.echo", "/bin/echo", &["marlin"]),
    )
    .expect("write catalog");

    let result = run_marlin_cli_from_args([
        "graph",
        "run",
        "--input",
        input.to_str().expect("utf8 path"),
        "--run-id",
        "process-command-run",
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
    assert!(diagnostics.contains(&"process-command.stdout:marlin".to_owned()));
    assert!(diagnostics.contains(&"process-command.node:plan".to_owned()));
}

#[test]
fn debug_cli_graph_run_reports_process_command_spawn_failure() {
    let dir = tempdir().expect("tempdir");
    let input = dir.path().join("graph.json");
    let catalog = dir.path().join("catalog.toml");
    let missing_command = dir.path().join("missing-command");
    fs::write(
        &input,
        serde_json::to_string(&single_node_graph_with_executor("custom.process.missing"))
            .expect("graph JSON"),
    )
    .expect("write graph");
    fs::write(
        &catalog,
        process_command_catalog_toml(
            "custom.process.missing",
            missing_command.to_str().expect("utf8 path"),
            &[],
        ),
    )
    .expect("write catalog");

    let result = run_marlin_cli_from_args([
        "graph",
        "run",
        "--input",
        input.to_str().expect("utf8 path"),
        "--run-id",
        "process-command-spawn-failure",
        "--catalog",
        catalog.to_str().expect("utf8 path"),
    ]);

    assert_eq!(result.status, 0, "{}", result.stderr);
    let receipt: GraphLoopExecutionResult =
        serde_json::from_str(&result.stdout).expect("execution result");
    assert_eq!(receipt.status, GraphLoopExecutionStatus::Failed);
    let diagnostics = &receipt.node_receipts[0].diagnostics;
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.starts_with("process-command.spawn_failed:"))
    );
    assert!(diagnostics.contains(&"process-command.exit_status:unknown".to_owned()));
    assert!(diagnostics.contains(&"process-command.node:plan".to_owned()));
}
