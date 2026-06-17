use marlin_agent_graph::AgentCoordinationPlan;
use marlin_agent_runtime::{
    RuntimeAgentCoordinationAdmissionStatus, RuntimeAgentCoordinationRejection,
    admit_agent_coordination_plan,
};

use super::support::{edge_id, graph_id, node_id, sample_graph};

#[test]
fn runtime_admits_coordination_plan_without_owning_topology_selection() {
    let graph = sample_graph();
    let plan = AgentCoordinationPlan {
        graph_id: graph_id("agent-graph.p1"),
        root_node: node_id("implementation"),
        candidate_edges: vec![edge_id("planner-to-implementation")],
    };

    let receipt = admit_agent_coordination_plan(&graph, plan.clone(), 42);

    assert_eq!(
        receipt.status,
        RuntimeAgentCoordinationAdmissionStatus::Accepted
    );
    assert_eq!(receipt.plan, plan);
    assert_eq!(receipt.observed_at_ms, 42);
    assert_eq!(
        receipt
            .root_loop_entry
            .expect("root loop entry")
            .graph
            .as_str(),
        "loop.implementation"
    );
    assert!(receipt.rejection.is_none());
}

#[test]
fn runtime_rejects_plan_for_a_different_agent_graph() {
    let graph = sample_graph();
    let plan = AgentCoordinationPlan {
        graph_id: graph_id("agent-graph.other"),
        root_node: node_id("implementation"),
        candidate_edges: vec![edge_id("planner-to-implementation")],
    };

    let receipt = admit_agent_coordination_plan(&graph, plan, 43);

    assert_eq!(
        receipt.status,
        RuntimeAgentCoordinationAdmissionStatus::Rejected
    );
    assert_eq!(
        receipt.rejection,
        Some(RuntimeAgentCoordinationRejection::GraphMismatch {
            plan_graph_id: graph_id("agent-graph.other"),
            graph_id: graph_id("agent-graph.p1")
        })
    );
    assert!(receipt.root_loop_entry.is_none());
}

#[test]
fn runtime_rejects_plan_with_missing_root_node() {
    let graph = sample_graph();
    let plan = AgentCoordinationPlan {
        graph_id: graph_id("agent-graph.p1"),
        root_node: node_id("missing"),
        candidate_edges: Vec::new(),
    };

    let receipt = admit_agent_coordination_plan(&graph, plan, 44);

    assert_eq!(
        receipt.rejection,
        Some(RuntimeAgentCoordinationRejection::MissingRootNode {
            node_id: node_id("missing")
        })
    );
}

#[test]
fn runtime_rejects_plan_with_missing_candidate_edge() {
    let graph = sample_graph();
    let plan = AgentCoordinationPlan {
        graph_id: graph_id("agent-graph.p1"),
        root_node: node_id("implementation"),
        candidate_edges: vec![edge_id("missing-edge")],
    };

    let receipt = admit_agent_coordination_plan(&graph, plan, 45);

    assert_eq!(
        receipt.rejection,
        Some(RuntimeAgentCoordinationRejection::MissingCandidateEdge {
            edge_id: edge_id("missing-edge")
        })
    );
}
