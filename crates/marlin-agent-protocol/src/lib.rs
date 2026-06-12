//! Agent protocol contracts for graph loops, scenarios, transcripts, and evidence.

mod event;
mod evidence;
mod graph;
mod hook;
mod model_route;
mod runtime_environment;
mod scenario;
mod sub_agent;
mod trace;

pub use event::{AgentEvent, AgentEventTopic};
pub use evidence::{
    LoopEvidence, LoopEvidenceKind, LoopPerformanceEvidence, LoopStabilityEvidence,
    PERFORMANCE_EVIDENCE_ALLOCATION_PROFILE, PERFORMANCE_EVIDENCE_BASELINE,
    PERFORMANCE_EVIDENCE_BENCHMARK_COMMAND, PERFORMANCE_EVIDENCE_KEYS,
    PERFORMANCE_EVIDENCE_LATENCY_OR_THROUGHPUT, PERFORMANCE_EVIDENCE_PROFILE_ARTIFACT,
    PERFORMANCE_EVIDENCE_REGRESSION_THRESHOLD, STABILITY_EVIDENCE_ARTIFACT,
    STABILITY_EVIDENCE_COMMAND, STABILITY_EVIDENCE_DETERMINISM,
    STABILITY_EVIDENCE_ITERATION_WINDOW, STABILITY_EVIDENCE_KEYS,
    STABILITY_EVIDENCE_LATENCY_DISTRIBUTION, STABILITY_EVIDENCE_RESOURCE_DELTA,
    STABILITY_EVIDENCE_STATE_GROWTH,
};
pub use graph::{
    ExecutorName, GraphId, GraphLoopExecutionRequest, GraphLoopExecutionResult,
    GraphLoopExecutionStatus, GraphNodeExecutionReceipt, GraphNodeExecutionStatus,
    GraphNodeInvocation, LoopEdgeSpec, LoopGraph, LoopNodeSpec, NodeId, RunId, RuntimePlanSnapshot,
};
pub use hook::{
    HookAgentScope, HookConfigurationReloadReceipt, HookConfigurationVersion,
    HookDispatchPolicyReceipt, HookDispatchPolicyReceiptInput, HookDispatchSelectionInput,
    HookDispatchSelectionReceipt, HookDurationMs, HookEventName, HookExecutionMode,
    HookHandlerType, HookMatcherStrategy, HookMatcherToken, HookOutputEntry, HookOutputEntryKind,
    HookPolicyDecision, HookPolicyDecisionReason, HookPolicyDecisionReceipt, HookPolicyExtension,
    HookPolicyExtensionKind, HookPolicyMode, HookRegistryUpdateKind, HookRegistryUpdateReceipt,
    HookRunId, HookRunStatus, HookRunSummary, HookSchemeModule, HookSchemeProcedure, HookScope,
    HookSelectedCandidateInput, HookSelectionCandidateReceipt, HookSelectionSkipReason,
    HookSkippedCandidateInput, HookSource, HookSourcePath, HookTimestampMs, HookTrustStatus,
};
pub use model_route::{
    LiteLlmModelId, ModelAlias, ModelCommandKind, ModelCommandMatcher, ModelContextForkMode,
    ModelEndpoint, ModelEndpointContractError, ModelName, ModelProviderId, ModelRouteAgentScope,
    ModelRouteDecision, ModelRouteReceipt, ModelRouteRequest, ModelRouteRule, ModelRouteRuleId,
    ModelRouteSessionId, ModelSessionLifecycle, ModelSessionPersistenceKey, ModelSessionPolicy,
    ModelSessionPoolId,
};
pub use runtime_environment::{
    RuntimeConfigLayer, RuntimeConfigLayerSource, RuntimeEnvironment, RuntimeEnvironmentActivation,
    RuntimeEnvironmentActivationPolicy, RuntimeEnvironmentActivationReceipt,
    RuntimeEnvironmentActivationStatus, RuntimeEnvironmentDelta, RuntimeEnvironmentResolution,
    RuntimeEnvrcPolicy, RuntimeHome, RuntimeHomeSource, RuntimeSandboxPolicy,
    RuntimeShellIsolationPolicy,
};
pub use scenario::{
    AGENT_SCENARIO_CONTRACT_SCHEMA_ID, AgentScenario, AgentScenarioContract, AgentScenarioStep,
};
pub use sub_agent::{
    SubAgentActivity, SubAgentActivityKind, SubAgentConfigSurface, SubAgentContextPolicy,
    SubAgentContextVisibility, SubAgentPerformanceBudget, SubAgentPermissionSet,
    SubAgentSearchReceipt, SubAgentSource, SubAgentSpawnConfig, SubAgentSpawnPolicy,
    SubAgentSpawnProfile, SubAgentSpawnStrategy, SubAgentType,
};
pub use trace::{
    AgentExecutionTrace, AgentExecutionTraceSummary, AgentSpanName, AgentTraceSpanRecord,
};
