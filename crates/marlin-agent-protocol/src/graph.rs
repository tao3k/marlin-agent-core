//! Graph-loop request, receipt, snapshot, and identifier contracts.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Runtime-ready loop graph produced from typed control-plane `IR`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LoopGraph {
    pub graph_id: String,
    pub nodes: Vec<LoopNodeSpec>,
    pub edges: Vec<LoopEdgeSpec>,
}

/// Protocol-owned node specification inside a runtime loop graph.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LoopNodeSpec {
    pub id: String,
    pub executor: String,
    pub config: BTreeMap<String, String>,
}

/// Protocol-owned directed edge specification between runtime graph nodes.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LoopEdgeSpec {
    pub from: String,
    pub to: String,
    pub condition: Option<String>,
}

/// Stable view of a running graph loop for status and recovery.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimePlanSnapshot {
    pub run_id: String,
    pub graph_id: String,
    pub active_node: Option<String>,
}

/// Request to execute a compiled graph loop on a Tokio-backed runtime.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GraphLoopExecutionRequest {
    pub run_id: String,
    pub graph: LoopGraph,
}

impl GraphLoopExecutionRequest {
    pub fn new(run_id: impl Into<String>, graph: LoopGraph) -> Self {
        Self {
            run_id: run_id.into(),
            graph,
        }
    }
}

/// Terminal status for a graph-loop execution.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GraphLoopExecutionStatus {
    Completed,
    Cancelled,
    Failed,
}

/// Receipt returned when a graph-loop execution reaches a terminal status.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GraphLoopExecutionResult {
    pub status: GraphLoopExecutionStatus,
    pub snapshot: RuntimePlanSnapshot,
    pub visited_nodes: Vec<String>,
    pub diagnostics: Vec<String>,
}

impl GraphLoopExecutionResult {
    pub fn completed(snapshot: RuntimePlanSnapshot, visited_nodes: Vec<String>) -> Self {
        Self {
            status: GraphLoopExecutionStatus::Completed,
            snapshot,
            visited_nodes,
            diagnostics: Vec::new(),
        }
    }

    pub fn cancelled(snapshot: RuntimePlanSnapshot, visited_nodes: Vec<String>) -> Self {
        Self {
            status: GraphLoopExecutionStatus::Cancelled,
            snapshot,
            visited_nodes,
            diagnostics: Vec::new(),
        }
    }

    pub fn failed(snapshot: RuntimePlanSnapshot, diagnostics: Vec<String>) -> Self {
        Self::failed_with_visited(snapshot, Vec::new(), diagnostics)
    }

    pub fn failed_with_visited(
        snapshot: RuntimePlanSnapshot,
        visited_nodes: Vec<String>,
        diagnostics: Vec<String>,
    ) -> Self {
        Self {
            status: GraphLoopExecutionStatus::Failed,
            snapshot,
            visited_nodes,
            diagnostics,
        }
    }

    pub fn with_diagnostics(mut self, diagnostics: Vec<String>) -> Self {
        self.diagnostics = diagnostics;
        self
    }
}

/// Stable graph-loop run identifier.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct RunId(String);

impl RunId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

impl From<String> for RunId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for RunId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// Stable compiled graph identifier.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct GraphId(String);

impl GraphId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

impl From<String> for GraphId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for GraphId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// Stable graph node identifier.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct NodeId(String);

impl NodeId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

impl From<String> for NodeId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for NodeId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// Named executor slot selected by a compiled graph node.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct ExecutorName(String);

impl ExecutorName {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

impl From<String> for ExecutorName {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for ExecutorName {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// Invocation passed from the graph-loop kernel to a node executor.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GraphNodeInvocation {
    pub run_id: RunId,
    pub graph_id: GraphId,
    pub node_id: NodeId,
    pub executor: ExecutorName,
    pub config: BTreeMap<String, String>,
}

impl GraphNodeInvocation {
    pub fn from_loop_node(run_id: RunId, graph_id: GraphId, node: &LoopNodeSpec) -> Self {
        Self {
            run_id,
            graph_id,
            node_id: NodeId::new(node.id.clone()),
            executor: ExecutorName::new(node.executor.clone()),
            config: node.config.clone(),
        }
    }
}

/// Terminal status for one graph node executor invocation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GraphNodeExecutionStatus {
    Completed,
    Failed,
}

/// Receipt returned by a graph node executor.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GraphNodeExecutionReceipt {
    pub status: GraphNodeExecutionStatus,
    pub node_id: NodeId,
    pub executor: ExecutorName,
    pub diagnostics: Vec<String>,
}

impl GraphNodeExecutionReceipt {
    pub fn completed(node_id: impl Into<NodeId>, executor: impl Into<ExecutorName>) -> Self {
        Self {
            status: GraphNodeExecutionStatus::Completed,
            node_id: node_id.into(),
            executor: executor.into(),
            diagnostics: Vec::new(),
        }
    }

    pub fn failed(
        node_id: impl Into<NodeId>,
        executor: impl Into<ExecutorName>,
        diagnostics: Vec<String>,
    ) -> Self {
        Self {
            status: GraphNodeExecutionStatus::Failed,
            node_id: node_id.into(),
            executor: executor.into(),
            diagnostics,
        }
    }
}
