// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Tokio-backed runtime extension traits used by the graph-loop kernel.

pub mod observability;
pub mod plugin;
pub mod process;

mod agent_graph;
mod graph_loop;
mod model_gateway;
mod model_tool_catalog;
mod model_tooling;
mod process_executor;
mod resilience;
mod tokio_runtime;

pub use agent_graph::{
    RuntimeAgentCoordinationAdmissionReceipt, RuntimeAgentCoordinationAdmissionStatus,
    RuntimeAgentCoordinationRejection, RuntimeAgentGraphExecutionReadinessReceipt,
    RuntimeAgentGraphExecutionReadinessRejection, RuntimeAgentGraphExecutionReadinessStatus,
    RuntimeAgentGraphExecutionRequest, RuntimeAgentGraphExecutionRequestRejection,
    RuntimeAgentGraphProjectionReceipt, RuntimeAgentGraphProjectionRejection,
    RuntimeAgentGraphProjectionStatus, RuntimeAgentGraphReadinessInconsistency,
    admit_agent_coordination_plan, build_agent_graph_execution_request,
    check_agent_graph_execution_readiness, project_agent_graph_planning_receipt,
    project_agent_graph_projection_request,
};
pub use graph_loop::{
    AgentFlowLoopStepRequest, GraphLoopRegistryRevision, GraphLoopRunCancelReceipt,
    GraphLoopRunCancelStatus, GraphLoopRunInspectReceipt, GraphLoopRunObservation,
    GraphLoopRunProgressUpdate, GraphLoopRunRegistry, GraphLoopRunRegistryError,
    GraphLoopRunRegistryHandle, GraphLoopRunRegistrySnapshot, GraphLoopRunStartReceipt,
    GraphLoopRunStatus, GraphLoopRunWaitReceipt, GraphLoopRunWaitStatus,
    project_agent_flow_loop_step,
};
pub use model_gateway::{
    ModelGateway, ModelGatewayCompletionChoice, ModelGatewayCompletionOptions,
    ModelGatewayCompletionResponse, ModelGatewayError, ModelGatewayFuture, ModelGatewayMessage,
    ModelGatewayMessageRole, ModelGatewayRequest, ModelGatewayResult, ModelGatewayTransport,
    RuntimeEdgeModelGateway, assistant_gateway_message, system_gateway_message,
    user_gateway_message,
};
pub use model_tool_catalog::{
    ModelToolCatalog, ModelToolCatalogError, ModelToolHandler, ModelToolOffer, ModelToolOutcome,
};
pub use model_tooling::{
    ModelToolCapability, ModelToolDeclaration, ModelToolGovernance, ShellToolDeclarationRequest,
    ShellToolHandler,
};
pub use process::{
    AsyncManagedChildProcess, ManagedChildProcessSpec, ManagedChildStdin,
    RuntimeProcessCleanupController, RuntimeProcessCleanupSweep,
    RuntimeProcessContainmentRequirement, RuntimeProcessExitStatus, RuntimeProcessLiveness,
    RuntimeProcessOutput, RuntimeProcessRegistryHandle, RuntimeProcessTerminator,
    SysinfoRuntimeProcessController,
};
pub use process_executor::ManagedProcessExecutor;
pub use resilience::{
    RuntimeEdgeLayer, RuntimeEdgePolicy, RuntimeEdgePolicyError, RuntimeEdgePolicyReceipt,
    RuntimeEdgeService,
};
pub use tokio_runtime::MAX_RUNTIME_EVENT_HEAP_BYTES;
pub use tokio_runtime::RuntimeProcessBudget;
pub use tokio_runtime::{
    AgentSessionContext, CancellationToken, ContextExpansionPolicy, ContextNamespace,
    ContextVisibility, EventStream, HookRuntime, ProviderRuntime, RuntimeAdmissionError,
    RuntimeBlockingBridgePolicy, RuntimeBlockingBridgeStrategy, RuntimeContext, RuntimeEnvironment,
    RuntimeEvent, RuntimeEventSendError, RuntimeEventSink, RuntimeEventSizeRejection,
    RuntimeEventStream, RuntimeExecutionIdentity, RuntimeFanoutJoinPolicy, RuntimeFanoutOutput,
    RuntimeFanoutReceipt, RuntimeFanoutResult, RuntimeFanoutTaskReceipt, RuntimeFanoutTaskStatus,
    RuntimeFuture, RuntimeOperationId, RuntimeResourceGeneration, RuntimeResourceId, RuntimeTask,
    RuntimeTaskJoinError, RuntimeTaskOutcome, RuntimeTaskScope, RuntimeTaskShutdownReceipt,
    RuntimeTaskShutdownReceiptInput, RuntimeTaskShutdownRequest, RuntimeTaskShutdownStatus,
    RuntimeTaskTracker, RuntimeTaskTrackerPolicy, RuntimeTaskTrackerShutdownState, SessionId,
    SessionIdError, SessionIdentity, SessionIsolationPolicy, SessionIsolationReceipt, SessionKind,
    SessionRuntimeSnapshot, SubAgentConfigSurface, SubAgentContextPolicy,
    SubAgentContextVisibility, SubAgentPerformanceBudget, SubAgentPermissionSet, SubAgentRuntime,
    SubAgentSpawnConfig, SubAgentSpawnPolicy, SubAgentSpawnProfile, SubAgentSpawnReceipt,
    SubAgentSpawnStrategy, TokioAgentRuntime, TokioAgentRuntimeBuildRequest,
    TokioRuntimeDiagnosticsPolicy, TokioRuntimeFlavor, TokioRuntimePolicy,
    TokioRuntimePolicyReceipt, ToolRuntime, WorkingCopyActiveBinding, WorkingCopyBaseRef,
    WorkingCopyBranchName, WorkingCopyCommandInvocation, WorkingCopyCommandProgram,
    WorkingCopyCommandProjection, WorkingCopyCommandReceipt, WorkingCopyCommandStatus,
    WorkingCopyCreateRequest, WorkingCopyFanoutBenchmarkReceipt, WorkingCopyFinalizeBranchRequest,
    WorkingCopyGitTopLevel, WorkingCopyHandle, WorkingCopyId, WorkingCopyIsolationOperationKind,
    WorkingCopyIsolationPlan, WorkingCopyIsolationPlanStep, WorkingCopyIsolationProvider,
    WorkingCopyIsolationReceipt, WorkingCopyIsolationRequest, WorkingCopyIsolationStatus,
    WorkingCopyParallelIsolationReceipt, WorkingCopyPullRequestCheckoutRequest,
    WorkingCopyPullRequestNumber, WorkingCopyRepositoryDiscoveryPath, WorkingCopyRetentionPolicy,
    WorkingCopyRetentionSweepReceipt, WorkingCopySubAgentFanoutItem,
};
