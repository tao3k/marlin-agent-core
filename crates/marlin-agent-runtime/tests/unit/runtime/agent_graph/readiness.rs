use marlin_agent_graph::{AgentGraphPlanningTarget, plan_agent_coordination};
use marlin_agent_protocol::AgentGraphProjectionRequest;
use marlin_agent_runtime::{
    RuntimeAgentGraphExecutionReadinessRejection, RuntimeAgentGraphExecutionReadinessStatus,
    RuntimeAgentGraphProjectionRejection, check_agent_graph_execution_readiness,
};
use marlin_agent_test_support::agent_graph_readiness_replay_artifact_fixture;

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
fn runtime_readiness_satisfies_replay_artifact_without_execution_surfaces() {
    let artifact = agent_graph_readiness_replay_artifact_fixture();
    let graph = sample_graph();
    let planning = plan_agent_coordination(
        &graph,
        AgentGraphPlanningTarget::new(graph_id("agent-graph.p1"), node_id("planner")),
    );
    let request = AgentGraphProjectionRequest::new(graph_id("agent-graph.p1"), planning, 55);

    let readiness = check_agent_graph_execution_readiness(&graph, request);
    let encoded = serde_json::to_value(&readiness).expect("readiness serializes");
    let root_loop_entry = readiness.root_loop_entry.as_ref().expect("root loop entry");
    let evidence = artifact.replay_evidence();

    assert_eq!(
        readiness.status,
        RuntimeAgentGraphExecutionReadinessStatus::Ready
    );
    assert_eq!(root_loop_entry.graph.as_str(), "loop.planner");
    assert!(detail_contains(evidence, "planning_status=Planned"));
    assert!(detail_contains(evidence, "projection_status=Projected"));
    assert!(detail_contains(evidence, "readiness_status=Ready"));
    assert!(detail_contains(evidence, "execution_request=false"));
    assert!(detail_contains(evidence, "graph_loop_execution=false"));
    assert!(detail_contains(evidence, "controller_execution=false"));
    assert!(detail_contains(evidence, "tool_execution=false"));
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

fn detail_contains(
    evidence: &[marlin_agent_harness_types::AgentHarnessEvidence],
    needle: &str,
) -> bool {
    evidence
        .iter()
        .filter_map(|entry| entry.detail.as_deref())
        .any(|detail| detail.contains(needle))
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
