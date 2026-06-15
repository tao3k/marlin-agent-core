//! Receipts for working-copy provider command execution.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::{
    WorkingCopyCommandInvocation, WorkingCopyCommandProgram, WorkingCopyGitTopLevel,
    WorkingCopyHandle, WorkingCopyIsolationOperationKind, WorkingCopyIsolationProvider,
    WorkingCopyIsolationRequest,
};
use crate::WorkspaceProjectId;

/// Runtime status for one projected provider command.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum WorkingCopyCommandStatus {
    Succeeded,
    Failed,
}

/// Receipt for one structured provider command execution.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkingCopyCommandReceipt {
    pub program: WorkingCopyCommandProgram,
    pub git_toplevel: WorkingCopyGitTopLevel,
    pub args: Vec<String>,
    pub status: WorkingCopyCommandStatus,
    pub status_code: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stdout: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stderr: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub working_copy: Option<WorkingCopyHandle>,
}

impl WorkingCopyCommandReceipt {
    /// Records a command that exited successfully.
    pub fn succeeded(
        invocation: &WorkingCopyCommandInvocation,
        status_code: Option<i32>,
        stdout: impl Into<String>,
        stderr: impl Into<String>,
    ) -> Self {
        Self::from_output(
            invocation,
            WorkingCopyCommandStatus::Succeeded,
            status_code,
            stdout,
            stderr,
        )
    }

    /// Records a command that failed or could not be observed as successful.
    pub fn failed(
        invocation: &WorkingCopyCommandInvocation,
        status_code: Option<i32>,
        stdout: impl Into<String>,
        stderr: impl Into<String>,
    ) -> Self {
        Self::from_output(
            invocation,
            WorkingCopyCommandStatus::Failed,
            status_code,
            stdout,
            stderr,
        )
    }

    fn from_output(
        invocation: &WorkingCopyCommandInvocation,
        status: WorkingCopyCommandStatus,
        status_code: Option<i32>,
        stdout: impl Into<String>,
        stderr: impl Into<String>,
    ) -> Self {
        let stdout = stdout.into();
        let stderr = stderr.into();
        Self {
            program: invocation.program.clone(),
            git_toplevel: invocation.git_toplevel.clone(),
            args: invocation.args.clone(),
            status,
            status_code,
            stdout: (!stdout.is_empty()).then_some(stdout),
            stderr: (!stderr.is_empty()).then_some(stderr),
            working_copy: invocation.expected_working_copy.clone(),
        }
    }
}

/// Status of a working-copy isolation operation.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum WorkingCopyIsolationStatus {
    #[default]
    Planned,
    Applied,
    Rejected,
}

/// Receipt for a provider-owned working-copy isolation operation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkingCopyIsolationReceipt {
    pub project_id: WorkspaceProjectId,
    pub provider: WorkingCopyIsolationProvider,
    pub operation: WorkingCopyIsolationOperationKind,
    pub status: WorkingCopyIsolationStatus,
    pub working_copy: Option<WorkingCopyHandle>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub command_receipts: Vec<WorkingCopyCommandReceipt>,
    pub reason: Option<String>,
}

impl WorkingCopyIsolationReceipt {
    /// Creates a planned receipt from a request.
    pub fn planned(request: &WorkingCopyIsolationRequest) -> Self {
        Self {
            project_id: request.project_id().clone(),
            provider: request.provider().clone(),
            operation: request.operation_kind(),
            status: WorkingCopyIsolationStatus::Planned,
            working_copy: request.working_copy().cloned(),
            command_receipts: Vec::new(),
            reason: None,
        }
    }

    /// Creates an applied receipt from a request.
    pub fn applied(request: &WorkingCopyIsolationRequest) -> Self {
        Self {
            status: WorkingCopyIsolationStatus::Applied,
            ..Self::planned(request)
        }
    }

    /// Creates a rejected receipt from a request.
    pub fn rejected(request: &WorkingCopyIsolationRequest, reason: impl Into<String>) -> Self {
        Self {
            project_id: request.project_id().clone(),
            provider: request.provider().clone(),
            operation: request.operation_kind(),
            status: WorkingCopyIsolationStatus::Rejected,
            working_copy: request.working_copy().cloned(),
            command_receipts: Vec::new(),
            reason: Some(reason.into()),
        }
    }

    /// Attaches provider command receipts produced by runtime adapters.
    pub fn with_command_receipts(
        mut self,
        command_receipts: impl IntoIterator<Item = WorkingCopyCommandReceipt>,
    ) -> Self {
        self.command_receipts = command_receipts.into_iter().collect();
        self
    }
}

/// Active runtime binding between an agent context and one isolated working copy.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkingCopyActiveBinding {
    pub project_id: WorkspaceProjectId,
    pub provider: WorkingCopyIsolationProvider,
    pub working_copy: WorkingCopyHandle,
}

impl WorkingCopyActiveBinding {
    /// Creates an active binding from protocol identity and a working-copy handle.
    pub fn new(
        project_id: impl Into<WorkspaceProjectId>,
        provider: WorkingCopyIsolationProvider,
        working_copy: WorkingCopyHandle,
    ) -> Self {
        Self {
            project_id: project_id.into(),
            provider,
            working_copy,
        }
    }

    /// Projects a successful isolation receipt into a runtime binding.
    pub fn from_receipt(receipt: &WorkingCopyIsolationReceipt) -> Option<Self> {
        if receipt.status != WorkingCopyIsolationStatus::Applied {
            return None;
        }
        Some(Self {
            project_id: receipt.project_id.clone(),
            provider: receipt.provider.clone(),
            working_copy: receipt.working_copy.clone()?,
        })
    }
}

/// Receipt for one agent fanout that isolates multiple working copies in parallel.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkingCopyParallelIsolationReceipt {
    pub project_id: WorkspaceProjectId,
    pub provider: WorkingCopyIsolationProvider,
    pub requested: usize,
    pub planned: usize,
    pub applied: usize,
    pub rejected: usize,
    pub max_parallelism: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub receipts: Vec<WorkingCopyIsolationReceipt>,
}

impl WorkingCopyParallelIsolationReceipt {
    /// Builds an aggregate receipt from per-working-copy isolation receipts.
    pub fn from_receipts(
        project_id: impl Into<WorkspaceProjectId>,
        provider: WorkingCopyIsolationProvider,
        max_parallelism: usize,
        receipts: impl IntoIterator<Item = WorkingCopyIsolationReceipt>,
    ) -> Self {
        let receipts = receipts.into_iter().collect::<Vec<_>>();
        let planned = receipts
            .iter()
            .filter(|receipt| receipt.status == WorkingCopyIsolationStatus::Planned)
            .count();
        let applied = receipts
            .iter()
            .filter(|receipt| receipt.status == WorkingCopyIsolationStatus::Applied)
            .count();
        let rejected = receipts
            .iter()
            .filter(|receipt| receipt.status == WorkingCopyIsolationStatus::Rejected)
            .count();

        Self {
            project_id: project_id.into(),
            provider,
            requested: receipts.len(),
            planned,
            applied,
            rejected,
            max_parallelism,
            receipts,
        }
    }

    /// Returns true when every child receipt belongs to this aggregate scope.
    pub fn has_consistent_scope(&self) -> bool {
        self.receipts.iter().all(|receipt| {
            receipt.project_id == self.project_id && receipt.provider == self.provider
        })
    }

    /// Returns true when all requested working copies were applied in this scope.
    pub fn is_success(&self) -> bool {
        self.requested > 0
            && self.planned == 0
            && self.applied == self.requested
            && self.rejected == 0
            && self.has_consistent_scope()
    }
}

/// Managed retention action taken for a provider-owned working copy.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum WorkingCopyRetentionActionKind {
    Keep,
    Snapshot,
    Remove,
    SnapshotAndRemove,
}

/// Receipt for one retention decision in a managed working-copy sweep.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkingCopyRetentionActionReceipt {
    pub working_copy: WorkingCopyHandle,
    pub action: WorkingCopyRetentionActionKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot_path: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl WorkingCopyRetentionActionReceipt {
    pub fn keep(working_copy: WorkingCopyHandle, reason: impl Into<String>) -> Self {
        Self {
            working_copy,
            action: WorkingCopyRetentionActionKind::Keep,
            snapshot_path: None,
            reason: Some(reason.into()),
        }
    }

    pub fn snapshot(working_copy: WorkingCopyHandle, snapshot_path: impl Into<PathBuf>) -> Self {
        Self {
            working_copy,
            action: WorkingCopyRetentionActionKind::Snapshot,
            snapshot_path: Some(snapshot_path.into()),
            reason: None,
        }
    }

    pub fn remove(working_copy: WorkingCopyHandle, reason: impl Into<String>) -> Self {
        Self {
            working_copy,
            action: WorkingCopyRetentionActionKind::Remove,
            snapshot_path: None,
            reason: Some(reason.into()),
        }
    }

    pub fn snapshot_and_remove(
        working_copy: WorkingCopyHandle,
        snapshot_path: impl Into<PathBuf>,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            working_copy,
            action: WorkingCopyRetentionActionKind::SnapshotAndRemove,
            snapshot_path: Some(snapshot_path.into()),
            reason: Some(reason.into()),
        }
    }
}

/// Receipt for one managed retention sweep over agent-owned working copies.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkingCopyRetentionSweepReceipt {
    pub project_id: WorkspaceProjectId,
    pub provider: WorkingCopyIsolationProvider,
    pub max_retained: usize,
    pub retained: usize,
    pub snapshotted: usize,
    pub removed: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<WorkingCopyRetentionActionReceipt>,
}

impl WorkingCopyRetentionSweepReceipt {
    pub fn from_actions(
        project_id: impl Into<WorkspaceProjectId>,
        provider: WorkingCopyIsolationProvider,
        max_retained: usize,
        actions: impl IntoIterator<Item = WorkingCopyRetentionActionReceipt>,
    ) -> Self {
        let actions = actions.into_iter().collect::<Vec<_>>();
        let retained = actions
            .iter()
            .filter(|receipt| receipt.action == WorkingCopyRetentionActionKind::Keep)
            .count();
        let snapshotted = actions
            .iter()
            .filter(|receipt| {
                matches!(
                    receipt.action,
                    WorkingCopyRetentionActionKind::Snapshot
                        | WorkingCopyRetentionActionKind::SnapshotAndRemove
                )
            })
            .count();
        let removed = actions
            .iter()
            .filter(|receipt| {
                matches!(
                    receipt.action,
                    WorkingCopyRetentionActionKind::Remove
                        | WorkingCopyRetentionActionKind::SnapshotAndRemove
                )
            })
            .count();
        Self {
            project_id: project_id.into(),
            provider,
            max_retained,
            retained,
            snapshotted,
            removed,
            actions,
        }
    }
}

/// Deterministic policy for pruning managed agent working copies.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkingCopyRetentionPolicy {
    pub max_retained: usize,
    pub snapshot_before_remove: bool,
}

impl WorkingCopyRetentionPolicy {
    pub fn new(max_retained: usize) -> Self {
        Self {
            max_retained,
            snapshot_before_remove: true,
        }
    }

    pub fn without_snapshot_before_remove(mut self) -> Self {
        self.snapshot_before_remove = false;
        self
    }

    /// Builds a sweep receipt from working copies ordered newest first.
    pub fn plan_sweep(
        &self,
        project_id: impl Into<WorkspaceProjectId>,
        provider: WorkingCopyIsolationProvider,
        newest_first: impl IntoIterator<Item = WorkingCopyHandle>,
        snapshot_root: impl Into<PathBuf>,
    ) -> WorkingCopyRetentionSweepReceipt {
        let snapshot_root = snapshot_root.into();
        let actions = newest_first
            .into_iter()
            .enumerate()
            .map(|(index, working_copy)| {
                if index < self.max_retained {
                    WorkingCopyRetentionActionReceipt::keep(working_copy, "within retention limit")
                } else if self.snapshot_before_remove {
                    let snapshot_path =
                        snapshot_root.join(format!("{}.patch", working_copy.id.as_str()));
                    WorkingCopyRetentionActionReceipt::snapshot_and_remove(
                        working_copy,
                        snapshot_path,
                        "exceeds retention limit",
                    )
                } else {
                    WorkingCopyRetentionActionReceipt::remove(
                        working_copy,
                        "exceeds retention limit",
                    )
                }
            })
            .collect::<Vec<_>>();
        WorkingCopyRetentionSweepReceipt::from_actions(
            project_id,
            provider,
            self.max_retained,
            actions,
        )
    }
}

/// Lightweight benchmark receipt for agent working-copy fanout.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkingCopyFanoutBenchmarkReceipt {
    pub project_id: WorkspaceProjectId,
    pub provider: WorkingCopyIsolationProvider,
    pub requested: usize,
    pub max_parallelism: usize,
    pub applied: usize,
    pub rejected: usize,
    pub elapsed_micros: u128,
}

impl WorkingCopyFanoutBenchmarkReceipt {
    pub fn from_parallel_receipt(
        receipt: &WorkingCopyParallelIsolationReceipt,
        elapsed_micros: u128,
    ) -> Self {
        Self {
            project_id: receipt.project_id.clone(),
            provider: receipt.provider.clone(),
            requested: receipt.requested,
            max_parallelism: receipt.max_parallelism,
            applied: receipt.applied,
            rejected: receipt.rejected,
            elapsed_micros,
        }
    }
}
