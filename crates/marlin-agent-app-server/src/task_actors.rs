// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Session-free `TaskActor` control plane for Marlin App Server clients.
//!
//! Wraps `marlin_agent_runtime::TaskActorRuntimeRegistry` so external UIs (GPUI, Emacs,
//! MyGO, or gRPC bridges) can register actors, admit fail-closed GraphEngine dispatches,
//! reconcile anti-split-brain receipts, and subscribe to live pinned-Index Workbench tabs
//! without coupling to `SessionId`.

use std::fmt;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use marlin_agent_runtime::{
    AdmittedGovernedContext, AdmittedMarlinDispatch, DetourExitKind, DetourMode, DetourNodeId,
    DetourState, GovernedContextCapsule, GraphEngineOracle, MarlinDispatchEnvelope,
    MarlinExecutionReceipt, ReceiptReconciliationDecision, TaskActorCursor, TaskActorId,
    TaskActorRuntimeError, TaskActorRuntimeRegistry, TaskActorRuntimeSnapshot, TaskActorScopeMode,
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

/// Handle controlling a background write-through persistence task for
/// `TaskActorControlPlane` backed by `marlin-agent-storage::AgentStorage`.
pub struct TaskActorPersistenceHandle {
    shutdown_tx: watch::Sender<bool>,
    task: tokio::task::JoinHandle<Result<(), TaskActorRuntimeError>>,
}

impl TaskActorPersistenceHandle {
    /// Signals graceful shutdown, awaits a final durable CAS checkpoint flush, and
    /// returns any persistence error observed during shutdown.
    pub async fn flush_and_stop(self) -> Result<(), TaskActorRuntimeError> {
        let _ = self.shutdown_tx.send(true);
        self.task
            .await
            .map_err(|err| TaskActorRuntimeError::PersistenceError {
                reason: format!("persistence task join error: {err}"),
            })?
    }
}

/// Session-free `TaskActor` control plane exposed by `marlin-agent-app-server`.
#[derive(Clone, Default)]
pub struct TaskActorControlPlane {
    registry: TaskActorRuntimeRegistry,
    oracle: Option<Arc<dyn GraphEngineOracle + Send + Sync>>,
}

impl fmt::Debug for TaskActorControlPlane {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TaskActorControlPlane")
            .field("registry", &self.registry)
            .field("has_oracle", &self.oracle.is_some())
            .finish()
    }
}

impl TaskActorControlPlane {
    /// Creates a new Session-free control plane backed by a fresh `TaskActorRuntimeRegistry`.
    pub fn new() -> Self {
        Self {
            registry: TaskActorRuntimeRegistry::new(),
            oracle: None,
        }
    }

    /// Mounts an existing `TaskActorRuntimeRegistry` onto this App Server control plane.
    pub fn with_registry(registry: TaskActorRuntimeRegistry) -> Self {
        Self {
            registry,
            oracle: None,
        }
    }

    /// Mounts an authoritative Scheme `GraphEngineOracle` (e.g. `GraphEngineLinkedBridge`)
    /// so dispatch admission, receipt reconciliation, governed context binding, and
    /// Workbench tab projection are cross-checked fail-closed against Scheme semantics.
    pub fn with_oracle(mut self, oracle: Arc<dyn GraphEngineOracle + Send + Sync>) -> Self {
        self.oracle = Some(oracle);
        self
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

    /// Admits a Scheme-proposed `MarlinDispatchEnvelope` fail-closed (cross-checking
    /// against the mounted `GraphEngineOracle` when configured).
    pub fn admit_dispatch(
        &self,
        envelope: MarlinDispatchEnvelope,
    ) -> Result<AdmittedMarlinDispatch, TaskActorRuntimeError> {
        if let Some(oracle) = &self.oracle {
            self.registry
                .admit_dispatch_with_oracle(oracle.as_ref(), envelope)
        } else {
            self.registry.admit_dispatch(envelope)
        }
    }

    /// Marks one anchored C4 steering branch as converged at `join_node_id`.
    pub fn complete_steering_branch(
        &self,
        task_id: &TaskActorId,
    ) -> Result<TaskActorCursor, TaskActorRuntimeError> {
        self.registry.complete_steering_branch(task_id)
    }

    /// Reconciles an execution receipt against the actor cursor with fencing-epoch protection
    /// (cross-checking against the mounted `GraphEngineOracle` when configured).
    pub fn reconcile_receipt(
        &self,
        receipt: &MarlinExecutionReceipt,
    ) -> Result<ReceiptReconciliationDecision, TaskActorRuntimeError> {
        if let Some(oracle) = &self.oracle {
            self.registry
                .reconcile_receipt_with_oracle(oracle.as_ref(), receipt)
        } else {
            self.registry.reconcile_receipt(receipt)
        }
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

    /// Binds a governed `ai-agentic-context` + Design 165 capsule onto a registered actor
    /// (cross-checking against the mounted `GraphEngineOracle` when configured).
    pub fn bind_governed_context(
        &self,
        capsule: GovernedContextCapsule,
    ) -> Result<AdmittedGovernedContext, TaskActorRuntimeError> {
        if let Some(oracle) = &self.oracle {
            self.registry
                .bind_governed_context_with_oracle(oracle.as_ref(), capsule)
        } else {
            self.registry.bind_governed_context(capsule)
        }
    }

    /// Projects canonical pinned-Index Workbench tabs (verifying against the mounted
    /// `GraphEngineOracle` fail-closed when configured).
    pub fn project_workbench_tabs(&self) -> Result<WorkbenchTabProjection, TaskActorRuntimeError> {
        if let Some(oracle) = &self.oracle {
            self.registry
                .project_workbench_tabs_with_oracle(oracle.as_ref())
        } else {
            Ok(self.registry.project_workbench_tabs())
        }
    }

    /// Enters a Scoped Detour (`BlockerActive` or `SideQuestActive`) for a registered actor.
    pub fn enter_detour(
        &self,
        task_id: &TaskActorId,
        mode: DetourMode,
        detour_node_id: DetourNodeId,
    ) -> Result<DetourState, TaskActorRuntimeError> {
        self.registry.enter_detour(task_id, mode, detour_node_id)
    }

    /// Exits an active Scoped Detour via one of the 4 canonical re-anchoring exits.
    pub fn exit_detour(
        &self,
        task_id: &TaskActorId,
        exit_kind: DetourExitKind,
        sibling_task: Option<(TaskActorId, String)>,
    ) -> Result<DetourState, TaskActorRuntimeError> {
        self.registry.exit_detour(task_id, exit_kind, sibling_task)
    }

    /// Captures a deterministic snapshot of all actors and the Workbench tab strip.
    pub fn snapshot(&self) -> TaskActorRuntimeSnapshot {
        self.registry.snapshot()
    }

    /// Subscribes to live `WorkbenchTabProjection` updates.
    pub fn subscribe_workbench_tabs(&self) -> watch::Receiver<WorkbenchTabProjection> {
        self.registry.subscribe_workbench_tabs()
    }

    /// Atomically persists all TaskActor cursors, governed contexts, and detour states to `path`.
    pub fn save_checkpoint_file(
        &self,
        path: &std::path::Path,
    ) -> Result<TaskActorRuntimeSnapshot, TaskActorRuntimeError> {
        self.registry.save_checkpoint_file(path)
    }

    /// Restores a Session-free `TaskActorControlPlane` from a durable checkpoint file.
    pub fn load_checkpoint_file(path: &std::path::Path) -> Result<Self, TaskActorRuntimeError> {
        Ok(Self::with_registry(
            TaskActorRuntimeRegistry::load_checkpoint_file(path)?,
        ))
    }

    /// Executes an admitted dispatch against an async runtime executor and reconciles its receipt
    /// (honoring the mounted `GraphEngineOracle` fail-closed when configured).
    pub async fn execute_and_reconcile<F, Fut>(
        &self,
        envelope: MarlinDispatchEnvelope,
        executor: F,
    ) -> Result<(AdmittedMarlinDispatch, ReceiptReconciliationDecision), TaskActorRuntimeError>
    where
        F: FnOnce(AdmittedMarlinDispatch) -> Fut,
        Fut: std::future::Future<Output = MarlinExecutionReceipt>,
    {
        let admitted = self.admit_dispatch(envelope)?;
        let receipt = executor(admitted.clone()).await;
        let decision = self.reconcile_receipt(&receipt)?;
        Ok((admitted, decision))
    }

    /// Persists the complete Session-free `TaskActor` snapshot as a CAS artifact in
    /// `marlin-agent-storage` (`TursoAgentStorage` WAL or `InMemoryAgentStorage`) and
    /// atomically advances `pointer_key` via optimistic compare-and-swap.
    pub async fn save_to_agent_storage<S: marlin_agent_storage::AgentStorage>(
        &self,
        storage: &S,
        project_id: &marlin_agent_storage::ProjectId,
        pointer_key: &marlin_agent_storage::ArtifactPointerKey,
        agent_id: &marlin_agent_storage::AgentId,
        event_id: &marlin_agent_storage::EventId,
    ) -> Result<marlin_agent_storage::ArtifactHash, TaskActorRuntimeError> {
        let (body, digest_str) = self.registry.snapshot_json_with_sha256()?;
        let artifact_hash = marlin_agent_storage::ArtifactHash::new(digest_str).map_err(|err| {
            TaskActorRuntimeError::PersistenceError {
                reason: format!("invalid artifact hash: {err}"),
            }
        })?;
        let control_session = marlin_agent_storage::SessionId::new("graph-engine-control-plane")
            .map_err(|err| TaskActorRuntimeError::PersistenceError {
                reason: format!("invalid control session id: {err}"),
            })?;
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        storage
            .put_artifact(marlin_agent_storage::ArtifactRecord {
                project_id: project_id.clone(),
                artifact_hash: artifact_hash.clone(),
                artifact_kind: "graph-engine-task-actor-snapshot".to_owned(),
                producer_session_id: control_session.clone(),
                producer_agent_id: agent_id.clone(),
                producer_event_id: event_id.clone(),
                media_type: "application/json".to_owned(),
                size_bytes: body.len() as u64,
                body,
                created_at_unix_ms: now_ms,
            })
            .await
            .map_err(|err| TaskActorRuntimeError::PersistenceError {
                reason: format!("put_artifact failed: {err}"),
            })?;
        let expected_artifact_hash = storage
            .get_artifact_pointer(project_id, pointer_key)
            .await
            .map_err(|err| TaskActorRuntimeError::PersistenceError {
                reason: format!("get_artifact_pointer failed: {err}"),
            })?
            .map(|ptr| ptr.target_artifact_hash);
        storage
            .compare_and_swap_artifact_pointer(marlin_agent_storage::ArtifactPointerUpdate {
                project_id: project_id.clone(),
                pointer_key: pointer_key.clone(),
                expected_artifact_hash,
                new_artifact_hash: artifact_hash.clone(),
                updated_by_session_id: control_session,
                updated_by_agent_id: agent_id.clone(),
                updated_by_event_id: event_id.clone(),
                updated_at_unix_ms: now_ms,
            })
            .await
            .map_err(|err| TaskActorRuntimeError::PersistenceError {
                reason: format!("compare_and_swap_artifact_pointer failed: {err}"),
            })?;
        Ok(artifact_hash)
    }

    /// Loads and restores a Session-free `TaskActorControlPlane` from a CAS pointer in
    /// `marlin-agent-storage` (`TursoAgentStorage` WAL or `InMemoryAgentStorage`).
    pub async fn load_from_agent_storage<S: marlin_agent_storage::AgentStorage>(
        storage: &S,
        project_id: &marlin_agent_storage::ProjectId,
        pointer_key: &marlin_agent_storage::ArtifactPointerKey,
    ) -> Result<Option<Self>, TaskActorRuntimeError> {
        let Some(pointer) = storage
            .get_artifact_pointer(project_id, pointer_key)
            .await
            .map_err(|err| TaskActorRuntimeError::PersistenceError {
                reason: format!("get_artifact_pointer failed: {err}"),
            })?
        else {
            return Ok(None);
        };
        let artifact = storage
            .get_artifact(project_id, &pointer.target_artifact_hash)
            .await
            .map_err(|err| TaskActorRuntimeError::PersistenceError {
                reason: format!("get_artifact failed: {err}"),
            })?
            .ok_or_else(|| TaskActorRuntimeError::PersistenceError {
                reason: format!(
                    "dangling task-actor pointer: missing artifact {}",
                    pointer.target_artifact_hash.as_str()
                ),
            })?;
        let registry = TaskActorRuntimeRegistry::from_snapshot_json_bytes(&artifact.body)?;
        Ok(Some(Self::with_registry(registry)))
    }

    /// Rehydrates a `TaskActorControlPlane` from `storage` if a checkpoint pointer already
    /// exists, or creates a fresh empty control plane when booting on an empty database.
    pub async fn rehydrate_or_new<S: marlin_agent_storage::AgentStorage>(
        storage: &S,
        project_id: &marlin_agent_storage::ProjectId,
        pointer_key: &marlin_agent_storage::ArtifactPointerKey,
    ) -> Result<Self, TaskActorRuntimeError> {
        match Self::load_from_agent_storage(storage, project_id, pointer_key).await? {
            Some(plane) => Ok(plane),
            None => Ok(Self::new()),
        }
    }

    /// Spawns a background write-through loop that persists every live mutation on this
    /// `TaskActorControlPlane` to `storage` (e.g. `TursoAgentStorage` WAL) and performs
    /// a guaranteed final flush when `TaskActorPersistenceHandle::flush_and_stop` is called.
    pub fn spawn_storage_persistence_loop<S>(
        &self,
        storage: S,
        project_id: marlin_agent_storage::ProjectId,
        pointer_key: marlin_agent_storage::ArtifactPointerKey,
        agent_id: marlin_agent_storage::AgentId,
    ) -> TaskActorPersistenceHandle
    where
        S: marlin_agent_storage::AgentStorage + Send + Sync + 'static,
    {
        let plane = self.clone();
        let mut tabs_rx = plane.subscribe_workbench_tabs();
        let (shutdown_tx, mut shutdown_rx) = watch::channel(false);
        let seq = AtomicU64::new(1);

        let task = tokio::spawn(async move {
            let persist = |seq_num: u64| {
                let plane = plane.clone();
                let storage = &storage;
                let project_id = &project_id;
                let pointer_key = &pointer_key;
                let agent_id = &agent_id;
                async move {
                    let event_id = marlin_agent_storage::EventId::new(format!(
                        "ev-task-actor-checkpoint-{seq_num}"
                    ))
                    .map_err(|err| {
                        TaskActorRuntimeError::PersistenceError {
                            reason: format!("invalid checkpoint event id: {err}"),
                        }
                    })?;
                    plane
                        .save_to_agent_storage(
                            storage,
                            project_id,
                            pointer_key,
                            agent_id,
                            &event_id,
                        )
                        .await
                        .map(|_| ())
                }
            };

            loop {
                tokio::select! {
                    changed = tabs_rx.changed() => {
                        match changed {
                            Ok(()) => {
                                let n = seq.fetch_add(1, Ordering::Relaxed);
                                persist(n).await?;
                            }
                            Err(_) => break,
                        }
                    }
                    _ = shutdown_rx.changed() => {
                        if *shutdown_rx.borrow() {
                            break;
                        }
                    }
                }
            }

            let final_n = seq.fetch_add(1, Ordering::Relaxed);
            persist(final_n).await
        });

        TaskActorPersistenceHandle { shutdown_tx, task }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use marlin_agent_runtime::{
        DispatchRequestId, ExecutionReceiptStatus, FencingEpoch, GraphConvergenceNodeId,
        GraphDispatchOperation, GraphSubstrateTarget, ReceiptReconciliationReason, TaskActorStatus,
        WorkbenchTabKind,
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
            crate::ui_grpc::MyGoBridgeHost::new(ReleaseTrust::new(BTreeMap::new()), |_| Ok(()))
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
}
