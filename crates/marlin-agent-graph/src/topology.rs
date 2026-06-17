//! Topology contracts for agent nodes, edges, and graph validation.

use std::collections::HashSet;
use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::ids::{
    AgentCapability, AgentDelegationReason, AgentEdgeId, AgentGraphId, AgentNodeId,
    AgentPolicyDecisionRef, AgentRole, GerbilPolicyScopeRef, GraphLoopGraphRef, GraphLoopNodeRef,
    OrgMemoryScopeRef,
};

/// Topology graph over agents, not tools or runtime processes.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentGraph {
    /// Stable graph identifier.
    pub graph_id: AgentGraphId,
    /// Agent nodes that may each execute a graph loop through a controller.
    pub nodes: Vec<AgentNode>,
    /// Directed topology edges between agent nodes.
    pub edges: Vec<AgentEdge>,
    /// Routing policy metadata for topology selection.
    pub topology_policy: AgentTopologyPolicy,
}

impl AgentGraph {
    /// Validates node and edge identity consistency without executing anything.
    pub fn validate(&self) -> Result<(), AgentGraphTopologyError> {
        if self.nodes.is_empty() {
            return Err(AgentGraphTopologyError::EmptyGraph {
                graph_id: self.graph_id.clone(),
            });
        }

        let mut node_ids = HashSet::new();
        for node in &self.nodes {
            if !node_ids.insert(node.node_id.clone()) {
                return Err(AgentGraphTopologyError::DuplicateNode {
                    node_id: node.node_id.clone(),
                });
            }
        }

        let mut edge_ids = HashSet::new();
        for edge in &self.edges {
            if !edge_ids.insert(edge.edge_id.clone()) {
                return Err(AgentGraphTopologyError::DuplicateEdge {
                    edge_id: edge.edge_id.clone(),
                });
            }

            if !node_ids.contains(&edge.from) {
                return Err(AgentGraphTopologyError::MissingEdgeSource {
                    edge_id: edge.edge_id.clone(),
                    node_id: edge.from.clone(),
                });
            }

            if !node_ids.contains(&edge.to) {
                return Err(AgentGraphTopologyError::MissingEdgeTarget {
                    edge_id: edge.edge_id.clone(),
                    node_id: edge.to.clone(),
                });
            }
        }

        Ok(())
    }
}

/// Agent node metadata for a graph-loop-backed agent.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentNode {
    /// Stable node identifier inside an `AgentGraph`.
    pub node_id: AgentNodeId,
    /// Role label used for topology and policy decisions.
    pub role: AgentRole,
    /// Capability labels advertised by this node.
    pub capabilities: Vec<AgentCapability>,
    /// Referenced graph-loop entrypoint.
    pub loop_entry: GraphLoopEntryRef,
    /// Optional Org memory scope reference.
    pub memory_scope: Option<OrgMemoryScopeRef>,
    /// Optional Gerbil policy scope reference.
    pub policy_scope: Option<GerbilPolicyScopeRef>,
}

/// Reference to a graph-loop entrypoint.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GraphLoopEntryRef {
    /// Referenced `LoopGraph` identity.
    pub graph: GraphLoopGraphRef,
    /// Referenced entry node inside the `LoopGraph`.
    pub entry_node: GraphLoopNodeRef,
}

/// Directed edge between two `AgentNode` values.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentEdge {
    /// Stable edge identifier.
    pub edge_id: AgentEdgeId,
    /// Source node identifier.
    pub from: AgentNodeId,
    /// Target node identifier.
    pub to: AgentNodeId,
    /// Coordination meaning for this edge.
    pub kind: AgentEdgeKind,
    /// Condition that must hold before this edge is selected.
    pub condition: AgentEdgeCondition,
}

/// Topology-level edge semantics.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum AgentEdgeKind {
    /// Transfer control to another agent.
    Handoff,
    /// Delegate a scoped unit of work.
    Delegate,
    /// Ask another agent to review output.
    Review,
    /// Split work across multiple downstream agents.
    Fanout,
    /// Join downstream work into a later agent.
    Fanin,
    /// Route through a voting step.
    Vote,
    /// Escalate to a higher authority or class of agent.
    Escalate,
}

/// Typed condition for edge selection.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum AgentEdgeCondition {
    /// Edge is always eligible.
    Always,
    /// Edge requires a node with a matching capability.
    CapabilityRequired(AgentCapability),
    /// Edge selection depends on an external policy receipt.
    PolicyDecision(AgentPolicyDecisionRef),
}

/// Policy selector for topology routing.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum AgentTopologyPolicy {
    /// Deterministic routing in declared graph order.
    Deterministic,
    /// Prefer nodes whose capabilities satisfy the edge condition.
    CapabilityFirst,
    /// Delegate routing choice to a typed Gerbil policy scope.
    PolicyScoped(GerbilPolicyScopeRef),
}

/// Delegation contract emitted before executing a downstream node.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentDelegation {
    /// Graph that owns this delegation.
    pub graph_id: AgentGraphId,
    /// Delegating source node.
    pub from: AgentNodeId,
    /// Delegation target node.
    pub to: AgentNodeId,
    /// Edge used for delegation.
    pub via_edge: AgentEdgeId,
    /// Typed reason for the delegation.
    pub reason: AgentDelegationReason,
}

/// Runtime-consumable coordination plan that keeps topology semantics upstream.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentCoordinationPlan {
    /// Graph selected for coordination.
    pub graph_id: AgentGraphId,
    /// Root node selected by topology planning.
    pub root_node: AgentNodeId,
    /// Candidate edge set for the next coordination step.
    pub candidate_edges: Vec<AgentEdgeId>,
}

/// Validation error for `AgentGraph` topology consistency.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AgentGraphTopologyError {
    /// A graph cannot be executed or planned with no nodes.
    EmptyGraph {
        /// Empty graph identifier.
        graph_id: AgentGraphId,
    },
    /// A node identifier appears more than once.
    DuplicateNode {
        /// Duplicated node identifier.
        node_id: AgentNodeId,
    },
    /// An edge identifier appears more than once.
    DuplicateEdge {
        /// Duplicated edge identifier.
        edge_id: AgentEdgeId,
    },
    /// An edge source does not exist in the graph node set.
    MissingEdgeSource {
        /// Edge with the missing source.
        edge_id: AgentEdgeId,
        /// Missing source node.
        node_id: AgentNodeId,
    },
    /// An edge target does not exist in the graph node set.
    MissingEdgeTarget {
        /// Edge with the missing target.
        edge_id: AgentEdgeId,
        /// Missing target node.
        node_id: AgentNodeId,
    },
}

impl fmt::Display for AgentGraphTopologyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyGraph { graph_id } => {
                write!(
                    formatter,
                    "agent graph {graph_id} must contain at least one node"
                )
            }
            Self::DuplicateNode { node_id } => {
                write!(formatter, "agent graph contains duplicate node {node_id}")
            }
            Self::DuplicateEdge { edge_id } => {
                write!(formatter, "agent graph contains duplicate edge {edge_id}")
            }
            Self::MissingEdgeSource { edge_id, node_id } => {
                write!(
                    formatter,
                    "edge {edge_id} references missing source node {node_id}"
                )
            }
            Self::MissingEdgeTarget { edge_id, node_id } => {
                write!(
                    formatter,
                    "edge {edge_id} references missing target node {node_id}"
                )
            }
        }
    }
}

impl Error for AgentGraphTopologyError {}
