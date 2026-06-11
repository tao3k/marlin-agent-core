//! Agent protocol contracts for graph loops, scenarios, transcripts, and evidence.

mod event;
mod evidence;
mod graph;
mod hook;
mod runtime_environment;
mod scenario;
mod sub_agent;
mod trace;

pub use event::{AgentEvent, AgentEventTopic};
pub use evidence::{
    LoopEvidence, LoopEvidenceKind, LoopPerformanceEvidence,
    PERFORMANCE_EVIDENCE_ALLOCATION_PROFILE, PERFORMANCE_EVIDENCE_BASELINE,
    PERFORMANCE_EVIDENCE_BENCHMARK_COMMAND, PERFORMANCE_EVIDENCE_KEYS,
    PERFORMANCE_EVIDENCE_LATENCY_OR_THROUGHPUT, PERFORMANCE_EVIDENCE_PROFILE_ARTIFACT,
    PERFORMANCE_EVIDENCE_REGRESSION_THRESHOLD,
};
pub use graph::{
    ExecutorName, GraphId, GraphLoopExecutionRequest, GraphLoopExecutionResult,
    GraphLoopExecutionStatus, GraphNodeExecutionReceipt, GraphNodeExecutionStatus,
    GraphNodeInvocation, LoopEdgeSpec, LoopGraph, LoopNodeSpec, NodeId, RunId, RuntimePlanSnapshot,
};
pub use hook::{
    HookDurationMs, HookEventName, HookExecutionMode, HookHandlerType, HookOutputEntry,
    HookOutputEntryKind, HookRunId, HookRunStatus, HookRunSummary, HookScope, HookSource,
    HookSourcePath, HookTimestampMs, HookTrustStatus,
};
pub use runtime_environment::{
    RuntimeConfigLayer, RuntimeConfigLayerSource, RuntimeEnvironment, RuntimeHome,
    RuntimeHomeSource, RuntimeSandboxPolicy,
};
pub use scenario::{
    AGENT_SCENARIO_CONTRACT_SCHEMA_ID, AgentScenario, AgentScenarioContract, AgentScenarioStep,
};
pub use sub_agent::{SubAgentActivity, SubAgentActivityKind, SubAgentSource};
pub use trace::{
    AgentExecutionTrace, AgentExecutionTraceSummary, AgentSpanName, AgentTraceSpanRecord,
};
