// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use marlin_agent_graph::{
    DispatchRequestId, ExecutionReceiptStatus, FencingEpoch, GraphConvergenceNodeId,
    GraphDispatchOperation, GraphDispatchRejection, GraphSubstrateTarget, MarlinDispatchEnvelope,
    MarlinExecutionReceipt, ReceiptReconciliationReason, TaskActorCursor, TaskActorId,
    TaskActorScopeMode, TaskActorStatus, WorkbenchActiveTaskInput, WorkbenchTabKind,
    admit_marlin_dispatch, project_workbench_tabs, reconcile_marlin_execution_receipt,
};

#[test]
fn dispatch_admission_enforces_steering_join_and_detached_isolation() {
    let valid = MarlinDispatchEnvelope {
        request_id: DispatchRequestId::new("req-1").expect("valid request id"),
        task_id: TaskActorId::new("task-alpha").expect("valid task id"),
        operation: GraphDispatchOperation::ExecuteFrontier,
        fencing_epoch: FencingEpoch::new(1).expect("non-zero epoch"),
        scope_mode: TaskActorScopeMode::Project,
        join_node_id: None,
        substrate_target: GraphSubstrateTarget::SqliteTurso,
    };
    let admitted = admit_marlin_dispatch(valid).expect("valid envelope must admit");
    assert_eq!(admitted.envelope().fencing_epoch.value(), 1);

    // Steering without join node must fail closed.
    let missing_join = MarlinDispatchEnvelope {
        request_id: DispatchRequestId::new("req-2").expect("valid request id"),
        task_id: TaskActorId::new("task-alpha").expect("valid task id"),
        operation: GraphDispatchOperation::ApplySteering,
        fencing_epoch: FencingEpoch::new(2).expect("non-zero epoch"),
        scope_mode: TaskActorScopeMode::Project,
        join_node_id: None,
        substrate_target: GraphSubstrateTarget::PureMemory,
    };
    assert_eq!(
        admit_marlin_dispatch(missing_join),
        Err(GraphDispatchRejection::SteeringRequiresJoinNode)
    );

    // Steering with explicit join node admits cleanly.
    let anchored_steering = MarlinDispatchEnvelope {
        request_id: DispatchRequestId::new("req-3").expect("valid request id"),
        task_id: TaskActorId::new("task-alpha").expect("valid task id"),
        operation: GraphDispatchOperation::ApplySteering,
        fencing_epoch: FencingEpoch::new(2).expect("non-zero epoch"),
        scope_mode: TaskActorScopeMode::Project,
        join_node_id: Some(GraphConvergenceNodeId::new("join-F").expect("valid join id")),
        substrate_target: GraphSubstrateTarget::PureMemory,
    };
    assert!(admit_marlin_dispatch(anchored_steering).is_ok());

    // Detached scope cannot commit project settlement.
    let detached_commit = MarlinDispatchEnvelope {
        request_id: DispatchRequestId::new("req-4").expect("valid request id"),
        task_id: TaskActorId::new("task-detached").expect("valid task id"),
        operation: GraphDispatchOperation::CommitSettlement,
        fencing_epoch: FencingEpoch::new(1).expect("non-zero epoch"),
        scope_mode: TaskActorScopeMode::Detached,
        join_node_id: None,
        substrate_target: GraphSubstrateTarget::PureMemory,
    };
    assert_eq!(
        admit_marlin_dispatch(detached_commit),
        Err(GraphDispatchRejection::DetachedScopeCannotCommitProjectSettlement)
    );
}

#[test]
fn receipt_reconciliation_enforces_fencing_epoch_and_c4_steering_gate() {
    let task_id = TaskActorId::new("task-alpha").expect("task id");
    let other_id = TaskActorId::new("task-beta").expect("other task id");
    let cursor = TaskActorCursor {
        task_id: task_id.clone(),
        fencing_epoch: FencingEpoch::new(3).expect("epoch"),
        status: TaskActorStatus::Running,
        scope_mode: TaskActorScopeMode::Project,
        pending_steering_branches: 2,
    };

    // Stale epoch rejected fail-closed (anti-split-brain).
    let stale = reconcile_marlin_execution_receipt(
        &cursor,
        &MarlinExecutionReceipt {
            request_id: DispatchRequestId::new("req-old").expect("req"),
            task_id: task_id.clone(),
            fencing_epoch: FencingEpoch::new(2).expect("epoch"),
            status: ExecutionReceiptStatus::Ok,
        },
    );
    assert!(!stale.accepted);
    assert_eq!(stale.reason, ReceiptReconciliationReason::StaleFencingEpoch);
    assert_eq!(stale.next_epoch.value(), 3);

    // Future epoch rejected fail-closed.
    let future = reconcile_marlin_execution_receipt(
        &cursor,
        &MarlinExecutionReceipt {
            request_id: DispatchRequestId::new("req-future").expect("req"),
            task_id: task_id.clone(),
            fencing_epoch: FencingEpoch::new(4).expect("epoch"),
            status: ExecutionReceiptStatus::Ok,
        },
    );
    assert!(!future.accepted);
    assert_eq!(
        future.reason,
        ReceiptReconciliationReason::FutureFencingEpoch
    );

    // Mismatched task ID rejected.
    let mismatch = reconcile_marlin_execution_receipt(
        &cursor,
        &MarlinExecutionReceipt {
            request_id: DispatchRequestId::new("req-mis").expect("req"),
            task_id: other_id,
            fencing_epoch: FencingEpoch::new(3).expect("epoch"),
            status: ExecutionReceiptStatus::Ok,
        },
    );
    assert!(!mismatch.accepted);
    assert_eq!(mismatch.reason, ReceiptReconciliationReason::TaskIdMismatch);

    // Fresh ok receipt while steering branches pending holds actor in WaitingSteering.
    let waiting = reconcile_marlin_execution_receipt(
        &cursor,
        &MarlinExecutionReceipt {
            request_id: DispatchRequestId::new("req-3").expect("req"),
            task_id: task_id.clone(),
            fencing_epoch: FencingEpoch::new(3).expect("epoch"),
            status: ExecutionReceiptStatus::Ok,
        },
    );
    assert!(waiting.accepted);
    assert_eq!(waiting.next_epoch.value(), 4);
    assert_eq!(waiting.next_status, TaskActorStatus::WaitingSteering);
    assert_eq!(
        waiting.reason,
        ReceiptReconciliationReason::AwaitingSteeringConvergence
    );

    // Cannot settle while pending_steering_branches > 0.
    let blocked_settle = reconcile_marlin_execution_receipt(
        &cursor,
        &MarlinExecutionReceipt {
            request_id: DispatchRequestId::new("req-settle-early").expect("req"),
            task_id: task_id.clone(),
            fencing_epoch: FencingEpoch::new(3).expect("epoch"),
            status: ExecutionReceiptStatus::Settled,
        },
    );
    assert!(!blocked_settle.accepted);
    assert_eq!(
        blocked_settle.reason,
        ReceiptReconciliationReason::CannotSettleWithPendingSteering
    );

    // Once all anchored steering branches complete, settlement succeeds and freezes actor.
    let converged_cursor = TaskActorCursor {
        pending_steering_branches: 0,
        fencing_epoch: FencingEpoch::new(4).expect("epoch"),
        status: TaskActorStatus::WaitingSteering,
        ..cursor
    };
    let settled = reconcile_marlin_execution_receipt(
        &converged_cursor,
        &MarlinExecutionReceipt {
            request_id: DispatchRequestId::new("req-settle").expect("req"),
            task_id: task_id.clone(),
            fencing_epoch: FencingEpoch::new(4).expect("epoch"),
            status: ExecutionReceiptStatus::Settled,
        },
    );
    assert!(settled.accepted);
    assert_eq!(settled.next_status, TaskActorStatus::Settled);
    assert_eq!(settled.next_epoch.value(), 5);

    let terminal_cursor = TaskActorCursor {
        fencing_epoch: settled.next_epoch,
        status: settled.next_status,
        ..converged_cursor
    };
    let post_terminal = reconcile_marlin_execution_receipt(
        &terminal_cursor,
        &MarlinExecutionReceipt {
            request_id: DispatchRequestId::new("req-late").expect("req"),
            task_id,
            fencing_epoch: FencingEpoch::new(5).expect("epoch"),
            status: ExecutionReceiptStatus::Ok,
        },
    );
    assert!(!post_terminal.accepted);
    assert_eq!(
        post_terminal.reason,
        ReceiptReconciliationReason::ActorAlreadyTerminal
    );
}

#[test]
fn workbench_tab_projection_pins_index_and_keeps_peeks_disjoint() {
    let t1 = TaskActorId::new("task-1").expect("t1");
    let t2 = TaskActorId::new("task-2").expect("t2");
    let archived = TaskActorId::new("task-archived-9").expect("archived");

    let projection = project_workbench_tabs(
        Some(&t2),
        &[
            WorkbenchActiveTaskInput {
                task_id: t1.clone(),
                title: "Auth".to_owned(),
                status: TaskActorStatus::Running,
                fencing_epoch: FencingEpoch::new(2).expect("epoch"),
            },
            WorkbenchActiveTaskInput {
                task_id: t2.clone(),
                title: "Graph".to_owned(),
                status: TaskActorStatus::WaitingSteering,
                fencing_epoch: FencingEpoch::new(4).expect("epoch"),
            },
        ],
        &[t2.clone(), archived.clone(), archived],
    );

    assert_eq!(projection.focused_task_id, Some(t2));
    assert_eq!(projection.tabs.len(), 4);
    assert_eq!(projection.tabs[0].kind, WorkbenchTabKind::PinnedIndex);
    assert_eq!(projection.tabs[1].kind, WorkbenchTabKind::ActiveTask);
    assert!(!projection.tabs[1].focused);
    assert_eq!(projection.tabs[2].kind, WorkbenchTabKind::ActiveTask);
    assert!(projection.tabs[2].focused);
    assert_eq!(projection.tabs[3].kind, WorkbenchTabKind::ReadOnlyPeek);
    assert_eq!(projection.tabs[3].id, "task-archived-9");
}
