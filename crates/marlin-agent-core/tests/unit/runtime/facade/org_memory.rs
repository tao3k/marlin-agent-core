use marlin_agent_core::{agent_graph, context::org};

#[test]
fn core_facade_exposes_org_agent_graph_projection_boundary() {
    let mut node =
        org::model::OrgNode::heading(org::model::OrgNodeId::from("org:planner"), "Planner");
    node.properties.insert(
        org::AGENT_GRAPH_ID_PROPERTY.to_string(),
        "core.agent-graph".to_string(),
    );
    node.properties.insert(
        org::AGENT_GRAPH_NODE_ID_PROPERTY.to_string(),
        "planner".to_string(),
    );
    node.properties.insert(
        org::AGENT_GRAPH_ROLE_PROPERTY.to_string(),
        "planner".to_string(),
    );
    node.properties.insert(
        org::AGENT_GRAPH_LOOP_GRAPH_PROPERTY.to_string(),
        "loop.planner".to_string(),
    );
    node.properties.insert(
        org::AGENT_GRAPH_LOOP_ENTRY_NODE_PROPERTY.to_string(),
        "entry".to_string(),
    );
    node.properties.insert(
        org::AGENT_GRAPH_CONTRACT_VALIDATED_PROPERTY.to_string(),
        "true".to_string(),
    );
    let workspace = org::MemoryOrgWorkspace::from_nodes(vec![node]);

    let receipt = workspace
        .project_agent_graph_from_loaded_nodes(org::AgentGraphOrgProjectionRequest::new(
            agent_graph::AgentGraphId::new("core.agent-graph").unwrap(),
        ))
        .expect("Org AgentGraph projection returns a receipt");

    assert_eq!(
        receipt.status,
        org::AgentGraphOrgProjectionStatus::Projected
    );
    let graph = receipt.graph.expect("projected AgentGraph");
    assert_eq!(graph.nodes.len(), 1);
    assert_eq!(graph.nodes[0].loop_entry.graph.as_str(), "loop.planner");
}
