//! Agent protocol contracts for graph loops, scenarios, and transcripts.

mod event;
mod graph;
mod hook;
mod model_gateway;
mod model_route;
mod project_runtime;
mod runtime_environment;
mod scenario;
mod sub_agent;
mod trace;

pub use event::{AgentEvent, AgentEventTopic};
pub use graph::{
    ExecutorName, GERBIL_LOOP_GRAPH_CONTINUATION_SCHEMA_ID,
    GERBIL_LOOP_GRAPH_POLICY_COMPILATION_SCHEMA_ID, GRAPH_POLICY_PROPOSAL_SCHEMA_ID,
    GerbilLoopGraphContinuationAction, GerbilLoopGraphContinuationCompileError,
    GerbilLoopGraphContinuationRequest, GerbilLoopGraphPolicyCompilationRequest, GraphId,
    GraphLoopContinuationAction, GraphLoopContinuationDecision, GraphLoopContinuationReceipt,
    GraphLoopEvent, GraphLoopEventEnvelope, GraphLoopEventId, GraphLoopEvidencePolicy,
    GraphLoopExecutionBudget, GraphLoopExecutionRequest, GraphLoopExecutionResult,
    GraphLoopExecutionStatus, GraphLoopInputDrainPolicy, GraphLoopInputLane,
    GraphLoopInputQueueReceipt, GraphLoopIterationId, GraphLoopIterationReport,
    GraphLoopMessageRole, GraphLoopNextAction, GraphLoopQueuedInput, GraphLoopRunRequest,
    GraphLoopStopPolicy, GraphLoopStopReason, GraphLoopStopReceipt, GraphLoopStrategy,
    GraphLoopStrategyId, GraphLoopStrategyRuntime, GraphLoopStrategyVersion, GraphNativeAbiId,
    GraphNativeAbiReadinessReceipt, GraphNativeAbiReadinessStatus, GraphNativeAbiRequirement,
    GraphNativeSymbol, GraphNodeExecutionId, GraphNodeExecutionReceipt, GraphNodeExecutionStatus,
    GraphNodeInvocation, GraphPolicyDigest, GraphPolicyProposal, GraphPolicyProposalReceipt,
    GraphPolicyProposalStatus, GraphPolicyProposalValidationReport, GraphToolBatchDecision,
    GraphToolBatchExecutionMode, GraphToolBatchExecutionReceipt, GraphToolCallId,
    GraphToolCallReceipt, GraphToolCallStatus, LoopEdgeSpec, LoopGraph, LoopNodeSpec, NodeId,
    RunId, RuntimePlanSnapshot, compile_gerbil_loop_graph, compile_gerbil_loop_graph_continuation,
    compile_gerbil_loop_graph_policy, validate_graph_policy_proposal,
};
pub use hook::{
    HookAgentClass, HookAgentLineageNode, HookAgentScope, HookConfigurationReloadReceipt,
    HookConfigurationVersion, HookDecisionContext, HookDispatchPolicyReceipt,
    HookDispatchPolicyReceiptInput, HookDispatchSelectionInput, HookDispatchSelectionReceipt,
    HookDurationMs, HookEventName, HookExecutionMode, HookHandlerType, HookMatcherStrategy,
    HookMatcherToken, HookOrgMemoryHit, HookOutputEntry, HookOutputEntryKind, HookPolicyDecision,
    HookPolicyDecisionReason, HookPolicyDecisionReceipt, HookPolicyDynamicAction,
    HookPolicyDynamicActionApplicationEffect, HookPolicyDynamicActionApplicationReason,
    HookPolicyDynamicActionApplicationReceipt, HookPolicyDynamicActionApplicationStatus,
    HookPolicyDynamicActionKind, HookPolicyDynamicActionReason, HookPolicyDynamicActionReplacement,
    HookPolicyDynamicActionTarget, HookPolicyExtension, HookPolicyExtensionKind, HookPolicyMode,
    HookRegistryUpdateKind, HookRegistryUpdateReceipt, HookRunId, HookRunStatus, HookRunSummary,
    HookSchemeModule, HookSchemeProcedure, HookScope, HookSelectedCandidateInput,
    HookSelectionCandidateReceipt, HookSelectionSkipReason, HookSessionId,
    HookSkippedCandidateInput, HookSource, HookSourcePath, HookTimestampMs, HookTrustStatus,
    HookWorkspaceStateFact,
};
pub use model_gateway::{
    ModelGateway, ModelGatewayCompletionChoice, ModelGatewayCompletionOptions,
    ModelGatewayCompletionResponse, ModelGatewayError, ModelGatewayFuture, ModelGatewayMessage,
    ModelGatewayMessageRole, ModelGatewayRequest, ModelGatewayResult, ModelGatewayTransport,
    assistant_gateway_message, system_gateway_message, user_gateway_message,
};
pub use model_route::{
    LiteLlmModelId, ModelAlias, ModelCommandKind, ModelCommandMatcher, ModelContextForkMode,
    ModelEndpoint, ModelEndpointContractError, ModelName, ModelProviderId, ModelRouteAgentScope,
    ModelRouteDecision, ModelRouteReceipt, ModelRouteRequest, ModelRouteRule, ModelRouteRuleId,
    ModelRouteSessionId, ModelSessionLifecycle, ModelSessionPersistenceKey, ModelSessionPolicy,
    ModelSessionPoolId,
};
pub use project_runtime::{
    AgentContentCompressionState, AgentContentNode, AgentContentNodeInput, AgentContentRole,
    AgentSessionFact, AgentSessionHistoryLimit, AgentSessionKind, ContentCompressionReceipt,
    ContentCompressionStatus, ContentTokenBudget, ContentTokenCount, ContentUsageKind,
    ContentUsageReceipt, ContentUsageReceiptInput, ContextPackReceipt, GraphQueryContext,
    GraphQueryExternalProjectPolicy, GraphQueryFallbackPolicy, GraphQueryFallbackScope,
    GraphQueryFamily, GraphQueryLimit, GraphQueryMatch, GraphQueryMatchRelationship,
    GraphQueryRelationshipFact, GraphQueryRequest, GraphQueryResponse, GraphQueryScoreBasisPoints,
    GraphQuerySecretVisibility, GraphQueryVisibility, GraphQueryVisibleSurface,
    MemoryTriggerReceipt, MemoryTriggerStatus, ProjectMemoryContextFact, ProjectMemoryContextPack,
    ProjectMemoryRecallIntent, ProjectMemoryRecallRequest, ProjectMemoryRecallTerm,
    ProjectRuntimeAgentId, ProjectRuntimeBranchRef, ProjectRuntimeContentBodyRef,
    ProjectRuntimeContentId, ProjectRuntimeContextPackId, ProjectRuntimeEvidenceId,
    ProjectRuntimeMemoryId, ProjectRuntimeProjectId, ProjectRuntimeReceiptId,
    ProjectRuntimeRootSessionId, ProjectRuntimeSessionId, ProjectRuntimeSourceAnchorId,
    ProjectRuntimeSourceSpanRef, ProjectRuntimeToolCapabilityId, ProjectRuntimeWorkspaceId,
    ProjectRuntimeWorktreeId,
};
pub use runtime_environment::{
    RuntimeConfigLayer, RuntimeConfigLayerSource, RuntimeEnvironment, RuntimeEnvironmentActivation,
    RuntimeEnvironmentActivationAction, RuntimeEnvironmentActivationActionReceipt,
    RuntimeEnvironmentActivationActionStatus, RuntimeEnvironmentActivationPolicy,
    RuntimeEnvironmentActivationReceipt, RuntimeEnvironmentActivationStatus,
    RuntimeEnvironmentDelta, RuntimeEnvironmentRefreshCachePolicy,
    RuntimeEnvironmentRefreshExecution, RuntimeEnvironmentRefreshReceipt,
    RuntimeEnvironmentRefreshStatus, RuntimeEnvironmentRefreshTimeout,
    RuntimeEnvironmentResolution, RuntimeEnvrcPolicy, RuntimeHome, RuntimeHomeSource,
    RuntimeSandboxPolicy, RuntimeShellIsolationPolicy, RuntimeWorkspaceProject,
    RuntimeWorkspaceProjectId, RuntimeWorkspaceProjectImportAction,
    RuntimeWorkspaceProjectImportActionReceipt, RuntimeWorkspaceProjectImportActionStatus,
    RuntimeWorkspaceProjectImportReceipt, RuntimeWorkspaceProjectImportStatus,
    RuntimeWorkspaceProjectTrust,
};
pub use scenario::{
    AGENT_SCENARIO_CONTRACT_SCHEMA_ID, AgentScenario, AgentScenarioContract, AgentScenarioStep,
};
pub use sub_agent::{
    SubAgentActivity, SubAgentActivityKind, SubAgentConfigSurface, SubAgentContextPolicy,
    SubAgentContextVisibility, SubAgentPerformanceBudget, SubAgentPermissionSet,
    SubAgentSearchReceipt, SubAgentSource, SubAgentSpawnConfig, SubAgentSpawnConfigError,
    SubAgentSpawnConfigSet, SubAgentSpawnPolicy, SubAgentSpawnProfile, SubAgentSpawnProfileId,
    SubAgentSpawnStrategy, SubAgentType,
};
pub use trace::{
    AgentExecutionTrace, AgentExecutionTraceSummary, AgentSpanName, AgentTraceSpanRecord,
    GRAPH_POLICY_PROPOSAL_SPAN_NAME,
};
