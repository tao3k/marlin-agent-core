// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use std::sync::Arc;

use marlin_agent_app_server::task_actors::{RegisterTaskActorRequest, TaskActorControlPlane};
use marlin_agent_kernel::{GovernanceAdmissionGate, GovernedSubjectName};
use marlin_agent_runtime::{
    AdmittedGovernedContext, AdmittedMarlinDispatch, DetourMode, DetourNodeId, DetourState,
    DispatchRequestId, ExecutionReceiptStatus, FencingEpoch, GovernedContextCapsule,
    GraphConvergenceNodeId, GraphDispatchOperation, GraphEngineOracle, GraphSubstrateTarget,
    MarlinDispatchEnvelope, MarlinExecutionReceipt, ReceiptReconciliationDecision,
    ReceiptReconciliationReason, TaskActorCursor, TaskActorId, TaskActorRuntimeError,
    TaskActorScopeMode, TaskActorStatus, WorkbenchTabKind, WorkbenchTabProjection,
};

#[tokio::test]
async fn control_plane_streams_workbench_tabs_and_enforces_epoch_and_steering_gates() {
    let plane = TaskActorControlPlane::new();
    let mut rx = plane.subscribe_workbench_tabs();

    let task_id = TaskActorId::new("task-app-1").unwrap();
    let cursor = plane
        .register_actor(RegisterTaskActorRequest {
            task_id: task_id.clone(),
            title: "Remote Graph".to_owned(),
            scope_mode: TaskActorScopeMode::Project,
        })
        .unwrap();
    assert_eq!(cursor.fencing_epoch, FencingEpoch::new(1).unwrap());

    rx.changed().await.unwrap();
    let tabs = rx.borrow().clone();
    assert_eq!(tabs.tabs.len(), 2);
    assert_eq!(tabs.tabs[0].kind, WorkbenchTabKind::PinnedIndex);
    assert_eq!(tabs.tabs[1].id, "task-app-1");

    // Admit C4 ApplySteering with join node -> advances to WaitingSteering on receipt.
    plane
        .admit_dispatch(MarlinDispatchEnvelope {
            request_id: DispatchRequestId::new("req-app-steer").unwrap(),
            task_id: task_id.clone(),
            operation: GraphDispatchOperation::ApplySteering,
            fencing_epoch: FencingEpoch::new(1).unwrap(),
            scope_mode: TaskActorScopeMode::Project,
            join_node_id: Some(GraphConvergenceNodeId::new("join-srv").unwrap()),
            substrate_target: GraphSubstrateTarget::SqliteTurso,
        })
        .unwrap();
    let d1 = plane
        .reconcile_receipt(&MarlinExecutionReceipt {
            request_id: DispatchRequestId::new("req-app-steer").unwrap(),
            task_id: task_id.clone(),
            fencing_epoch: FencingEpoch::new(1).unwrap(),
            status: ExecutionReceiptStatus::Ok,
        })
        .unwrap();
    assert!(d1.accepted);
    assert_eq!(d1.next_status, TaskActorStatus::WaitingSteering);
    assert_eq!(d1.next_epoch, FencingEpoch::new(2).unwrap());

    // Stale epoch 1 receipt rejected without mutating epoch 2 cursor.
    let stale = plane
        .reconcile_receipt(&MarlinExecutionReceipt {
            request_id: DispatchRequestId::new("req-stale-srv").unwrap(),
            task_id: task_id.clone(),
            fencing_epoch: FencingEpoch::new(1).unwrap(),
            status: ExecutionReceiptStatus::Ok,
        })
        .unwrap();
    assert!(!stale.accepted);
    assert_eq!(stale.reason, ReceiptReconciliationReason::StaleFencingEpoch);

    // Converge steering branch and settle -> auto-archives and enables read-only peek.
    plane.complete_steering_branch(&task_id).unwrap();
    let settled = plane
        .reconcile_receipt(&MarlinExecutionReceipt {
            request_id: DispatchRequestId::new("req-app-settle").unwrap(),
            task_id: task_id.clone(),
            fencing_epoch: FencingEpoch::new(2).unwrap(),
            status: ExecutionReceiptStatus::Settled,
        })
        .unwrap();
    assert!(settled.accepted);
    assert_eq!(settled.next_status, TaskActorStatus::Settled);

    let snap = plane.snapshot();
    assert_eq!(snap.workbench_tabs.tabs.len(), 1);
    assert_eq!(
        snap.workbench_tabs.tabs[0].kind,
        WorkbenchTabKind::PinnedIndex
    );

    let peek_tabs = plane.open_read_only_peek(task_id.clone()).unwrap();
    assert_eq!(peek_tabs.tabs.len(), 2);
    assert_eq!(peek_tabs.tabs[1].kind, WorkbenchTabKind::ReadOnlyPeek);
    let closed = plane.close_read_only_peek(&task_id);
    assert_eq!(closed.tabs.len(), 1);
}

#[cfg(feature = "ui-grpc")]
#[tokio::test]
async fn mygo_bridge_host_shares_live_task_actor_control_plane_updates() {
    use marlin_agent_runtime::plugin::ReleaseTrust;
    use std::collections::BTreeMap;

    let shared = TaskActorControlPlane::new();
    let host =
        marlin_agent_app_server::ui_grpc::MyGoBridgeHost::new(ReleaseTrust::new(BTreeMap::new()), |_| Ok(()))
            .with_task_actors(shared.clone());

    let mut rx = host.task_actors().subscribe_workbench_tabs();
    let task_id = TaskActorId::new("task-bridge-1").unwrap();
    shared
        .register_actor(RegisterTaskActorRequest {
            task_id: task_id.clone(),
            title: "Bridge Graph".to_owned(),
            scope_mode: TaskActorScopeMode::Project,
        })
        .unwrap();

    rx.changed().await.unwrap();
    let tabs = rx.borrow().clone();
    assert_eq!(tabs.tabs.len(), 2);
    assert_eq!(tabs.tabs[0].kind, WorkbenchTabKind::PinnedIndex);
    assert_eq!(tabs.tabs[1].id, "task-bridge-1");
}

#[tokio::test]
async fn turso_wal_storage_persists_and_restores_task_actor_control_plane_with_epoch_protection()
 {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("task-actors-turso.db");
    let config = marlin_agent_storage::TursoAgentStorageConfig {
        path: db_path,
        optimization_profile: marlin_agent_storage::TursoOptimizationProfile::AsyncIoWithMvcc,
        batch_transaction_mode: marlin_agent_storage::TursoBatchTransactionMode::Concurrent,
    };
    let storage = marlin_agent_storage::TursoAgentStorage::open_local(config.clone())
        .await
        .expect("open turso wal");

    let project_id = marlin_agent_storage::ProjectId::new("proj-graph-01").unwrap();
    let pointer_key =
        marlin_agent_storage::ArtifactPointerKey::new("pointer:graph-engine:task-actors")
            .unwrap();
    let agent_id = marlin_agent_storage::AgentId::new("agent-graph").unwrap();
    let event_id = marlin_agent_storage::EventId::new("ev-checkpoint-1").unwrap();

    let plane = TaskActorControlPlane::new();
    let task_id = TaskActorId::new("task-turso-01").unwrap();
    plane
        .register_actor(RegisterTaskActorRequest {
            task_id: task_id.clone(),
            title: "Turso Durable Task".to_owned(),
            scope_mode: TaskActorScopeMode::Project,
        })
        .unwrap();
    let (_admitted, decision) = plane
        .execute_and_reconcile(
            MarlinDispatchEnvelope {
                request_id: DispatchRequestId::new("req-turso-1").unwrap(),
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
    assert!(decision.accepted);
    assert_eq!(decision.next_epoch.value(), 2);

    plane
        .enter_detour(
            &task_id,
            DetourMode::BlockerActive,
            DetourNodeId::new("detour-turso-blk").unwrap(),
        )
        .unwrap();

    let saved_hash = plane
        .save_to_agent_storage(&storage, &project_id, &pointer_key, &agent_id, &event_id)
        .await
        .expect("save snapshot to Turso WAL");
    assert!(saved_hash.as_str().starts_with("sha256:"));
    let expected_snapshot = plane.snapshot();
    drop(storage);

    let reopened = marlin_agent_storage::TursoAgentStorage::open_local(config)
        .await
        .expect("reopen turso wal");
    let restored =
        TaskActorControlPlane::load_from_agent_storage(&reopened, &project_id, &pointer_key)
            .await
            .expect("load snapshot from Turso WAL")
            .expect("checkpoint must exist");
    assert_eq!(restored.snapshot(), expected_snapshot);

    let stale = restored
        .reconcile_receipt(&MarlinExecutionReceipt {
            request_id: DispatchRequestId::new("req-turso-1").unwrap(),
            task_id: task_id.clone(),
            fencing_epoch: FencingEpoch::new(1).unwrap(),
            status: ExecutionReceiptStatus::Ok,
        })
        .unwrap();
    assert!(!stale.accepted);
    assert_eq!(stale.reason, ReceiptReconciliationReason::StaleFencingEpoch);
}

struct RejectingSchemeOracle;

impl GraphEngineOracle for RejectingSchemeOracle {
    fn project_dispatch(
        &self,
        _envelope: &MarlinDispatchEnvelope,
    ) -> Result<
        Result<AdmittedMarlinDispatch, marlin_agent_runtime::GraphDispatchRejection>,
        String,
    > {
        Err("scheme oracle rejected dispatch".to_owned())
    }

    fn reconcile_receipt(
        &self,
        _cursor: &TaskActorCursor,
        _receipt: &MarlinExecutionReceipt,
    ) -> Result<ReceiptReconciliationDecision, String> {
        Err("scheme oracle rejected receipt".to_owned())
    }

    fn project_workbench_tabs(
        &self,
        _focused_task_id: Option<&TaskActorId>,
        _active_tasks: &[marlin_agent_runtime::WorkbenchActiveTaskInput],
        _peek_task_ids: &[TaskActorId],
    ) -> Result<WorkbenchTabProjection, String> {
        Err("scheme oracle rejected workbench projection".to_owned())
    }

    fn project_governed_context(
        &self,
        _capsule: &GovernedContextCapsule,
    ) -> Result<
        Result<AdmittedGovernedContext, marlin_agent_runtime::GovernedContextRejection>,
        String,
    > {
        Err("scheme oracle rejected governed context".to_owned())
    }
}

#[tokio::test]
async fn control_plane_with_authoritative_oracle_enforces_fail_closed_agreement() {
    let plane = TaskActorControlPlane::new().with_oracle(Arc::new(RejectingSchemeOracle));
    let task_id = TaskActorId::new("task-oracle-01").unwrap();
    plane
        .register_actor(RegisterTaskActorRequest {
            task_id: task_id.clone(),
            title: "Oracle Guarded Task".to_owned(),
            scope_mode: TaskActorScopeMode::Project,
        })
        .unwrap();

    let err = plane
        .admit_dispatch(MarlinDispatchEnvelope {
            request_id: DispatchRequestId::new("req-oracle-1").unwrap(),
            task_id: task_id.clone(),
            operation: GraphDispatchOperation::ExecuteFrontier,
            fencing_epoch: FencingEpoch::new(1).unwrap(),
            scope_mode: TaskActorScopeMode::Project,
            join_node_id: None,
            substrate_target: GraphSubstrateTarget::SqliteTurso,
        })
        .unwrap_err();
    assert!(matches!(
        err,
        TaskActorRuntimeError::OracleDivergence { .. }
    ));

    let tabs_err = plane.project_workbench_tabs().unwrap_err();
    assert!(matches!(
        tabs_err,
        TaskActorRuntimeError::OracleDivergence { .. }
    ));
}

#[tokio::test]
async fn rehydrate_or_new_and_persistence_loop_survive_host_restart_on_turso_wal() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("host-restart-turso.db");
    let config = marlin_agent_storage::TursoAgentStorageConfig {
        path: db_path,
        optimization_profile: marlin_agent_storage::TursoOptimizationProfile::AsyncIoWithMvcc,
        batch_transaction_mode: marlin_agent_storage::TursoBatchTransactionMode::Concurrent,
    };
    let project_id = marlin_agent_storage::ProjectId::new("proj-host-01").unwrap();
    let pointer_key =
        marlin_agent_storage::ArtifactPointerKey::new("pointer:graph-engine:task-actors")
            .unwrap();
    let agent_id = marlin_agent_storage::AgentId::new("marlin-mygo-host").unwrap();

    // Boot #1 on empty Turso DB: rehydrate_or_new yields fresh plane + live persistence loop.
    let storage1 = marlin_agent_storage::TursoAgentStorage::open_local(config.clone())
        .await
        .expect("open boot #1 turso wal");
    let plane1 = TaskActorControlPlane::rehydrate_or_new(&storage1, &project_id, &pointer_key)
        .await
        .expect("boot #1 empty rehydrate");
    let handle = plane1.spawn_storage_persistence_loop(
        storage1.clone(),
        project_id.clone(),
        pointer_key.clone(),
        agent_id.clone(),
    );

    let task_id = TaskActorId::new("task-host-restart-01").unwrap();
    plane1
        .register_actor(RegisterTaskActorRequest {
            task_id: task_id.clone(),
            title: "Host Durable Actor".to_owned(),
            scope_mode: TaskActorScopeMode::Project,
        })
        .unwrap();
    plane1
        .enter_detour(
            &task_id,
            DetourMode::SideQuestActive,
            DetourNodeId::new("detour-side-01").unwrap(),
        )
        .unwrap();

    // Graceful shutdown flushes final durable checkpoint into Turso WAL.
    handle.flush_and_stop().await.expect("flush and stop");
    let expected_snapshot = plane1.snapshot();
    drop(storage1);

    // Boot #2 after simulated process restart: rehydrate_or_new recovers exact state.
    let storage2 = marlin_agent_storage::TursoAgentStorage::open_local(config)
        .await
        .expect("open boot #2 turso wal");
    let plane2 = TaskActorControlPlane::rehydrate_or_new(&storage2, &project_id, &pointer_key)
        .await
        .expect("boot #2 rehydrate");
    assert_eq!(plane2.snapshot(), expected_snapshot);
    assert_eq!(
        plane2.registry().detour_state(&task_id),
        Some(DetourState::Active {
            mode: DetourMode::SideQuestActive,
            origin_epoch: FencingEpoch::new(1).unwrap(),
            detour_node_id: DetourNodeId::new("detour-side-01").unwrap(),
        })
    );
}

#[test]
fn sm25_adv_l4_control_plane_admit_dispatch_blocks_fail_closed_on_estop() {
    use marlin_agent_kernel::{ExecutionCircuitBreaker, ToolBudgetUnits};

    let breaker = Arc::new(ExecutionCircuitBreaker::new(ToolBudgetUnits::new(8)));
    let gate = Arc::new(GovernanceAdmissionGate::new(breaker));
    let plane = TaskActorControlPlane::new().with_governance_gate(
        Arc::clone(&gate),
        GovernedSubjectName::new("marlin-mygo-host").unwrap(),
    );
    let task_id = TaskActorId::new("task-sm25-cp").unwrap();
    plane
        .register_actor(RegisterTaskActorRequest {
            task_id: task_id.clone(),
            title: "SM25 Control Plane Task".to_owned(),
            scope_mode: TaskActorScopeMode::Project,
        })
        .unwrap();

    gate.trip_operator_estop("operator-alice", "sm25-cp-estop")
        .expect("trip estop");
    let denied = plane.admit_dispatch(MarlinDispatchEnvelope {
        request_id: DispatchRequestId::new("req-sm25-cp-1").unwrap(),
        task_id,
        operation: GraphDispatchOperation::ExecuteFrontier,
        fencing_epoch: FencingEpoch::new(1).unwrap(),
        scope_mode: TaskActorScopeMode::Project,
        join_node_id: None,
        substrate_target: GraphSubstrateTarget::SqliteTurso,
    });
    assert!(matches!(
        denied,
        Err(TaskActorRuntimeError::GovernanceDenied { .. })
    ));
}
