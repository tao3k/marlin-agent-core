//! Agent protocol contracts for graph loops, scenarios, transcripts, and evidence.

mod event;
mod evidence;
mod graph;
mod hook;
mod runtime_environment;
mod scenario;
mod sub_agent;

pub use event::AgentEvent;
pub use evidence::{LoopEvidence, LoopEvidenceKind};
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
pub use scenario::{AgentScenario, AgentScenarioStep};
pub use sub_agent::{SubAgentActivity, SubAgentActivityKind, SubAgentSource};
