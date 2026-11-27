// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Typed Session-free `TaskActor`, C4 `Steering`, `fencing-epoch` anti-split-brain
//! dispatch/receipt contracts, and pinned-Index Workbench tab projections.

use std::collections::BTreeSet;
use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::ids::AgentGraphValidationError;

macro_rules! graph_engine_id {
    ($name:ident) => {
        #[doc = concat!("Typed semantic identifier for `", stringify!($name), "`.")]
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(String);

        impl $name {
            /// Builds a non-empty semantic identifier free of control characters.
            pub fn new(value: impl Into<String>) -> Result<Self, AgentGraphValidationError> {
                let value = value.into();
                if value.trim().is_empty() || value.chars().any(char::is_control) {
                    return Err(AgentGraphValidationError::empty_named(stringify!($name)));
                }
                Ok(Self(value))
            }

            /// Returns the identifier as a stable string slice.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl TryFrom<&str> for $name {
            type Error = AgentGraphValidationError;

            fn try_from(value: &str) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(self.as_str())
            }
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.serialize_str(self.as_str())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;
                Self::new(value).map_err(serde::de::Error::custom)
            }
        }
    };
}

graph_engine_id!(TaskActorId);
graph_engine_id!(DispatchRequestId);
graph_engine_id!(GraphConvergenceNodeId);

/// Monotonic anti-split-brain fencing epoch (strictly >= 1).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct FencingEpoch(u64);

impl<'de> Deserialize<'de> for FencingEpoch {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = u64::deserialize(deserializer)?;
        Self::new(raw).map_err(serde::de::Error::custom)
    }
}

impl FencingEpoch {
    /// Creates a non-zero fencing epoch.
    pub fn new(value: u64) -> Result<Self, GraphDispatchRejection> {
        if value == 0 {
            return Err(GraphDispatchRejection::InvalidFencingEpoch);
        }
        Ok(Self(value))
    }

    /// Returns the raw monotonic epoch value.
    pub const fn value(self) -> u64 {
        self.0
    }

    /// Advances the fencing epoch by one step with saturating arithmetic.
    pub const fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }
}

/// Isolation scope owned by a Session-free `TaskActor`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum TaskActorScopeMode {
    /// Bound to a project workspace and permitted to commit project settlements.
    Project,
    /// Detached exploration scope; forbidden from mutating or committing project settlements.
    Detached,
}

/// Lifecycle state of a Session-free `TaskActor`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum TaskActorStatus {
    /// Lightweight focus draft before promotion into a running actor.
    OpenFocus,
    /// Actively executing a FunFlow / GraphEngine frontier.
    Running,
    /// Holding trunk continuation at `join_node_id` until anchored steering branches converge.
    WaitingSteering,
    /// All branches reached terminal settlement cleanly.
    Settled,
    /// Execution failed terminally.
    Failed,
    /// Explicitly aborted terminally.
    Aborted,
}

impl TaskActorStatus {
    /// Returns true once the actor has entered an immutable terminal state.
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Settled | Self::Failed | Self::Aborted)
    }
}

/// Operation dispatched from Scheme `GraphEngine` to Marlin's native runtime.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GraphDispatchOperation {
    /// Advance ready frontier nodes.
    ExecuteFrontier,
    /// Anchor a C4 mid-flight `SteeringMessage` branch into a required `join_node_id`.
    ApplySteering,
    /// Converge subordinate branches into the trunk apex.
    ConvergeApex,
    /// Commit all-terminal settlement into `TaskActorIndex`.
    CommitSettlement,
}

/// Target durable substrate for a dispatched GraphEngine operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GraphSubstrateTarget {
    /// Pure in-memory evaluation (zero durable I/O).
    PureMemory,
    /// Opt-in SQLite/Turso WAL journal boundary.
    SqliteTurso,
}

/// Typed dispatch envelope crossing from Scheme `GraphEngine` into Marlin Rust.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MarlinDispatchEnvelope {
    /// Unique request identifier.
    pub request_id: DispatchRequestId,
    /// Owning Session-free `TaskActor` identifier.
    pub task_id: TaskActorId,
    /// Dispatched GraphEngine operation.
    pub operation: GraphDispatchOperation,
    /// Monotonic fencing epoch issued for this dispatch.
    pub fencing_epoch: FencingEpoch,
    /// Scope isolation mode (`Project` or `Detached`).
    pub scope_mode: TaskActorScopeMode,
    /// Convergence join anchor required by `ApplySteering`.
    pub join_node_id: Option<GraphConvergenceNodeId>,
    /// Execution substrate target.
    pub substrate_target: GraphSubstrateTarget,
}

/// Admitted dispatch envelope that has passed Marlin's fail-closed readiness gate.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AdmittedMarlinDispatch {
    envelope: MarlinDispatchEnvelope,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AdmittedMarlinDispatchWire {
    envelope: MarlinDispatchEnvelope,
}

impl<'de> Deserialize<'de> for AdmittedMarlinDispatch {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = AdmittedMarlinDispatchWire::deserialize(deserializer)?;
        admit_marlin_dispatch(wire.envelope).map_err(serde::de::Error::custom)
    }
}

impl AdmittedMarlinDispatch {
    /// Borrows the admitted dispatch envelope.
    pub fn envelope(&self) -> &MarlinDispatchEnvelope {
        &self.envelope
    }

    /// Consumes the witness and yields the underlying envelope.
    pub fn into_envelope(self) -> MarlinDispatchEnvelope {
        self.envelope
    }
}

/// Fail-closed rejection reasons for `admit_marlin_dispatch`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GraphDispatchRejection {
    /// Fencing epoch must be >= 1.
    InvalidFencingEpoch,
    /// C4 `ApplySteering` requires a concrete `join_node_id` convergence gate.
    SteeringRequiresJoinNode,
    /// A `Detached` scope cannot commit a project-level settlement.
    DetachedScopeCannotCommitProjectSettlement,
}

impl fmt::Display for GraphDispatchRejection {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFencingEpoch => formatter.write_str("invalid-fencing-epoch"),
            Self::SteeringRequiresJoinNode => formatter.write_str("steering-requires-join-node-id"),
            Self::DetachedScopeCannotCommitProjectSettlement => {
                formatter.write_str("detached-scope-cannot-commit-project-settlement")
            }
        }
    }
}

impl std::error::Error for GraphDispatchRejection {}

/// Fail-closed Rust admission gate for Scheme-proposed `MarlinDispatchEnvelope` values.
pub fn admit_marlin_dispatch(
    envelope: MarlinDispatchEnvelope,
) -> Result<AdmittedMarlinDispatch, GraphDispatchRejection> {
    if envelope.fencing_epoch.value() == 0 {
        return Err(GraphDispatchRejection::InvalidFencingEpoch);
    }
    if envelope.operation == GraphDispatchOperation::ApplySteering
        && envelope.join_node_id.is_none()
    {
        return Err(GraphDispatchRejection::SteeringRequiresJoinNode);
    }
    if envelope.scope_mode == TaskActorScopeMode::Detached
        && envelope.operation == GraphDispatchOperation::CommitSettlement
    {
        return Err(GraphDispatchRejection::DetachedScopeCannotCommitProjectSettlement);
    }
    Ok(AdmittedMarlinDispatch { envelope })
}

/// Current actor cursor against which execution receipts are reconciled.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TaskActorCursor {
    /// Owning `TaskActor` identifier.
    pub task_id: TaskActorId,
    /// Current expected monotonic fencing epoch.
    pub fencing_epoch: FencingEpoch,
    /// Current lifecycle state.
    pub status: TaskActorStatus,
    /// Scope isolation mode.
    pub scope_mode: TaskActorScopeMode,
    /// Number of anchored C4 steering branches not yet terminal before `join_node_id`.
    pub pending_steering_branches: u32,
}

/// Terminal outcome reported by the native executor for a dispatched request.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ExecutionReceiptStatus {
    /// Frontier step completed successfully.
    Ok,
    /// Final convergence/settlement completed successfully.
    Settled,
    /// Execution failed.
    Failed,
    /// Executor rejected the request.
    Rejected,
}

/// Typed receipt returned by the Rust runtime to reconcile against `TaskActorCursor`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MarlinExecutionReceipt {
    /// Request identifier echoed from dispatch.
    pub request_id: DispatchRequestId,
    /// Task identifier echoed from dispatch.
    pub task_id: TaskActorId,
    /// Fencing epoch observed during execution.
    pub fencing_epoch: FencingEpoch,
    /// Execution outcome.
    pub status: ExecutionReceiptStatus,
}

/// Typed reconciliation outcome reason.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ReceiptReconciliationReason {
    /// Fresh receipt accepted; actor remains running.
    Accepted,
    /// Fresh receipt accepted; actor remains gated in `WaitingSteering` until branches converge.
    AwaitingSteeringConvergence,
    /// All branches converged and settled terminally.
    Settled,
    /// Execution reported failure; actor transitioned to terminal `Failed`.
    ExecutionFailed,
    /// Receipt targeted a different `TaskActorId`.
    TaskIdMismatch,
    /// Actor is already in a terminal state and rejects further mutations.
    ActorAlreadyTerminal,
    /// Receipt carried a stale `fencing_epoch < actor.fencing_epoch` (anti-split-brain rejection).
    StaleFencingEpoch,
    /// Receipt carried an unissued future `fencing_epoch > actor.fencing_epoch`.
    FutureFencingEpoch,
    /// Settlement attempted while anchored C4 steering branches are still pending.
    CannotSettleWithPendingSteering,
    /// Settlement attempted on a `Detached` exploration scope.
    DetachedScopeCannotSettle,
    /// Actor fencing epoch reached `u64::MAX` and cannot advance monotonically.
    FencingEpochExhausted,
}

/// Deterministic reconciliation decision preserving anti-split-brain and C4 convergence invariants.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReceiptReconciliationDecision {
    /// Whether the receipt was accepted to advance actor state.
    pub accepted: bool,
    /// Next actor fencing epoch (unchanged on rejection; incremented on acceptance).
    pub next_epoch: FencingEpoch,
    /// Next actor lifecycle status (unchanged on rejection).
    pub next_status: TaskActorStatus,
    /// Typed reason explaining acceptance or fail-closed rejection.
    pub reason: ReceiptReconciliationReason,
}

/// Reconciles an execution receipt against the actor cursor fail-closed.
pub fn reconcile_marlin_execution_receipt(
    cursor: &TaskActorCursor,
    receipt: &MarlinExecutionReceipt,
) -> ReceiptReconciliationDecision {
    let reject = |reason: ReceiptReconciliationReason| ReceiptReconciliationDecision {
        accepted: false,
        next_epoch: cursor.fencing_epoch,
        next_status: cursor.status,
        reason,
    };

    if cursor.task_id != receipt.task_id {
        return reject(ReceiptReconciliationReason::TaskIdMismatch);
    }
    if cursor.status.is_terminal() {
        return reject(ReceiptReconciliationReason::ActorAlreadyTerminal);
    }
    if receipt.fencing_epoch < cursor.fencing_epoch {
        return reject(ReceiptReconciliationReason::StaleFencingEpoch);
    }
    if receipt.fencing_epoch > cursor.fencing_epoch {
        return reject(ReceiptReconciliationReason::FutureFencingEpoch);
    }
    let Some(next_epoch) = cursor
        .fencing_epoch
        .value()
        .checked_add(1)
        .and_then(|v| FencingEpoch::new(v).ok())
    else {
        return reject(ReceiptReconciliationReason::FencingEpochExhausted);
    };

    match receipt.status {
        ExecutionReceiptStatus::Ok => {
            if cursor.pending_steering_branches > 0 {
                ReceiptReconciliationDecision {
                    accepted: true,
                    next_epoch,
                    next_status: TaskActorStatus::WaitingSteering,
                    reason: ReceiptReconciliationReason::AwaitingSteeringConvergence,
                }
            } else {
                ReceiptReconciliationDecision {
                    accepted: true,
                    next_epoch,
                    next_status: TaskActorStatus::Running,
                    reason: ReceiptReconciliationReason::Accepted,
                }
            }
        }
        ExecutionReceiptStatus::Settled => {
            if cursor.scope_mode == TaskActorScopeMode::Detached {
                reject(ReceiptReconciliationReason::DetachedScopeCannotSettle)
            } else if cursor.pending_steering_branches > 0 {
                reject(ReceiptReconciliationReason::CannotSettleWithPendingSteering)
            } else {
                ReceiptReconciliationDecision {
                    accepted: true,
                    next_epoch,
                    next_status: TaskActorStatus::Settled,
                    reason: ReceiptReconciliationReason::Settled,
                }
            }
        }
        ExecutionReceiptStatus::Failed | ExecutionReceiptStatus::Rejected => {
            ReceiptReconciliationDecision {
                accepted: true,
                next_epoch,
                next_status: TaskActorStatus::Failed,
                reason: ReceiptReconciliationReason::ExecutionFailed,
            }
        }
    }
}

/// Kind of tab rendered in the Session-free Task Workbench bar.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum WorkbenchTabKind {
    /// Always pinned at slot 0 (`TaskActorIndex`).
    PinnedIndex,
    /// Writable active `TaskActor` tab.
    ActiveTask,
    /// Strictly read-only peek tab for an archived/settled task ID.
    ReadOnlyPeek,
}

/// Active task input used to project Workbench tabs.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkbenchActiveTaskInput {
    /// Active task identifier.
    pub task_id: TaskActorId,
    /// Human-readable tab label.
    pub title: String,
    /// Current lifecycle status.
    pub status: TaskActorStatus,
    /// Current fencing epoch.
    pub fencing_epoch: FencingEpoch,
}

/// Single projected tab entry in the Workbench tab strip.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkbenchTabEntry {
    /// Tab classification.
    pub kind: WorkbenchTabKind,
    /// Stable tab identifier (`"index"` for pinned index, or `TaskActorId`).
    pub id: String,
    /// Display title.
    pub title: String,
    /// Status badge label.
    pub status_label: String,
    /// Whether this tab currently holds focus.
    pub focused: bool,
    /// Observed fencing epoch (0 for index and read-only peeks).
    pub fencing_epoch: u64,
}

/// Complete Session-free Workbench tab projection.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkbenchTabProjection {
    /// Currently focused active task ID, if any.
    pub focused_task_id: Option<TaskActorId>,
    /// Ordered tab entries with `PinnedIndex` guaranteed at index 0.
    pub tabs: Vec<WorkbenchTabEntry>,
}

/// Projects active tasks and read-only peek IDs into a canonical `WorkbenchTabProjection`.
pub fn project_workbench_tabs(
    focused_task_id: Option<&TaskActorId>,
    active_tasks: &[WorkbenchActiveTaskInput],
    peek_task_ids: &[TaskActorId],
) -> WorkbenchTabProjection {
    let mut tabs = Vec::with_capacity(1 + active_tasks.len() + peek_task_ids.len());
    tabs.push(WorkbenchTabEntry {
        kind: WorkbenchTabKind::PinnedIndex,
        id: "index".to_owned(),
        title: "Tasks".to_owned(),
        status_label: "pinned".to_owned(),
        focused: false,
        fencing_epoch: 0,
    });

    let mut active_ids = BTreeSet::new();
    for task in active_tasks {
        if active_ids.insert(task.task_id.as_str().to_owned()) {
            let title = if task.title.trim().is_empty() {
                task.task_id.as_str().to_owned()
            } else {
                task.title.clone()
            };
            tabs.push(WorkbenchTabEntry {
                kind: WorkbenchTabKind::ActiveTask,
                id: task.task_id.as_str().to_owned(),
                title,
                status_label: format!("{:?}", task.status),
                focused: focused_task_id.is_some_and(|focused| focused == &task.task_id),
                fencing_epoch: task.fencing_epoch.value(),
            });
        }
    }

    let mut peek_ids = BTreeSet::new();
    for peek in peek_task_ids {
        let key = peek.as_str();
        if !active_ids.contains(key) && peek_ids.insert(key.to_owned()) {
            tabs.push(WorkbenchTabEntry {
                kind: WorkbenchTabKind::ReadOnlyPeek,
                id: key.to_owned(),
                title: key.to_owned(),
                status_label: "peek".to_owned(),
                focused: false,
                fencing_epoch: 0,
            });
        }
    }

    WorkbenchTabProjection {
        focused_task_id: focused_task_id.cloned(),
        tabs,
    }
}
