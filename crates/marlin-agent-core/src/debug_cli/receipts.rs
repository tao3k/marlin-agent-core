//! JSON receipts emitted by the `marlin` harness/debug CLI.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{
    GraphId, GraphLoopExecutionResult, GraphLoopExecutionStatus, GraphLoopIterationReport, RunId,
    RuntimeHomeSource,
    protocol::{
        GraphLoopFailureKind, GraphQueryFamily, GraphQueryRelationshipFact,
        ModelRouteAdmissionRequest, ModelRouteAdmissionResponse, ProjectRuntimeAgentId,
        ProjectRuntimeContentId, ProjectRuntimeEvidenceId, ProjectRuntimeMemoryId,
        ProjectRuntimeProjectId, ProjectRuntimeReceiptId, ProjectRuntimeRootSessionId,
        ProjectRuntimeSessionId, ProjectRuntimeSourceAnchorId, ProjectRuntimeToolCapabilityId,
    },
    runtime::GraphLoopRunObservation,
};

/// Summary returned by `marlin graph query`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GraphQuerySummary {
    pub graph_id: String,
    pub node_count: usize,
    pub edge_count: usize,
    pub executors: Vec<String>,
    pub root_nodes: Vec<String>,
}

/// Summary returned by `marlin graph query` for loop reports.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LoopQuerySummary {
    pub run_ids: Vec<RunId>,
    pub graph_ids: Vec<GraphId>,
    pub iteration_count: usize,
    pub terminal_status: Option<GraphLoopExecutionStatus>,
    pub replayable: bool,
    pub missing_trace_count: usize,
    pub statuses: Vec<GraphLoopExecutionStatus>,
    pub visited_nodes_by_iteration: Vec<Vec<String>>,
    pub diagnostic_count: usize,
    pub node_receipt_count: usize,
    pub continuation_receipt_count: usize,
    pub human_gate_receipt_count: usize,
    pub human_decision_receipt_count: usize,
    pub failure_classification_receipt_count: usize,
    pub failure_kinds: Vec<GraphLoopFailureKind>,
    pub trace_event_count: usize,
}

/// Summary returned by `marlin graph query` for graph-loop events.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LoopEventQuerySummary {
    pub run_ids: Vec<RunId>,
    pub event_count: usize,
    pub iteration_ids: Vec<u64>,
    pub node_ids: Vec<String>,
    pub trace_ids: Vec<String>,
    pub event_types: Vec<String>,
    pub tool_event_count: usize,
    pub terminal_status: Option<GraphLoopExecutionStatus>,
}

/// Summary returned by `marlin graph query` for project-runtime graph receipts.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProjectRuntimeQuerySummary {
    pub receipt_id: ProjectRuntimeReceiptId,
    pub family: GraphQueryFamily,
    pub query: String,
    pub match_count: usize,
    pub source_project_ids: Vec<ProjectRuntimeProjectId>,
    pub source_root_session_ids: Vec<ProjectRuntimeRootSessionId>,
    pub source_session_ids: Vec<ProjectRuntimeSessionId>,
    pub source_agent_ids: Vec<ProjectRuntimeAgentId>,
    pub source_anchor_ids: Vec<ProjectRuntimeSourceAnchorId>,
    pub memory_ids: Vec<ProjectRuntimeMemoryId>,
    pub content_ids: Vec<ProjectRuntimeContentId>,
    pub tool_capability_ids: Vec<ProjectRuntimeToolCapabilityId>,
    pub evidence_ids: Vec<ProjectRuntimeEvidenceId>,
    pub match_receipt_ids: Vec<ProjectRuntimeReceiptId>,
    pub relationship_facts: Vec<GraphQueryRelationshipFact>,
    pub score_basis_points: Vec<u16>,
}

/// Read-only graph query output.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GraphQueryOutput {
    Graph(GraphQuerySummary),
    Loop(LoopQuerySummary),
    LoopEvents(LoopEventQuerySummary),
    ProjectRuntime(ProjectRuntimeQuerySummary),
}

/// Run receipt returned by `marlin loop run`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LoopRunReceipt {
    pub run_id: RunId,
    pub report_path: Option<PathBuf>,
    pub iteration_count: usize,
    pub terminal_status: Option<GraphLoopExecutionStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_observation: Option<GraphLoopRunObservation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub governance_receipt: Option<LoopGovernanceReceipt>,
    pub replayable: bool,
    pub missing_trace_count: usize,
    pub reports: Vec<GraphLoopIterationReport>,
}

/// Governance receipt returned by `marlin loop run` when a request carries governance policy.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LoopGovernanceReceipt {
    pub run_id: RunId,
    pub state: LoopGovernanceStateReceipt,
    pub sandbox: LoopGovernanceSandboxReceipt,
    pub session: LoopGovernanceSessionReceipt,
    pub verifier: LoopGovernanceVerifierReceipt,
}

/// State receipt for one governed loop request.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LoopGovernanceStateReceipt {
    pub read_before_run: bool,
    pub write_receipt_on_pass: bool,
    pub state_ref: Option<String>,
}

/// Sandbox materialization receipt for one governed loop request.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LoopGovernanceSandboxReceipt {
    pub backend: String,
    pub profile_ref: String,
    pub filesystem_scope: Option<String>,
    pub network_access: bool,
    pub runtime_owner: String,
    pub materialized_by: String,
}

/// Session isolation receipt projection for one governed loop request.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LoopGovernanceSessionReceipt {
    pub parent_session_id: String,
    pub child_session_id: String,
    pub requested_namespaces: Vec<String>,
    pub granted_namespaces: Vec<String>,
    pub denied_namespaces: Vec<String>,
    pub max_history_items: Option<usize>,
}

/// Verifier decision receipt for one governed loop request.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LoopGovernanceVerifierReceipt {
    pub decision: LoopGovernanceVerifierDecision,
    pub terminal_status: Option<GraphLoopExecutionStatus>,
    pub retryable: bool,
    pub human_audit_required: bool,
    pub diagnostics: Vec<String>,
}

/// State machine outcome selected by the governed loop verifier.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LoopGovernanceVerifierDecision {
    Passed,
    Retry,
    HumanAudit,
}

/// File-mode replay receipt returned by `marlin loop replay`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LoopReplayReceipt {
    pub source: PathBuf,
    pub replayable: bool,
    pub iteration_count: usize,
    pub missing_trace_count: usize,
    pub statuses: Vec<GraphLoopExecutionStatus>,
    pub run_ids: Vec<RunId>,
    pub graph_ids: Vec<GraphId>,
    pub diagnostics: Vec<String>,
}

/// Run-store inspection receipt returned by `marlin loop inspect`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LoopInspectReceipt {
    pub run_id: RunId,
    pub report_path: PathBuf,
    pub iteration_count: usize,
    pub terminal_status: Option<GraphLoopExecutionStatus>,
    pub terminal_graph_id: Option<GraphId>,
    pub replayable: bool,
    pub missing_trace_count: usize,
    pub diagnostics: Vec<String>,
}

/// Runtime smoke scenario executed by `marlin smoke runtime`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SmokeRuntimeScenario {
    BuiltinAdapters,
    ModelRouteDryRun,
    ProcessCommandFanout,
    StateHomeEnv,
}

/// LLM mode used by a smoke scenario.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SmokeLlmMode {
    NoLiveLlm,
}

/// Runtime smoke receipt returned by `marlin smoke runtime`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SmokeRuntimeReceipt {
    pub scenario: SmokeRuntimeScenario,
    pub llm_mode: SmokeLlmMode,
    pub run_id: String,
    pub graph_id: String,
    pub terminal_status: GraphLoopExecutionStatus,
    pub passed: bool,
    pub node_count: usize,
    pub visited_nodes: Vec<String>,
    pub node_receipt_count: usize,
    pub completed_node_receipt_count: usize,
    pub failed_node_receipt_count: usize,
    pub tool_spawn_count: usize,
    pub provider_spawn_count: usize,
    pub subagent_spawn_count: usize,
    pub process_spawn_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state_home: Option<SmokeRuntimeStateHome>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_route: Option<SmokeRuntimeModelRouteDryRun>,
    pub diagnostics: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_result: Option<GraphLoopExecutionResult>,
}

/// State-home paths resolved by a runtime smoke scenario.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SmokeRuntimeStateHome {
    pub home: PathBuf,
    pub source: RuntimeHomeSource,
    pub directory_count: usize,
    pub session_path: PathBuf,
    pub memory_shard_path: PathBuf,
    pub receipt_path: PathBuf,
    pub graph_cache_path: PathBuf,
}

/// Deterministic model-route dry-run facts resolved by a runtime smoke scenario.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SmokeRuntimeModelRouteDryRun {
    pub rule_count: usize,
    pub request: ModelRouteAdmissionRequest,
    pub response: ModelRouteAdmissionResponse,
}
