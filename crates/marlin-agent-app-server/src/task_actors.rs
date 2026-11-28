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

use marlin_agent_kernel::{GovernanceAdmissionGate, GovernedSubjectName, ToolIfcContext};
use marlin_agent_runtime::{
    AdmittedGovernedContext, AdmittedMarlinDispatch, DetourExitKind, DetourMode, DetourNodeId,
    DetourState, GovernedContextCapsule, GraphEngineOracle, MarlinDispatchEnvelope,
    MarlinExecutionReceipt, ReceiptReconciliationDecision, TaskActorCursor, TaskActorId,
    TaskActorRuntimeError, TaskActorRuntimeRegistry, TaskActorRuntimeSnapshot, TaskActorScopeMode,
    WorkbenchTabProjection,
};
use tokio::sync::watch;

static PERSIST_LOOP_BOOT_SEQ: AtomicU64 = AtomicU64::new(1);

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
    governance_gate: Option<(Arc<GovernanceAdmissionGate>, GovernedSubjectName)>,
}

impl fmt::Debug for TaskActorControlPlane {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TaskActorControlPlane")
            .field("registry", &self.registry)
            .field("has_oracle", &self.oracle.is_some())
            .field("has_governance_gate", &self.governance_gate.is_some())
            .finish()
    }
}

impl TaskActorControlPlane {
    /// Creates a new Session-free control plane backed by a fresh `TaskActorRuntimeRegistry`.
    pub fn new() -> Self {
        Self {
            registry: TaskActorRuntimeRegistry::new(),
            oracle: None,
            governance_gate: None,
        }
    }

    /// Mounts an existing `TaskActorRuntimeRegistry` onto this App Server control plane.
    pub fn with_registry(registry: TaskActorRuntimeRegistry) -> Self {
        Self {
            registry,
            oracle: None,
            governance_gate: None,
        }
    }

    /// Mounts an authoritative Scheme `GraphEngineOracle` (e.g. `GraphEngineLinkedBridge`)
    /// so dispatch admission, receipt reconciliation, governed context binding, and
    /// Workbench tab projection are cross-checked fail-closed against Scheme semantics.
    pub fn with_oracle(mut self, oracle: Arc<dyn GraphEngineOracle + Send + Sync>) -> Self {
        self.oracle = Some(oracle);
        self
    }

    /// Mounts the shared authoritative `GovernanceAdmissionGate` onto this GraphEngine
    /// control plane so E-Stop, Cedar `forbid`, and ASCENT fences block dispatch admission
    /// fail-closed (SM25).
    pub fn with_governance_gate(
        mut self,
        gate: Arc<GovernanceAdmissionGate>,
        actor: GovernedSubjectName,
    ) -> Self {
        self.governance_gate = Some((gate, actor));
        self
    }

    /// Borrows the mounted `GovernanceAdmissionGate`, if configured.
    pub fn governance_gate(&self) -> Option<&Arc<GovernanceAdmissionGate>> {
        self.governance_gate.as_ref().map(|(g, _)| g)
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

    /// Admits a Scheme-proposed `MarlinDispatchEnvelope` fail-closed (enforcing the
    /// mounted `GovernanceAdmissionGate` and `GraphEngineOracle` when configured).
    pub fn admit_dispatch(
        &self,
        envelope: MarlinDispatchEnvelope,
    ) -> Result<AdmittedMarlinDispatch, TaskActorRuntimeError> {
        if let Some((gate, actor)) = &self.governance_gate {
            let tool =
                GovernedSubjectName::new(format!("graph_dispatch:{:?}", envelope.operation))
                    .unwrap_or_else(|_| GovernedSubjectName::new("graph_dispatch").unwrap());
            let resource = GovernedSubjectName::new(envelope.task_id.as_str())
                .unwrap_or_else(|_| GovernedSubjectName::new("task_actor").unwrap());
            gate.authorize_side_effect_on_resource(
                None,
                actor,
                &tool,
                &resource,
                &ToolIfcContext::clean_trusted(),
            )
            .map_err(|err| TaskActorRuntimeError::GovernanceDenied {
                reason: err.to_string(),
            })?;
        }
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
        let pid = std::process::id();
        let boot_ns = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let boot_seq = PERSIST_LOOP_BOOT_SEQ.fetch_add(1, Ordering::Relaxed);

        let task = tokio::spawn(async move {
            let persist = |seq_num: u64| {
                let plane = plane.clone();
                let storage = &storage;
                let project_id = &project_id;
                let pointer_key = &pointer_key;
                let agent_id = &agent_id;
                async move {
                    let event_id = marlin_agent_storage::EventId::new(format!(
                        "ev-cp-{pid:x}-{boot_ns:x}-{boot_seq:x}-{seq_num:x}"
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
