// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Cross-crate runtime fixture integration tests relocated from
//! `marlin-agent-runtime` to keep the Cargo local dependency graph acyclic for
//! `asp-rust` workspace policy enforcement.

use std::sync::Arc;

use marlin_agent_graph::{
    AgentCapability, AgentEdge, AgentEdgeCondition, AgentEdgeId, AgentEdgeKind, AgentGraph,
    AgentGraphId, AgentNode, AgentNodeId, AgentRole, AgentTopologyPolicy, GraphLoopEntryRef,
    GraphLoopGraphRef, GraphLoopNodeRef, AgentGraphPlanningTarget, plan_agent_coordination,
};
use marlin_agent_protocol::AgentGraphProjectionRequest;
use marlin_agent_runtime::{
    AgentSessionContext, CancellationToken, RuntimeAgentGraphExecutionReadinessStatus,
    RuntimeContext, RuntimeEnvironment, RuntimeFuture, SubAgentRuntime, TokioAgentRuntime,
    check_agent_graph_execution_readiness,
};
use marlin_agent_test_support::{
    agent_graph_readiness_replay_artifact_fixture, assert_sub_agent_memory_session_fixture,
    sub_agent_memory_allowed_fixture, sub_agent_memory_denied_fixture,
    sub_agent_memory_session_replay_evidence,
};

#[tokio::test]
async fn configured_sub_agent_inherits_memory_when_parent_session_allows_it() {
    let fixture = sub_agent_memory_allowed_fixture();
    let (runtime, _events) = TokioAgentRuntime::with_session(
        4,
        CancellationToken::new(),
        RuntimeEnvironment::default(),
        fixture.parent_session().clone(),
    );

    let (task, receipt) = runtime.spawn_sub_agent_with_config(
        Arc::new(SessionEchoSubAgent),
        (),
        fixture.config().clone(),
    );
    let child_session = task
        .join()
        .await
        .expect("configured sub-agent session task should finish");

    assert_sub_agent_memory_session_fixture(
        &fixture,
        &child_session,
        receipt.config(),
        receipt.isolation_receipt(),
    );
}

#[tokio::test]
async fn configured_sub_agent_memory_visibility_is_denied_without_parent_grant() {
    let fixture = sub_agent_memory_denied_fixture();
    let (runtime, _events) = TokioAgentRuntime::with_session(
        4,
        CancellationToken::new(),
        RuntimeEnvironment::default(),
        fixture.parent_session().clone(),
    );

    let (task, receipt) = runtime.spawn_sub_agent_with_config(
        Arc::new(SessionEchoSubAgent),
        (),
        fixture.config().clone(),
    );
    let child_session = task
        .join()
        .await
        .expect("configured sub-agent session task should finish");

    assert_sub_agent_memory_session_fixture(
        &fixture,
        &child_session,
        receipt.config(),
        receipt.isolation_receipt(),
    );
}

#[tokio::test]
async fn configured_sub_agent_memory_replay_evidence_records_visibility_contraction() {
    let fixture = sub_agent_memory_denied_fixture();
    let (runtime, _events) = TokioAgentRuntime::with_session(
        4,
        CancellationToken::new(),
        RuntimeEnvironment::default(),
        fixture.parent_session().clone(),
    );

    let (task, receipt) = runtime.spawn_sub_agent_with_config(
        Arc::new(SessionEchoSubAgent),
        (),
        fixture.config().clone(),
    );
    let child_session = task
        .join()
        .await
        .expect("configured sub-agent session task should finish");
    let evidence =
        sub_agent_memory_session_replay_evidence(&child_session, receipt.isolation_receipt());
    let detail = evidence.detail.as_deref().expect("replay detail");

    assert!(detail.contains("parent_session_id=session/root"));
    assert!(detail.contains("requested_namespaces=[System,User,Workspace,Memory]"));
    assert!(detail.contains("granted_namespaces=[System,User,Workspace]"));
    assert!(detail.contains("denied_namespaces=[Memory]"));
    assert!(detail.contains("visibility_contracted=true"));
    assert!(detail.contains("live_llm=false"));
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

#[derive(Clone, Debug)]
struct SessionEchoSubAgent;

impl SubAgentRuntime for SessionEchoSubAgent {
    type Input = ();
    type Output = AgentSessionContext;

    fn run_sub_agent(
        &self,
        _input: Self::Input,
        context: RuntimeContext,
    ) -> RuntimeFuture<Self::Output> {
        let session = context.session().clone();
        Box::pin(async move { session })
    }
}

fn sample_graph() -> AgentGraph {
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

fn graph_id(value: &str) -> AgentGraphId {
    AgentGraphId::new(value).unwrap()
}

fn node_id(value: &str) -> AgentNodeId {
    AgentNodeId::new(value).unwrap()
}

fn edge_id(value: &str) -> AgentEdgeId {
    AgentEdgeId::new(value).unwrap()
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
