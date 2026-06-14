//! Tokio-backed runtime extension traits used by the graph-loop kernel.

pub mod observability;

mod model_route;
mod tokio_runtime;

pub use model_route::{
    ActivatedModelRouteProfileSpawnRequest, CompiledModelRouteResolver, ModelGateway,
    ModelGatewayCompletionChoice, ModelGatewayCompletionOptions, ModelGatewayCompletionResponse,
    ModelGatewayError, ModelGatewayFuture, ModelGatewayMessage, ModelGatewayMessageRole,
    ModelGatewayRequest, ModelGatewayResult, ModelGatewayTransport, ModelRouteCompileError,
    ModelRouteConfig, ModelRouteConfigError, ModelRouteSelectionProjectionError,
    ModelRouteSelectionProjectionReceipt, ModelRouteSelectionProjectionSource,
    ModelRouteSessionBinding, ProjectedModelRouteDecision, RoutedSubAgentSpawn,
    assistant_gateway_message, system_gateway_message, user_gateway_message,
};
pub use observability::process::{
    AsyncManagedChildProcess, ManagedChildProcessSpec, RuntimeProcessCleanupController,
    RuntimeProcessCleanupSweep, RuntimeProcessExitStatus, RuntimeProcessLiveness,
    RuntimeProcessOutput, RuntimeProcessRegistryHandle, RuntimeProcessTerminator,
    SysinfoRuntimeProcessController,
};
pub use tokio_runtime::{
    AgentSessionContext, CancellationToken, ContextExpansionPolicy, ContextNamespace,
    ContextVisibility, EventStream, HookRuntime, ProviderRuntime, RuntimeContext,
    RuntimeEnvironment, RuntimeEvent, RuntimeEventSink, RuntimeEventStream,
    RuntimeExecutionIdentity, RuntimeFuture, RuntimeTask, RuntimeTaskOutcome, SessionId,
    SessionIdError, SessionIdentity, SessionIsolationPolicy, SessionIsolationReceipt, SessionKind,
    SubAgentConfigSurface, SubAgentContextPolicy, SubAgentContextVisibility,
    SubAgentPerformanceBudget, SubAgentPermissionSet, SubAgentRuntime, SubAgentSpawnConfig,
    SubAgentSpawnPolicy, SubAgentSpawnProfile, SubAgentSpawnReceipt, SubAgentSpawnStrategy,
    TokioAgentRuntime, ToolRuntime,
};
