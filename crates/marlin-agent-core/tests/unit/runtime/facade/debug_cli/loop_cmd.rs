use marlin_agent_core::{
    GraphLoopExecutionBudget, GraphLoopExecutionRequest, GraphLoopExecutionStatus,
    GraphLoopNextAction, GraphLoopRunRequest, GraphLoopStopPolicy, LoopInspectReceipt,
    LoopReplayReceipt, LoopRunReceipt, RunId, run_marlin_cli_from_args,
    runtime::GraphLoopRunStatus,
};
use std::fs;
use tempfile::tempdir;

use super::fixtures::{
    catalog_toml, process_command_catalog_toml, single_node_graph, single_node_graph_with_executor,
    two_step_graph,
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
