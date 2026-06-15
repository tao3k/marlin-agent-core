//! Lean `marlin-agent-core` facade for graph-loop runtime boundaries.

mod debug_cli;
mod release;

pub use debug_cli::{
    GraphQueryOutput, GraphQuerySummary, LoopInspectReceipt, LoopQuerySummary, LoopReplayReceipt,
    LoopRunReceipt, MarlinCliResult, run_marlin_cli, run_marlin_cli_from_args,
};
pub use marlin_agent_environment as environment;
pub use marlin_agent_environment::{
    PROJECT_CONFIG_PRECEDENCE, RuntimeEnvironmentError, RuntimeEnvironmentRequest,
    RuntimeEnvironmentResolver, SESSION_FLAGS_CONFIG_PRECEDENCE, SUB_AGENT_CONFIG_PRECEDENCE,
    SYSTEM_CONFIG_PRECEDENCE, SubAgentEnvironmentRequest, USER_CONFIG_PRECEDENCE,
};
pub use marlin_agent_harness as harness;
pub use marlin_agent_harness::{
    AGENT_HARNESS_GRAPH_POLICY_PROPOSAL_VISIBILITY_SUBJECT_PREFIX,
    AGENT_HARNESS_PERFORMANCE_EVIDENCE_ALLOCATION_PROFILE,
    AGENT_HARNESS_PERFORMANCE_EVIDENCE_BASELINE,
    AGENT_HARNESS_PERFORMANCE_EVIDENCE_BENCHMARK_COMMAND, AGENT_HARNESS_PERFORMANCE_EVIDENCE_KEYS,
    AGENT_HARNESS_PERFORMANCE_EVIDENCE_LATENCY_OR_THROUGHPUT,
    AGENT_HARNESS_PERFORMANCE_EVIDENCE_PROFILE_ARTIFACT,
    AGENT_HARNESS_PERFORMANCE_EVIDENCE_REGRESSION_THRESHOLD,
    AGENT_HARNESS_STABILITY_EVIDENCE_ARTIFACT, AGENT_HARNESS_STABILITY_EVIDENCE_COMMAND,
    AGENT_HARNESS_STABILITY_EVIDENCE_DETERMINISM,
    AGENT_HARNESS_STABILITY_EVIDENCE_ITERATION_WINDOW, AGENT_HARNESS_STABILITY_EVIDENCE_KEYS,
    AGENT_HARNESS_STABILITY_EVIDENCE_LATENCY_DISTRIBUTION,
    AGENT_HARNESS_STABILITY_EVIDENCE_RESOURCE_DELTA, AGENT_HARNESS_STABILITY_EVIDENCE_STATE_GROWTH,
    AgentHarnessEvidence, AgentHarnessEvidenceKind, AgentHarnessGerbilLoopContinuationError,
    AgentHarnessGerbilLoopContinuationPlanner, AgentHarnessGerbilLoopContinuationProjector,
    AgentHarnessGraphLoopExecutionReport, AgentHarnessGraphLoopExecutionSummary,
    AgentHarnessPerformanceEvidence, AgentHarnessStabilityEvidence, ReleaseGateExecutionReceipt,
    ReleaseGateExecutionStatus, agent_harness_graph_policy_proposal_visibility_evidence,
    release_gate_execution_receipt, release_gate_visibility_evidence,
    release_topology_execution_receipts, release_topology_visibility_evidence,
    release_visibility_evidence,
};
pub use marlin_agent_hooks as hooks;
pub use marlin_agent_hooks::{
    HookDispatchPolicy, HookDispatchPolicyFinalizer, HookDispatchPolicyFinalizerInput,
    HookDispatchReport, HookDispatcher, HookInvocation, HookRegistration, HookRegistrationCatalog,
    HookRegistry, RegisteredHookPolicyFinalizer, RegisteredHookRegistrationCatalog,
    RegisteredHookRuntime,
};
pub use marlin_agent_kernel::{
    ExecutorName, GraphId, GraphLoopContinuationInput, GraphLoopContinuationPlanner,
    GraphLoopController, GraphLoopExecutionBudget, GraphLoopExecutionRequest,
    GraphLoopExecutionResult, GraphLoopExecutionStatus, GraphLoopKernel, GraphLoopStrategy,
    GraphLoopStrategyId, GraphLoopStrategyRuntime, GraphLoopStrategyVersion,
    GraphNodeExecutionReceipt, GraphNodeExecutionStatus, GraphNodeExecutor, GraphNodeInvocation,
    GraphPolicyProposal, GraphPolicyProposalCompilation, GraphPolicyProposalReceipt,
    GraphPolicyProposalStatus, LoopEdgeSpec, LoopGraph, LoopNodeSpec, NodeId, ProviderNodeAdapter,
    RunId, RuntimePlanSnapshot, SubAgentNodeAdapter, TerminalGraphLoopContinuationPlanner,
    TokioGraphLoopController, TokioGraphLoopKernel, ToolNodeAdapter,
    compile_graph_policy_proposal_with_native_abi_readiness,
};
pub use marlin_agent_protocol as protocol;
pub use marlin_agent_protocol::{
    AgentEventTopic, AgentExecutionTrace, AgentExecutionTraceSummary, AgentSpanName,
    AgentTraceSpanRecord, GERBIL_LOOP_GRAPH_CONTINUATION_SCHEMA_ID,
    GERBIL_LOOP_GRAPH_POLICY_COMPILATION_SCHEMA_ID, GRAPH_POLICY_PROPOSAL_SPAN_NAME,
    GerbilLoopGraphContinuationAction, GerbilLoopGraphContinuationRequest,
    GerbilLoopGraphPolicyCompilationRequest, GraphLoopEvidencePolicy, GraphLoopIterationReport,
    GraphLoopNextAction, GraphLoopRunRequest, GraphLoopStopPolicy, GraphNativeAbiId,
    GraphNativeAbiReadinessReceipt, GraphNativeAbiReadinessStatus, GraphNativeAbiRequirement,
    GraphNativeSymbol, HookAgentScope, HookDispatchPolicyReceipt, HookDispatchPolicyReceiptInput,
    HookDurationMs, HookEventName, HookExecutionMode, HookHandlerType, HookOutputEntry,
    HookOutputEntryKind, HookPolicyDecision, HookPolicyDecisionReason, HookPolicyDynamicAction,
    HookPolicyDynamicActionApplicationEffect, HookPolicyDynamicActionApplicationReason,
    HookPolicyDynamicActionApplicationReceipt, HookPolicyDynamicActionApplicationStatus,
    HookPolicyDynamicActionKind, HookPolicyDynamicActionReason, HookPolicyDynamicActionReplacement,
    HookPolicyDynamicActionTarget, HookPolicyExtension, HookRunId, HookRunStatus, HookRunSummary,
    HookScope, HookSource, HookSourcePath, HookTimestampMs, HookTrustStatus, LiteLlmModelId,
    ModelAlias, ModelCommandKind, ModelCommandMatcher, ModelContextForkMode, ModelEndpoint,
    ModelName, ModelProviderId, ModelRouteDecision, ModelRouteReceipt, ModelRouteRequest,
    ModelRouteRule, ModelRouteRuleId, ModelRouteSessionId, ModelSessionLifecycle,
    ModelSessionPersistenceKey, ModelSessionPolicy, ModelSessionPoolId, RuntimeConfigLayer,
    RuntimeConfigLayerSource, RuntimeHome, RuntimeHomeSource, RuntimeSandboxPolicy,
    SubAgentActivity, SubAgentActivityKind, SubAgentSource, compile_gerbil_loop_graph,
    compile_gerbil_loop_graph_continuation, compile_gerbil_loop_graph_policy,
};
pub use marlin_agent_runtime as runtime;
pub use marlin_agent_runtime::observability;
pub use marlin_agent_runtime::{
    AgentSessionContext, CancellationToken, CompiledModelRouteResolver, ContextExpansionPolicy,
    ContextNamespace, ContextVisibility, EventStream, HookRuntime, ModelGateway,
    ModelGatewayCompletionChoice, ModelGatewayCompletionOptions, ModelGatewayCompletionResponse,
    ModelGatewayError, ModelGatewayFuture, ModelGatewayMessage, ModelGatewayMessageRole,
    ModelGatewayRequest, ModelGatewayResult, ModelGatewayTransport, ModelRouteCompileError,
    ModelRouteConfig, ModelRouteConfigError, ModelRouteSessionBinding, ProviderRuntime,
    RoutedSubAgentSpawn, RuntimeContext, RuntimeEnvironment, RuntimeEvent, RuntimeEventSink,
    RuntimeEventStream, RuntimeExecutionIdentity, RuntimeFuture, RuntimeTask, RuntimeTaskOutcome,
    SessionId, SessionIdError, SessionIdentity, SessionIsolationPolicy, SessionIsolationReceipt,
    SessionKind, SubAgentRuntime, TokioAgentRuntime, ToolRuntime, assistant_gateway_message,
    system_gateway_message, user_gateway_message,
};
pub use marlin_agent_sessions as sessions;

pub use marlin_gerbil_ir as gerbil_ir;
pub use marlin_gerbil_scheme as gerbil_scheme;
pub use marlin_gerbil_scheme::{
    GERBIL_LOOP_GRAPH_CONTINUATION_NATIVE_PROJECTION_ABI_ID,
    GERBIL_LOOP_GRAPH_CONTINUATION_NATIVE_PROJECTION_ABI_VERSION,
    GERBIL_LOOP_GRAPH_CONTINUATION_NATIVE_PROJECTION_PACKAGE_ID,
    GERBIL_LOOP_GRAPH_CONTINUATION_NATIVE_SYMBOL, GERBIL_LOOP_GRAPH_CONTINUATION_TYPE_ID,
    GerbilCommandSpec, decode_gerbil_loop_graph_continuation_native_projection,
    decode_gerbil_loop_graph_continuation_projection,
    gerbil_loop_graph_continuation_native_projection_abi_contract,
    gerbil_loop_graph_continuation_native_projection_package_manifest,
    gerbil_loop_graph_continuation_native_projection_readiness_plan,
    gerbil_loop_graph_continuation_native_projection_request,
    gerbil_loop_graph_continuation_projection_contract,
    gerbil_loop_graph_continuation_type_manifest, project_gerbil_loop_graph_continuation_action,
    project_gerbil_loop_graph_continuation_native_action,
};
pub use marlin_org_model as org_model;
pub use marlin_org_patch as org_patch;
pub use marlin_org_store as org_store;
pub use marlin_org_store::{FileSystemReleaseStatusStore, OrgSourceStoreResult};
pub use marlin_org_workflow as org_workflow;
pub use marlin_org_workspace as org_workspace;
pub use marlin_org_workspace::{
    STANDARD_AGENT_LOOP_CONTRACT_DOCUMENT_ID, STANDARD_AGENT_LOOP_CONTRACT_ORG,
    STANDARD_AGENT_MEMORY_CONTRACT_DOCUMENT_ID, STANDARD_AGENT_MEMORY_CONTRACT_ORG,
    STANDARD_AGENT_PLAN_CONTRACT_DOCUMENT_ID, STANDARD_AGENT_PLAN_CONTRACT_ORG,
    STANDARD_AGENT_TASK_CONTRACT_DOCUMENT_ID, STANDARD_AGENT_TASK_CONTRACT_ORG,
    load_standard_agent_contract_workspace, standard_agent_contract_documents,
};
pub use marlin_workspace_protocol as workspace;
pub use marlin_workspace_protocol::{
    ReleaseGateReceipt, ReleaseGateState, ReleaseGateStatus, ReleaseLandingReport, ReleaseStatus,
    ReleaseVisibilityStatus,
};
pub use release::{
    ProcessReleaseGateCommandRunner, ReleaseGateCommandOutput, ReleaseGateCommandRunner,
    ReleaseGateRecordRequest, ReleaseGateRunOptions, commit_release_gate_execution_receipts,
    execute_and_record_release_gate_with_runner, execute_release_gate,
    execute_release_gate_with_runner, gerbil_release_status_commit_from_execution_receipts,
    record_release_gate_execution_receipt, release_gate_execution_receipt_from_output,
    release_gate_state_from_execution, release_gate_status_receipt,
};
