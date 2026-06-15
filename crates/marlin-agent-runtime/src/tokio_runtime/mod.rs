//! Tokio-backed execution substrate for agent providers, tools, and sub-agents.

mod cleanup;
mod context;
mod events;
mod handle;
mod receipt;
mod task;
mod traits;

pub use context::{RuntimeContext, RuntimeExecutionIdentity};
pub use events::{EventStream, RuntimeEventSink, RuntimeEventStream};
pub use handle::{TokioAgentRuntime, TokioAgentRuntimeBuildRequest, WorkingCopySubAgentFanoutItem};
pub use marlin_agent_environment::{
    WorkingCopyActiveBinding, WorkingCopyBaseRef, WorkingCopyBranchName,
    WorkingCopyCommandInvocation, WorkingCopyCommandProgram, WorkingCopyCommandProjection,
    WorkingCopyCommandReceipt, WorkingCopyCommandStatus, WorkingCopyCreateRequest,
    WorkingCopyFanoutBenchmarkReceipt, WorkingCopyFinalizeBranchRequest, WorkingCopyGitTopLevel,
    WorkingCopyHandle, WorkingCopyId, WorkingCopyIsolationOperationKind, WorkingCopyIsolationPlan,
    WorkingCopyIsolationPlanStep, WorkingCopyIsolationProvider, WorkingCopyIsolationReceipt,
    WorkingCopyIsolationRequest, WorkingCopyIsolationStatus, WorkingCopyParallelIsolationReceipt,
    WorkingCopyPullRequestCheckoutRequest, WorkingCopyPullRequestNumber,
    WorkingCopyRepositoryDiscoveryPath, WorkingCopyRetentionPolicy,
    WorkingCopyRetentionSweepReceipt,
};
pub use marlin_agent_protocol::{
    AgentEvent as RuntimeEvent, GraphId, RunId, RuntimeEnvironment, SubAgentConfigSurface,
    SubAgentContextPolicy, SubAgentContextVisibility, SubAgentPerformanceBudget,
    SubAgentPermissionSet, SubAgentSpawnConfig, SubAgentSpawnPolicy, SubAgentSpawnProfile,
    SubAgentSpawnStrategy,
};
pub use marlin_agent_sessions::{
    AgentSessionContext, ContextExpansionPolicy, ContextNamespace, ContextVisibility,
    RuntimeBlockingBridgePolicy, RuntimeBlockingBridgeStrategy, RuntimeFanoutJoinPolicy,
    RuntimeTaskTrackerPolicy, SessionId, SessionIdError, SessionIdentity, SessionIsolationPolicy,
    SessionIsolationReceipt, SessionKind, SessionRuntimeSnapshot, TokioRuntimeDiagnosticsPolicy,
    TokioRuntimeFlavor, TokioRuntimePolicy, TokioRuntimePolicyReceipt,
};
pub use receipt::{
    RuntimeFanoutOutput, RuntimeFanoutReceipt, RuntimeFanoutResult, RuntimeFanoutTaskReceipt,
    RuntimeFanoutTaskStatus, RuntimeTaskShutdownReceipt, RuntimeTaskShutdownReceiptInput,
    RuntimeTaskShutdownRequest, RuntimeTaskShutdownStatus, RuntimeTaskTrackerShutdownState,
    SubAgentSpawnReceipt,
};
pub use task::{RuntimeTask, RuntimeTaskOutcome, RuntimeTaskTracker};
pub use tokio_util::sync::CancellationToken;
pub use traits::{HookRuntime, ProviderRuntime, RuntimeFuture, SubAgentRuntime, ToolRuntime};
