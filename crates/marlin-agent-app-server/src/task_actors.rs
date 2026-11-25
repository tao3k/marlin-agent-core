// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Session-free `TaskActor` control plane for Marlin App Server clients.
//!
//! Wraps `marlin_agent_runtime::TaskActorRuntimeRegistry` so external UIs (GPUI, Emacs,
//! MyGO, or gRPC bridges) can register actors, admit fail-closed GraphEngine dispatches,
//! reconcile anti-split-brain receipts, and subscribe to live pinned-Index Workbench tabs
//! without coupling to `SessionId`.

use marlin_agent_runtime::{
    AdmittedMarlinDispatch, MarlinDispatchEnvelope, MarlinExecutionReceipt,
    ReceiptReconciliationDecision, TaskActorCursor, TaskActorId, TaskActorRuntimeError,
    TaskActorRuntimeRegistry, TaskActorRuntimeSnapshot, TaskActorScopeMode,
    WorkbenchTabProjection,
};
use tokio::sync::watch;

/// Registration request for a Session-free `TaskActor` on the App Server control plane.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegisterTaskActorRequest {
    /// Stable Session-free task identifier.
    pub task_id: TaskActorId,
    /// Human-readable title displayed in Workbench tabs.
    pub title: String,
    /// Workspace isolation scope (`Project` or `Detached`).
    pub scope_mode: TaskActorScopeMode,
}

/// Session-free `TaskActor` control plane exposed by `marlin-agent-app-server`.
#[derive(Clone, Debug, Default)]
pub struct TaskActorControlPlane {
    registry: TaskActorRuntimeRegistry,
}

impl TaskActorControlPlane {
    /// Creates a new Session-free control plane backed by a fresh `TaskActorRuntimeRegistry`.
    pub fn new() -> Self {
        Self {
            registry: TaskActorRuntimeRegistry::new(),
        }
    }

    /// Mounts an existing `TaskActorRuntimeRegistry` onto this App Server control plane.
    pub fn with_registry(registry: TaskActorRuntimeRegistry) -> Self {
        Self { registry }
    }

    /// Borrows the underlying runtime registry.
    pub fn registry(&self) -> &TaskActorRuntimeRegistry {
        &self.registry
    }

    /// Registers a new Session-free `TaskActor` at `FencingEpoch(1)`.
    pub fn register_actor(
        &self,
        request: RegisterTaskActorRequest,
    ) -> Result<TaskActorCursor, TaskActorRuntimeError> {
        self.registry
            .register_actor(request.task_id, request.title, request.scope_mode)
    }

    /// Admits a Scheme-proposed `MarlinDispatchEnvelope` fail-closed.
    pub fn admit_dispatch(
        &self,
        envelope: MarlinDispatchEnvelope,
    ) -> Result<AdmittedMarlinDispatch, TaskActorRuntimeError> {
        self.registry.admit_dispatch(envelope)
    }

    /// Marks one anchored C4 steering branch as converged at `join_node_id`.
    pub fn complete_steering_branch(
        &self,
        task_id: &TaskActorId,
    ) -> Result<TaskActorCursor, TaskActorRuntimeError> {
        self.registry.complete_steering_branch(task_id)
    }

    /// Reconciles an execution receipt against the actor cursor with fencing-epoch protection.
    pub fn reconcile_receipt(
        &self,
        receipt: &MarlinExecutionReceipt,
    ) -> Result<ReceiptReconciliationDecision, TaskActorRuntimeError> {
        self.registry.reconcile_receipt(receipt)
    }

    /// Focuses an active task tab (or `None` to return focus to the pinned Index).
    pub fn set_focus(
        &self,
        task_id: Option<TaskActorId>,
    ) -> Result<WorkbenchTabProjection, TaskActorRuntimeError> {
        self.registry.set_focus(task_id)
    }

    /// Opens a read-only peek tab for an archived/settled task ID.
    pub fn open_read_only_peek(
        &self,
        task_id: TaskActorId,
    ) -> Result<WorkbenchTabProjection, TaskActorRuntimeError> {
        self.registry.open_read_only_peek(task_id)
    }

    /// Closes a read-only peek tab.
    pub fn close_read_only_peek(&self, task_id: &TaskActorId) -> WorkbenchTabProjection {
        self.registry.close_read_only_peek(task_id)
    }

    /// Captures a deterministic snapshot of all actors and the Workbench tab strip.
    pub fn snapshot(&self) -> TaskActorRuntimeSnapshot {
        self.registry.snapshot()
    }

    /// Subscribes to live `WorkbenchTabProjection` updates.
    pub fn subscribe_workbench_tabs(&self) -> watch::Receiver<WorkbenchTabProjection> {
        self.registry.subscribe_workbench_tabs()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use marlin_agent_runtime::{
        DispatchRequestId, ExecutionReceiptStatus, FencingEpoch, GraphConvergenceNodeId,
        GraphDispatchOperation, GraphSubstrateTarget, ReceiptReconciliationReason,
        TaskActorStatus, WorkbenchTabKind,
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
        assert_eq!(snap.workbench_tabs.tabs[0].kind, WorkbenchTabKind::PinnedIndex);

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
        let host = crate::ui_grpc::MyGoBridgeHost::new(ReleaseTrust::new(BTreeMap::new()), |_| {
            Ok(())
        })
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
}
