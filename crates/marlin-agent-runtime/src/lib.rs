//! Tokio-backed runtime extension traits used by the graph-loop kernel.

pub mod observability;

mod agent_graph;
mod graph_loop;
mod model_route;
mod resilience;
mod tokio_runtime;

pub use agent_graph::{
    RuntimeAgentCoordinationAdmissionReceipt, RuntimeAgentCoordinationAdmissionStatus,
    RuntimeAgentCoordinationRejection, RuntimeAgentGraphExecutionReadinessReceipt,
    RuntimeAgentGraphExecutionReadinessRejection, RuntimeAgentGraphExecutionReadinessStatus,
    RuntimeAgentGraphExecutionRequest, RuntimeAgentGraphExecutionRequestRejection,
    RuntimeAgentGraphProjectionReceipt, RuntimeAgentGraphProjectionRejection,
    RuntimeAgentGraphProjectionStatus, admit_agent_coordination_plan,
    build_agent_graph_execution_request, check_agent_graph_execution_readiness,
    project_agent_graph_planning_receipt, project_agent_graph_projection_request,
};
pub use graph_loop::{
    GraphLoopRunCancelReceipt, GraphLoopRunCancelStatus, GraphLoopRunInspectReceipt,
    GraphLoopRunObservation, GraphLoopRunProgressUpdate, GraphLoopRunRegistry,
    GraphLoopRunRegistryError, GraphLoopRunRegistryHandle, GraphLoopRunRegistrySnapshot,
    GraphLoopRunStartReceipt, GraphLoopRunStatus, GraphLoopRunWaitReceipt, GraphLoopRunWaitStatus,
};
pub use model_route::{
    ActivatedModelRouteProfileSpawnRequest, CompiledModelRouteResolver, ModelGateway,
    ModelGatewayCompletionChoice, ModelGatewayCompletionOptions, ModelGatewayCompletionResponse,
    ModelGatewayError, ModelGatewayFuture, ModelGatewayMessage, ModelGatewayMessageRole,
    ModelGatewayRequest, ModelGatewayResult, ModelGatewayTransport, ModelRouteAdmissionError,
    ModelRouteCompileError, ModelRouteConfig, ModelRouteConfigError,
    ModelRouteSelectionProjectionError, ModelRouteSelectionProjectionReceipt,
    ModelRouteSelectionProjectionSource, ModelRouteSessionBinding, ProjectedModelRouteDecision,
    RoutedSubAgentSpawn, RuntimeEdgeModelGateway, admit_model_route_with_resolver,
    assistant_gateway_message, system_gateway_message, user_gateway_message,
};
pub use observability::process::{
    AsyncManagedChildProcess, ManagedChildProcessSpec, RuntimeProcessCleanupController,
    RuntimeProcessCleanupSweep, RuntimeProcessExitStatus, RuntimeProcessLiveness,
    RuntimeProcessOutput, RuntimeProcessRegistryHandle, RuntimeProcessTerminator,
    SysinfoRuntimeProcessController,
};
pub use resilience::{
    RuntimeEdgeLayer, RuntimeEdgePolicy, RuntimeEdgePolicyError, RuntimeEdgePolicyReceipt,
    RuntimeEdgeService,
};
pub use tokio_runtime::{
    AgentSessionContext, CancellationToken, ContextExpansionPolicy, ContextNamespace,
    ContextVisibility, EventStream, HookRuntime, ProviderRuntime, RuntimeBlockingBridgePolicy,
    RuntimeBlockingBridgeStrategy, RuntimeContext, RuntimeEnvironment, RuntimeEvent,
    RuntimeEventSink, RuntimeEventStream, RuntimeExecutionIdentity, RuntimeFanoutJoinPolicy,
    RuntimeFanoutOutput, RuntimeFanoutReceipt, RuntimeFanoutResult, RuntimeFanoutTaskReceipt,
    RuntimeFanoutTaskStatus, RuntimeFuture, RuntimeTask, RuntimeTaskOutcome,
    RuntimeTaskShutdownReceipt, RuntimeTaskShutdownReceiptInput, RuntimeTaskShutdownRequest,
    RuntimeTaskShutdownStatus, RuntimeTaskTracker, RuntimeTaskTrackerPolicy,
    RuntimeTaskTrackerShutdownState, SessionId, SessionIdError, SessionIdentity,
    SessionIsolationPolicy, SessionIsolationReceipt, SessionKind, SessionRuntimeSnapshot,
    SubAgentConfigSurface, SubAgentContextPolicy, SubAgentContextVisibility,
    SubAgentPerformanceBudget, SubAgentPermissionSet, SubAgentRuntime, SubAgentSpawnConfig,
    SubAgentSpawnPolicy, SubAgentSpawnProfile, SubAgentSpawnReceipt, SubAgentSpawnStrategy,
    TokioAgentRuntime, TokioAgentRuntimeBuildRequest, TokioRuntimeDiagnosticsPolicy,
    TokioRuntimeFlavor, TokioRuntimePolicy, TokioRuntimePolicyReceipt, ToolRuntime,
    WorkingCopyActiveBinding, WorkingCopyBaseRef, WorkingCopyBranchName,
    WorkingCopyCommandInvocation, WorkingCopyCommandProgram, WorkingCopyCommandProjection,
    WorkingCopyCommandReceipt, WorkingCopyCommandStatus, WorkingCopyCreateRequest,
    WorkingCopyFanoutBenchmarkReceipt, WorkingCopyFinalizeBranchRequest, WorkingCopyGitTopLevel,
    WorkingCopyHandle, WorkingCopyId, WorkingCopyIsolationOperationKind, WorkingCopyIsolationPlan,
    WorkingCopyIsolationPlanStep, WorkingCopyIsolationProvider, WorkingCopyIsolationReceipt,
    WorkingCopyIsolationRequest, WorkingCopyIsolationStatus, WorkingCopyParallelIsolationReceipt,
    WorkingCopyPullRequestCheckoutRequest, WorkingCopyPullRequestNumber,
    WorkingCopyRepositoryDiscoveryPath, WorkingCopyRetentionPolicy,
    WorkingCopyRetentionSweepReceipt, WorkingCopySubAgentFanoutItem,
};
