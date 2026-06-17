use marlin_agent_graph::{
    AgentEdge, AgentEdgeCondition, AgentEdgeKind, AgentGraph, AgentGraphPlanningRejection,
    AgentGraphPlanningStatus, AgentGraphPlanningTarget, AgentTopologyPolicy,
    plan_agent_coordination,
};

use super::support::{edge_id, graph_id, node, node_id};

#[test]
fn planning_rejects_invalid_graph_before_runtime_admission() {
    let graph = AgentGraph {
        graph_id: graph_id("agent-graph.empty"),
        nodes: Vec::new(),
        edges: Vec::new(),
        topology_policy: AgentTopologyPolicy::Deterministic,
    };
    let target = AgentGraphPlanningTarget::new(graph_id("agent-graph.empty"), node_id("planner"));

    let receipt = plan_agent_coordination(&graph, target);

    assert_eq!(receipt.status, AgentGraphPlanningStatus::Rejected);
    assert!(matches!(
        receipt.rejection,
        Some(AgentGraphPlanningRejection::InvalidGraph { .. })
    ));
    assert!(receipt.plan.is_none());
    assert!(receipt.coordination.is_none());
}

#[test]
fn planning_rejects_missing_root_node_before_runtime_admission() {
    let graph = AgentGraph {
        graph_id: graph_id("agent-graph.root"),
        nodes: vec![
            node("planner", "planner", "loop.plan", "entry"),
            node(
                "implementation",
                "implementer",
                "loop.implementation",
                "entry",
            ),
        ],
        edges: vec![AgentEdge {
            edge_id: edge_id("planner-to-implementation"),
            from: node_id("planner"),
            to: node_id("implementation"),
            kind: AgentEdgeKind::Handoff,
            condition: AgentEdgeCondition::Always,
        }],
        topology_policy: AgentTopologyPolicy::Deterministic,
    };
    let target =
        AgentGraphPlanningTarget::new(graph_id("agent-graph.root"), node_id("verification"));

    let receipt = plan_agent_coordination(&graph, target);

    assert_eq!(receipt.status, AgentGraphPlanningStatus::Rejected);
    assert_eq!(
        receipt.rejection,
        Some(AgentGraphPlanningRejection::MissingRootNode {
            node_id: node_id("verification")
        })
    );
    assert!(receipt.plan.is_none());
    assert!(receipt.coordination.is_none());
}
