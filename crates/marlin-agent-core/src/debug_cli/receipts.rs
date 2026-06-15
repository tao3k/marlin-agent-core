//! JSON receipts emitted by the `marlin` harness/debug CLI.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{
    GraphId, GraphLoopExecutionStatus, GraphLoopIterationReport, RunId,
    protocol::{
        GraphQueryFamily, GraphQueryRelationshipFact, ProjectRuntimeContentId,
        ProjectRuntimeMemoryId, ProjectRuntimeProjectId, ProjectRuntimeReceiptId,
        ProjectRuntimeRootSessionId, ProjectRuntimeSessionId,
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
    pub memory_ids: Vec<ProjectRuntimeMemoryId>,
    pub content_ids: Vec<ProjectRuntimeContentId>,
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
    pub replayable: bool,
    pub missing_trace_count: usize,
    pub reports: Vec<GraphLoopIterationReport>,
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
