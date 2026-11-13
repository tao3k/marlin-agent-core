// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Working-copy isolation contracts for agent-owned parallel work.

mod command_projection;
mod ids;
mod plan;
mod provider;
mod receipt;
mod request;

pub use command_projection::{
    WorkingCopyCommandInvocation, WorkingCopyCommandProgram, WorkingCopyCommandProjection,
    WorkingCopyCommandProjectionError,
};
pub use ids::{
    WorkingCopyBaseRef, WorkingCopyBranchName, WorkingCopyGitTopLevel, WorkingCopyId,
    WorkingCopyPullRequestNumber, WorkingCopyRepositoryDiscoveryPath,
    WorkspaceProjectGitHubRepository, WorkspaceProjectId,
};
pub use plan::{
    WorkingCopyIsolationPlan, WorkingCopyIsolationPlanError, WorkingCopyIsolationPlanStep,
};
pub use provider::{WorkingCopyIsolationOperationKind, WorkingCopyIsolationProvider};
pub use receipt::{
    WorkingCopyActiveBinding, WorkingCopyCommandReceipt, WorkingCopyCommandStatus,
    WorkingCopyFanoutBenchmarkReceipt, WorkingCopyIsolationReceipt, WorkingCopyIsolationStatus,
    WorkingCopyParallelIsolationReceipt, WorkingCopyRetentionActionKind,
    WorkingCopyRetentionActionReceipt, WorkingCopyRetentionPolicy,
    WorkingCopyRetentionSweepReceipt,
};
pub use request::{
    WorkingCopyCreateRequest, WorkingCopyFinalizeBranchRequest, WorkingCopyHandle,
    WorkingCopyIsolationRequest, WorkingCopyListOptions, WorkingCopyListRequest,
    WorkingCopyPullRequestCheckoutRequest, WorkingCopyRemovalMode, WorkingCopyRemoveRequest,
    WorkingCopySwitchMode, WorkingCopySwitchRequest,
};
