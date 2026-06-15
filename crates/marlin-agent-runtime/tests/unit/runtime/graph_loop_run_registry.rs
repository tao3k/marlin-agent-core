use marlin_agent_protocol::{GraphLoopExecutionStatus, GraphLoopIterationId, NodeId, RunId};
use marlin_agent_runtime::{
    GraphLoopRunCancelStatus, GraphLoopRunProgressUpdate, GraphLoopRunRegistry,
    GraphLoopRunRegistryError, GraphLoopRunRegistryHandle, GraphLoopRunStatus,
    GraphLoopRunWaitStatus,
};

#[test]
fn graph_loop_run_registry_starts_and_inspects_active_run() {
    let mut registry = GraphLoopRunRegistry::new();
    let start = registry
        .start_run("run-1", "graph-1", 10)
        .expect("start run");

    assert_eq!(start.observation.run_id.as_str(), "run-1");
    assert_eq!(start.observation.graph_id.as_str(), "graph-1");
    assert_eq!(start.observation.status, GraphLoopRunStatus::Active);

    let inspect = registry.inspect_run(&RunId::new("run-1"), 11);
    let observation = inspect.observation.expect("known run");
    assert_eq!(observation.started_at_ms, 10);
    assert_eq!(observation.updated_at_ms, 10);

    let snapshot = registry.snapshot(12);
    assert_eq!(snapshot.active_count, 1);
    assert_eq!(snapshot.run_count, 1);
}

#[test]
fn graph_loop_run_registry_records_progress_and_cancellation_request() {
    let mut registry = GraphLoopRunRegistry::new();
    let run_id = RunId::new("run-1");
    registry
        .start_run(run_id.clone(), "graph-1", 10)
        .expect("start run");

    let progress = registry
        .record_progress(
            GraphLoopRunProgressUpdate::new(
                run_id.clone(),
                GraphLoopIterationId::new(2),
                20,
                "event-2",
            )
            .with_pending_node_ids(vec![NodeId::new("tool-batch")]),
        )
        .expect("record progress");
    assert_eq!(progress.current_iteration_id.expect("iteration").get(), 2);
    assert_eq!(progress.pending_node_ids[0].as_str(), "tool-batch");
    assert_eq!(
        progress.last_event_id.expect("last event").as_str(),
        "event-2"
    );

    let cancel = registry.cancel_run(&run_id, 30);
    assert_eq!(cancel.status, GraphLoopRunCancelStatus::Requested);
    assert_eq!(
        cancel.observation.expect("cancelled run").status,
        GraphLoopRunStatus::Cancelling
    );

    let wait = registry.wait_run(&run_id, 31);
    assert_eq!(wait.status, GraphLoopRunWaitStatus::Active);
}

#[test]
fn graph_loop_run_registry_completes_run_and_wait_reports_idle() {
    let mut registry = GraphLoopRunRegistry::new();
    let run_id = RunId::new("run-1");
    registry
        .start_run(run_id.clone(), "graph-1", 10)
        .expect("start run");

    let completed = registry
        .complete_run(
            &run_id,
            GraphLoopExecutionStatus::Completed,
            40,
            "event-final",
        )
        .expect("complete run");
    assert_eq!(completed.status, GraphLoopRunStatus::Completed);
    assert_eq!(
        completed.terminal_status,
        Some(GraphLoopExecutionStatus::Completed)
    );
    assert!(completed.pending_node_ids.is_empty());

    let wait = registry.wait_run(&run_id, 41);
    assert_eq!(wait.status, GraphLoopRunWaitStatus::Idle);

    let cancel = registry.cancel_run(&run_id, 42);
    assert_eq!(cancel.status, GraphLoopRunCancelStatus::AlreadyTerminal);
}

#[test]
fn graph_loop_run_registry_rejects_duplicate_runs_and_reports_missing_run() {
    let mut registry = GraphLoopRunRegistry::new();
    registry
        .start_run("run-1", "graph-1", 10)
        .expect("start run");

    let duplicate = registry
        .start_run("run-1", "graph-2", 20)
        .expect_err("duplicate run rejected");
    assert_eq!(
        duplicate,
        GraphLoopRunRegistryError::DuplicateRun {
            run_id: RunId::new("run-1")
        }
    );

    let missing = registry.cancel_run(&RunId::new("missing"), 30);
    assert_eq!(missing.status, GraphLoopRunCancelStatus::NotFound);
    assert!(missing.observation.is_none());
}

#[test]
fn graph_loop_run_registry_handle_shares_registry_state() {
    let handle = GraphLoopRunRegistryHandle::new();
    handle.with_registry(|registry| {
        registry
            .start_run("run-1", "graph-1", 10)
            .expect("start run");
    });

    let snapshot = handle.read_registry(|registry| registry.snapshot(20));
    assert_eq!(snapshot.active_count, 1);
    assert_eq!(snapshot.runs[0].run_id.as_str(), "run-1");
}
