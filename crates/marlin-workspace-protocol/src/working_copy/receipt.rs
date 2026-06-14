//! Receipts for working-copy provider command execution.

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
