// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Lean `marlin-agent-core` facade for graph-loop runtime boundaries.

mod release;

#[cfg(feature = "app-server")]
pub use marlin_agent_app_server as app_server;

pub use marlin_agent_context as context;
pub use marlin_agent_context::org::workspace::initialize_org_runtime;
pub use marlin_agent_environment as environment;
pub use marlin_agent_environment::{
    PROJECT_CONFIG_PRECEDENCE, RuntimeEnvironmentError, RuntimeEnvironmentRequest,
    RuntimeEnvironmentResolver, SESSION_FLAGS_CONFIG_PRECEDENCE, SUB_AGENT_CONFIG_PRECEDENCE,
    SYSTEM_CONFIG_PRECEDENCE, SubAgentEnvironmentRequest, USER_CONFIG_PRECEDENCE,
};
pub use marlin_agent_graph::{
    self as agent_graph, AgentCapability, AgentCoordinationDecision, AgentCoordinationEvidenceKind,
    AgentCoordinationEvidenceRef, AgentCoordinationPlan, AgentCoordinationReceipt, AgentDelegation,
    AgentDelegationReason, AgentEdge, AgentEdgeCondition, AgentEdgeId, AgentEdgeKind,
    AgentElectionReason, AgentElectionReceipt, AgentEvidenceId, AgentGraph, AgentGraphId,
    AgentGraphPlanningReceipt, AgentGraphPlanningRejection, AgentGraphPlanningStatus,
    AgentGraphPlanningTarget, AgentGraphTopologyError, AgentNode, AgentNodeId,
    AgentPolicyDecisionRef, AgentPolicyRoutingDecision, AgentPolicyRoutingReceipt, AgentRole,
    AgentRoutingReason, AgentRoutingReceipt, AgentTopologyPolicy, AgentTopologyPolicyId,
    GerbilPolicyScopeRef, GraphLoopEntryRef, GraphLoopGraphRef, GraphLoopNodeRef,
    OrgMemoryScopeRef, plan_agent_coordination, plan_agent_coordination_with_policy_receipt,
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
    HookDispatchPolicy, HookDispatchReport, HookDispatcher, HookInvocation, HookRegistration,
    HookRegistrationCatalog, HookRegistry, RegisteredHookRegistrationCatalog,
    RegisteredHookRuntime,
};
pub use marlin_agent_kernel::{
    ExecutorName, FailureClassificationId, FailureClassificationReceipt, GraphId,
    GraphLoopContinuationInput, GraphLoopContinuationPlanner, GraphLoopController,
    GraphLoopExecutionBudget, GraphLoopExecutionRequest, GraphLoopExecutionResult,
    GraphLoopExecutionStatus, GraphLoopFailureKind, GraphLoopGovernancePolicy,
    GraphLoopGovernedContextNamespace, GraphLoopGovernedSessionKind, GraphLoopKernel,
    GraphLoopSandboxBackend, GraphLoopSandboxPolicy, GraphLoopSessionPolicy, GraphLoopStatePolicy,
    GraphLoopStrategy, GraphLoopStrategyId, GraphLoopStrategyRuntime, GraphLoopStrategyVersion,
    GraphLoopVerifierPolicy, GraphNodeExecutionReceipt, GraphNodeExecutionStatus,
    GraphNodeExecutor, GraphNodeInvocation, GraphPolicyProposal, GraphPolicyProposalCompilation,
    GraphPolicyProposalReceipt, GraphPolicyProposalStatus, HumanDecision, HumanDecisionReceipt,
    HumanGateId, HumanGateReceipt, HumanReviewKind, HumanReviewerId, LoopContinuationCapability,
    LoopContinuationPolicy, LoopEdgeSpec, LoopEvidenceCapturePolicy, LoopFailurePolicy, LoopGraph,
    LoopHumanGatePolicy, LoopMemoryPolicy, LoopModelRoutePolicy, LoopNodeSpec, LoopPolicyProfile,
    LoopPolicyProfileId, LoopQueuePolicy, LoopSelfEvolutionPolicy, LoopToolBatchPolicy, NodeId,
    ProviderNodeAdapter, RunId, RuntimePlanSnapshot, SubAgentNodeAdapter,
    TerminalGraphLoopContinuationPlanner, TokioGraphLoopController, TokioGraphLoopKernel,
    ToolNodeAdapter, compile_graph_policy_proposal_with_c_abi_readiness,
};
pub use marlin_agent_protocol as protocol;
pub use marlin_agent_protocol::{
    AGENT_GRAPH_PROJECTION_REQUEST_SCHEMA_ID, AgentEventTopic, AgentExecutionTrace,
    AgentExecutionTraceSummary, AgentGraphProjectionRequest, AgentSpanName, AgentTraceSpanRecord,
    GERBIL_LOOP_GRAPH_CONTINUATION_SCHEMA_ID, GERBIL_LOOP_GRAPH_POLICY_COMPILATION_SCHEMA_ID,
    GRAPH_POLICY_PROPOSAL_SPAN_NAME, GerbilLoopGraphContinuationAction,
    GerbilLoopGraphContinuationRequest, GerbilLoopGraphPolicyCompilationRequest, GraphAbiSymbol,
    GraphCAbiId, GraphCAbiReadinessReceipt, GraphCAbiReadinessStatus, GraphCAbiRequirement,
    GraphLoopContinuationAction, GraphLoopContinuationDecision, GraphLoopContinuationReceipt,
    GraphLoopEvent, GraphLoopEventEnvelope, GraphLoopEventId, GraphLoopEvidencePolicy,
    GraphLoopInputDrainPolicy, GraphLoopInputLane, GraphLoopInputQueueReceipt,
    GraphLoopIterationId, GraphLoopIterationReport, GraphLoopMessageRole, GraphLoopNextAction,
    GraphLoopQueuedInput, GraphLoopRunRequest, GraphLoopStopPolicy, GraphLoopStopReason,
    GraphLoopStopReceipt, GraphNodeExecutionId, GraphToolBatchDecision,
    GraphToolBatchExecutionMode, GraphToolBatchExecutionReceipt, GraphToolCallId,
    GraphToolCallReceipt, GraphToolCallStatus, HookAgentScope, HookDispatchPolicyReceipt,
    HookDispatchPolicyReceiptInput, HookDurationMs, HookEventName, HookExecutionMode,
    HookHandlerType, HookOutputEntry, HookOutputEntryKind, HookPolicyDecision,
    HookPolicyDecisionReason, HookPolicyDynamicAction, HookPolicyDynamicActionApplicationEffect,
    HookPolicyDynamicActionApplicationReason, HookPolicyDynamicActionApplicationReceipt,
    HookPolicyDynamicActionApplicationStatus, HookPolicyDynamicActionKind,
    HookPolicyDynamicActionReason, HookPolicyDynamicActionReplacement,
    HookPolicyDynamicActionTarget, HookPolicyExtension, HookRunId, HookRunStatus, HookRunSummary,
    HookScope, HookSource, HookSourcePath, HookTimestampMs, HookTrustStatus, LiteLlmModelId,
    MARLIN_HOME_ENV_VAR, MARLIN_SESSION_ID_ENV_VAR, ModelAlias, ModelEndpoint,
    ModelEndpointContractError, ModelName, ModelProviderId, RuntimeConfigLayer,
    RuntimeConfigLayerSource, RuntimeHome, RuntimeHomeSource, RuntimeSandboxPolicy, RuntimeSession,
    RuntimeSessionId, RuntimeSessionIdSource, SubAgentActivity, SubAgentActivityKind,
    SubAgentSource,
};
pub use marlin_agent_runtime::{
    self as runtime, AgentSessionContext, CancellationToken, ContextExpansionPolicy,
    ContextNamespace, ContextVisibility, EventStream, HookRuntime, ModelGateway,
    ModelGatewayCompletionChoice, ModelGatewayCompletionOptions, ModelGatewayCompletionResponse,
    ModelGatewayError, ModelGatewayFuture, ModelGatewayMessage, ModelGatewayMessageRole,
    ModelGatewayRequest, ModelGatewayResult, ModelGatewayTransport, ProviderRuntime,
    RuntimeAgentCoordinationAdmissionReceipt, RuntimeAgentCoordinationAdmissionStatus,
    RuntimeAgentCoordinationRejection, RuntimeAgentGraphExecutionReadinessReceipt,
    RuntimeAgentGraphExecutionReadinessRejection, RuntimeAgentGraphExecutionReadinessStatus,
    RuntimeAgentGraphProjectionReceipt, RuntimeAgentGraphProjectionRejection,
    RuntimeAgentGraphProjectionStatus, RuntimeContext, RuntimeEnvironment, RuntimeEvent,
    RuntimeEventSink, RuntimeEventStream, RuntimeExecutionIdentity, RuntimeFuture, RuntimeTask,
    RuntimeTaskOutcome, SessionId, SessionIdError, SessionIdentity, SessionIsolationPolicy,
    SessionIsolationReceipt, SessionKind, SubAgentRuntime, TokioAgentRuntime, ToolRuntime,
    admit_agent_coordination_plan, assistant_gateway_message,
    check_agent_graph_execution_readiness, observability, project_agent_graph_planning_receipt,
    project_agent_graph_projection_request, system_gateway_message, user_gateway_message,
};
pub use marlin_agent_sessions as sessions;
#[cfg(feature = "actor-runtime")]
pub use marlin_deck_runtime::{
    DeckContextModule, DeckMailboxMetrics, DeckModuleInstanceId, DeckModuleRetirementReceipt,
    DeckRuntime, DeckRuntimeConfig, DeckRuntimeMetadata, DeckRuntimeMetadataError,
    DeckRuntimeMetrics, DeckRuntimeModuleId, DeckRuntimeRejection, DeckRuntimeShutdownError,
    DeckRuntimeShutdownReceipt, DeckRuntimeStartupError, DeckRuntimeStopReason, DeckTimerId,
    DeckTimerReceipt,
};

pub use marlin_gerbil_ir as gerbil_ir;
pub use marlin_gerbil_scheme as gerbil_scheme;
pub use marlin_gerbil_scheme::{
    GERBIL_LOOP_GRAPH_CONTINUATION_ABI_PROJECTION_ABI_ID,
    GERBIL_LOOP_GRAPH_CONTINUATION_ABI_PROJECTION_ABI_VERSION,
    GERBIL_LOOP_GRAPH_CONTINUATION_ABI_PROJECTION_PACKAGE_ID,
    GERBIL_LOOP_GRAPH_CONTINUATION_ABI_SYMBOL, GERBIL_LOOP_GRAPH_CONTINUATION_TYPE_ID,
    decode_gerbil_loop_graph_continuation_abi_projection,
    decode_gerbil_loop_graph_continuation_projection,
    gerbil_loop_graph_continuation_abi_projection_abi_contract,
    gerbil_loop_graph_continuation_abi_projection_package_manifest,
    gerbil_loop_graph_continuation_abi_projection_readiness_plan,
    gerbil_loop_graph_continuation_abi_projection_request,
    gerbil_loop_graph_continuation_projection_contract,
    gerbil_loop_graph_continuation_type_manifest,
    project_gerbil_loop_graph_continuation_abi_action,
    project_gerbil_loop_graph_continuation_action,
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
