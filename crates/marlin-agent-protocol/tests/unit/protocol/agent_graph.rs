use marlin_agent_graph::{
    AgentCapability, AgentEdge, AgentEdgeCondition, AgentEdgeKind, AgentGraph, AgentGraphId,
    AgentGraphPlanningStatus, AgentGraphPlanningTarget, AgentNode, AgentNodeId, AgentRole,
    AgentTopologyPolicy, GraphLoopEntryRef, GraphLoopGraphRef, GraphLoopNodeRef,
    plan_agent_coordination,
};
use marlin_agent_protocol::{
    AGENT_GRAPH_PROJECTION_REQUEST_SCHEMA_ID, AgentGraphProjectionRequest,
};

#[test]
fn agent_graph_projection_request_wraps_planning_receipt_without_runtime_execution() {
    let graph = sample_graph();
    let planning = plan_agent_coordination(
        &graph,
        AgentGraphPlanningTarget::new(graph_id("agent-graph.protocol"), node_id("planner")),
    );

    let request =
        AgentGraphProjectionRequest::new(graph_id("agent-graph.protocol"), planning.clone(), 1_000);

    assert!(request.has_current_schema());
    assert_eq!(request.schema_id, AGENT_GRAPH_PROJECTION_REQUEST_SCHEMA_ID);
    assert_eq!(request.graph_id, graph_id("agent-graph.protocol"));
    assert_eq!(request.planning.status, AgentGraphPlanningStatus::Planned);
    assert_eq!(request.planning, planning);
    assert_eq!(request.observed_at_ms, 1_000);

    let encoded = serde_json::to_value(&request).expect("request serializes");
    assert_eq!(
        encoded["schema_id"],
        AGENT_GRAPH_PROJECTION_REQUEST_SCHEMA_ID
    );
    assert_eq!(encoded["graph_id"], "agent-graph.protocol");
    assert!(encoded.get("execution_request").is_none());
    assert!(encoded.get("controller").is_none());
    assert!(encoded.get("tool").is_none());
}

fn sample_graph() -> AgentGraph {
    AgentGraph {
        graph_id: graph_id("agent-graph.protocol"),
        nodes: vec![node("planner", "planner", "loop.planner")],
        edges: vec![AgentEdge {
            edge_id: marlin_agent_graph::AgentEdgeId::new("planner-self").unwrap(),
            from: node_id("planner"),
            to: node_id("planner"),
            kind: AgentEdgeKind::Review,
            condition: AgentEdgeCondition::Always,
        }],
        topology_policy: AgentTopologyPolicy::Deterministic,
    }
}

fn node(id: &str, role: &str, loop_graph: &str) -> AgentNode {
    AgentNode {
        node_id: node_id(id),
        role: AgentRole::new(role).unwrap(),
        capabilities: vec![AgentCapability::new(role).unwrap()],
        loop_entry: GraphLoopEntryRef {
            graph: GraphLoopGraphRef::new(loop_graph).unwrap(),
            entry_node: GraphLoopNodeRef::new("entry").unwrap(),
        },
        memory_scope: None,
        policy_scope: None,
    }
}

fn graph_id(value: &str) -> AgentGraphId {
    AgentGraphId::new(value).unwrap()
}

fn node_id(value: &str) -> AgentNodeId {
    AgentNodeId::new(value).unwrap()
}
