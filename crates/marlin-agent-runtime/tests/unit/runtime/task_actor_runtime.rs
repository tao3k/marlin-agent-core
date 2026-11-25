// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use marlin_agent_runtime::{
    DispatchRequestId, ExecutionReceiptStatus, FencingEpoch, GraphConvergenceNodeId,
    GraphDispatchOperation, GraphDispatchRejection, GraphSubstrateTarget, MarlinDispatchEnvelope,
    MarlinExecutionReceipt, ReceiptReconciliationReason, TaskActorId, TaskActorRuntimeError,
    TaskActorRuntimeRegistry, TaskActorScopeMode, TaskActorStatus, WorkbenchTabKind,
};

#[tokio::test]
async fn task_actor_runtime_enforces_c4_steering_anti_split_brain_and_auto_archive() {
    let registry = TaskActorRuntimeRegistry::new();
    let mut tab_rx = registry.subscribe_workbench_tabs();

    // Initial projection pins Index at slot 0 with zero active tasks.
    let initial_tabs = tab_rx.borrow().clone();
    assert_eq!(initial_tabs.tabs.len(), 1);
    assert_eq!(initial_tabs.tabs[0].kind, WorkbenchTabKind::PinnedIndex);
    assert_eq!(initial_tabs.tabs[0].id, "index");

    let task_id = TaskActorId::new("task-graph-1").unwrap();
    let cursor = registry
        .register_actor(task_id.clone(), "Graph Engine", TaskActorScopeMode::Project)
        .unwrap();
    assert_eq!(cursor.fencing_epoch, FencingEpoch::new(1).unwrap());
    assert_eq!(cursor.status, TaskActorStatus::OpenFocus);

    // Focus the active task and verify watch channel updated.
    let tabs = registry.set_focus(Some(task_id.clone())).unwrap();
    assert_eq!(tabs.focused_task_id.as_ref(), Some(&task_id));
    assert_eq!(tabs.tabs.len(), 2);
    assert_eq!(tabs.tabs[0].kind, WorkbenchTabKind::PinnedIndex);
    assert_eq!(tabs.tabs[1].kind, WorkbenchTabKind::ActiveTask);
    assert!(tabs.tabs[1].focused);

    // Active task cannot simultaneously be opened as a read-only peek.
    assert_eq!(
        registry.open_read_only_peek(task_id.clone()),
        Err(TaskActorRuntimeError::CannotPeekActiveActor {
            task_id: task_id.clone(),
        })
    );

    // Step 1: ExecuteFrontier at epoch 1 -> advances to epoch 2, Running.
    let (admitted_1, decision_1) = registry
        .execute_admitted_dispatch(
            MarlinDispatchEnvelope {
                request_id: DispatchRequestId::new("req-1").unwrap(),
                task_id: task_id.clone(),
                operation: GraphDispatchOperation::ExecuteFrontier,
                fencing_epoch: FencingEpoch::new(1).unwrap(),
                scope_mode: TaskActorScopeMode::Project,
                join_node_id: None,
                substrate_target: GraphSubstrateTarget::SqliteTurso,
            },
            |admitted| async move {
                MarlinExecutionReceipt {
                    request_id: admitted.envelope().request_id.clone(),
                    task_id: admitted.envelope().task_id.clone(),
                    fencing_epoch: admitted.envelope().fencing_epoch,
                    status: ExecutionReceiptStatus::Ok,
                }
            },
        )
        .await
        .unwrap();
    assert_eq!(
        admitted_1.envelope().operation,
        GraphDispatchOperation::ExecuteFrontier
    );
    assert!(decision_1.accepted);
    assert_eq!(decision_1.next_epoch, FencingEpoch::new(2).unwrap());
    assert_eq!(decision_1.next_status, TaskActorStatus::Running);

    // Anti-split-brain: stale epoch 1 receipt must be rejected without mutating cursor.
    let stale_decision = registry
        .reconcile_receipt(&MarlinExecutionReceipt {
            request_id: DispatchRequestId::new("req-stale").unwrap(),
            task_id: task_id.clone(),
            fencing_epoch: FencingEpoch::new(1).unwrap(),
            status: ExecutionReceiptStatus::Ok,
        })
        .unwrap();
    assert!(TaskActorRuntimeRegistry::is_stale_split_brain_rejection(
        &stale_decision
    ));
    assert_eq!(
        registry.cursor(&task_id).unwrap().fencing_epoch,
        FencingEpoch::new(2).unwrap()
    );

    // Step 2: C4 ApplySteering without join_node_id must fail closed before mutation.
    let missing_join = registry.admit_dispatch(MarlinDispatchEnvelope {
        request_id: DispatchRequestId::new("req-bad-steer").unwrap(),
        task_id: task_id.clone(),
        operation: GraphDispatchOperation::ApplySteering,
        fencing_epoch: FencingEpoch::new(2).unwrap(),
        scope_mode: TaskActorScopeMode::Project,
        join_node_id: None,
        substrate_target: GraphSubstrateTarget::PureMemory,
    });
    assert_eq!(
        missing_join,
        Err(TaskActorRuntimeError::DispatchRejected(
            GraphDispatchRejection::SteeringRequiresJoinNode
        ))
    );

    // Step 3: Valid C4 ApplySteering with join_node_id anchors 1 pending branch.
    let (_admitted_steer, decision_steer) = registry
        .execute_admitted_dispatch(
            MarlinDispatchEnvelope {
                request_id: DispatchRequestId::new("req-steer").unwrap(),
                task_id: task_id.clone(),
                operation: GraphDispatchOperation::ApplySteering,
                fencing_epoch: FencingEpoch::new(2).unwrap(),
                scope_mode: TaskActorScopeMode::Project,
                join_node_id: Some(GraphConvergenceNodeId::new("join-apex").unwrap()),
                substrate_target: GraphSubstrateTarget::PureMemory,
            },
            |admitted| async move {
                MarlinExecutionReceipt {
                    request_id: admitted.envelope().request_id.clone(),
                    task_id: admitted.envelope().task_id.clone(),
                    fencing_epoch: admitted.envelope().fencing_epoch,
                    status: ExecutionReceiptStatus::Ok,
                }
            },
        )
        .await
        .unwrap();
    assert!(decision_steer.accepted);
    assert_eq!(decision_steer.next_epoch, FencingEpoch::new(3).unwrap());
    assert_eq!(
        decision_steer.next_status,
        TaskActorStatus::WaitingSteering
    );
    assert_eq!(
        decision_steer.reason,
        ReceiptReconciliationReason::AwaitingSteeringConvergence
    );

    // Premature settlement while steering branch is still pending is rejected.
    let blocked_settle = registry
        .reconcile_receipt(&MarlinExecutionReceipt {
            request_id: DispatchRequestId::new("req-premature-settle").unwrap(),
            task_id: task_id.clone(),
            fencing_epoch: FencingEpoch::new(3).unwrap(),
            status: ExecutionReceiptStatus::Settled,
        })
        .unwrap();
    assert!(!blocked_settle.accepted);
    assert_eq!(
        blocked_settle.reason,
        ReceiptReconciliationReason::CannotSettleWithPendingSteering
    );

    // Converge the pending steering branch -> actor unblocks back to Running.
    let converged_cursor = registry.complete_steering_branch(&task_id).unwrap();
    assert_eq!(converged_cursor.pending_steering_branches, 0);
    assert_eq!(converged_cursor.status, TaskActorStatus::Running);

    // Step 4: CommitSettlement at epoch 3 -> Settled & auto-archived from active tabs.
    let (_admitted_settle, decision_settle) = registry
        .execute_admitted_dispatch(
            MarlinDispatchEnvelope {
                request_id: DispatchRequestId::new("req-settle").unwrap(),
                task_id: task_id.clone(),
                operation: GraphDispatchOperation::CommitSettlement,
                fencing_epoch: FencingEpoch::new(3).unwrap(),
                scope_mode: TaskActorScopeMode::Project,
                join_node_id: None,
                substrate_target: GraphSubstrateTarget::SqliteTurso,
            },
            |admitted| async move {
                MarlinExecutionReceipt {
                    request_id: admitted.envelope().request_id.clone(),
                    task_id: admitted.envelope().task_id.clone(),
                    fencing_epoch: admitted.envelope().fencing_epoch,
                    status: ExecutionReceiptStatus::Settled,
                }
            },
        )
        .await
        .unwrap();
    assert!(decision_settle.accepted);
    assert_eq!(decision_settle.next_epoch, FencingEpoch::new(4).unwrap());
    assert_eq!(decision_settle.next_status, TaskActorStatus::Settled);

    // Task-as-Tab Auto-Archive: active tabs only keep pinned Index, focus cleared.
    tab_rx.changed().await.unwrap();
    let archived_tabs = tab_rx.borrow().clone();
    assert_eq!(archived_tabs.focused_task_id, None);
    assert_eq!(archived_tabs.tabs.len(), 1);
    assert_eq!(archived_tabs.tabs[0].kind, WorkbenchTabKind::PinnedIndex);

    // Once settled, the task can be opened as a read-only peek tab!
    let peek_tabs = registry.open_read_only_peek(task_id.clone()).unwrap();
    assert_eq!(peek_tabs.tabs.len(), 2);
    assert_eq!(peek_tabs.tabs[0].kind, WorkbenchTabKind::PinnedIndex);
    assert_eq!(peek_tabs.tabs[1].kind, WorkbenchTabKind::ReadOnlyPeek);
    assert_eq!(peek_tabs.tabs[1].id, "task-graph-1");

    // Post-terminal dispatch is rejected fail-closed.
    assert!(matches!(
        registry.admit_dispatch(MarlinDispatchEnvelope {
            request_id: DispatchRequestId::new("req-after-settled").unwrap(),
            task_id: task_id.clone(),
            operation: GraphDispatchOperation::ExecuteFrontier,
            fencing_epoch: FencingEpoch::new(4).unwrap(),
            scope_mode: TaskActorScopeMode::Project,
            join_node_id: None,
            substrate_target: GraphSubstrateTarget::PureMemory,
        }),
        Err(TaskActorRuntimeError::ActorTerminal {
            status: TaskActorStatus::Settled,
            ..
        })
    ));
}

#[test]
fn detached_actor_cannot_commit_project_settlement_or_cross_scope() {
    let registry = TaskActorRuntimeRegistry::new();
    let detached_id = TaskActorId::new("task-detached").unwrap();
    registry
        .register_actor(
            detached_id.clone(),
            "Detached Spike",
            TaskActorScopeMode::Detached,
        )
        .unwrap();

    // Attempting to spoof Project scope on a Detached actor fails closed.
    let spoofed = registry.admit_dispatch(MarlinDispatchEnvelope {
        request_id: DispatchRequestId::new("req-spoof").unwrap(),
        task_id: detached_id.clone(),
        operation: GraphDispatchOperation::CommitSettlement,
        fencing_epoch: FencingEpoch::new(1).unwrap(),
        scope_mode: TaskActorScopeMode::Project,
        join_node_id: None,
        substrate_target: GraphSubstrateTarget::PureMemory,
    });
    assert_eq!(
        spoofed,
        Err(TaskActorRuntimeError::ScopeModeMismatch {
            task_id: detached_id.clone(),
            expected: TaskActorScopeMode::Detached,
            received: TaskActorScopeMode::Project,
        })
    );

    // Attempting CommitSettlement in Detached scope fails the graph dispatch gate.
    let rejected = registry.admit_dispatch(MarlinDispatchEnvelope {
        request_id: DispatchRequestId::new("req-det-commit").unwrap(),
        task_id: detached_id,
        operation: GraphDispatchOperation::CommitSettlement,
        fencing_epoch: FencingEpoch::new(1).unwrap(),
        scope_mode: TaskActorScopeMode::Detached,
        join_node_id: None,
        substrate_target: GraphSubstrateTarget::PureMemory,
    });
    assert_eq!(
        rejected,
        Err(TaskActorRuntimeError::DispatchRejected(
            GraphDispatchRejection::DetachedScopeCannotCommitProjectSettlement
        ))
    );
}
