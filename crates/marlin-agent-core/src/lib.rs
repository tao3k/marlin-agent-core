//! Lean `marlin-agent-core` facade for graph-loop runtime boundaries.

mod release;

pub use marlin_agent_environment as environment;
pub use marlin_agent_environment::{
    PROJECT_CONFIG_PRECEDENCE, RuntimeEnvironmentError, RuntimeEnvironmentRequest,
    RuntimeEnvironmentResolver, SESSION_FLAGS_CONFIG_PRECEDENCE, SUB_AGENT_CONFIG_PRECEDENCE,
    SYSTEM_CONFIG_PRECEDENCE, SubAgentEnvironmentRequest, USER_CONFIG_PRECEDENCE,
};
pub use marlin_agent_harness as harness;
pub use marlin_agent_harness::{
    ReleaseGateExecutionReceipt, ReleaseGateExecutionStatus, release_gate_execution_receipt,
    release_gate_visibility_evidence, release_topology_execution_receipts,
    release_topology_visibility_evidence, release_visibility_evidence,
};
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
    AgentEventTopic, AgentExecutionTrace, AgentExecutionTraceSummary, AgentSpanName,
    AgentTraceSpanRecord, HookDurationMs, HookEventName, HookExecutionMode, HookHandlerType,
    HookOutputEntry, HookOutputEntryKind, HookRunId, HookRunStatus, HookRunSummary, HookScope,
    HookSource, HookSourcePath, HookTimestampMs, HookTrustStatus, LoopEvidence, LoopEvidenceKind,
    LoopPerformanceEvidence, PERFORMANCE_EVIDENCE_ALLOCATION_PROFILE,
    PERFORMANCE_EVIDENCE_BASELINE, PERFORMANCE_EVIDENCE_BENCHMARK_COMMAND,
    PERFORMANCE_EVIDENCE_KEYS, PERFORMANCE_EVIDENCE_LATENCY_OR_THROUGHPUT,
    PERFORMANCE_EVIDENCE_PROFILE_ARTIFACT, PERFORMANCE_EVIDENCE_REGRESSION_THRESHOLD,
    RuntimeConfigLayer, RuntimeConfigLayerSource, RuntimeHome, RuntimeHomeSource,
    RuntimeSandboxPolicy, SubAgentActivity, SubAgentActivityKind, SubAgentSource,
};
pub use marlin_agent_runtime as runtime;
pub use marlin_agent_runtime::observability;
pub use marlin_agent_runtime::{
    CancellationToken, EventStream, HookRuntime, ProviderRuntime, RuntimeContext,
    RuntimeEnvironment, RuntimeEvent, RuntimeEventSink, RuntimeEventStream,
    RuntimeExecutionIdentity, RuntimeFuture, RuntimeTask, RuntimeTaskOutcome, SubAgentRuntime,
    TokioAgentRuntime, ToolRuntime,
};

pub use marlin_gerbil_ir as gerbil_ir;
pub use marlin_org_model as org_model;
pub use marlin_org_patch as org_patch;
pub use marlin_org_store as org_store;
pub use marlin_org_workflow as org_workflow;
pub use marlin_org_workspace as org_workspace;
pub use marlin_workspace_protocol as workspace;
pub use marlin_workspace_protocol::{
    ReleaseGateReceipt, ReleaseGateState, ReleaseGateStatus, ReleaseStatus, ReleaseVisibilityStatus,
};
pub use release::{release_gate_state_from_execution, release_gate_status_receipt};
