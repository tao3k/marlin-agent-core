// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Session-free `TaskActor` runtime registry, fail-closed dispatch admission,
//! anti-split-brain fencing-epoch receipt reconciliation, and reactive
//! pinned-Index Workbench tab projection.

use std::collections::{BTreeMap, BTreeSet};
use std::future::Future;
use std::sync::Arc;

use marlin_agent_graph::{
    AdmittedGovernedContext, AdmittedMarlinDispatch, DetourExitKind, DetourMode, DetourNodeId,
    DetourState, FencingEpoch, GovernedContextCapsule, GovernedContextRejection,
    GraphDispatchOperation, GraphDispatchRejection, MarlinDispatchEnvelope, MarlinExecutionReceipt,
    ReceiptReconciliationDecision, ReceiptReconciliationReason, TaskActorCursor, TaskActorId,
    TaskActorScopeMode, TaskActorStatus, WorkbenchActiveTaskInput, WorkbenchTabProjection,
    admit_governed_context_capsule, admit_marlin_dispatch, project_workbench_tabs,
    reconcile_marlin_execution_receipt,
};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::watch;

/// Point-in-time record owned by `TaskActorRuntimeRegistry` for one Session-free actor.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TaskActorRuntimeRecord {
    /// Human-readable display title projected to Workbench tabs.
    pub title: String,
    /// Current anti-split-brain cursor (`task_id`, `fencing_epoch`, `status`, `scope_mode`, `pending_steering_branches`).
    pub cursor: TaskActorCursor,
    /// Latest admitted governed context capsule (`ai-agentic-context` + Design 165), if bound.
    pub governed_context: Option<AdmittedGovernedContext>,
    /// Current Scoped Detour state (`Mainline`, `BlockerActive`, or `SideQuestActive`).
    pub detour_state: DetourState,
}

/// Snapshot of all Session-free `TaskActor` cursors and the projected Workbench tab strip.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TaskActorRuntimeSnapshot {
    /// All known actors keyed deterministically by `TaskActorId`.
    pub actors: BTreeMap<TaskActorId, TaskActorRuntimeRecord>,
    /// Canonical pinned-Index Workbench tab projection.
    pub workbench_tabs: WorkbenchTabProjection,
}

/// Typed fail-closed error returned by `TaskActorRuntimeRegistry`.
#[derive(Clone, Debug, Eq, PartialEq, Error)]
pub enum TaskActorRuntimeError {
    /// An actor with the same `TaskActorId` is already registered.
    #[error("task actor already exists: {task_id}")]
    DuplicateActor { task_id: TaskActorId },
    /// No actor with the given `TaskActorId` is registered.
    #[error("task actor not found: {task_id}")]
    ActorNotFound { task_id: TaskActorId },
    /// Actor has already entered an immutable terminal state.
    #[error("task actor {task_id} is terminal ({status:?})")]
    ActorTerminal {
        task_id: TaskActorId,
        status: TaskActorStatus,
    },
    /// Dispatch carried a stale or unissued fencing epoch.
    #[error("fencing epoch mismatch for {task_id}: expected {expected}, received {received}")]
    FencingEpochMismatch {
        task_id: TaskActorId,
        expected: u64,
        received: u64,
    },
    /// Dispatch attempted to cross isolation scope boundaries.
    #[error("scope mode mismatch for {task_id}: expected {expected:?}, received {received:?}")]
    ScopeModeMismatch {
        task_id: TaskActorId,
        expected: TaskActorScopeMode,
        received: TaskActorScopeMode,
    },
    /// Dispatch failed the typed `marlin-agent-graph` fail-closed admission gate.
    #[error("dispatch gate rejected: {0}")]
    DispatchRejected(#[from] GraphDispatchRejection),
    /// Governed context capsule failed fail-closed `ai-agentic-context` admission.
    #[error("governed context rejected: {0}")]
    GovernedContextRejected(#[from] GovernedContextRejection),
    /// Governed context cut regressed below a previously admitted cut.
    #[error("stale context cut for {task_id}: current {current}, received {received}")]
    StaleContextCut {
        task_id: TaskActorId,
        current: u64,
        received: u64,
    },
    /// Attempted to enter an invalid or nested Scoped Detour.
    #[error("invalid detour transition for {task_id}: {reason}")]
    InvalidDetourTransition {
        task_id: TaskActorId,
        reason: &'static str,
    },
    /// Trunk settlement is blocked while a `BlockerActive` Scoped Detour is open.
    #[error("blocker detour {detour_node_id} blocks settlement for {task_id}")]
    BlockerDetourActive {
        task_id: TaskActorId,
        detour_node_id: DetourNodeId,
    },
    /// A read-only peek cannot be opened for an actor that is currently an active writable tab.
    #[error("cannot open read-only peek for active task actor: {task_id}")]
    CannotPeekActiveActor { task_id: TaskActorId },
    /// No pending C4 steering branches remain to complete.
    #[error("no pending steering branches for task actor: {task_id}")]
    NoPendingSteeringBranches { task_id: TaskActorId },
    /// Authoritative Scheme `GraphEngineOracle` failed or disagreed with Rust state.
    #[error("scheme graph-engine oracle rejected transition: {reason}")]
    OracleDivergence { reason: String },
    /// Authoritative `GovernanceAdmissionGate` / `ExecutionCircuitBreaker` denied dispatch (SM25).
    #[error("governance admission gate denied task actor dispatch: {reason}")]
    GovernanceDenied { reason: String },
    /// Durable checkpoint serialization or file persistence failed.
    #[error("task actor durable persistence error: {reason}")]
    PersistenceError { reason: String },
}

#[derive(Debug)]
struct RegistryState {
    actors: BTreeMap<TaskActorId, TaskActorRuntimeRecord>,
    focused_task_id: Option<TaskActorId>,
    read_only_peeks: BTreeSet<TaskActorId>,
}

impl RegistryState {
    fn new() -> Self {
        Self {
            actors: BTreeMap::new(),
            focused_task_id: None,
            read_only_peeks: BTreeSet::new(),
        }
    }

    fn project_tabs(&self) -> WorkbenchTabProjection {
        let active_tasks: Vec<WorkbenchActiveTaskInput> = self
            .actors
            .values()
            .filter(|record| !record.cursor.status.is_terminal())
            .map(|record| WorkbenchActiveTaskInput {
                task_id: record.cursor.task_id.clone(),
                title: record.title.clone(),
                status: record.cursor.status,
                fencing_epoch: record.cursor.fencing_epoch,
            })
            .collect();
        let peek_ids: Vec<TaskActorId> = self.read_only_peeks.iter().cloned().collect();
        project_workbench_tabs(self.focused_task_id.as_ref(), &active_tasks, &peek_ids)
    }
}

/// Thread-safe, Session-free runtime controller for `GraphEngine` `TaskActor`s.
///
/// Owns monotonic `FencingEpoch` cursors, enforces fail-closed dispatch admission and
/// anti-split-brain receipt reconciliation, and broadcasts canonical `WorkbenchTabProjection`
/// updates (slot 0 pinned `Tasks` Index + active tasks + disjoint read-only peeks).
#[derive(Clone, Debug)]
pub struct TaskActorRuntimeRegistry {
    state: Arc<Mutex<RegistryState>>,
    tabs_tx: Arc<watch::Sender<WorkbenchTabProjection>>,
}

impl Default for TaskActorRuntimeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskActorRuntimeRegistry {
    /// Creates an empty Session-free `TaskActorRuntimeRegistry` with slot 0 pinned to Index.
    pub fn new() -> Self {
        let initial = RegistryState::new();
        let initial_tabs = initial.project_tabs();
        let (tabs_tx, _rx) = watch::channel(initial_tabs);
        Self {
            state: Arc::new(Mutex::new(initial)),
            tabs_tx: Arc::new(tabs_tx),
        }
    }

    /// Subscribes to live `WorkbenchTabProjection` updates.
    pub fn subscribe_workbench_tabs(&self) -> watch::Receiver<WorkbenchTabProjection> {
        self.tabs_tx.subscribe()
    }

    /// Returns the current canonical `WorkbenchTabProjection`.
    pub fn project_workbench_tabs(&self) -> WorkbenchTabProjection {
        self.state.lock().project_tabs()
    }

    /// Captures a deterministic snapshot of all registered actors and Workbench tabs.
    pub fn snapshot(&self) -> TaskActorRuntimeSnapshot {
        let guard = self.state.lock();
        TaskActorRuntimeSnapshot {
            actors: guard.actors.clone(),
            workbench_tabs: guard.project_tabs(),
        }
    }

    /// Reads the current `TaskActorCursor` for `task_id`, if registered.
    pub fn cursor(&self, task_id: &TaskActorId) -> Option<TaskActorCursor> {
        self.state
            .lock()
            .actors
            .get(task_id)
            .map(|record| record.cursor.clone())
    }

    /// Reads the currently bound `AdmittedGovernedContext` for `task_id`, if any.
    pub fn governed_context(&self, task_id: &TaskActorId) -> Option<AdmittedGovernedContext> {
        self.state
            .lock()
            .actors
            .get(task_id)
            .and_then(|record| record.governed_context.clone())
    }

    /// Reads the current Design 165 `DetourState` for `task_id`, if registered.
    pub fn detour_state(&self, task_id: &TaskActorId) -> Option<DetourState> {
        self.state
            .lock()
            .actors
            .get(task_id)
            .map(|record| record.detour_state.clone())
    }

    /// Registers a new Session-free `TaskActor` at `FencingEpoch(1)` in `OpenFocus`.
    pub fn register_actor(
        &self,
        task_id: TaskActorId,
        title: impl Into<String>,
        scope_mode: TaskActorScopeMode,
    ) -> Result<TaskActorCursor, TaskActorRuntimeError> {
        let mut guard = self.state.lock();
        if guard.actors.contains_key(&task_id) {
            return Err(TaskActorRuntimeError::DuplicateActor { task_id });
        }
        let initial_epoch = FencingEpoch::new(1).expect("epoch 1 is strictly positive");
        let cursor = TaskActorCursor {
            task_id: task_id.clone(),
            fencing_epoch: initial_epoch,
            status: TaskActorStatus::OpenFocus,
            scope_mode,
            pending_steering_branches: 0,
        };
        let title_raw = title.into();
        let title = if title_raw.trim().is_empty() {
            task_id.as_str().to_owned()
        } else {
            title_raw
        };
        guard.read_only_peeks.remove(&task_id);
        guard.actors.insert(
            task_id,
            TaskActorRuntimeRecord {
                title,
                cursor: cursor.clone(),
                governed_context: None,
                detour_state: DetourState::Mainline,
            },
        );
        let tabs = guard.project_tabs();
        drop(guard);
        let _ = self.tabs_tx.send(tabs);
        Ok(cursor)
    }

    /// Admits a Scheme-proposed `MarlinDispatchEnvelope` against the actor's live cursor fail-closed.
    pub fn admit_dispatch(
        &self,
        envelope: MarlinDispatchEnvelope,
    ) -> Result<AdmittedMarlinDispatch, TaskActorRuntimeError> {
        let mut guard = self.state.lock();
        let record = guard.actors.get_mut(&envelope.task_id).ok_or_else(|| {
            TaskActorRuntimeError::ActorNotFound {
                task_id: envelope.task_id.clone(),
            }
        })?;
        if record.cursor.status.is_terminal() {
            return Err(TaskActorRuntimeError::ActorTerminal {
                task_id: envelope.task_id.clone(),
                status: record.cursor.status,
            });
        }
        if envelope.fencing_epoch != record.cursor.fencing_epoch {
            return Err(TaskActorRuntimeError::FencingEpochMismatch {
                task_id: envelope.task_id.clone(),
                expected: record.cursor.fencing_epoch.value(),
                received: envelope.fencing_epoch.value(),
            });
        }
        if envelope.scope_mode != record.cursor.scope_mode {
            return Err(TaskActorRuntimeError::ScopeModeMismatch {
                task_id: envelope.task_id.clone(),
                expected: record.cursor.scope_mode,
                received: envelope.scope_mode,
            });
        }
        if envelope.operation == GraphDispatchOperation::CommitSettlement
            && let DetourState::Active {
                mode: DetourMode::BlockerActive,
                detour_node_id,
                ..
            } = &record.detour_state
        {
            return Err(TaskActorRuntimeError::BlockerDetourActive {
                task_id: envelope.task_id.clone(),
                detour_node_id: detour_node_id.clone(),
            });
        }

        let admitted = admit_marlin_dispatch(envelope)?;
        match admitted.envelope().operation {
            GraphDispatchOperation::ApplySteering => {
                record.cursor.pending_steering_branches =
                    record.cursor.pending_steering_branches.saturating_add(1);
                record.cursor.status = TaskActorStatus::WaitingSteering;
            }
            GraphDispatchOperation::ExecuteFrontier
            | GraphDispatchOperation::ConvergeApex
            | GraphDispatchOperation::CommitSettlement => {
                if record.cursor.status == TaskActorStatus::OpenFocus {
                    record.cursor.status = TaskActorStatus::Running;
                }
            }
        }
        let tabs = guard.project_tabs();
        drop(guard);
        let _ = self.tabs_tx.send(tabs);
        Ok(admitted)
    }

    /// Records convergence of one anchored C4 steering branch at `join_node_id`.
    pub fn complete_steering_branch(
        &self,
        task_id: &TaskActorId,
    ) -> Result<TaskActorCursor, TaskActorRuntimeError> {
        let mut guard = self.state.lock();
        let record =
            guard
                .actors
                .get_mut(task_id)
                .ok_or_else(|| TaskActorRuntimeError::ActorNotFound {
                    task_id: task_id.clone(),
                })?;
        if record.cursor.status.is_terminal() {
            return Err(TaskActorRuntimeError::ActorTerminal {
                task_id: task_id.clone(),
                status: record.cursor.status,
            });
        }
        if record.cursor.pending_steering_branches == 0 {
            return Err(TaskActorRuntimeError::NoPendingSteeringBranches {
                task_id: task_id.clone(),
            });
        }
        record.cursor.pending_steering_branches -= 1;
        if record.cursor.pending_steering_branches == 0
            && record.cursor.status == TaskActorStatus::WaitingSteering
        {
            record.cursor.status = TaskActorStatus::Running;
        }
        let cursor = record.cursor.clone();
        let tabs = guard.project_tabs();
        drop(guard);
        let _ = self.tabs_tx.send(tabs);
        Ok(cursor)
    }

    /// Reconciles an execution receipt against the registered actor cursor fail-closed.
    ///
    /// Stale/future/mismatched/post-terminal/premature-settlement receipts return
    /// `Ok(decision)` with `decision.accepted == false` and leave actor state unchanged.
    pub fn reconcile_receipt(
        &self,
        receipt: &MarlinExecutionReceipt,
    ) -> Result<ReceiptReconciliationDecision, TaskActorRuntimeError> {
        let mut guard = self.state.lock();
        let record = guard.actors.get_mut(&receipt.task_id).ok_or_else(|| {
            TaskActorRuntimeError::ActorNotFound {
                task_id: receipt.task_id.clone(),
            }
        })?;
        let decision = reconcile_marlin_execution_receipt(&record.cursor, receipt);
        if decision.accepted {
            record.cursor.fencing_epoch = decision.next_epoch;
            record.cursor.status = decision.next_status;
            // Task-as-Tab Auto-Archive: clear active focus when a task reaches a terminal state.
            if record.cursor.status.is_terminal()
                && guard.focused_task_id.as_ref() == Some(&receipt.task_id)
            {
                guard.focused_task_id = None;
            }
        }
        let tabs = guard.project_tabs();
        drop(guard);
        let _ = self.tabs_tx.send(tabs);
        Ok(decision)
    }

    /// Runs a full fail-closed dispatch -> async native executor -> receipt reconciliation cycle.
    pub async fn execute_admitted_dispatch<F, Fut>(
        &self,
        envelope: MarlinDispatchEnvelope,
        executor: F,
    ) -> Result<(AdmittedMarlinDispatch, ReceiptReconciliationDecision), TaskActorRuntimeError>
    where
        F: FnOnce(AdmittedMarlinDispatch) -> Fut,
        Fut: Future<Output = MarlinExecutionReceipt>,
    {
        let admitted = self.admit_dispatch(envelope)?;
        let receipt = executor(admitted.clone()).await;
        let decision = self.reconcile_receipt(&receipt)?;
        Ok((admitted, decision))
    }

    /// Sets the focused active task tab (or clears focus back to the pinned Index).
    pub fn set_focus(
        &self,
        task_id: Option<TaskActorId>,
    ) -> Result<WorkbenchTabProjection, TaskActorRuntimeError> {
        let mut guard = self.state.lock();
        if let Some(ref target) = task_id {
            let record =
                guard
                    .actors
                    .get(target)
                    .ok_or_else(|| TaskActorRuntimeError::ActorNotFound {
                        task_id: target.clone(),
                    })?;
            if record.cursor.status.is_terminal() {
                return Err(TaskActorRuntimeError::ActorTerminal {
                    task_id: target.clone(),
                    status: record.cursor.status,
                });
            }
        }
        guard.focused_task_id = task_id;
        let tabs = guard.project_tabs();
        drop(guard);
        let _ = self.tabs_tx.send(tabs.clone());
        Ok(tabs)
    }

    /// Opens a read-only peek tab for an archived/settled or external task ID.
    /// Rejects opening a peek for a currently active writable task to keep tab sets disjoint.
    pub fn open_read_only_peek(
        &self,
        task_id: TaskActorId,
    ) -> Result<WorkbenchTabProjection, TaskActorRuntimeError> {
        let mut guard = self.state.lock();
        if let Some(record) = guard.actors.get(&task_id)
            && !record.cursor.status.is_terminal()
        {
            return Err(TaskActorRuntimeError::CannotPeekActiveActor { task_id });
        }
        guard.read_only_peeks.insert(task_id);
        let tabs = guard.project_tabs();
        drop(guard);
        let _ = self.tabs_tx.send(tabs.clone());
        Ok(tabs)
    }

    /// Closes a read-only peek tab if present and broadcasts the updated projection.
    pub fn close_read_only_peek(&self, task_id: &TaskActorId) -> WorkbenchTabProjection {
        let mut guard = self.state.lock();
        guard.read_only_peeks.remove(task_id);
        let tabs = guard.project_tabs();
        drop(guard);
        let _ = self.tabs_tx.send(tabs.clone());
        tabs
    }

    /// Binds a governed `ai-agentic-context` + Design 165 capsule onto a non-terminal actor,
    /// enforcing L3 active-defeater disclosure floor, Lethal-Trifecta sandbox admission, Cedar
    /// handoff validity, blocker auto-archive confinement, and monotonic `context_cut`.
    pub fn bind_governed_context(
        &self,
        capsule: GovernedContextCapsule,
    ) -> Result<AdmittedGovernedContext, TaskActorRuntimeError> {
        let admitted = admit_governed_context_capsule(capsule)?;
        let task_id = admitted.capsule().task_id.clone();
        let mut guard = self.state.lock();
        let record =
            guard
                .actors
                .get_mut(&task_id)
                .ok_or_else(|| TaskActorRuntimeError::ActorNotFound {
                    task_id: task_id.clone(),
                })?;
        if record.cursor.status.is_terminal() {
            return Err(TaskActorRuntimeError::ActorTerminal {
                task_id,
                status: record.cursor.status,
            });
        }
        if let Some(prev) = &record.governed_context {
            let current = prev.capsule().context_cut;
            let received = admitted.capsule().context_cut;
            if received < current {
                return Err(TaskActorRuntimeError::StaleContextCut {
                    task_id,
                    current,
                    received,
                });
            }
        }
        record.governed_context = Some(admitted.clone());
        let tabs = guard.project_tabs();
        drop(guard);
        let _ = self.tabs_tx.send(tabs);
        Ok(admitted)
    }

    /// Enters a Scoped Detour (`BlockerActive` or `SideQuestActive`) from `Mainline`.
    pub fn enter_detour(
        &self,
        task_id: &TaskActorId,
        mode: DetourMode,
        detour_node_id: DetourNodeId,
    ) -> Result<DetourState, TaskActorRuntimeError> {
        if mode == DetourMode::Mainline {
            return Err(TaskActorRuntimeError::InvalidDetourTransition {
                task_id: task_id.clone(),
                reason: "detour mode must be BlockerActive or SideQuestActive",
            });
        }
        let mut guard = self.state.lock();
        let record =
            guard
                .actors
                .get_mut(task_id)
                .ok_or_else(|| TaskActorRuntimeError::ActorNotFound {
                    task_id: task_id.clone(),
                })?;
        if record.cursor.status.is_terminal() {
            return Err(TaskActorRuntimeError::ActorTerminal {
                task_id: task_id.clone(),
                status: record.cursor.status,
            });
        }
        if record.detour_state != DetourState::Mainline {
            return Err(TaskActorRuntimeError::InvalidDetourTransition {
                task_id: task_id.clone(),
                reason: "nested scoped detour is not permitted",
            });
        }
        let next = DetourState::Active {
            mode,
            origin_epoch: record.cursor.fencing_epoch,
            detour_node_id,
        };
        record.detour_state = next.clone();
        let tabs = guard.project_tabs();
        drop(guard);
        let _ = self.tabs_tx.send(tabs);
        Ok(next)
    }

    /// Exits an active Scoped Detour via one of the 4 canonical re-anchoring exits:
    /// `ReturnToOrigin`, `AbsorbAndContinue`, `SpawnSiblingTask`, or `AbortDetour`.
    pub fn exit_detour(
        &self,
        task_id: &TaskActorId,
        exit_kind: DetourExitKind,
        sibling_task: Option<(TaskActorId, String)>,
    ) -> Result<DetourState, TaskActorRuntimeError> {
        let mut guard = self.state.lock();
        let scope_mode = validate_active_detour(&guard, task_id)?;
        if exit_kind == DetourExitKind::SpawnSiblingTask {
            spawn_detour_sibling(&mut guard, task_id, scope_mode, sibling_task)?;
        }
        let record = guard
            .actors
            .get_mut(task_id)
            .expect("actor verified present above");
        if exit_kind == DetourExitKind::AbsorbAndContinue
            && let Some(ctx) = &record.governed_context
        {
            record.governed_context = Some(ctx.with_next_cut());
        }
        record.detour_state = DetourState::Mainline;
        let tabs = guard.project_tabs();
        drop(guard);
        let _ = self.tabs_tx.send(tabs);
        Ok(DetourState::Mainline)
    }

    /// Convenience helper confirming a reconciliation decision rejected due to stale epoch.
    pub const fn is_stale_split_brain_rejection(decision: &ReceiptReconciliationDecision) -> bool {
        !decision.accepted
            && matches!(
                decision.reason,
                ReceiptReconciliationReason::StaleFencingEpoch
            )
    }

    /// Reconstructs a `TaskActorRuntimeRegistry` from a previously persisted durable snapshot,
    /// preserving monotonic `FencingEpoch` cursors, `GovernedContextCapsule` cuts,
    /// `DetourState` (`BlockerActive` / `SideQuestActive`), focus, and read-only peeks.
    pub fn from_snapshot(snapshot: TaskActorRuntimeSnapshot) -> Self {
        let mut read_only_peeks = BTreeSet::new();
        for tab in &snapshot.workbench_tabs.tabs {
            if tab.kind == marlin_agent_graph::WorkbenchTabKind::ReadOnlyPeek
                && let Ok(id) = TaskActorId::new(tab.id.clone())
            {
                read_only_peeks.insert(id);
            }
        }
        let state = RegistryState {
            actors: snapshot.actors,
            focused_task_id: snapshot.workbench_tabs.focused_task_id,
            read_only_peeks,
        };
        let tabs = state.project_tabs();
        let (tabs_tx, _rx) = watch::channel(tabs);
        Self {
            state: Arc::new(Mutex::new(state)),
            tabs_tx: Arc::new(tabs_tx),
        }
    }

    /// Atomically persists the full `TaskActorRuntimeSnapshot` to `path` as JSON
    /// (unique per-writer temp file in parent directory + `sync_all()` + atomic rename
    /// + parent-dir sync) for crash-safe restart recovery (SM21).
    pub fn save_checkpoint_file(
        &self,
        path: &std::path::Path,
    ) -> Result<TaskActorRuntimeSnapshot, TaskActorRuntimeError> {
        use std::io::Write as _;
        use std::sync::atomic::{AtomicU64, Ordering};

        static CHECKPOINT_TEMP_SEQ: AtomicU64 = AtomicU64::new(0);

        let snapshot = self.snapshot();
        let bytes = serde_json::to_vec_pretty(&snapshot).map_err(|err| {
            TaskActorRuntimeError::PersistenceError {
                reason: format!("serialize snapshot: {err}"),
            }
        })?;
        let parent = path
            .parent()
            .filter(|dir| !dir.as_os_str().is_empty())
            .unwrap_or_else(|| std::path::Path::new("."));
        std::fs::create_dir_all(parent).map_err(|err| TaskActorRuntimeError::PersistenceError {
            reason: format!("create checkpoint parent dir: {err}"),
        })?;
        let seq = CHECKPOINT_TEMP_SEQ.fetch_add(1, Ordering::Relaxed);
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("checkpoint");
        let tmp = parent.join(format!(
            ".{file_name}.{}.{seq}.tmp",
            std::process::id()
        ));
        let write_result = (|| -> std::io::Result<()> {
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&tmp)?;
            file.write_all(&bytes)?;
            file.sync_all()?;
            Ok(())
        })();
        if let Err(err) = write_result {
            let _ = std::fs::remove_file(&tmp);
            return Err(TaskActorRuntimeError::PersistenceError {
                reason: format!("write temp checkpoint: {err}"),
            });
        }
        std::fs::rename(&tmp, path).map_err(|err| {
            let _ = std::fs::remove_file(&tmp);
            TaskActorRuntimeError::PersistenceError {
                reason: format!("rename checkpoint atomically: {err}"),
            }
        })?;
        if let Ok(dir) = std::fs::File::open(parent) {
            let _ = dir.sync_all();
        }
        Ok(snapshot)
    }

    /// Serializes the full `TaskActorRuntimeSnapshot` into deterministic JSON bytes
    /// paired with a `sha256:<hex>` content digest suitable for `AgentStorage` CAS artifacts.
    pub fn snapshot_json_with_sha256(&self) -> Result<(Vec<u8>, String), TaskActorRuntimeError> {
        let snapshot = self.snapshot();
        let bytes = serde_json::to_vec(&snapshot).map_err(|err| {
            TaskActorRuntimeError::PersistenceError {
                reason: format!("serialize snapshot json: {err}"),
            }
        })?;
        let digest = ring::digest::digest(&ring::digest::SHA256, &bytes);
        let mut hex = String::with_capacity("sha256:".len() + digest.as_ref().len() * 2);
        hex.push_str("sha256:");
        for byte in digest.as_ref() {
            use std::fmt::Write as _;
            let _ = write!(&mut hex, "{byte:02x}");
        }
        Ok((bytes, hex))
    }

    /// Reconstructs a `TaskActorRuntimeRegistry` from canonical JSON snapshot bytes.
    pub fn from_snapshot_json_bytes(bytes: &[u8]) -> Result<Self, TaskActorRuntimeError> {
        let snapshot: TaskActorRuntimeSnapshot = serde_json::from_slice(bytes).map_err(|err| {
            TaskActorRuntimeError::PersistenceError {
                reason: format!("parse snapshot json bytes: {err}"),
            }
        })?;
        Ok(Self::from_snapshot(snapshot))
    }

    /// Loads and restores a `TaskActorRuntimeRegistry` from a JSON checkpoint file.
    pub fn load_checkpoint_file(path: &std::path::Path) -> Result<Self, TaskActorRuntimeError> {
        let bytes = std::fs::read(path).map_err(|err| TaskActorRuntimeError::PersistenceError {
            reason: format!("read checkpoint file: {err}"),
        })?;
        let snapshot: TaskActorRuntimeSnapshot = serde_json::from_slice(&bytes).map_err(|err| {
            TaskActorRuntimeError::PersistenceError {
                reason: format!("parse checkpoint snapshot: {err}"),
            }
        })?;
        Ok(Self::from_snapshot(snapshot))
    }

    /// Cross-checks `envelope` through `oracle` (Scheme `GraphEngine`) before admitting locally.
    pub fn admit_dispatch_with_oracle<O: marlin_agent_graph::GraphEngineOracle + ?Sized>(
        &self,
        oracle: &O,
        envelope: MarlinDispatchEnvelope,
    ) -> Result<AdmittedMarlinDispatch, TaskActorRuntimeError> {
        match oracle.project_dispatch(&envelope) {
            Ok(Ok(_)) => self.admit_dispatch(envelope),
            Ok(Err(rejection)) => Err(TaskActorRuntimeError::DispatchRejected(rejection)),
            Err(reason) => Err(TaskActorRuntimeError::OracleDivergence { reason }),
        }
    }

    /// Cross-checks `receipt` through `oracle` (Scheme `GraphEngine`) and rejects any divergence.
    pub fn reconcile_receipt_with_oracle<O: marlin_agent_graph::GraphEngineOracle + ?Sized>(
        &self,
        oracle: &O,
        receipt: &MarlinExecutionReceipt,
    ) -> Result<ReceiptReconciliationDecision, TaskActorRuntimeError> {
        let cursor =
            self.cursor(&receipt.task_id)
                .ok_or_else(|| TaskActorRuntimeError::ActorNotFound {
                    task_id: receipt.task_id.clone(),
                })?;
        let scheme_decision = oracle
            .reconcile_receipt(&cursor, receipt)
            .map_err(|reason| TaskActorRuntimeError::OracleDivergence { reason })?;
        let runtime_decision = self.reconcile_receipt(receipt)?;
        if scheme_decision != runtime_decision {
            return Err(TaskActorRuntimeError::OracleDivergence {
                reason: "scheme/runtime receipt reconciliation divergence".to_owned(),
            });
        }
        Ok(runtime_decision)
    }

    /// Cross-checks `capsule` through `oracle` (Scheme `GraphEngine`) before binding locally.
    pub fn bind_governed_context_with_oracle<O: marlin_agent_graph::GraphEngineOracle + ?Sized>(
        &self,
        oracle: &O,
        capsule: GovernedContextCapsule,
    ) -> Result<AdmittedGovernedContext, TaskActorRuntimeError> {
        match oracle.project_governed_context(&capsule) {
            Ok(Ok(_)) => self.bind_governed_context(capsule),
            Ok(Err(rejection)) => Err(TaskActorRuntimeError::GovernedContextRejected(rejection)),
            Err(reason) => Err(TaskActorRuntimeError::OracleDivergence { reason }),
        }
    }

    /// Projects the registry's current Workbench tabs through `oracle` and verifies exact parity.
    pub fn project_workbench_tabs_with_oracle<O: marlin_agent_graph::GraphEngineOracle + ?Sized>(
        &self,
        oracle: &O,
    ) -> Result<WorkbenchTabProjection, TaskActorRuntimeError> {
        let snapshot = self.snapshot();
        let active_tasks: Vec<WorkbenchActiveTaskInput> = snapshot
            .actors
            .values()
            .filter(|rec| !rec.cursor.status.is_terminal())
            .map(|rec| WorkbenchActiveTaskInput {
                task_id: rec.cursor.task_id.clone(),
                title: rec.title.clone(),
                status: rec.cursor.status,
                fencing_epoch: rec.cursor.fencing_epoch,
            })
            .collect();
        let peek_ids: Vec<TaskActorId> = snapshot
            .workbench_tabs
            .tabs
            .iter()
            .filter(|tab| tab.kind == marlin_agent_graph::WorkbenchTabKind::ReadOnlyPeek)
            .filter_map(|tab| TaskActorId::new(tab.id.clone()).ok())
            .collect();
        let focused = snapshot.workbench_tabs.focused_task_id.as_ref();
        let scheme_tabs = oracle
            .project_workbench_tabs(focused, &active_tasks, &peek_ids)
            .map_err(|reason| TaskActorRuntimeError::OracleDivergence { reason })?;
        if scheme_tabs != snapshot.workbench_tabs {
            return Err(TaskActorRuntimeError::OracleDivergence {
                reason: "scheme/runtime workbench tab projection divergence".to_owned(),
            });
        }
        Ok(scheme_tabs)
    }

    /// Executes an admitted dispatch envelope asynchronously against a caller-supplied
    /// runtime executor (e.g. FunFlow / Marlin tool / turn boundary), then automatically
    /// reconciles the returned `MarlinExecutionReceipt` against the actor's `FencingEpoch`.
    pub async fn execute_and_reconcile<F, Fut>(
        &self,
        envelope: MarlinDispatchEnvelope,
        executor: F,
    ) -> Result<(AdmittedMarlinDispatch, ReceiptReconciliationDecision), TaskActorRuntimeError>
    where
        F: FnOnce(AdmittedMarlinDispatch) -> Fut,
        Fut: Future<Output = MarlinExecutionReceipt>,
    {
        let admitted = self.admit_dispatch(envelope)?;
        let receipt = executor(admitted.clone()).await;
        let decision = self.reconcile_receipt(&receipt)?;
        Ok((admitted, decision))
    }
}

fn validate_active_detour(
    state: &RegistryState,
    task_id: &TaskActorId,
) -> Result<TaskActorScopeMode, TaskActorRuntimeError> {
    let record = state
        .actors
        .get(task_id)
        .ok_or_else(|| TaskActorRuntimeError::ActorNotFound {
            task_id: task_id.clone(),
        })?;
    if record.cursor.status.is_terminal() {
        return Err(TaskActorRuntimeError::ActorTerminal {
            task_id: task_id.clone(),
            status: record.cursor.status,
        });
    }
    if record.detour_state == DetourState::Mainline {
        return Err(TaskActorRuntimeError::InvalidDetourTransition {
            task_id: task_id.clone(),
            reason: "actor is not inside an active scoped detour",
        });
    }
    Ok(record.cursor.scope_mode)
}

fn spawn_detour_sibling(
    state: &mut RegistryState,
    task_id: &TaskActorId,
    scope_mode: TaskActorScopeMode,
    sibling_task: Option<(TaskActorId, String)>,
) -> Result<(), TaskActorRuntimeError> {
    let Some((sibling_id, sibling_title)) = sibling_task else {
        return Err(TaskActorRuntimeError::InvalidDetourTransition {
            task_id: task_id.clone(),
            reason: "SpawnSiblingTask exit requires a sibling (task_id, title)",
        });
    };
    if state.actors.contains_key(&sibling_id) {
        return Err(TaskActorRuntimeError::DuplicateActor {
            task_id: sibling_id,
        });
    }
    let title = if sibling_title.trim().is_empty() {
        sibling_id.as_str().to_owned()
    } else {
        sibling_title
    };
    state.read_only_peeks.remove(&sibling_id);
    state.actors.insert(
        sibling_id.clone(),
        TaskActorRuntimeRecord {
            title,
            cursor: TaskActorCursor {
                task_id: sibling_id,
                fencing_epoch: FencingEpoch::new(1).expect("epoch 1 is strictly positive"),
                status: TaskActorStatus::OpenFocus,
                scope_mode,
                pending_steering_branches: 0,
            },
            governed_context: None,
            detour_state: DetourState::Mainline,
        },
    );
    Ok(())
}

#[cfg(test)]
mod sm21_atomic_persistence_tests {
    use super::*;
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };

    #[test]
    fn sm21_concurrent_save_checkpoint_file_has_zero_tmp_collisions_and_zero_torn_reads() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = Arc::new(dir.path().join("task-actor-state.json"));
        let registry = Arc::new(TaskActorRuntimeRegistry::new());
        registry
            .register_actor(
                TaskActorId::new("actor-sm21-a").expect("id"),
                "SM21 Initial Actor",
                TaskActorScopeMode::Project,
            )
            .expect("register");
        registry
            .save_checkpoint_file(&path)
            .expect("initial checkpoint save");

        let done = Arc::new(AtomicBool::new(false));
        let reader_path = Arc::clone(&path);
        let reader_done = Arc::clone(&done);
        let reader = std::thread::spawn(move || {
            let mut reads = 0usize;
            while !reader_done.load(Ordering::Acquire) {
                let bytes = std::fs::read(&*reader_path)
                    .expect("SM21 live checkpoint file must always exist");
                let parsed = TaskActorRuntimeRegistry::from_snapshot_json_bytes(&bytes);
                assert!(
                    parsed.is_ok(),
                    "SM21 torn/corrupt checkpoint read ({} bytes): {:?}",
                    bytes.len(),
                    parsed.err()
                );
                reads += 1;
            }
            reads
        });

        let mut writers = Vec::new();
        for w in 0..8usize {
            let p = Arc::clone(&path);
            let reg = Arc::clone(&registry);
            writers.push(std::thread::spawn(move || {
                for iter in 0..30usize {
                    let res = reg.save_checkpoint_file(&p);
                    assert!(
                        res.is_ok(),
                        "SM21 shared-.tmp collision on writer {w} iter {iter}: {:?}",
                        res.err()
                    );
                }
            }));
        }

        for handle in writers {
            handle.join().expect("writer thread panicked");
        }
        done.store(true, Ordering::Release);
        let reads = reader.join().expect("reader thread panicked");
        assert!(reads > 0, "reader must observe live checkpoint reads");

        // Verify zero leftover temp files in directory after quiescence.
        let leftover_tmps: Vec<_> = std::fs::read_dir(dir.path())
            .expect("read_dir")
            .filter_map(Result::ok)
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|name| name.contains(".tmp"))
            .collect();
        assert!(
            leftover_tmps.is_empty(),
            "SM21 orphaned temp files remained: {leftover_tmps:?}"
        );
    }
}
