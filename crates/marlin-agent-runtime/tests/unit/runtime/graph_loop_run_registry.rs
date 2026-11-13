// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use marlin_agent_protocol::{
    GraphLoopExecutionStatus, GraphLoopIterationId, GraphLoopNextAction, HumanDecision,
    HumanDecisionReceipt, HumanGateReceipt, LoopGraph, NodeId, RunId,
};
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

#[tokio::test]
async fn registry_watch_coalesces_wakes_and_retains_typed_pending_gate() {
    let handle = GraphLoopRunRegistryHandle::new();
    let mut changes = handle.subscribe();
    let run_id = RunId::new("watched-run");
    handle.with_registry(|registry| {
        registry
            .start_run(run_id.clone(), "graph", 0)
            .expect("start");
    });
    handle.with_registry(|registry| {
        registry
            .record_progress(GraphLoopRunProgressUpdate::new(
                run_id.clone(),
                0,
                1,
                "progress",
            ))
            .expect("progress");
    });
    handle.with_registry(|registry| {
        registry
            .defer_run(
                HumanGateReceipt::new("watched-gate", run_id.clone(), 0, "review"),
                2,
                "gate",
            )
            .expect("gate");
    });
    changes.changed().await.expect("registry change");
    assert_eq!(changes.borrow_and_update().get(), 3);
    let gate = handle.read_registry(|registry| {
        registry
            .pending_human_gate(&run_id)
            .cloned()
            .expect("typed gate remains in registry")
    });
    assert_eq!(gate.gate_id.as_str(), "watched-gate");

    handle.with_registry(|registry| {
        registry.cancel_run(&run_id, 4);
    });
    changes.changed().await.expect("cancellation change");
    assert_eq!(changes.borrow_and_update().get(), 4);
    assert!(handle.read_registry(|registry| registry.pending_human_gate(&run_id).is_none()));
}

#[test]
fn graph_loop_run_registry_holds_gate_without_claiming_completion() {
    let mut registry = GraphLoopRunRegistry::new();
    let run_id = RunId::new("run-1");
    registry
        .start_run(run_id.clone(), "graph-1", 10)
        .expect("start run");
    registry
        .record_progress(GraphLoopRunProgressUpdate::new(
            run_id.clone(),
            0,
            20,
            "progress",
        ))
        .expect("record progress");
    let gate = HumanGateReceipt::new("gate-1", run_id.clone(), 0, "review")
        .with_proposed_next_action(GraphLoopNextAction::StopCompleted);
    let suspended = registry
        .defer_run(gate.clone(), 21, "human-gate")
        .expect("suspend at matching gate");
    assert_eq!(suspended.status, GraphLoopRunStatus::AwaitingHuman);
    assert_eq!(suspended.pending_human_gate, Some(gate.clone()));
    assert_eq!(suspended.terminal_status, None);
    assert_eq!(
        registry.wait_run(&run_id, 22).status,
        GraphLoopRunWaitStatus::AwaitingHuman
    );
    assert_eq!(registry.snapshot(22).active_count, 1);
    assert!(matches!(
        registry.complete_run(
            &run_id,
            GraphLoopExecutionStatus::Completed,
            23,
            "wrong-terminal"
        ),
        Err(GraphLoopRunRegistryError::InvalidRunState {
            status: GraphLoopRunStatus::AwaitingHuman,
            ..
        })
    ));
    assert!(matches!(
        registry.defer_run(gate, 24, "duplicate-gate"),
        Err(GraphLoopRunRegistryError::InvalidRunState {
            status: GraphLoopRunStatus::AwaitingHuman,
            ..
        })
    ));

    let cancelled = registry.cancel_run(&run_id, 25);
    assert_eq!(cancelled.status, GraphLoopRunCancelStatus::Requested);
    let observation = cancelled.observation.expect("cancelled run");
    assert_eq!(observation.status, GraphLoopRunStatus::Cancelled);
    assert_eq!(
        observation.terminal_status,
        Some(GraphLoopExecutionStatus::Cancelled)
    );
    assert!(observation.pending_human_gate.is_none());
    assert_eq!(
        registry.wait_run(&run_id, 26).status,
        GraphLoopRunWaitStatus::Idle
    );
    assert!(matches!(
        registry.decide_human_gate(
            &run_id,
            &HumanDecisionReceipt::new("gate-1", HumanDecision::Approved),
            27,
            "late-approval",
        ),
        Err(GraphLoopRunRegistryError::InvalidRunState {
            status: GraphLoopRunStatus::Cancelled,
            ..
        })
    ));
}

#[test]
fn graph_loop_run_registry_rejects_gate_for_another_iteration() {
    let mut registry = GraphLoopRunRegistry::new();
    registry
        .start_run("run-1", "graph-1", 10)
        .expect("start run");
    registry
        .record_progress(GraphLoopRunProgressUpdate::new("run-1", 1, 20, "progress"))
        .expect("record progress");
    assert_eq!(
        registry.defer_run(
            HumanGateReceipt::new("gate-0", "run-1", 0, "review"),
            21,
            "gate"
        ),
        Err(GraphLoopRunRegistryError::GateIterationMismatch {
            run_id: RunId::new("run-1"),
            iteration_id: GraphLoopIterationId::new(0),
        })
    );
    assert_eq!(
        registry
            .inspect_run(&RunId::new("run-1"), 22)
            .observation
            .expect("run")
            .status,
        GraphLoopRunStatus::Active
    );
}

#[test]
fn cancellation_before_gate_admission_finishes_without_a_pending_gate() {
    let mut registry = GraphLoopRunRegistry::new();
    let run_id = RunId::new("run-1");
    registry
        .start_run(run_id.clone(), "graph-1", 10)
        .expect("start run");
    registry
        .record_progress(GraphLoopRunProgressUpdate::new(
            run_id.clone(),
            0,
            20,
            "progress",
        ))
        .expect("record progress");
    assert_eq!(
        registry.cancel_run(&run_id, 21).status,
        GraphLoopRunCancelStatus::Requested
    );
    let observation = registry
        .defer_run(
            HumanGateReceipt::new("gate-0", run_id.clone(), 0, "review"),
            22,
            "gate",
        )
        .expect("finish cancelling run");
    assert_eq!(observation.status, GraphLoopRunStatus::Cancelled);
    assert_eq!(
        observation.terminal_status,
        Some(GraphLoopExecutionStatus::Cancelled)
    );
    assert!(observation.pending_human_gate.is_none());
    assert_eq!(
        registry.wait_run(&run_id, 23).status,
        GraphLoopRunWaitStatus::Idle
    );
}

#[test]
fn human_decision_requires_exact_pending_gate_and_can_be_admitted_once() {
    let mut registry = GraphLoopRunRegistry::new();
    let run_id = RunId::new("run-1");
    registry
        .start_run(run_id.clone(), "graph-1", 10)
        .expect("start");
    registry
        .record_progress(GraphLoopRunProgressUpdate::new(
            run_id.clone(),
            0,
            20,
            "progress",
        ))
        .expect("progress");
    let graph = LoopGraph {
        graph_id: "next".into(),
        nodes: Vec::new(),
        edges: Vec::new(),
    };
    registry
        .defer_run(
            HumanGateReceipt::new("gate-1", run_id.clone(), 0, "review")
                .with_proposed_next_action(GraphLoopNextAction::ContinueWithGraph(graph.clone())),
            21,
            "gate",
        )
        .expect("gate");

    assert!(matches!(
        registry.decide_human_gate(
            &run_id,
            &HumanDecisionReceipt::new("stale", HumanDecision::Approved),
            22,
            "wrong",
        ),
        Err(GraphLoopRunRegistryError::GateMismatch { .. })
    ));
    assert_eq!(
        registry
            .decide_human_gate(
                &run_id,
                &HumanDecisionReceipt::new("gate-1", HumanDecision::NeedsQuestion),
                23,
                "question",
            )
            .expect("question keeps gate pending"),
        None
    );
    assert_eq!(
        registry.wait_run(&run_id, 24).status,
        GraphLoopRunWaitStatus::AwaitingHuman
    );
    let approved = HumanDecisionReceipt::new("gate-1", HumanDecision::Approved);
    assert_eq!(
        registry
            .decide_human_gate(&run_id, &approved, 25, "approved")
            .expect("approval"),
        Some(graph)
    );
    assert!(matches!(
        registry.decide_human_gate(&run_id, &approved, 26, "replay"),
        Err(GraphLoopRunRegistryError::InvalidRunState {
            status: GraphLoopRunStatus::Active,
            ..
        })
    ));
}

#[test]
fn rejection_terminates_pending_gate_and_invalid_decisions_do_not() {
    let mut registry = GraphLoopRunRegistry::new();
    let run_id = RunId::new("run-1");
    registry
        .start_run(run_id.clone(), "graph-1", 10)
        .expect("start");
    registry
        .record_progress(GraphLoopRunProgressUpdate::new(
            run_id.clone(),
            0,
            20,
            "progress",
        ))
        .expect("progress");
    registry
        .defer_run(
            HumanGateReceipt::new("gate-1", run_id.clone(), 0, "review"),
            21,
            "gate",
        )
        .expect("gate");
    assert!(matches!(
        registry.decide_human_gate(
            &run_id,
            &HumanDecisionReceipt::new("gate-1", HumanDecision::Approved),
            22,
            "invalid-approval",
        ),
        Err(GraphLoopRunRegistryError::InvalidHumanDecision { .. })
    ));
    assert!(matches!(
        registry.decide_human_gate(
            &run_id,
            &HumanDecisionReceipt::new("gate-1", HumanDecision::Rejected).with_approved_next_graph(
                LoopGraph {
                    graph_id: "wrong".into(),
                    nodes: Vec::new(),
                    edges: Vec::new()
                }
            ),
            23,
            "invalid-rejection",
        ),
        Err(GraphLoopRunRegistryError::InvalidHumanDecision { .. })
    ));
    assert_eq!(
        registry
            .decide_human_gate(
                &run_id,
                &HumanDecisionReceipt::new("gate-1", HumanDecision::Rejected),
                24,
                "rejected",
            )
            .expect("rejection"),
        None
    );
    let observation = registry.inspect_run(&run_id, 25).observation.expect("run");
    assert_eq!(observation.status, GraphLoopRunStatus::Failed);
    assert_eq!(
        observation.terminal_status,
        Some(GraphLoopExecutionStatus::Failed)
    );
    assert!(observation.pending_human_gate.is_none());
}

#[test]
fn cancellation_wins_over_late_successful_completion() {
    let mut registry = GraphLoopRunRegistry::new();
    let run_id = RunId::new("run-1");
    registry
        .start_run(run_id.clone(), "graph-1", 10)
        .expect("start");
    assert_eq!(
        registry.cancel_run(&run_id, 20).status,
        GraphLoopRunCancelStatus::Requested
    );
    let observation = registry
        .complete_run(&run_id, GraphLoopExecutionStatus::Completed, 21, "terminal")
        .expect("finish cancelling run");
    assert_eq!(observation.status, GraphLoopRunStatus::Cancelled);
    assert_eq!(
        observation.terminal_status,
        Some(GraphLoopExecutionStatus::Cancelled)
    );
}

#[test]
#[ignore = "manual fixed-load registry lookup and wake benchmark"]
fn benchmark_graph_loop_status_lookup_through_shared_registry() {
    const LOOKUPS: usize = 100_000;
    let handle = GraphLoopRunRegistryHandle::new();
    let run_id = RunId::new("benchmark-run");
    handle.with_registry(|registry| {
        registry
            .start_run(run_id.clone(), "benchmark-graph", 0)
            .expect("start benchmark run");
    });
    let started = std::time::Instant::now();
    for _ in 0..LOOKUPS {
        let status = handle.read_registry(|registry| registry.run_status(&run_id));
        std::hint::black_box(status);
    }
    let elapsed = started.elapsed();
    eprintln!(
        "shared graph-loop status lookup: {LOOKUPS} calls in {elapsed:?}, {:.1} ns/call",
        elapsed.as_nanos() as f64 / LOOKUPS as f64
    );
    let mut changes = handle.subscribe();
    let started = std::time::Instant::now();
    for _ in 0..LOOKUPS {
        handle.with_registry(|_| ());
    }
    let elapsed = started.elapsed();
    assert_eq!(changes.borrow_and_update().get(), LOOKUPS as u64 + 1);
    eprintln!(
        "shared graph-loop wake publication: {LOOKUPS} scopes in {elapsed:?}, {:.1} ns/scope",
        elapsed.as_nanos() as f64 / LOOKUPS as f64
    );
}
