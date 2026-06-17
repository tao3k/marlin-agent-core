use marlin_agent_graph::{AgentGraphPlanningTarget, plan_agent_coordination};
use marlin_agent_protocol::AgentGraphProjectionRequest;
use marlin_agent_runtime::{
    RuntimeAgentGraphExecutionReadinessRejection, RuntimeAgentGraphExecutionReadinessStatus,
    RuntimeAgentGraphProjectionRejection, check_agent_graph_execution_readiness,
};

use super::support::{graph_id, node_id, sample_graph};

#[test]
fn runtime_readiness_projects_root_loop_entry_without_execution_request() {
    let graph = sample_graph();
    let planning = plan_agent_coordination(
        &graph,
        AgentGraphPlanningTarget::new(graph_id("agent-graph.p1"), node_id("planner")),
    );
    let request = AgentGraphProjectionRequest::new(graph_id("agent-graph.p1"), planning, 52);

    let readiness = check_agent_graph_execution_readiness(&graph, request);
    let encoded = serde_json::to_value(&readiness).expect("readiness serializes");

    assert_eq!(
        readiness.status,
        RuntimeAgentGraphExecutionReadinessStatus::Ready
    );
    assert_eq!(
        readiness
            .root_loop_entry
            .expect("root loop entry")
            .graph
            .as_str(),
        "loop.planner"
    );
    assert!(readiness.rejection.is_none());
    assert!(encoded.get("execution_request").is_none());
    assert!(encoded.get("controller").is_none());
    assert!(encoded.get("tool").is_none());
}

#[test]
fn runtime_readiness_preserves_projection_rejection() {
    let graph = sample_graph();
    let planning = plan_agent_coordination(
        &graph,
        AgentGraphPlanningTarget::new(graph_id("agent-graph.other"), node_id("planner")),
    );
    let request = AgentGraphProjectionRequest::new(graph_id("agent-graph.p1"), planning, 53);

    let readiness = check_agent_graph_execution_readiness(&graph, request);

    assert_eq!(
        readiness.status,
        RuntimeAgentGraphExecutionReadinessStatus::NotReady
    );
    assert_eq!(
        readiness.rejection,
        Some(
            RuntimeAgentGraphExecutionReadinessRejection::ProjectionRejected(
                RuntimeAgentGraphProjectionRejection::PlanningRejected
            )
        )
    );
    assert!(readiness.root_loop_entry.is_none());
}

#[test]
fn runtime_readiness_rejects_unsupported_protocol_schema() {
    let graph = sample_graph();
    let planning = plan_agent_coordination(
        &graph,
        AgentGraphPlanningTarget::new(graph_id("agent-graph.p1"), node_id("planner")),
    );
    let mut request = AgentGraphProjectionRequest::new(graph_id("agent-graph.p1"), planning, 54);
    request.schema_id = "marlin.agent_graph.projection_request.v0".to_owned();

    let readiness = check_agent_graph_execution_readiness(&graph, request);

    assert_eq!(
        readiness.status,
        RuntimeAgentGraphExecutionReadinessStatus::NotReady
    );
    assert_eq!(
        readiness.rejection,
        Some(
            RuntimeAgentGraphExecutionReadinessRejection::ProjectionRejected(
                RuntimeAgentGraphProjectionRejection::UnsupportedRequestSchema {
                    schema_id: "marlin.agent_graph.projection_request.v0".to_owned()
                }
            )
        )
    );
    assert!(readiness.root_loop_entry.is_none());
}
