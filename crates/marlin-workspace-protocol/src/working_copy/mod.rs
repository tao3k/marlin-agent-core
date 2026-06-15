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
