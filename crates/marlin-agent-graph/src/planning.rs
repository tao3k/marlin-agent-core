//! Planning boundary for producing `AgentCoordinationPlan` values.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::ids::{AgentCapability, AgentEdgeId, AgentGraphId, AgentNodeId, GerbilPolicyScopeRef};
use crate::receipt::{
    AgentCoordinationDecision, AgentCoordinationEvidenceRef, AgentCoordinationReceipt,
    AgentPolicyRoutingDecision, AgentPolicyRoutingReceipt,
};
use crate::topology::{
    AgentCoordinationPlan, AgentEdge, AgentEdgeCondition, AgentGraph, AgentTopologyPolicy,
};

/// Planning target for a topology-level coordination step.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentGraphPlanningTarget {
    /// Graph expected by the caller.
    pub graph_id: AgentGraphId,
    /// Root node to project into a runtime-consumable plan.
    pub root_node: AgentNodeId,
    /// Optional capability required by this planning step.
    pub required_capability: Option<AgentCapability>,
    /// Typed evidence references used by the planner.
    pub evidence: Vec<AgentCoordinationEvidenceRef>,
}

impl AgentGraphPlanningTarget {
    /// Creates a planning target for a root node.
    pub fn new(graph_id: AgentGraphId, root_node: AgentNodeId) -> Self {
        Self {
            graph_id,
            root_node,
            required_capability: None,
            evidence: Vec::new(),
        }
    }

    /// Adds a required capability for capability-aware planning.
    pub fn with_required_capability(mut self, capability: AgentCapability) -> Self {
        self.required_capability = Some(capability);
        self
    }

    /// Adds typed evidence references for the planning receipt.
    pub fn with_evidence(mut self, evidence: Vec<AgentCoordinationEvidenceRef>) -> Self {
        self.evidence = evidence;
        self
    }
}

/// Receipt emitted by `AgentGraph` planning before runtime admission.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentGraphPlanningReceipt {
    /// Original planning target.
    pub target: AgentGraphPlanningTarget,
    /// Planning status.
    pub status: AgentGraphPlanningStatus,
    /// Runtime-consumable plan, when planning succeeds.
    pub plan: Option<AgentCoordinationPlan>,
    /// Coordination receipt for the root-node selection.
    pub coordination: Option<AgentCoordinationReceipt>,
    /// Typed rejection reason, when planning fails.
    pub rejection: Option<AgentGraphPlanningRejection>,
}

impl AgentGraphPlanningReceipt {
    fn planned(
        target: AgentGraphPlanningTarget,
        plan: AgentCoordinationPlan,
        coordination: AgentCoordinationReceipt,
    ) -> Self {
        Self {
            target,
            status: AgentGraphPlanningStatus::Planned,
            plan: Some(plan),
            coordination: Some(coordination),
            rejection: None,
        }
    }

    fn rejected(target: AgentGraphPlanningTarget, rejection: AgentGraphPlanningRejection) -> Self {
        Self {
            target,
            status: AgentGraphPlanningStatus::Rejected,
            plan: None,
            coordination: None,
            rejection: Some(rejection),
        }
    }
}

/// Planning status for an `AgentGraph` coordination attempt.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum AgentGraphPlanningStatus {
    /// A runtime-consumable plan was produced.
    Planned,
    /// Planning failed before producing a runtime-consumable plan.
    Rejected,
}

/// Typed reason for rejecting a topology planning target.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum AgentGraphPlanningRejection {
    /// The graph failed topology validation.
    InvalidGraph {
        /// Validation failure rendered at the planning boundary.
        message: String,
    },
    /// The target graph id does not match the supplied graph.
    GraphMismatch {
        /// Graph id from the target.
        target_graph_id: AgentGraphId,
        /// Graph id from the supplied graph.
        graph_id: AgentGraphId,
    },
    /// The target root node does not exist in the supplied graph.
    MissingRootNode {
        /// Missing root node.
        node_id: AgentNodeId,
    },
    /// A `PolicyScoped` graph requires an explicit typed policy routing receipt.
    MissingPolicyReceipt {
        /// Required Gerbil policy scope.
        policy_scope: GerbilPolicyScopeRef,
    },
    /// A policy routing receipt was supplied for a graph that does not use policy-scoped routing.
    UnexpectedPolicyReceipt,
    /// Policy receipt graph id does not match the supplied graph.
    PolicyReceiptGraphMismatch {
        /// Graph id from the policy receipt.
        receipt_graph_id: AgentGraphId,
        /// Graph id from the supplied graph.
        graph_id: AgentGraphId,
    },
    /// Policy receipt scope does not match the graph policy scope.
    PolicyReceiptScopeMismatch {
        /// Scope from the policy receipt.
        receipt_policy_scope: GerbilPolicyScopeRef,
        /// Scope required by the graph policy.
        policy_scope: GerbilPolicyScopeRef,
    },
    /// Policy receipt root node does not match the planning target.
    PolicyReceiptRootMismatch {
        /// Root node from the policy receipt.
        receipt_root_node: AgentNodeId,
        /// Root node from the planning target.
        target_root_node: AgentNodeId,
    },
    /// Policy receipt denied route selection.
    PolicyReceiptDenied,
    /// Policy receipt deferred route selection.
    PolicyReceiptDeferred,
    /// Policy receipt selected an edge that is not a valid candidate for the target.
    PolicyReceiptEdgeNotCandidate {
        /// Invalid candidate edge.
        edge_id: AgentEdgeId,
    },
}

/// Produces a runtime-consumable `AgentCoordinationPlan` without executing it.
pub fn plan_agent_coordination(
    graph: &AgentGraph,
    target: AgentGraphPlanningTarget,
) -> AgentGraphPlanningReceipt {
    plan_agent_coordination_internal(graph, target, None)
}

/// Produces a runtime-consumable plan using a typed Gerbil policy routing receipt.
pub fn plan_agent_coordination_with_policy_receipt(
    graph: &AgentGraph,
    target: AgentGraphPlanningTarget,
    policy_receipt: AgentPolicyRoutingReceipt,
) -> AgentGraphPlanningReceipt {
    plan_agent_coordination_internal(graph, target, Some(policy_receipt))
}

fn plan_agent_coordination_internal(
    graph: &AgentGraph,
    target: AgentGraphPlanningTarget,
    policy_receipt: Option<AgentPolicyRoutingReceipt>,
) -> AgentGraphPlanningReceipt {
    if let Err(error) = graph.validate() {
        return AgentGraphPlanningReceipt::rejected(
            target,
            AgentGraphPlanningRejection::InvalidGraph {
                message: error.to_string(),
            },
        );
    }

    if target.graph_id != graph.graph_id {
        let target_graph_id = target.graph_id.clone();
        let graph_id = graph.graph_id.clone();
        return AgentGraphPlanningReceipt::rejected(
            target,
            AgentGraphPlanningRejection::GraphMismatch {
                target_graph_id,
                graph_id,
            },
        );
    }

    if !graph
        .nodes
        .iter()
        .any(|node| node.node_id == target.root_node)
    {
        let node_id = target.root_node.clone();
        return AgentGraphPlanningReceipt::rejected(
            target,
            AgentGraphPlanningRejection::MissingRootNode { node_id },
        );
    }

    let candidate_edges = match candidate_edges_for_target(graph, &target, policy_receipt.as_ref())
    {
        Ok(candidate_edges) => candidate_edges,
        Err(rejection) => return AgentGraphPlanningReceipt::rejected(target, rejection),
    };
    let evidence = coordination_evidence(&target, policy_receipt.as_ref());
    let plan = AgentCoordinationPlan {
        graph_id: target.graph_id.clone(),
        root_node: target.root_node.clone(),
        candidate_edges,
    };
    let coordination = AgentCoordinationReceipt {
        graph_id: target.graph_id.clone(),
        selected_node: target.root_node.clone(),
        selected_edge: None,
        decision: AgentCoordinationDecision::SelectNode,
        evidence,
    };

    AgentGraphPlanningReceipt::planned(target, plan, coordination)
}

fn candidate_edges_for_target(
    graph: &AgentGraph,
    target: &AgentGraphPlanningTarget,
    policy_receipt: Option<&AgentPolicyRoutingReceipt>,
) -> Result<Vec<AgentEdgeId>, AgentGraphPlanningRejection> {
    let node_capabilities = graph
        .nodes
        .iter()
        .map(|node| {
            (
                node.node_id.clone(),
                node.capabilities.iter().cloned().collect::<HashSet<_>>(),
            )
        })
        .collect::<HashMap<_, _>>();

    match &graph.topology_policy {
        AgentTopologyPolicy::Deterministic | AgentTopologyPolicy::CapabilityFirst => {
            if policy_receipt.is_some() {
                return Err(AgentGraphPlanningRejection::UnexpectedPolicyReceipt);
            }
            Ok(graph
                .edges
                .iter()
                .filter(|edge| edge.from == target.root_node)
                .filter(|edge| {
                    edge_matches_topology_policy(edge, graph, target, &node_capabilities)
                })
                .map(|edge| edge.edge_id.clone())
                .collect())
        }
        AgentTopologyPolicy::PolicyScoped(policy_scope) => {
            let Some(policy_receipt) = policy_receipt else {
                return Err(AgentGraphPlanningRejection::MissingPolicyReceipt {
                    policy_scope: policy_scope.clone(),
                });
            };
            policy_scoped_candidate_edges(graph, target, policy_scope, policy_receipt)
        }
    }
}

fn edge_matches_topology_policy(
    edge: &AgentEdge,
    graph: &AgentGraph,
    target: &AgentGraphPlanningTarget,
    node_capabilities: &HashMap<AgentNodeId, HashSet<AgentCapability>>,
) -> bool {
    match &graph.topology_policy {
        AgentTopologyPolicy::Deterministic => edge_condition_matches(edge, target),
        AgentTopologyPolicy::CapabilityFirst => {
            let Some(required_capability) = target.required_capability.as_ref() else {
                return edge_condition_matches(edge, target);
            };
            edge_condition_matches(edge, target)
                && node_capabilities
                    .get(&edge.to)
                    .is_some_and(|capabilities| capabilities.contains(required_capability))
        }
        AgentTopologyPolicy::PolicyScoped(_) => false,
    }
}

fn policy_scoped_candidate_edges(
    graph: &AgentGraph,
    target: &AgentGraphPlanningTarget,
    policy_scope: &GerbilPolicyScopeRef,
    policy_receipt: &AgentPolicyRoutingReceipt,
) -> Result<Vec<AgentEdgeId>, AgentGraphPlanningRejection> {
    if policy_receipt.graph_id != graph.graph_id {
        return Err(AgentGraphPlanningRejection::PolicyReceiptGraphMismatch {
            receipt_graph_id: policy_receipt.graph_id.clone(),
            graph_id: graph.graph_id.clone(),
        });
    }
    if &policy_receipt.policy_scope != policy_scope {
        return Err(AgentGraphPlanningRejection::PolicyReceiptScopeMismatch {
            receipt_policy_scope: policy_receipt.policy_scope.clone(),
            policy_scope: policy_scope.clone(),
        });
    }
    if policy_receipt.root_node != target.root_node {
        return Err(AgentGraphPlanningRejection::PolicyReceiptRootMismatch {
            receipt_root_node: policy_receipt.root_node.clone(),
            target_root_node: target.root_node.clone(),
        });
    }

    match policy_receipt.decision {
        AgentPolicyRoutingDecision::Deny => {
            return Err(AgentGraphPlanningRejection::PolicyReceiptDenied);
        }
        AgentPolicyRoutingDecision::Defer => {
            return Err(AgentGraphPlanningRejection::PolicyReceiptDeferred);
        }
        AgentPolicyRoutingDecision::SelectEdges => {}
    }

    let outgoing_edges = graph
        .edges
        .iter()
        .filter(|edge| edge.from == target.root_node)
        .map(|edge| (edge.edge_id.clone(), edge))
        .collect::<HashMap<_, _>>();

    for edge_id in &policy_receipt.candidate_edges {
        let Some(edge) = outgoing_edges.get(edge_id) else {
            return Err(AgentGraphPlanningRejection::PolicyReceiptEdgeNotCandidate {
                edge_id: edge_id.clone(),
            });
        };
        if !edge_condition_matches(edge, target) {
            return Err(AgentGraphPlanningRejection::PolicyReceiptEdgeNotCandidate {
                edge_id: edge_id.clone(),
            });
        }
    }

    Ok(policy_receipt.candidate_edges.clone())
}

fn coordination_evidence(
    target: &AgentGraphPlanningTarget,
    policy_receipt: Option<&AgentPolicyRoutingReceipt>,
) -> Vec<AgentCoordinationEvidenceRef> {
    let mut evidence = target.evidence.clone();
    if let Some(policy_receipt) = policy_receipt {
        evidence.extend(policy_receipt.evidence.clone());
    }
    evidence
}

fn edge_condition_matches(edge: &AgentEdge, target: &AgentGraphPlanningTarget) -> bool {
    match &edge.condition {
        AgentEdgeCondition::Always => true,
        AgentEdgeCondition::CapabilityRequired(required_capability) => target
            .required_capability
            .as_ref()
            .is_some_and(|capability| capability == required_capability),
        AgentEdgeCondition::PolicyDecision(_) => true,
    }
}
