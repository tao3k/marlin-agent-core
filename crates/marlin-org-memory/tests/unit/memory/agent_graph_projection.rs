use marlin_agent_graph::{
    AgentEdgeCondition, AgentGraphId, AgentGraphTopologyError, AgentTopologyPolicy,
};
use marlin_org_memory::{
    AGENT_GRAPH_CAPABILITIES_PROPERTY, AGENT_GRAPH_CONTRACT_VALIDATED_PROPERTY,
    AGENT_GRAPH_EDGE_CONDITION_PROPERTY, AGENT_GRAPH_EDGE_FROM_PROPERTY,
    AGENT_GRAPH_EDGE_ID_PROPERTY, AGENT_GRAPH_EDGE_KIND_PROPERTY, AGENT_GRAPH_EDGE_TO_PROPERTY,
    AGENT_GRAPH_ID_PROPERTY, AGENT_GRAPH_LOOP_ENTRY_NODE_PROPERTY, AGENT_GRAPH_LOOP_GRAPH_PROPERTY,
    AGENT_GRAPH_MEMORY_SCOPE_REF_PROPERTY, AGENT_GRAPH_NODE_ID_PROPERTY,
    AGENT_GRAPH_POLICY_SCOPE_REF_PROPERTY, AGENT_GRAPH_ROLE_PROPERTY,
    AGENT_GRAPH_TOPOLOGY_POLICY_PROPERTY, AgentGraphOrgProjectionRejection,
    AgentGraphOrgProjectionRequest, AgentGraphOrgProjectionStatus, MemoryOrgWorkspace,
};
use marlin_org_model::{OrgNode, OrgNodeId};

#[test]
fn org_agent_graph_projection_builds_typed_topology_without_memory_body() {
    let workspace = MemoryOrgWorkspace::from_nodes(vec![
        agent_node(AgentNodeFixture {
            org_id: "org:planner",
            graph_id: "agent-graph.project",
            node_id: "planner",
            role: "planner",
            capabilities: "plan,rust-edit",
            loop_graph: "loop.planner",
            loop_entry: "entry",
            memory_scope: Some("memory.scope.project"),
            policy_scope: Some("policy.scope.planner"),
            topology_policy: Some("capability-first"),
            contract_validated: true,
        }),
        agent_node(AgentNodeFixture {
            org_id: "org:implementation",
            graph_id: "agent-graph.project",
            node_id: "implementation",
            role: "implementer",
            capabilities: "rust-edit",
            loop_graph: "loop.implementation",
            loop_entry: "entry",
            memory_scope: Some("memory.scope.project"),
            policy_scope: None,
            topology_policy: Some("capability-first"),
            contract_validated: true,
        }),
        edge_node(EdgeNodeFixture {
            org_id: "org:planner-to-implementation",
            graph_id: "agent-graph.project",
            edge_id: "planner-to-implementation",
            from: "planner",
            to: "implementation",
            kind: "handoff",
            condition: "capability:rust-edit",
            contract_validated: true,
        }),
    ]);

    let receipt = workspace
        .project_agent_graph_from_loaded_nodes(AgentGraphOrgProjectionRequest::new(
            AgentGraphId::new("agent-graph.project").unwrap(),
        ))
        .expect("projection succeeds");

    assert_eq!(receipt.status, AgentGraphOrgProjectionStatus::Projected);
    assert_eq!(receipt.projected_node_count, 2);
    assert_eq!(receipt.projected_edge_count, 1);
    let graph = receipt.graph.expect("projected graph");
    assert_eq!(graph.topology_policy, AgentTopologyPolicy::CapabilityFirst);
    let planner = graph
        .nodes
        .iter()
        .find(|node| node.node_id.as_str() == "planner")
        .expect("planner node");
    assert_eq!(
        planner.memory_scope.as_ref().map(|scope| scope.as_str()),
        Some("memory.scope.project")
    );
    assert_eq!(
        planner.policy_scope.as_ref().map(|scope| scope.as_str()),
        Some("policy.scope.planner")
    );
    assert_eq!(
        graph.edges[0].condition,
        AgentEdgeCondition::CapabilityRequired(
            marlin_agent_graph::AgentCapability::new("rust-edit").unwrap()
        )
    );
}

#[test]
fn org_agent_graph_projection_rejects_unvalidated_contract_nodes() {
    let workspace = MemoryOrgWorkspace::from_nodes(vec![agent_node(AgentNodeFixture {
        org_id: "org:planner",
        graph_id: "agent-graph.project",
        node_id: "planner",
        role: "planner",
        capabilities: "plan",
        loop_graph: "loop.planner",
        loop_entry: "entry",
        memory_scope: None,
        policy_scope: None,
        topology_policy: None,
        contract_validated: false,
    })]);

    let receipt = workspace
        .project_agent_graph_from_loaded_nodes(AgentGraphOrgProjectionRequest::new(
            AgentGraphId::new("agent-graph.project").unwrap(),
        ))
        .expect("projection receipt is returned");

    assert_eq!(receipt.status, AgentGraphOrgProjectionStatus::Rejected);
    assert_eq!(
        receipt.rejection,
        Some(AgentGraphOrgProjectionRejection::UnvalidatedContract {
            node_id: "org:planner".to_string()
        })
    );
    assert!(receipt.graph.is_none());
}

#[test]
fn org_agent_graph_projection_rejects_invalid_topology_before_runtime() {
    let workspace = MemoryOrgWorkspace::from_nodes(vec![
        agent_node(AgentNodeFixture {
            org_id: "org:planner",
            graph_id: "agent-graph.project",
            node_id: "planner",
            role: "planner",
            capabilities: "plan",
            loop_graph: "loop.planner",
            loop_entry: "entry",
            memory_scope: None,
            policy_scope: None,
            topology_policy: None,
            contract_validated: true,
        }),
        edge_node(EdgeNodeFixture {
            org_id: "org:planner-to-missing",
            graph_id: "agent-graph.project",
            edge_id: "planner-to-missing",
            from: "planner",
            to: "missing",
            kind: "handoff",
            condition: "always",
            contract_validated: true,
        }),
    ]);

    let receipt = workspace
        .project_agent_graph_from_loaded_nodes(AgentGraphOrgProjectionRequest::new(
            AgentGraphId::new("agent-graph.project").unwrap(),
        ))
        .expect("projection receipt is returned");

    assert_eq!(receipt.status, AgentGraphOrgProjectionStatus::Rejected);
    assert!(matches!(
        receipt.rejection,
        Some(AgentGraphOrgProjectionRejection::InvalidTopology(
            AgentGraphTopologyError::MissingEdgeTarget { .. }
        ))
    ));
    assert!(receipt.graph.is_none());
}

struct AgentNodeFixture<'a> {
    org_id: &'a str,
    graph_id: &'a str,
    node_id: &'a str,
    role: &'a str,
    capabilities: &'a str,
    loop_graph: &'a str,
    loop_entry: &'a str,
    memory_scope: Option<&'a str>,
    policy_scope: Option<&'a str>,
    topology_policy: Option<&'a str>,
    contract_validated: bool,
}

fn agent_node(fixture: AgentNodeFixture<'_>) -> OrgNode {
    let mut node = OrgNode::heading(OrgNodeId::from(fixture.org_id), fixture.node_id);
    node.properties.insert(
        AGENT_GRAPH_ID_PROPERTY.to_string(),
        fixture.graph_id.to_string(),
    );
    node.properties.insert(
        AGENT_GRAPH_NODE_ID_PROPERTY.to_string(),
        fixture.node_id.to_string(),
    );
    node.properties.insert(
        AGENT_GRAPH_ROLE_PROPERTY.to_string(),
        fixture.role.to_string(),
    );
    node.properties.insert(
        AGENT_GRAPH_CAPABILITIES_PROPERTY.to_string(),
        fixture.capabilities.to_string(),
    );
    node.properties.insert(
        AGENT_GRAPH_LOOP_GRAPH_PROPERTY.to_string(),
        fixture.loop_graph.to_string(),
    );
    node.properties.insert(
        AGENT_GRAPH_LOOP_ENTRY_NODE_PROPERTY.to_string(),
        fixture.loop_entry.to_string(),
    );
    if let Some(memory_scope) = fixture.memory_scope {
        node.properties.insert(
            AGENT_GRAPH_MEMORY_SCOPE_REF_PROPERTY.to_string(),
            memory_scope.to_string(),
        );
    }
    if let Some(policy_scope) = fixture.policy_scope {
        node.properties.insert(
            AGENT_GRAPH_POLICY_SCOPE_REF_PROPERTY.to_string(),
            policy_scope.to_string(),
        );
    }
    if let Some(topology_policy) = fixture.topology_policy {
        node.properties.insert(
            AGENT_GRAPH_TOPOLOGY_POLICY_PROPERTY.to_string(),
            topology_policy.to_string(),
        );
    }
    node.properties.insert(
        AGENT_GRAPH_CONTRACT_VALIDATED_PROPERTY.to_string(),
        fixture.contract_validated.to_string(),
    );
    node
}

struct EdgeNodeFixture<'a> {
    org_id: &'a str,
    graph_id: &'a str,
    edge_id: &'a str,
    from: &'a str,
    to: &'a str,
    kind: &'a str,
    condition: &'a str,
    contract_validated: bool,
}

fn edge_node(fixture: EdgeNodeFixture<'_>) -> OrgNode {
    let mut node = OrgNode::heading(OrgNodeId::from(fixture.org_id), fixture.edge_id);
    node.properties.insert(
        AGENT_GRAPH_ID_PROPERTY.to_string(),
        fixture.graph_id.to_string(),
    );
    node.properties.insert(
        AGENT_GRAPH_EDGE_ID_PROPERTY.to_string(),
        fixture.edge_id.to_string(),
    );
    node.properties.insert(
        AGENT_GRAPH_EDGE_FROM_PROPERTY.to_string(),
        fixture.from.to_string(),
    );
    node.properties.insert(
        AGENT_GRAPH_EDGE_TO_PROPERTY.to_string(),
        fixture.to.to_string(),
    );
    node.properties.insert(
        AGENT_GRAPH_EDGE_KIND_PROPERTY.to_string(),
        fixture.kind.to_string(),
    );
    node.properties.insert(
        AGENT_GRAPH_EDGE_CONDITION_PROPERTY.to_string(),
        fixture.condition.to_string(),
    );
    node.properties.insert(
        AGENT_GRAPH_CONTRACT_VALIDATED_PROPERTY.to_string(),
        fixture.contract_validated.to_string(),
    );
    node
}
