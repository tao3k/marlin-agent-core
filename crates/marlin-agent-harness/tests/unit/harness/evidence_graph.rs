use marlin_agent_harness::{
    AgentHarnessEvidence, AgentHarnessEvidenceGraph, AgentHarnessEvidenceGraphEdge,
    AgentHarnessEvidenceGraphEdgeKind, AgentHarnessEvidenceGraphNode,
    AgentHarnessEvidenceGraphNodeKind, AgentHarnessEvidenceKind,
};
use marlin_agent_protocol::{
    GraphLoopExecutionResult, GraphLoopExecutionStatus, GraphLoopStrategy,
    GraphNodeExecutionReceipt, GraphPolicyProposal, GraphPolicyProposalReceipt, LoopGraph,
    LoopNodeSpec, RuntimePlanSnapshot,
};
use std::collections::BTreeMap;

#[test]
fn harness_evidence_graph_projects_harness_evidence_facts() {
    let evidence = vec![
        AgentHarnessEvidence::present(AgentHarnessEvidenceKind::Runtime, "runtime:tokio"),
        AgentHarnessEvidence::missing(AgentHarnessEvidenceKind::Safety, "safety:missing-review"),
    ];

    let graph =
        AgentHarnessEvidenceGraph::from_agent_harness_evidence("harness:evidence-facts", &evidence);
    let summary = graph.summary();

    assert!(graph.has_node_kind(AgentHarnessEvidenceGraphNodeKind::ExecutionReceipt));
    assert!(graph.has_node_kind(AgentHarnessEvidenceGraphNodeKind::EvidenceFact));
    assert_eq!(summary.nodes, 2);
    assert_eq!(summary.missing_nodes, 1);
    assert_eq!(
        graph.nodes[0].source_evidence_kind,
        Some(AgentHarnessEvidenceKind::Runtime)
    );
}

#[test]
fn harness_evidence_graph_accepts_reliability_core_nodes() {
    let graph = AgentHarnessEvidenceGraph::new("harness:reliability-core")
        .with_node(AgentHarnessEvidenceGraphNode::present(
            "intent:ship-runtime",
            AgentHarnessEvidenceGraphNodeKind::HumanIntent,
            "ship runtime safely",
        ))
        .with_node(AgentHarnessEvidenceGraphNode::present(
            "invariant:typed-native-abi",
            AgentHarnessEvidenceGraphNodeKind::TypeInvariant,
            "typed native ABI",
        ))
        .with_node(AgentHarnessEvidenceGraphNode::present(
            "test:no-llm-loop",
            AgentHarnessEvidenceGraphNodeKind::TestBehavior,
            "no LLM deterministic loop",
        ))
        .with_node(AgentHarnessEvidenceGraphNode::present(
            "proof:acyclic-loop",
            AgentHarnessEvidenceGraphNodeKind::ProofResult,
            "acyclic loop validation",
        ))
        .with_node(AgentHarnessEvidenceGraphNode::missing(
            "counterexample:budget-overrun",
            AgentHarnessEvidenceGraphNodeKind::Counterexample,
            "budget overrun",
        ))
        .with_node(AgentHarnessEvidenceGraphNode::present(
            "review:maintainer-accepted",
            AgentHarnessEvidenceGraphNodeKind::ReviewJudgment,
            "maintainer accepted",
        ))
        .with_edge(AgentHarnessEvidenceGraphEdge::new(
            "invariant:typed-native-abi",
            "proof:acyclic-loop",
            AgentHarnessEvidenceGraphEdgeKind::Proves,
        ))
        .with_edge(AgentHarnessEvidenceGraphEdge::new(
            "counterexample:budget-overrun",
            "intent:ship-runtime",
            AgentHarnessEvidenceGraphEdgeKind::Refutes,
        ));
    let summary = graph.summary();

    assert!(graph.has_node_kind(AgentHarnessEvidenceGraphNodeKind::HumanIntent));
    assert!(graph.has_node_kind(AgentHarnessEvidenceGraphNodeKind::TypeInvariant));
    assert!(graph.has_node_kind(AgentHarnessEvidenceGraphNodeKind::TestBehavior));
    assert!(graph.has_node_kind(AgentHarnessEvidenceGraphNodeKind::ProofResult));
    assert!(graph.has_node_kind(AgentHarnessEvidenceGraphNodeKind::Counterexample));
    assert!(graph.has_node_kind(AgentHarnessEvidenceGraphNodeKind::ReviewJudgment));
    assert_eq!(summary.nodes, 6);
    assert_eq!(summary.edges, 2);
    assert_eq!(summary.missing_nodes, 1);
    assert_eq!(summary.counterexamples, 1);
    assert_eq!(summary.review_judgments, 1);
}

#[test]
fn harness_evidence_graph_projects_policy_receipts_into_review_and_outcome_nodes() {
    let proposal = GraphPolicyProposal::new(
        GraphLoopStrategy::new(
            "static-loop-policy",
            "v1",
            marlin_agent_protocol::GraphLoopStrategyRuntime::StaticPolicy,
        ),
        LoopGraph {
            graph_id: "graph".to_owned(),
            nodes: vec![LoopNodeSpec {
                id: "plan".to_owned(),
                executor: "planner".to_owned(),
                config: BTreeMap::new(),
            }],
            edges: Vec::new(),
        },
        "sha256:input",
        "sha256:output",
    );
    let accepted = GraphPolicyProposalReceipt::accepted(&proposal);
    let rejected = GraphPolicyProposalReceipt::rejected(
        &proposal,
        vec!["graph_policy_proposal.node_executor_empty:plan".to_owned()],
    );

    let accepted_graph = AgentHarnessEvidenceGraph::from_graph_policy_proposal_receipt(
        "proposal:accepted",
        &accepted,
    );
    let rejected_graph = AgentHarnessEvidenceGraph::from_graph_policy_proposal_receipt(
        "proposal:rejected",
        &rejected,
    );

    assert!(accepted_graph.has_node_kind(AgentHarnessEvidenceGraphNodeKind::ReviewJudgment));
    assert!(accepted_graph.has_node_kind(AgentHarnessEvidenceGraphNodeKind::ProofResult));
    assert!(accepted_graph.edges.iter().any(|edge| {
        edge.kind == AgentHarnessEvidenceGraphEdgeKind::Proves
            && edge.from == "review:graph-policy-proposal:static-loop-policy"
    }));
    assert!(rejected_graph.has_node_kind(AgentHarnessEvidenceGraphNodeKind::ReviewJudgment));
    assert!(rejected_graph.has_node_kind(AgentHarnessEvidenceGraphNodeKind::Counterexample));
    assert_eq!(rejected_graph.summary().counterexamples, 1);
    assert!(rejected_graph.edges.iter().any(|edge| {
        edge.kind == AgentHarnessEvidenceGraphEdgeKind::Refutes
            && edge.to == "invariant:graph-policy-proposal:static-loop-policy"
    }));
}

#[test]
fn harness_evidence_graph_projects_execution_results_into_receipts_and_outcomes() {
    let completed = GraphLoopExecutionResult::completed(
        RuntimePlanSnapshot {
            run_id: "run-ok".to_owned(),
            graph_id: "graph".to_owned(),
            active_node: None,
        },
        vec!["plan".to_owned(), "apply".to_owned()],
    )
    .with_node_receipts(vec![
        GraphNodeExecutionReceipt::completed("plan", "planner"),
        GraphNodeExecutionReceipt::completed("apply", "tool"),
    ]);
    let failed = GraphLoopExecutionResult::failed(
        RuntimePlanSnapshot {
            run_id: "run-fail".to_owned(),
            graph_id: "graph".to_owned(),
            active_node: None,
        },
        vec!["budget exceeded".to_owned()],
    )
    .with_node_receipts(vec![GraphNodeExecutionReceipt::failed(
        "plan",
        "planner",
        vec!["budget exceeded".to_owned()],
    )]);

    let completed_graph = AgentHarnessEvidenceGraph::new("execution:completed")
        .with_graph_execution_result(&completed);
    let failed_graph =
        AgentHarnessEvidenceGraph::new("execution:failed").with_graph_execution_result(&failed);

    assert!(completed_graph.has_node_kind(AgentHarnessEvidenceGraphNodeKind::ExecutionReceipt));
    assert!(completed_graph.has_node_kind(AgentHarnessEvidenceGraphNodeKind::ProofResult));
    assert!(completed_graph.nodes.iter().any(|node| {
        node.subject == "graph-loop-node:run-ok:plan"
            && node.detail.as_deref().is_some_and(|detail| {
                detail.contains("executor=planner") && detail.contains("status=Completed")
            })
    }));
    assert!(completed_graph.edges.iter().any(|edge| {
        edge.kind == AgentHarnessEvidenceGraphEdgeKind::Observes
            && edge.from == "execution:graph-loop:run-ok"
    }));
    assert!(failed_graph.has_node_kind(AgentHarnessEvidenceGraphNodeKind::ExecutionReceipt));
    assert!(failed_graph.has_node_kind(AgentHarnessEvidenceGraphNodeKind::Counterexample));
    assert_eq!(failed_graph.summary().counterexamples, 1);
    assert!(failed_graph.nodes.iter().any(|node| {
        node.subject == "graph-loop-node:run-fail:plan"
            && node
                .detail
                .as_deref()
                .is_some_and(|detail| detail.contains("status=Failed"))
    }));
    assert!(failed_graph.nodes.iter().any(|node| {
        node.subject == "graph-loop-Failed:run-fail"
            && node
                .detail
                .as_deref()
                .is_some_and(|detail| detail.contains("budget exceeded"))
    }));
    assert_eq!(failed.status, GraphLoopExecutionStatus::Failed);
}
