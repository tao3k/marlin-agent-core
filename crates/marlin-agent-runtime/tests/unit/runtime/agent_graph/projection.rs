use marlin_agent_graph::{
    AgentGraph, AgentGraphPlanningReceipt, AgentGraphPlanningStatus, AgentGraphPlanningTarget,
    AgentTopologyPolicy, plan_agent_coordination,
};
use marlin_agent_protocol::AgentGraphProjectionRequest;
use marlin_agent_runtime::{
    RuntimeAgentCoordinationRejection, RuntimeAgentGraphProjectionRejection,
    RuntimeAgentGraphProjectionStatus, project_agent_graph_planning_receipt,
    project_agent_graph_projection_request,
};

use super::support::{edge_id, graph_id, node_id, sample_graph};

#[test]
fn runtime_projects_successful_agent_graph_planning_receipt_to_admission() {
    let graph = sample_graph();
    let planning = plan_agent_coordination(
        &graph,
        AgentGraphPlanningTarget::new(graph_id("agent-graph.p1"), node_id("planner")),
    );

    let projection = project_agent_graph_planning_receipt(&graph, planning.clone(), 46);

    assert_eq!(
        projection.status,
        RuntimeAgentGraphProjectionStatus::Projected
    );
    assert_eq!(projection.planning, planning);
    assert_eq!(
        projection
            .admission
            .expect("admission receipt")
            .root_loop_entry
            .expect("root loop entry")
            .graph
            .as_str(),
        "loop.planner"
    );
    assert!(projection.rejection.is_none());
}

#[test]
fn runtime_projection_preserves_planning_rejection_without_admission() {
    let graph = sample_graph();
    let planning = plan_agent_coordination(
        &graph,
        AgentGraphPlanningTarget::new(graph_id("agent-graph.other"), node_id("planner")),
    );

    let projection = project_agent_graph_planning_receipt(&graph, planning, 47);

    assert_eq!(
        projection.status,
        RuntimeAgentGraphProjectionStatus::Rejected
    );
    assert_eq!(
        projection.rejection,
        Some(RuntimeAgentGraphProjectionRejection::PlanningRejected)
    );
    assert!(projection.admission.is_none());
}

#[test]
fn runtime_projection_rejects_planned_receipt_without_plan() {
    let graph = sample_graph();
    let planning = AgentGraphPlanningReceipt {
        target: AgentGraphPlanningTarget::new(graph_id("agent-graph.p1"), node_id("planner")),
        status: AgentGraphPlanningStatus::Planned,
        plan: None,
        coordination: None,
        rejection: None,
    };

    let projection = project_agent_graph_planning_receipt(&graph, planning, 49);

    assert_eq!(
        projection.status,
        RuntimeAgentGraphProjectionStatus::Rejected
    );
    assert_eq!(
        projection.rejection,
        Some(RuntimeAgentGraphProjectionRejection::MissingPlan)
    );
    assert!(projection.admission.is_none());
}

#[test]
fn runtime_projection_preserves_admission_rejection_after_planning() {
    let graph = sample_graph();
    let planning = plan_agent_coordination(
        &graph,
        AgentGraphPlanningTarget::new(graph_id("agent-graph.p1"), node_id("planner")),
    );
    let admission_graph = AgentGraph {
        graph_id: graph_id("agent-graph.p1"),
        nodes: graph.nodes.clone(),
        edges: Vec::new(),
        topology_policy: AgentTopologyPolicy::Deterministic,
    };

    let projection = project_agent_graph_planning_receipt(&admission_graph, planning, 48);

    assert_eq!(
        projection.status,
        RuntimeAgentGraphProjectionStatus::Rejected
    );
    assert_eq!(
        projection.rejection,
        Some(RuntimeAgentGraphProjectionRejection::AdmissionRejected(
            RuntimeAgentCoordinationRejection::MissingCandidateEdge {
                edge_id: edge_id("planner-to-implementation")
            }
        ))
    );
    assert!(projection.admission.is_some());
}

#[test]
fn runtime_projects_agent_graph_protocol_request() {
    let graph = sample_graph();
    let planning = plan_agent_coordination(
        &graph,
        AgentGraphPlanningTarget::new(graph_id("agent-graph.p1"), node_id("planner")),
    );
    let request = AgentGraphProjectionRequest::new(graph_id("agent-graph.p1"), planning, 50);

    let projection = project_agent_graph_projection_request(&graph, request);

    assert_eq!(
        projection.status,
        RuntimeAgentGraphProjectionStatus::Projected
    );
    assert_eq!(projection.observed_at_ms, 50);
    assert!(projection.admission.is_some());
}

#[test]
fn runtime_rejects_protocol_request_with_unsupported_schema() {
    let graph = sample_graph();
    let planning = plan_agent_coordination(
        &graph,
        AgentGraphPlanningTarget::new(graph_id("agent-graph.p1"), node_id("planner")),
    );
    let mut request = AgentGraphProjectionRequest::new(graph_id("agent-graph.p1"), planning, 51);
    request.schema_id = "marlin.agent_graph.projection_request.v0".to_owned();

    let projection = project_agent_graph_projection_request(&graph, request);

    assert_eq!(
        projection.status,
        RuntimeAgentGraphProjectionStatus::Rejected
    );
    assert_eq!(
        projection.rejection,
        Some(
            RuntimeAgentGraphProjectionRejection::UnsupportedRequestSchema {
                schema_id: "marlin.agent_graph.projection_request.v0".to_owned()
            }
        )
    );
    assert!(projection.admission.is_none());
}

#[test]
fn runtime_rejects_protocol_request_for_different_agent_graph() {
    let graph = sample_graph();
    let planning = plan_agent_coordination(
        &graph,
        AgentGraphPlanningTarget::new(graph_id("agent-graph.p1"), node_id("planner")),
    );
    let request = AgentGraphProjectionRequest::new(graph_id("agent-graph.other"), planning, 52);

    let projection = project_agent_graph_projection_request(&graph, request);

    assert_eq!(
        projection.status,
        RuntimeAgentGraphProjectionStatus::Rejected
    );
    assert_eq!(
        projection.rejection,
        Some(RuntimeAgentGraphProjectionRejection::RequestGraphMismatch {
            request_graph_id: graph_id("agent-graph.other"),
            graph_id: graph_id("agent-graph.p1")
        })
    );
    assert!(projection.admission.is_none());
}
