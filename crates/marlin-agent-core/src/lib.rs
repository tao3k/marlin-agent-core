//! Lean `marlin-agent-core` facade for graph-loop runtime boundaries.

pub use marlin_agent_environment as environment;
pub use marlin_agent_environment::{
    PROJECT_CONFIG_PRECEDENCE, RuntimeEnvironmentError, RuntimeEnvironmentRequest,
    RuntimeEnvironmentResolver, SESSION_FLAGS_CONFIG_PRECEDENCE, SUB_AGENT_CONFIG_PRECEDENCE,
    SYSTEM_CONFIG_PRECEDENCE, SubAgentEnvironmentRequest, USER_CONFIG_PRECEDENCE,
};
pub use marlin_agent_harness as harness;
pub use marlin_agent_hooks as hooks;
pub use marlin_agent_hooks::{
    HookDispatchReport, HookDispatcher, HookInvocation, HookRegistration, HookRegistry,
    RegisteredHookRuntime,
};
pub use marlin_agent_kernel::{
    ExecutorName, GraphId, GraphLoopExecutionRequest, GraphLoopExecutionResult,
    GraphLoopExecutionStatus, GraphLoopKernel, GraphNodeExecutionReceipt, GraphNodeExecutionStatus,
    GraphNodeExecutor, GraphNodeInvocation, LoopEdgeSpec, LoopGraph, LoopNodeSpec, NodeId,
    ProviderNodeAdapter, RunId, RuntimePlanSnapshot, SubAgentNodeAdapter, TokioGraphLoopKernel,
    ToolNodeAdapter,
};
pub use marlin_agent_protocol as protocol;
pub use marlin_agent_protocol::{
    HookDurationMs, HookEventName, HookExecutionMode, HookHandlerType, HookOutputEntry,
    HookOutputEntryKind, HookRunId, HookRunStatus, HookRunSummary, HookScope, HookSource,
    HookSourcePath, HookTimestampMs, HookTrustStatus, RuntimeConfigLayer, RuntimeConfigLayerSource,
    RuntimeHome, RuntimeHomeSource, RuntimeSandboxPolicy, SubAgentActivity, SubAgentActivityKind,
    SubAgentSource,
};
pub use marlin_agent_runtime as runtime;
pub use marlin_agent_runtime::{
    CancellationToken, EventStream, HookRuntime, ProviderRuntime, RuntimeContext,
    RuntimeEnvironment, RuntimeEvent, RuntimeEventSink, RuntimeEventStream, RuntimeFuture,
    RuntimeTask, RuntimeTaskOutcome, SubAgentRuntime, TokioAgentRuntime, ToolRuntime,
};

pub use marlin_gerbil_ir as gerbil_ir;
pub use marlin_org_model as org_model;
pub use marlin_org_patch as org_patch;
pub use marlin_org_store as org_store;
pub use marlin_org_workflow as org_workflow;
pub use marlin_org_workspace as org_workspace;
pub use marlin_workspace_protocol as workspace;
