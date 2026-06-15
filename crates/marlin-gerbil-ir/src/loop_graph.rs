//! Typed loop graph `IR` emitted by the `Gerbil` control plane.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Compiled graph specification ready for Rust-side validation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CompiledLoopGraph {
    pub graph_id: String,
    pub nodes: Vec<LoopNodeSpec>,
    pub edges: Vec<LoopEdgeSpec>,
}

impl CompiledLoopGraph {
    /// Validates graph shape before any runtime executor can schedule it.
    pub fn validate(&self) -> Result<(), LoopGraphValidationError> {
        if self.graph_id.trim().is_empty() {
            return Err(LoopGraphValidationError::EmptyGraphId);
        }

        let mut node_ids = BTreeSet::new();
        for (index, node) in self.nodes.iter().enumerate() {
            if node.id.trim().is_empty() {
                return Err(LoopGraphValidationError::EmptyNodeId { index });
            }
            if node.executor.trim().is_empty() {
                return Err(LoopGraphValidationError::EmptyNodeExecutor {
                    node_id: node.id.clone(),
                });
            }
            if !node_ids.insert(node.id.clone()) {
                return Err(LoopGraphValidationError::DuplicateNodeId {
                    node_id: node.id.clone(),
                });
            }
        }

        for (index, edge) in self.edges.iter().enumerate() {
            if !node_ids.contains(&edge.from) {
                return Err(LoopGraphValidationError::UnknownEdgeSource {
                    edge_index: index,
                    node_id: edge.from.clone(),
                });
            }
            if !node_ids.contains(&edge.to) {
                return Err(LoopGraphValidationError::UnknownEdgeTarget {
                    edge_index: index,
                    node_id: edge.to.clone(),
                });
            }
        }

        Ok(())
    }

    /// Compiles validated graph intent into bounded acyclic execution frontiers.
    pub fn compile_execution_plan(
        &self,
        limits: LoopGraphCompileLimits,
    ) -> Result<LoopGraphExecutionPlan, LoopGraphCompileError> {
        let required_node_executions = self.validate_compile_inputs(limits)?;
        let compile_graph = LoopGraphCompileAdjacency::from_compiled_graph(self);
        let frontiers = compile_graph.into_execution_frontiers(&self.nodes)?;

        Ok(LoopGraphExecutionPlan {
            graph_id: self.graph_id.clone(),
            frontiers,
            required_node_executions,
        })
    }

    fn validate_compile_inputs(
        &self,
        limits: LoopGraphCompileLimits,
    ) -> Result<u64, LoopGraphCompileError> {
        self.validate().map_err(LoopGraphCompileError::Validation)?;

        if self.nodes.is_empty() {
            return Err(LoopGraphCompileError::EmptyGraph);
        }

        let required_node_executions = self.nodes.len() as u64;
        if let Some(max_node_executions) = limits.max_node_executions
            && required_node_executions > max_node_executions
        {
            return Err(LoopGraphCompileError::NodeExecutionBudgetExceeded {
                max_node_executions,
                required_node_executions,
            });
        }

        Ok(required_node_executions)
    }
}

struct LoopGraphCompileAdjacency {
    incoming_counts: BTreeMap<String, usize>,
    outgoing_edges: BTreeMap<String, Vec<String>>,
}

impl LoopGraphCompileAdjacency {
    fn from_compiled_graph(graph: &CompiledLoopGraph) -> Self {
        let mut adjacency = Self {
            incoming_counts: graph
                .nodes
                .iter()
                .map(|node| (node.id.clone(), 0))
                .collect(),
            outgoing_edges: graph
                .nodes
                .iter()
                .map(|node| (node.id.clone(), Vec::new()))
                .collect(),
        };

        for edge in &graph.edges {
            adjacency.add_edge(edge);
        }

        adjacency
    }

    fn add_edge(&mut self, edge: &LoopEdgeSpec) {
        self.outgoing_edges
            .entry(edge.from.clone())
            .or_default()
            .push(edge.to.clone());
        *self.incoming_counts.entry(edge.to.clone()).or_default() += 1;
    }

    fn into_execution_frontiers(
        mut self,
        declared_nodes: &[LoopNodeSpec],
    ) -> Result<Vec<LoopGraphExecutionFrontier>, LoopGraphCompileError> {
        let mut traversal = LoopGraphFrontierTraversal::new(self.ready_node_ids());

        while let Some(frontier_node_ids) = traversal.take_ready_frontier() {
            let next_ready =
                self.consume_frontier_nodes(&frontier_node_ids, traversal.processed_nodes_mut());
            traversal.record_frontier(frontier_node_ids, next_ready);
        }

        traversal.finish(declared_nodes)
    }

    fn ready_node_ids(&self) -> BTreeSet<String> {
        self.incoming_counts
            .iter()
            .filter_map(|(node_id, count)| (*count == 0).then_some(node_id.clone()))
            .collect()
    }

    fn consume_frontier_nodes(
        &mut self,
        frontier_node_ids: &[String],
        processed_nodes: &mut BTreeSet<String>,
    ) -> BTreeSet<String> {
        frontier_node_ids
            .iter()
            .flat_map(|node_id| self.consume_frontier_node(node_id, processed_nodes))
            .collect()
    }

    fn consume_frontier_node(
        &mut self,
        node_id: &str,
        processed_nodes: &mut BTreeSet<String>,
    ) -> Vec<String> {
        processed_nodes.insert(node_id.to_string());
        self.outgoing_edges
            .get(node_id)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter(|target_id| self.reduce_incoming_count(target_id))
            .collect()
    }

    fn reduce_incoming_count(&mut self, node_id: &str) -> bool {
        let Some(incoming_count) = self.incoming_counts.get_mut(node_id) else {
            return false;
        };
        *incoming_count -= 1;
        *incoming_count == 0
    }
}

struct LoopGraphFrontierTraversal {
    ready: BTreeSet<String>,
    processed_nodes: BTreeSet<String>,
    frontiers: Vec<LoopGraphExecutionFrontier>,
}

impl LoopGraphFrontierTraversal {
    fn new(ready: BTreeSet<String>) -> Self {
        Self {
            ready,
            processed_nodes: BTreeSet::new(),
            frontiers: Vec::new(),
        }
    }

    fn take_ready_frontier(&mut self) -> Option<Vec<String>> {
        (!self.ready.is_empty()).then(|| self.ready.iter().cloned().collect())
    }

    fn processed_nodes_mut(&mut self) -> &mut BTreeSet<String> {
        &mut self.processed_nodes
    }

    fn record_frontier(&mut self, node_ids: Vec<String>, next_ready: BTreeSet<String>) {
        self.frontiers.push(LoopGraphExecutionFrontier { node_ids });
        self.ready = next_ready;
    }

    fn finish(
        self,
        declared_nodes: &[LoopNodeSpec],
    ) -> Result<Vec<LoopGraphExecutionFrontier>, LoopGraphCompileError> {
        if self.processed_nodes.len() == declared_nodes.len() {
            return Ok(self.frontiers);
        }

        Err(LoopGraphCompileError::CycleDetected {
            remaining_node_ids: declared_nodes
                .iter()
                .filter_map(|node| {
                    (!self.processed_nodes.contains(&node.id)).then_some(node.id.clone())
                })
                .collect(),
        })
    }
}

/// Rust-side compile limits for Gerbil-produced loop graph intent.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct LoopGraphCompileLimits {
    pub max_node_executions: Option<u64>,
}

impl LoopGraphCompileLimits {
    /// Returns true when no compile-time limits are configured.
    pub fn is_default(&self) -> bool {
        self == &Self::default()
    }
}

/// Runtime-ready graph loop plan owned by Rust.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoopGraphExecutionPlan {
    pub graph_id: String,
    pub frontiers: Vec<LoopGraphExecutionFrontier>,
    pub required_node_executions: u64,
}

/// A single acyclic frontier whose nodes may be scheduled by Rust runtime.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoopGraphExecutionFrontier {
    pub node_ids: Vec<String>,
}

/// Node specification inside a compiled loop graph.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LoopNodeSpec {
    pub id: String,
    pub executor: String,
    pub config: BTreeMap<String, String>,
}

/// Directed edge specification between loop graph nodes.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LoopEdgeSpec {
    pub from: String,
    pub to: String,
    pub condition: Option<String>,
}

/// Rust-owned validation errors for compiled loop graph IR.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LoopGraphValidationError {
    EmptyGraphId,
    EmptyNodeId { index: usize },
    EmptyNodeExecutor { node_id: String },
    DuplicateNodeId { node_id: String },
    UnknownEdgeSource { edge_index: usize, node_id: String },
    UnknownEdgeTarget { edge_index: usize, node_id: String },
}

/// Rust-owned compilation errors for runtime-ready loop plans.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LoopGraphCompileError {
    Validation(LoopGraphValidationError),
    EmptyGraph,
    CycleDetected {
        remaining_node_ids: Vec<String>,
    },
    NodeExecutionBudgetExceeded {
        max_node_executions: u64,
        required_node_executions: u64,
    },
}
