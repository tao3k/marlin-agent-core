//! Typed coordination receipts emitted by `AgentGraph` planning.

use serde::{Deserialize, Serialize};

use crate::ids::{
    AgentEdgeId, AgentElectionReason, AgentEvidenceId, AgentGraphId, AgentNodeId,
    AgentRoutingReason, GerbilPolicyScopeRef,
};

/// Receipt for a single topology coordination decision.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentCoordinationReceipt {
    /// Graph that owns the decision.
    pub graph_id: AgentGraphId,
    /// Node selected by the decision.
    pub selected_node: AgentNodeId,
    /// Edge selected by the decision, when the decision follows an edge.
    pub selected_edge: Option<AgentEdgeId>,
    /// Typed coordination outcome.
    pub decision: AgentCoordinationDecision,
    /// Typed evidence references supporting the decision.
    pub evidence: Vec<AgentCoordinationEvidenceRef>,
}

/// Coordination outcome emitted by topology planning.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum AgentCoordinationDecision {
    /// Select a node without traversing an edge.
    SelectNode,
    /// Follow an edge to the selected node.
    FollowEdge,
    /// Defer selection until more evidence is available.
    Defer,
    /// Deny the proposed route.
    Deny,
    /// Escalate route selection to a higher policy layer.
    Escalate,
}

/// Typed reference to evidence that supports a coordination decision.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentCoordinationEvidenceRef {
    /// Evidence category.
    pub kind: AgentCoordinationEvidenceKind,
    /// Stable evidence identifier.
    pub evidence_id: AgentEvidenceId,
}

/// Evidence source class for coordination receipts.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum AgentCoordinationEvidenceKind {
    /// Evidence from a graph-loop receipt.
    LoopReceipt,
    /// Evidence from Org memory.
    OrgMemoryReceipt,
    /// Evidence from Gerbil policy.
    GerbilPolicyReceipt,
    /// Evidence from hook evaluation.
    HookReceipt,
    /// Evidence from runtime execution.
    RuntimeReceipt,
}

/// Typed Gerbil policy routing receipt consumed by `PolicyScoped` planning.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentPolicyRoutingReceipt {
    /// Graph that owns the scoped routing decision.
    pub graph_id: AgentGraphId,
    /// Gerbil policy scope that produced the decision.
    pub policy_scope: GerbilPolicyScopeRef,
    /// Root node the policy evaluated.
    pub root_node: AgentNodeId,
    /// Policy routing outcome.
    pub decision: AgentPolicyRoutingDecision,
    /// Candidate edge ids allowed by the policy.
    pub candidate_edges: Vec<AgentEdgeId>,
    /// Typed evidence references supporting the policy decision.
    pub evidence: Vec<AgentCoordinationEvidenceRef>,
}

/// Routing outcome emitted by an external Gerbil policy scope.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum AgentPolicyRoutingDecision {
    /// Select the listed candidate edges.
    SelectEdges,
    /// Deny route selection.
    Deny,
    /// Defer route selection until more evidence is available.
    Defer,
}

/// Receipt for selecting one agent from a candidate set.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentElectionReceipt {
    /// Graph that owns the election.
    pub graph_id: AgentGraphId,
    /// Candidate nodes considered by the election.
    pub candidates: Vec<AgentNodeId>,
    /// Selected node.
    pub selected: AgentNodeId,
    /// Typed election reason.
    pub reason: AgentElectionReason,
    /// Typed evidence references supporting the election.
    pub evidence: Vec<AgentCoordinationEvidenceRef>,
}

/// Receipt for routing from one node to another through an edge.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentRoutingReceipt {
    /// Graph that owns the route.
    pub graph_id: AgentGraphId,
    /// Source node.
    pub from: AgentNodeId,
    /// Target node.
    pub to: AgentNodeId,
    /// Edge used for routing.
    pub via_edge: AgentEdgeId,
    /// Typed routing reason.
    pub reason: AgentRoutingReason,
    /// Typed evidence references supporting the route.
    pub evidence: Vec<AgentCoordinationEvidenceRef>,
}
