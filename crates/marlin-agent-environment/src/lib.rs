//! Runtime environment resolution for custom homes, config layers, and sub-agents.

#![forbid(unsafe_code)]

mod activation;
mod import;
mod refresh;
mod resolver;
mod state_storage;
mod working_copy;

pub use activation::{
    DirenvCommandRunner, ProcessDirenvCommandRunner, RuntimeEnvironmentActivationError,
    RuntimeEnvironmentActivationRequest, RuntimeEnvironmentActivationResult,
    RuntimeEnvironmentActivator,
};
pub use import::{
    RuntimeWorkspaceProjectImportRequest, RuntimeWorkspaceProjectImportResult,
    RuntimeWorkspaceProjectImporter,
};
pub use refresh::{
    RuntimeEnvironmentRefreshRequest, RuntimeEnvironmentRefreshResult, RuntimeEnvironmentRefresher,
};
pub use resolver::{
    HOST_HOME_ENV_VAR, PROJECT_CONFIG_PRECEDENCE, RuntimeEnvironmentError,
    RuntimeEnvironmentRequest, RuntimeEnvironmentResolver, SESSION_FLAGS_CONFIG_PRECEDENCE,
    SUB_AGENT_CONFIG_PRECEDENCE, SYSTEM_CONFIG_PRECEDENCE, SubAgentEnvironmentRequest,
    USER_CONFIG_PRECEDENCE,
};
pub use state_storage::RuntimeStateStorageInitializer;
pub use working_copy::{
    ProcessWorkingCopyCommandRunner, ProcessWorkingCopyGitRepositoryResolver,
    WorkingCopyCommandOutput, WorkingCopyCommandRunner, WorkingCopyGitRepositoryResolver,
    WorkingCopyIsolationDriver, WorkingCopyIsolationDriverError, WorkingCopyIsolationResult,
    WorkingCopyParallelIsolationResult, WorkingCopyProviderExecutableProbe,
    WorkingCopyProviderExecutableStatus,
};

pub use marlin_workspace_protocol::{
    WorkingCopyActiveBinding, WorkingCopyBaseRef, WorkingCopyBranchName,
    WorkingCopyCommandInvocation, WorkingCopyCommandProgram, WorkingCopyCommandProjection,
    WorkingCopyCommandProjectionError, WorkingCopyCommandReceipt, WorkingCopyCommandStatus,
    WorkingCopyCreateRequest, WorkingCopyFanoutBenchmarkReceipt, WorkingCopyFinalizeBranchRequest,
    WorkingCopyGitTopLevel, WorkingCopyHandle, WorkingCopyId, WorkingCopyIsolationOperationKind,
    WorkingCopyIsolationPlan, WorkingCopyIsolationPlanError, WorkingCopyIsolationPlanStep,
    WorkingCopyIsolationProvider, WorkingCopyIsolationReceipt, WorkingCopyIsolationRequest,
    WorkingCopyIsolationStatus, WorkingCopyListOptions, WorkingCopyListRequest,
    WorkingCopyParallelIsolationReceipt, WorkingCopyPullRequestCheckoutRequest,
    WorkingCopyPullRequestNumber, WorkingCopyRemovalMode, WorkingCopyRemoveRequest,
    WorkingCopyRepositoryDiscoveryPath, WorkingCopyRetentionActionKind,
    WorkingCopyRetentionActionReceipt, WorkingCopyRetentionPolicy,
    WorkingCopyRetentionSweepReceipt, WorkingCopySwitchMode, WorkingCopySwitchRequest,
};
