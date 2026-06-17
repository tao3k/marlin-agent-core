//! `Org` topology contract projection into typed `AgentGraph` metadata.

use std::{error::Error, fmt};

use marlin_agent_graph::{
    AgentCapability, AgentEdge, AgentEdgeCondition, AgentEdgeId, AgentGraph, AgentGraphId,
    AgentGraphTopologyError, AgentNode, AgentNodeId, AgentPolicyDecisionRef, AgentRole,
    AgentTopologyPolicy, GerbilPolicyScopeRef, GraphLoopEntryRef, GraphLoopGraphRef,
    GraphLoopNodeRef, OrgMemoryScopeRef,
};
use marlin_org_model::OrgNode;

/// `Org` property carrying the target `AgentGraphId`.
pub const AGENT_GRAPH_ID_PROPERTY: &str = "AGENT_GRAPH_ID";
/// `Org` property carrying the topology policy selector.
pub const AGENT_GRAPH_TOPOLOGY_POLICY_PROPERTY: &str = "AGENT_GRAPH_TOPOLOGY_POLICY";
/// `Org` property carrying the stable agent node id.
pub const AGENT_GRAPH_NODE_ID_PROPERTY: &str = "AGENT_GRAPH_NODE_ID";
/// `Org` property carrying the stable agent role.
pub const AGENT_GRAPH_ROLE_PROPERTY: &str = "AGENT_GRAPH_ROLE";
/// `Org` property carrying comma-separated agent capabilities.
pub const AGENT_GRAPH_CAPABILITIES_PROPERTY: &str = "AGENT_GRAPH_CAPABILITIES";
/// `Org` property carrying the referenced LoopGraph id.
pub const AGENT_GRAPH_LOOP_GRAPH_PROPERTY: &str = "AGENT_GRAPH_LOOP_GRAPH";
/// `Org` property carrying the referenced LoopGraph entry node.
pub const AGENT_GRAPH_LOOP_ENTRY_NODE_PROPERTY: &str = "AGENT_GRAPH_LOOP_ENTRY_NODE";
/// `Org` property carrying an Org memory scope reference.
pub const AGENT_GRAPH_MEMORY_SCOPE_REF_PROPERTY: &str = "AGENT_GRAPH_MEMORY_SCOPE_REF";
/// `Org` property carrying a Gerbil policy scope reference.
pub const AGENT_GRAPH_POLICY_SCOPE_REF_PROPERTY: &str = "AGENT_GRAPH_POLICY_SCOPE_REF";
/// `Org` property carrying the stable topology edge id.
pub const AGENT_GRAPH_EDGE_ID_PROPERTY: &str = "AGENT_GRAPH_EDGE_ID";
/// `Org` property carrying the source agent node id for an edge.
pub const AGENT_GRAPH_EDGE_FROM_PROPERTY: &str = "AGENT_GRAPH_EDGE_FROM";
/// `Org` property carrying the target agent node id for an edge.
pub const AGENT_GRAPH_EDGE_TO_PROPERTY: &str = "AGENT_GRAPH_EDGE_TO";
/// `Org` property carrying the edge kind selector.
pub const AGENT_GRAPH_EDGE_KIND_PROPERTY: &str = "AGENT_GRAPH_EDGE_KIND";
/// `Org` property carrying the edge condition selector.
pub const AGENT_GRAPH_EDGE_CONDITION_PROPERTY: &str = "AGENT_GRAPH_EDGE_CONDITION";
/// `Org` property recording whether the AgentGraph contract was validated.
pub const AGENT_GRAPH_CONTRACT_VALIDATED_PROPERTY: &str = "CONTRACT_VALIDATED";

/// Request for projecting loaded Org topology contract nodes into an `AgentGraph`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentGraphOrgProjectionRequest {
    /// Graph id to project.
    pub graph_id: AgentGraphId,
    /// Whether every participating Org node must carry `CONTRACT_VALIDATED=true`.
    pub require_contract_validated: bool,
}

impl AgentGraphOrgProjectionRequest {
    /// Create a projection request that requires contract-validated Org nodes.
    pub fn new(graph_id: AgentGraphId) -> Self {
        Self {
            graph_id,
            require_contract_validated: true,
        }
    }

    /// Allow projection from unvalidated nodes for fixtures or exploratory imports.
    pub fn allowing_unvalidated_contracts(mut self) -> Self {
        self.require_contract_validated = false;
        self
    }
}

/// Projection receipt for the Org-to-AgentGraph boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentGraphOrgProjectionReceipt {
    /// Requested graph id.
    pub graph_id: AgentGraphId,
    /// Projection status.
    pub status: AgentGraphOrgProjectionStatus,
    /// Projected typed graph, when successful.
    pub graph: Option<AgentGraph>,
    /// Typed rejection, when projection failed.
    pub rejection: Option<AgentGraphOrgProjectionRejection>,
    /// Number of projected agent nodes.
    pub projected_node_count: usize,
    /// Number of projected agent edges.
    pub projected_edge_count: usize,
}

impl AgentGraphOrgProjectionReceipt {
    fn projected(graph: AgentGraph) -> Self {
        Self {
            graph_id: graph.graph_id.clone(),
            projected_node_count: graph.nodes.len(),
            projected_edge_count: graph.edges.len(),
            status: AgentGraphOrgProjectionStatus::Projected,
            graph: Some(graph),
            rejection: None,
        }
    }

    fn rejected(
        graph_id: AgentGraphId,
        rejection: AgentGraphOrgProjectionRejection,
        projected_node_count: usize,
        projected_edge_count: usize,
    ) -> Self {
        Self {
            graph_id,
            status: AgentGraphOrgProjectionStatus::Rejected,
            graph: None,
            rejection: Some(rejection),
            projected_node_count,
            projected_edge_count,
        }
    }
}

/// Status for Org-to-AgentGraph projection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AgentGraphOrgProjectionStatus {
    /// A typed `AgentGraph` was projected.
    Projected,
    /// Projection stopped before a typed graph could be admitted.
    Rejected,
}

/// Typed rejection for Org-to-AgentGraph projection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AgentGraphOrgProjectionRejection {
    /// No Org nodes matched the requested graph id.
    EmptyProjection,
    /// A participating Org node was not contract validated.
    UnvalidatedContract { node_id: String },
    /// A required property was missing.
    MissingProperty {
        node_id: String,
        property: &'static str,
    },
    /// A property failed semantic id validation.
    InvalidSemanticId {
        node_id: String,
        property: &'static str,
        reason: String,
    },
    /// The edge kind selector is not part of the AgentGraph protocol.
    UnsupportedEdgeKind { node_id: String, value: String },
    /// The edge condition selector is not part of the AgentGraph protocol.
    UnsupportedEdgeCondition { node_id: String, value: String },
    /// The topology policy selector is not part of the AgentGraph protocol.
    UnsupportedTopologyPolicy { node_id: String, value: String },
    /// A graph-scoped topology policy selector conflicts across Org nodes.
    ConflictingTopologyPolicy {
        first: AgentTopologyPolicy,
        second: AgentTopologyPolicy,
    },
    /// The projected `AgentGraph` failed topology validation.
    InvalidTopology(AgentGraphTopologyError),
}

impl fmt::Display for AgentGraphOrgProjectionRejection {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyProjection => formatter.write_str("no Org AgentGraph nodes matched"),
            Self::UnvalidatedContract { node_id } => {
                write!(formatter, "Org node {node_id} is not contract validated")
            }
            Self::MissingProperty { node_id, property } => {
                write!(
                    formatter,
                    "Org node {node_id} is missing property {property}"
                )
            }
            Self::InvalidSemanticId {
                node_id,
                property,
                reason,
            } => write!(
                formatter,
                "Org node {node_id} has invalid semantic id in {property}: {reason}"
            ),
            Self::UnsupportedEdgeKind { node_id, value } => {
                write!(
                    formatter,
                    "Org node {node_id} has unsupported edge kind {value}"
                )
            }
            Self::UnsupportedEdgeCondition { node_id, value } => write!(
                formatter,
                "Org node {node_id} has unsupported edge condition {value}"
            ),
            Self::UnsupportedTopologyPolicy { node_id, value } => write!(
                formatter,
                "Org node {node_id} has unsupported topology policy {value}"
            ),
            Self::ConflictingTopologyPolicy { first, second } => write!(
                formatter,
                "conflicting AgentGraph topology policies: {first:?} vs {second:?}"
            ),
            Self::InvalidTopology(error) => {
                write!(formatter, "invalid AgentGraph topology: {error:?}")
            }
        }
    }
}

impl Error for AgentGraphOrgProjectionRejection {}

pub(super) fn project_agent_graph_from_org_nodes<'a>(
    nodes: impl IntoIterator<Item = &'a OrgNode>,
    request: &AgentGraphOrgProjectionRequest,
) -> AgentGraphOrgProjectionReceipt {
    let mut matched_graph_nodes = 0;
    let mut projected_nodes = Vec::new();
    let mut projected_edges = Vec::new();
    let mut topology_policy: Option<AgentTopologyPolicy> = None;

    for node in nodes {
        if property(node, AGENT_GRAPH_ID_PROPERTY) != Some(request.graph_id.as_str()) {
            continue;
        }
        matched_graph_nodes += 1;

        if request.require_contract_validated
            && !property_is_truthy(node, AGENT_GRAPH_CONTRACT_VALIDATED_PROPERTY)
        {
            return AgentGraphOrgProjectionReceipt::rejected(
                request.graph_id.clone(),
                AgentGraphOrgProjectionRejection::UnvalidatedContract {
                    node_id: node.id.as_str().to_owned(),
                },
                projected_nodes.len(),
                projected_edges.len(),
            );
        }

        if let Some(policy) = property(node, AGENT_GRAPH_TOPOLOGY_POLICY_PROPERTY) {
            let parsed_policy = match parse_topology_policy(node, policy) {
                Ok(policy) => policy,
                Err(rejection) => {
                    return AgentGraphOrgProjectionReceipt::rejected(
                        request.graph_id.clone(),
                        rejection,
                        projected_nodes.len(),
                        projected_edges.len(),
                    );
                }
            };
            if let Some(existing_policy) = &topology_policy {
                if existing_policy != &parsed_policy {
                    return AgentGraphOrgProjectionReceipt::rejected(
                        request.graph_id.clone(),
                        AgentGraphOrgProjectionRejection::ConflictingTopologyPolicy {
                            first: existing_policy.clone(),
                            second: parsed_policy,
                        },
                        projected_nodes.len(),
                        projected_edges.len(),
                    );
                }
            } else {
                topology_policy = Some(parsed_policy);
            }
        }

        let node_id = property(node, AGENT_GRAPH_NODE_ID_PROPERTY);
        let edge_id = property(node, AGENT_GRAPH_EDGE_ID_PROPERTY);
        match (node_id, edge_id) {
            (Some(_), None) => match project_agent_node(node) {
                Ok(agent_node) => projected_nodes.push(agent_node),
                Err(rejection) => {
                    return AgentGraphOrgProjectionReceipt::rejected(
                        request.graph_id.clone(),
                        rejection,
                        projected_nodes.len(),
                        projected_edges.len(),
                    );
                }
            },
            (None, Some(_)) => match project_agent_edge(node) {
                Ok(agent_edge) => projected_edges.push(agent_edge),
                Err(rejection) => {
                    return AgentGraphOrgProjectionReceipt::rejected(
                        request.graph_id.clone(),
                        rejection,
                        projected_nodes.len(),
                        projected_edges.len(),
                    );
                }
            },
            _ => {}
        }
    }

    if matched_graph_nodes == 0 {
        return AgentGraphOrgProjectionReceipt::rejected(
            request.graph_id.clone(),
            AgentGraphOrgProjectionRejection::EmptyProjection,
            0,
            0,
        );
    }

    let graph = AgentGraph {
        graph_id: request.graph_id.clone(),
        nodes: projected_nodes,
        edges: projected_edges,
        topology_policy: topology_policy.unwrap_or(AgentTopologyPolicy::Deterministic),
    };

    if let Err(error) = graph.validate() {
        return AgentGraphOrgProjectionReceipt::rejected(
            request.graph_id.clone(),
            AgentGraphOrgProjectionRejection::InvalidTopology(error),
            graph.nodes.len(),
            graph.edges.len(),
        );
    }

    AgentGraphOrgProjectionReceipt::projected(graph)
}

fn project_agent_node(node: &OrgNode) -> Result<AgentNode, AgentGraphOrgProjectionRejection> {
    Ok(AgentNode {
        node_id: typed_property(node, AGENT_GRAPH_NODE_ID_PROPERTY, AgentNodeId::new)?,
        role: typed_property(node, AGENT_GRAPH_ROLE_PROPERTY, AgentRole::new)?,
        capabilities: capabilities(node)?,
        loop_entry: GraphLoopEntryRef {
            graph: typed_property(
                node,
                AGENT_GRAPH_LOOP_GRAPH_PROPERTY,
                GraphLoopGraphRef::new,
            )?,
            entry_node: typed_property(
                node,
                AGENT_GRAPH_LOOP_ENTRY_NODE_PROPERTY,
                GraphLoopNodeRef::new,
            )?,
        },
        memory_scope: optional_typed_property(
            node,
            AGENT_GRAPH_MEMORY_SCOPE_REF_PROPERTY,
            OrgMemoryScopeRef::new,
        )?,
        policy_scope: optional_typed_property(
            node,
            AGENT_GRAPH_POLICY_SCOPE_REF_PROPERTY,
            GerbilPolicyScopeRef::new,
        )?,
    })
}

fn project_agent_edge(node: &OrgNode) -> Result<AgentEdge, AgentGraphOrgProjectionRejection> {
    Ok(AgentEdge {
        edge_id: typed_property(node, AGENT_GRAPH_EDGE_ID_PROPERTY, AgentEdgeId::new)?,
        from: typed_property(node, AGENT_GRAPH_EDGE_FROM_PROPERTY, AgentNodeId::new)?,
        to: typed_property(node, AGENT_GRAPH_EDGE_TO_PROPERTY, AgentNodeId::new)?,
        kind: parse_edge_kind(
            node,
            required_property(node, AGENT_GRAPH_EDGE_KIND_PROPERTY)?,
        )?,
        condition: property(node, AGENT_GRAPH_EDGE_CONDITION_PROPERTY)
            .map(|condition| parse_edge_condition(node, condition))
            .transpose()?
            .unwrap_or(AgentEdgeCondition::Always),
    })
}

fn capabilities(node: &OrgNode) -> Result<Vec<AgentCapability>, AgentGraphOrgProjectionRejection> {
    let Some(value) = property(node, AGENT_GRAPH_CAPABILITIES_PROPERTY) else {
        return Ok(Vec::new());
    };

    value
        .split(',')
        .map(str::trim)
        .filter(|capability| !capability.is_empty())
        .map(|capability| {
            AgentCapability::new(capability).map_err(|error| {
                AgentGraphOrgProjectionRejection::InvalidSemanticId {
                    node_id: node.id.as_str().to_owned(),
                    property: AGENT_GRAPH_CAPABILITIES_PROPERTY,
                    reason: error.to_string(),
                }
            })
        })
        .collect()
}

fn parse_topology_policy(
    node: &OrgNode,
    value: &str,
) -> Result<AgentTopologyPolicy, AgentGraphOrgProjectionRejection> {
    match value {
        "deterministic" => Ok(AgentTopologyPolicy::Deterministic),
        "capability-first" => Ok(AgentTopologyPolicy::CapabilityFirst),
        policy if policy.starts_with("policy:") => {
            let scope = policy.trim_start_matches("policy:");
            GerbilPolicyScopeRef::new(scope)
                .map(AgentTopologyPolicy::PolicyScoped)
                .map_err(
                    |error| AgentGraphOrgProjectionRejection::InvalidSemanticId {
                        node_id: node.id.as_str().to_owned(),
                        property: AGENT_GRAPH_TOPOLOGY_POLICY_PROPERTY,
                        reason: error.to_string(),
                    },
                )
        }
        _ => Err(
            AgentGraphOrgProjectionRejection::UnsupportedTopologyPolicy {
                node_id: node.id.as_str().to_owned(),
                value: value.to_owned(),
            },
        ),
    }
}

fn parse_edge_kind(
    node: &OrgNode,
    value: &str,
) -> Result<marlin_agent_graph::AgentEdgeKind, AgentGraphOrgProjectionRejection> {
    match value {
        "handoff" => Ok(marlin_agent_graph::AgentEdgeKind::Handoff),
        "delegate" => Ok(marlin_agent_graph::AgentEdgeKind::Delegate),
        "review" => Ok(marlin_agent_graph::AgentEdgeKind::Review),
        "fanout" => Ok(marlin_agent_graph::AgentEdgeKind::Fanout),
        "fanin" => Ok(marlin_agent_graph::AgentEdgeKind::Fanin),
        "vote" => Ok(marlin_agent_graph::AgentEdgeKind::Vote),
        "escalate" => Ok(marlin_agent_graph::AgentEdgeKind::Escalate),
        _ => Err(AgentGraphOrgProjectionRejection::UnsupportedEdgeKind {
            node_id: node.id.as_str().to_owned(),
            value: value.to_owned(),
        }),
    }
}

fn parse_edge_condition(
    node: &OrgNode,
    value: &str,
) -> Result<AgentEdgeCondition, AgentGraphOrgProjectionRejection> {
    match value {
        "always" => Ok(AgentEdgeCondition::Always),
        condition if condition.starts_with("capability:") => {
            let capability = condition.trim_start_matches("capability:");
            AgentCapability::new(capability)
                .map(AgentEdgeCondition::CapabilityRequired)
                .map_err(
                    |error| AgentGraphOrgProjectionRejection::InvalidSemanticId {
                        node_id: node.id.as_str().to_owned(),
                        property: AGENT_GRAPH_EDGE_CONDITION_PROPERTY,
                        reason: error.to_string(),
                    },
                )
        }
        condition if condition.starts_with("policy:") => {
            let policy = condition.trim_start_matches("policy:");
            AgentPolicyDecisionRef::new(policy)
                .map(AgentEdgeCondition::PolicyDecision)
                .map_err(
                    |error| AgentGraphOrgProjectionRejection::InvalidSemanticId {
                        node_id: node.id.as_str().to_owned(),
                        property: AGENT_GRAPH_EDGE_CONDITION_PROPERTY,
                        reason: error.to_string(),
                    },
                )
        }
        _ => Err(AgentGraphOrgProjectionRejection::UnsupportedEdgeCondition {
            node_id: node.id.as_str().to_owned(),
            value: value.to_owned(),
        }),
    }
}

fn required_property<'a>(
    node: &'a OrgNode,
    property_name: &'static str,
) -> Result<&'a str, AgentGraphOrgProjectionRejection> {
    property(node, property_name).ok_or_else(|| AgentGraphOrgProjectionRejection::MissingProperty {
        node_id: node.id.as_str().to_owned(),
        property: property_name,
    })
}

fn typed_property<T>(
    node: &OrgNode,
    property_name: &'static str,
    constructor: impl FnOnce(String) -> Result<T, marlin_agent_graph::AgentGraphValidationError>,
) -> Result<T, AgentGraphOrgProjectionRejection> {
    let value = required_property(node, property_name)?;
    constructor(value.to_owned()).map_err(|error| {
        AgentGraphOrgProjectionRejection::InvalidSemanticId {
            node_id: node.id.as_str().to_owned(),
            property: property_name,
            reason: error.to_string(),
        }
    })
}

fn optional_typed_property<T>(
    node: &OrgNode,
    property_name: &'static str,
    constructor: impl FnOnce(String) -> Result<T, marlin_agent_graph::AgentGraphValidationError>,
) -> Result<Option<T>, AgentGraphOrgProjectionRejection> {
    property(node, property_name)
        .map(|value| constructor(value.to_owned()))
        .transpose()
        .map_err(
            |error| AgentGraphOrgProjectionRejection::InvalidSemanticId {
                node_id: node.id.as_str().to_owned(),
                property: property_name,
                reason: error.to_string(),
            },
        )
}

fn property<'a>(node: &'a OrgNode, key: &str) -> Option<&'a str> {
    node.properties.get(key).map(String::as_str)
}

fn property_is_truthy(node: &OrgNode, key: &str) -> bool {
    property(node, key).is_some_and(|value| matches!(value, "true" | "yes" | "1"))
}
