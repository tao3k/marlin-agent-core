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
        let (scope_mode, is_terminal, in_detour) =
            {
                let record = guard.actors.get(task_id).ok_or_else(|| {
                    TaskActorRuntimeError::ActorNotFound {
                        task_id: task_id.clone(),
                    }
                })?;
                (
                    record.cursor.scope_mode,
                    record.cursor.status.is_terminal(),
                    record.detour_state != DetourState::Mainline,
                )
            };
        if is_terminal {
            let status = guard.actors[task_id].cursor.status;
            return Err(TaskActorRuntimeError::ActorTerminal {
                task_id: task_id.clone(),
                status,
            });
        }
        if !in_detour {
            return Err(TaskActorRuntimeError::InvalidDetourTransition {
                task_id: task_id.clone(),
                reason: "actor is not inside an active scoped detour",
            });
        }

        if exit_kind == DetourExitKind::SpawnSiblingTask {
            let Some((sibling_id, sibling_title)) = sibling_task else {
                return Err(TaskActorRuntimeError::InvalidDetourTransition {
                    task_id: task_id.clone(),
                    reason: "SpawnSiblingTask exit requires a sibling (task_id, title)",
                });
            };
            if guard.actors.contains_key(&sibling_id) {
                return Err(TaskActorRuntimeError::DuplicateActor {
                    task_id: sibling_id,
                });
            }
            let title = if sibling_title.trim().is_empty() {
                sibling_id.as_str().to_owned()
            } else {
                sibling_title
            };
            guard.read_only_peeks.remove(&sibling_id);
            guard.actors.insert(
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
}
