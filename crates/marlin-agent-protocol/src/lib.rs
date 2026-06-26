//! Agent protocol contracts for graph loops, scenarios, and transcripts.

mod agent_flow;
mod agent_graph;
mod event;
mod graph;
mod hook;
mod loop_policy_ir;
mod model_gateway;
mod model_route;
mod project_runtime;
mod runtime_environment;
mod runtime_state_home;
mod scenario;
mod sub_agent;
mod trace;

pub use agent_flow::{
    AgentFlowDerivedSession, AgentFlowIntent, AgentFlowIntentId, AgentFlowMemoryIntent,
    AgentFlowMemoryOperation, AgentFlowMemoryTarget, AgentFlowPlacementIntent,
    AgentFlowPlacementOperation, AgentFlowPlacementTarget, AgentFlowReceipt, AgentFlowReceiptId,
    AgentFlowReceiptStatus, AgentFlowRuntimeHandoff, AgentFlowRuntimeHandoffId, AgentFlowSession,
    AgentFlowSessionId, AgentFlowSessionStatus, AgentFlowSessionTransform, AgentFlowToolIntent,
    AgentFlowToolName, AgentFlowTransformId, AgentFlowTransformRejection,
    build_agent_flow_runtime_handoff, derive_agent_flow_session,
};
pub use agent_graph::{AGENT_GRAPH_PROJECTION_REQUEST_SCHEMA_ID, AgentGraphProjectionRequest};
pub use event::{AgentEvent, AgentEventTopic};
pub use graph::{
    ExecutorName, FailureClassificationId, FailureClassificationReceipt,
    GERBIL_LOOP_GRAPH_CONTINUATION_SCHEMA_ID, GERBIL_LOOP_GRAPH_POLICY_COMPILATION_SCHEMA_ID,
    GRAPH_POLICY_PROPOSAL_SCHEMA_ID, GerbilLoopGraphContinuationAction,
    GerbilLoopGraphContinuationCompileError, GerbilLoopGraphContinuationRequest,
    GerbilLoopGraphPolicyCompilationRequest, GraphId, GraphLoopContinuationAction,
    GraphLoopContinuationDecision, GraphLoopContinuationReceipt, GraphLoopEvent,
    GraphLoopEventEnvelope, GraphLoopEventId, GraphLoopEvidencePolicy, GraphLoopExecutionBudget,
    GraphLoopExecutionRequest, GraphLoopExecutionResult, GraphLoopExecutionStatus,
    GraphLoopFailureKind, GraphLoopGovernancePolicy, GraphLoopGovernedContextNamespace,
    GraphLoopGovernedSessionKind, GraphLoopInputDrainPolicy, GraphLoopInputLane,
    GraphLoopInputQueueReceipt, GraphLoopIterationId, GraphLoopIterationReport,
    GraphLoopMessageRole, GraphLoopNextAction, GraphLoopQueuedInput, GraphLoopRunRequest,
    GraphLoopSandboxBackend, GraphLoopSandboxPolicy, GraphLoopSessionPolicy, GraphLoopStatePolicy,
    GraphLoopStopPolicy, GraphLoopStopReason, GraphLoopStopReceipt, GraphLoopStrategy,
    GraphLoopStrategyId, GraphLoopStrategyRuntime, GraphLoopStrategyVersion,
    GraphLoopVerifierPolicy, GraphNativeAbiId, GraphNativeAbiReadinessReceipt,
    GraphNativeAbiReadinessStatus, GraphNativeAbiRequirement, GraphNativeSymbol,
    GraphNodeExecutionId, GraphNodeExecutionReceipt, GraphNodeExecutionStatus, GraphNodeInvocation,
    GraphPolicyDigest, GraphPolicyProposal, GraphPolicyProposalReceipt, GraphPolicyProposalStatus,
    GraphPolicyProposalValidationReport, GraphToolBatchDecision, GraphToolBatchExecutionMode,
    GraphToolBatchExecutionReceipt, GraphToolCallId, GraphToolCallReceipt, GraphToolCallStatus,
    HumanDecision, HumanDecisionReceipt, HumanGateId, HumanGateReceipt, HumanReviewKind,
    HumanReviewerId, LoopContinuationCapability, LoopContinuationPolicy, LoopEdgeSpec,
    LoopEvidenceCapturePolicy, LoopFailurePolicy, LoopGraph, LoopHumanGatePolicy, LoopMemoryPolicy,
    LoopModelRoutePolicy, LoopNodeSpec, LoopPolicyProfile, LoopPolicyProfileId, LoopQueuePolicy,
    LoopSelfEvolutionPolicy, LoopToolBatchPolicy, NodeId, RunId, RuntimePlanSnapshot,
    compile_gerbil_loop_graph, compile_gerbil_loop_graph_continuation,
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
pub use loop_policy_ir::{
    AuditLoopPolicyPack, BudgetCaps, CompiledLoopEdge, CompiledLoopNode, CompiledRouteBucket,
    CompiledRouteIndex, ContinuationOp, ForcedSlot, HotLoopPolicyPack, LOOP_PROGRAM_SCHEMA_VERSION,
    LoopMechanismPolicyId, LoopPolicyAgentProfileId, LoopPolicyConditionId,
    LoopPolicyDeltaTemplateId, LoopPolicyDiagnostic, LoopPolicyDiagnosticCode,
    LoopPolicyDiagnosticSeverity, LoopPolicyDigest, LoopPolicyEpoch, LoopPolicyExecutorId,
    LoopPolicyExplanation, LoopPolicyGateId, LoopPolicyGraphTemplateId, LoopPolicyNodeId,
    LoopPolicyReasonCode, LoopPolicyResourceClassId, LoopPolicyRoleId, LoopPolicyRouteBucketId,
    LoopPolicyRouteTargetId, LoopPolicySlotId, LoopPolicySourceLocationId, LoopPolicySourcePath,
    LoopProgram, LoopProgramActionKind, LoopProgramEventKind, LoopProgramId, LoopProgramInput,
    LoopProgramStateId, LoopProgramTransition, LoopProgramTransitionId,
    RESOLVED_LOOP_POLICY_PACK_SCHEMA_VERSION, ResolvedLoopPolicyPack, ResourceClass, SlotHotness,
    SlotMergeAlgebra, SlotMergeReceipt, SlotMergeStatus, SlotProvenance, SourceLocation,
};
pub use model_gateway::{
    ModelGateway, ModelGatewayCompletionChoice, ModelGatewayCompletionOptions,
    ModelGatewayCompletionResponse, ModelGatewayError, ModelGatewayFuture, ModelGatewayMessage,
    ModelGatewayMessageRole, ModelGatewayRequest, ModelGatewayResult, ModelGatewayTransport,
    assistant_gateway_message, system_gateway_message, user_gateway_message,
};
pub use model_route::{
    LiteLlmModelId, MODEL_ROUTE_ADMISSION_SCHEMA_ID, ModelAlias, ModelCommandKind,
    ModelCommandMatcher, ModelContextForkMode, ModelEndpoint, ModelEndpointContractError,
    ModelName, ModelProviderId, ModelRouteAdmissionMode, ModelRouteAdmissionRequest,
    ModelRouteAdmissionResponse, ModelRouteAgentScope, ModelRouteArtifactProjection,
    ModelRouteArtifactRef, ModelRouteDecision, ModelRouteEvidenceProfile, ModelRouteIntent,
    ModelRouteModality, ModelRoutePrecisionTier, ModelRoutePrivacyTier, ModelRouteReceipt,
    ModelRouteRequest, ModelRouteRule, ModelRouteRuleId, ModelRouteSessionId, ModelRouteSourceKind,
    ModelRouteTaskKind, ModelSessionLifecycle, ModelSessionPersistenceKey, ModelSessionPolicy,
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
    ProjectRuntimeAgentId, ProjectRuntimeBackendRequirementId, ProjectRuntimeBranchRef,
    ProjectRuntimeContentBodyRef, ProjectRuntimeContentId, ProjectRuntimeContextPackId,
    ProjectRuntimeEvidenceId, ProjectRuntimeIsolationRequirementId, ProjectRuntimeMemoryCitation,
    ProjectRuntimeMemoryCitationId, ProjectRuntimeMemoryId, ProjectRuntimeProjectId,
    ProjectRuntimeReceiptId, ProjectRuntimeRootSessionId, ProjectRuntimeSessionId,
    ProjectRuntimeSourceAnchorId, ProjectRuntimeSourceSpanRef, ProjectRuntimeSteeringItemId,
    ProjectRuntimeToolCapabilityCard, ProjectRuntimeToolCapabilityId, ProjectRuntimeTurnId,
    ProjectRuntimeWorkspaceId, ProjectRuntimeWorktreeId, TurnContextItemKind, TurnContextItemView,
    TurnContextItemViewReceipt, TurnContextOmissionReason, TurnContextOmittedItem,
    TurnContextSelectedItem, TurnContextSteeringReceipt, TurnContextSteeringReceiptInput,
};
pub use runtime_environment::{
    MARLIN_DEFAULT_HOME_DIR_NAME, MARLIN_HOME_ENV_VAR, RuntimeConfigLayer,
    RuntimeConfigLayerSource, RuntimeEnvironment, RuntimeEnvironmentActivation,
    RuntimeEnvironmentActivationAction, RuntimeEnvironmentActivationActionReceipt,
    RuntimeEnvironmentActivationActionStatus, RuntimeEnvironmentActivationPolicy,
    RuntimeEnvironmentActivationReceipt, RuntimeEnvironmentActivationStatus,
    RuntimeEnvironmentDelta, RuntimeEnvironmentRefreshCachePolicy,
    RuntimeEnvironmentRefreshExecution, RuntimeEnvironmentRefreshReceipt,
    RuntimeEnvironmentRefreshStatus, RuntimeEnvironmentRefreshTimeout,
    RuntimeEnvironmentResolution, RuntimeEnvrcPolicy, RuntimeHome, RuntimeHomeSource,
    RuntimeSandboxPolicy, RuntimeShellIsolationPolicy, RuntimeStateDirectory,
    RuntimeStateDirectoryKind, RuntimeStateLayout, RuntimeStateObjectFileStem,
    RuntimeStateObjectKey, RuntimeStateObjectKind, RuntimeStateObjectPath,
    RuntimeStateStorageReceipt, RuntimeStateStorageStatus, RuntimeWorkspaceProject,
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
