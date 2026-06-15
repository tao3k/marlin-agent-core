//! JSON receipts emitted by the `marlin` harness/debug CLI.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{GraphId, GraphLoopExecutionStatus, GraphLoopIterationReport, RunId};

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

/// Read-only graph query output.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GraphQueryOutput {
    Graph(GraphQuerySummary),
    Loop(LoopQuerySummary),
}

/// Run receipt returned by `marlin loop run`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LoopRunReceipt {
    pub run_id: RunId,
    pub report_path: Option<PathBuf>,
    pub iteration_count: usize,
    pub terminal_status: Option<GraphLoopExecutionStatus>,
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
