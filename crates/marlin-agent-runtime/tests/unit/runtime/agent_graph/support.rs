use marlin_agent_graph::{
    AgentCapability, AgentEdge, AgentEdgeCondition, AgentEdgeId, AgentEdgeKind, AgentGraph,
    AgentGraphId, AgentNode, AgentNodeId, AgentRole, AgentTopologyPolicy, GraphLoopEntryRef,
    GraphLoopGraphRef, GraphLoopNodeRef,
};

pub fn sample_graph() -> AgentGraph {
    AgentGraph {
        graph_id: graph_id("agent-graph.p1"),
        nodes: vec![
            node("planner", "planner", "loop.planner"),
            node("implementation", "implementer", "loop.implementation"),
        ],
        edges: vec![AgentEdge {
            edge_id: edge_id("planner-to-implementation"),
            from: node_id("planner"),
            to: node_id("implementation"),
            kind: AgentEdgeKind::Handoff,
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

pub fn graph_id(value: &str) -> AgentGraphId {
    AgentGraphId::new(value).unwrap()
}

pub fn node_id(value: &str) -> AgentNodeId {
    AgentNodeId::new(value).unwrap()
}

pub fn edge_id(value: &str) -> AgentEdgeId {
    AgentEdgeId::new(value).unwrap()
}
