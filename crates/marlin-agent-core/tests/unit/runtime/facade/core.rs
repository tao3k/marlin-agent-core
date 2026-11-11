// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use marlin_agent_core::context::org::workspace::{
    STANDARD_AGENT_MEMORY_CONTRACT_DOCUMENT_ID, STANDARD_AGENT_PLAN_CONTRACT_DOCUMENT_ID,
    STANDARD_AGENT_TOPOLOGY_CONTRACT_DOCUMENT_ID, load_standard_agent_contract_workspace,
    standard_agent_contract_documents,
};
use marlin_agent_core::{
    AGENT_HARNESS_PERFORMANCE_EVIDENCE_KEYS, AgentExecutionTrace, AgentExecutionTraceSummary,
    AgentGraph, AgentGraphPlanningStatus, AgentGraphPlanningTarget, AgentHarnessEvidence,
    AgentHarnessEvidenceKind, AgentHarnessPerformanceEvidence, AgentNode, AgentSpanName,
    AgentTopologyPolicy, AgentTraceSpanRecord, GraphLoopEntryRef, GraphLoopExecutionStatus,
    GraphLoopGraphRef, GraphLoopNodeRef, HookDispatcher, HookRegistry, ModelEndpoint,
    ModelGatewayMessageRole, ModelGatewayRequest, ModelGatewayTransport,
    RuntimeAgentGraphExecutionReadinessStatus, RuntimeEnvironmentRequest,
    RuntimeEnvironmentResolver, RuntimeExecutionIdentity, agent_graph,
    check_agent_graph_execution_readiness, plan_agent_coordination, system_gateway_message,
};

#[test]
fn core_facade_exposes_environment_resolver() {
    let environment = RuntimeEnvironmentResolver::new().resolve(
        RuntimeEnvironmentRequest::default()
            .with_custom_home("/tmp/marlin-home")
            .with_cwd("/tmp/workspace"),
    );

    assert_eq!(
        environment
            .home
            .as_ref()
            .expect("home should be resolved")
            .path,
        std::path::PathBuf::from("/tmp/marlin-home")
    );
}

#[test]
fn core_facade_exposes_hook_dispatcher() {
    let dispatcher = HookDispatcher::new(HookRegistry::new());

    assert_eq!(dispatcher.registry().registrations().len(), 0);
}

#[test]
fn core_facade_exposes_runtime_execution_identity() {
    let identity = RuntimeExecutionIdentity::new("run-core", "graph-core");

    assert_eq!(identity.run_id(), "run-core");
    assert_eq!(identity.graph_id(), "graph-core");
}

#[test]
fn core_facade_exposes_standard_agent_contract_library() {
    let documents = standard_agent_contract_documents();
    let workspace =
        load_standard_agent_contract_workspace().expect("standard agent contracts load");

    assert_eq!(documents.len(), 5);
    assert!(
        documents
            .iter()
            .any(|document| document.id.as_str() == STANDARD_AGENT_PLAN_CONTRACT_DOCUMENT_ID)
    );
    assert!(
        documents
            .iter()
            .any(|document| document.id.as_str() == STANDARD_AGENT_MEMORY_CONTRACT_DOCUMENT_ID)
    );
    assert!(
        documents
            .iter()
            .any(|document| document.id.as_str() == STANDARD_AGENT_TOPOLOGY_CONTRACT_DOCUMENT_ID)
    );
    assert!(
        workspace
            .contracts
            .contracts
            .iter()
            .any(|contract| contract.id.as_str() == "agent.plan.v1")
    );
    assert!(
        workspace
            .contracts
            .contracts
            .iter()
            .any(|contract| contract.id.as_str() == "agent.topology.v1")
    );
    assert!(
        workspace
            .contracts
            .contracts
            .iter()
            .any(|contract| contract.id.as_str() == "agent.memory.v1")
    );
}

#[test]
fn core_facade_exposes_agent_graph_planning_and_runtime_projection() {
    let graph = AgentGraph {
        graph_id: agent_graph::AgentGraphId::new("core.agent-graph").unwrap(),
        nodes: vec![AgentNode {
            node_id: agent_graph::AgentNodeId::new("planner").unwrap(),
            role: agent_graph::AgentRole::new("planner").unwrap(),
            capabilities: vec![agent_graph::AgentCapability::new("planning").unwrap()],
            loop_entry: GraphLoopEntryRef {
                graph: GraphLoopGraphRef::new("loop.planner").unwrap(),
                entry_node: GraphLoopNodeRef::new("entry").unwrap(),
            },
            memory_scope: None,
            policy_scope: None,
        }],
        edges: Vec::new(),
        topology_policy: AgentTopologyPolicy::Deterministic,
    };

    let planning = plan_agent_coordination(
        &graph,
        AgentGraphPlanningTarget::new(
            agent_graph::AgentGraphId::new("core.agent-graph").unwrap(),
            agent_graph::AgentNodeId::new("planner").unwrap(),
        ),
    );
    let request = marlin_agent_core::AgentGraphProjectionRequest::new(
        agent_graph::AgentGraphId::new("core.agent-graph").unwrap(),
        planning.clone(),
        7,
    );
    let readiness = check_agent_graph_execution_readiness(&graph, request);

    assert_eq!(planning.status, AgentGraphPlanningStatus::Planned);
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
}

#[test]
fn core_facade_exposes_model_gateway_protocol_boundary() {
    let request = ModelGatewayRequest::new(
        ModelEndpoint::new("openai", "gpt-5-mini"),
        vec![system_gateway_message("system")],
    )
    .with_transport(ModelGatewayTransport::WebSocket);

    assert_eq!(request.transport(), &ModelGatewayTransport::WebSocket);
    assert_eq!(request.messages()[0].role, ModelGatewayMessageRole::System);
}

#[test]
fn core_facade_exposes_execution_trace_summary() {
    let result_span = AgentTraceSpanRecord::new("harness.result")
        .with_field("run_id", "run")
        .with_field("status", "Completed");
    let trace = AgentExecutionTrace::new("run", "graph", GraphLoopExecutionStatus::Completed)
        .with_spans(vec![result_span]);
    let summary: AgentExecutionTraceSummary = trace.summary();

    assert_eq!(summary.run_id.as_str(), "run");
    assert_eq!(summary.graph_id.as_str(), "graph");
    assert_eq!(summary.status, GraphLoopExecutionStatus::Completed);
    assert!(
        trace
            .find_span_with_field(&AgentSpanName::new("harness.result"), "run_id", "run")
            .is_some()
    );
}

#[test]
fn core_facade_exposes_performance_evidence_contract() {
    let evidence: AgentHarnessEvidence = AgentHarnessPerformanceEvidence {
        subject: "core-runtime".to_owned(),
        benchmark_command: "cargo bench -p marlin-agent-core".to_owned(),
        baseline: "p95=10ms".to_owned(),
        regression_threshold: "5%".to_owned(),
        latency_or_throughput: "throughput=1000/s".to_owned(),
        allocation_profile: "allocations=steady".to_owned(),
        profile_artifact: "target/criterion/core/index.html".to_owned(),
    }
    .into();
    let detail = evidence.detail.as_deref().expect("performance detail");

    assert_eq!(evidence.kind, AgentHarnessEvidenceKind::Performance);
    assert_eq!(evidence.subject, "core-runtime");
    for key in AGENT_HARNESS_PERFORMANCE_EVIDENCE_KEYS {
        assert!(
            detail.contains(key),
            "missing performance evidence key {key}"
        );
    }
}
