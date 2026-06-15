use marlin_agent_harness::{
    HarnessEvidence, HarnessEvidenceGraph, HarnessEvidenceGraphEdge, HarnessEvidenceGraphEdgeKind,
    HarnessEvidenceGraphNode, HarnessEvidenceGraphNodeKind, HarnessEvidenceKind,
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
        HarnessEvidence::present(HarnessEvidenceKind::Runtime, "runtime:tokio"),
        HarnessEvidence::missing(HarnessEvidenceKind::Safety, "safety:missing-review"),
    ];

    let graph = HarnessEvidenceGraph::from_harness_evidence("harness:evidence-facts", &evidence);
    let summary = graph.summary();

    assert!(graph.has_node_kind(HarnessEvidenceGraphNodeKind::ExecutionReceipt));
    assert!(graph.has_node_kind(HarnessEvidenceGraphNodeKind::EvidenceFact));
    assert_eq!(summary.nodes, 2);
    assert_eq!(summary.missing_nodes, 1);
    assert_eq!(
        graph.nodes[0].source_evidence_kind,
        Some(HarnessEvidenceKind::Runtime)
    );
}

#[test]
fn harness_evidence_graph_accepts_reliability_core_nodes() {
    let graph = HarnessEvidenceGraph::new("harness:reliability-core")
        .with_node(HarnessEvidenceGraphNode::present(
            "intent:ship-runtime",
            HarnessEvidenceGraphNodeKind::HumanIntent,
            "ship runtime safely",
        ))
        .with_node(HarnessEvidenceGraphNode::present(
            "invariant:typed-native-abi",
            HarnessEvidenceGraphNodeKind::TypeInvariant,
            "typed native ABI",
        ))
        .with_node(HarnessEvidenceGraphNode::present(
            "test:no-llm-loop",
            HarnessEvidenceGraphNodeKind::TestBehavior,
            "no LLM deterministic loop",
        ))
        .with_node(HarnessEvidenceGraphNode::present(
            "proof:acyclic-loop",
            HarnessEvidenceGraphNodeKind::ProofResult,
            "acyclic loop validation",
        ))
        .with_node(HarnessEvidenceGraphNode::missing(
            "counterexample:budget-overrun",
            HarnessEvidenceGraphNodeKind::Counterexample,
            "budget overrun",
        ))
        .with_node(HarnessEvidenceGraphNode::present(
            "review:maintainer-accepted",
            HarnessEvidenceGraphNodeKind::ReviewJudgment,
            "maintainer accepted",
        ))
        .with_edge(HarnessEvidenceGraphEdge::new(
            "invariant:typed-native-abi",
            "proof:acyclic-loop",
            HarnessEvidenceGraphEdgeKind::Proves,
        ))
        .with_edge(HarnessEvidenceGraphEdge::new(
            "counterexample:budget-overrun",
            "intent:ship-runtime",
            HarnessEvidenceGraphEdgeKind::Refutes,
        ));
    let summary = graph.summary();

    assert!(graph.has_node_kind(HarnessEvidenceGraphNodeKind::HumanIntent));
    assert!(graph.has_node_kind(HarnessEvidenceGraphNodeKind::TypeInvariant));
    assert!(graph.has_node_kind(HarnessEvidenceGraphNodeKind::TestBehavior));
    assert!(graph.has_node_kind(HarnessEvidenceGraphNodeKind::ProofResult));
    assert!(graph.has_node_kind(HarnessEvidenceGraphNodeKind::Counterexample));
    assert!(graph.has_node_kind(HarnessEvidenceGraphNodeKind::ReviewJudgment));
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

    let accepted_graph =
        HarnessEvidenceGraph::from_graph_policy_proposal_receipt("proposal:accepted", &accepted);
    let rejected_graph =
        HarnessEvidenceGraph::from_graph_policy_proposal_receipt("proposal:rejected", &rejected);

    assert!(accepted_graph.has_node_kind(HarnessEvidenceGraphNodeKind::ReviewJudgment));
    assert!(accepted_graph.has_node_kind(HarnessEvidenceGraphNodeKind::ProofResult));
    assert!(accepted_graph.edges.iter().any(|edge| {
        edge.kind == HarnessEvidenceGraphEdgeKind::Proves
            && edge.from == "review:graph-policy-proposal:static-loop-policy"
    }));
    assert!(rejected_graph.has_node_kind(HarnessEvidenceGraphNodeKind::ReviewJudgment));
    assert!(rejected_graph.has_node_kind(HarnessEvidenceGraphNodeKind::Counterexample));
    assert_eq!(rejected_graph.summary().counterexamples, 1);
    assert!(rejected_graph.edges.iter().any(|edge| {
        edge.kind == HarnessEvidenceGraphEdgeKind::Refutes
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

    let completed_graph =
        HarnessEvidenceGraph::new("execution:completed").with_graph_execution_result(&completed);
    let failed_graph =
        HarnessEvidenceGraph::new("execution:failed").with_graph_execution_result(&failed);

    assert!(completed_graph.has_node_kind(HarnessEvidenceGraphNodeKind::ExecutionReceipt));
    assert!(completed_graph.has_node_kind(HarnessEvidenceGraphNodeKind::ProofResult));
    assert!(completed_graph.nodes.iter().any(|node| {
        node.subject == "graph-loop-node:run-ok:plan"
            && node.detail.as_deref().is_some_and(|detail| {
                detail.contains("executor=planner") && detail.contains("status=Completed")
            })
    }));
    assert!(completed_graph.edges.iter().any(|edge| {
        edge.kind == HarnessEvidenceGraphEdgeKind::Observes
            && edge.from == "execution:graph-loop:run-ok"
    }));
    assert!(failed_graph.has_node_kind(HarnessEvidenceGraphNodeKind::ExecutionReceipt));
    assert!(failed_graph.has_node_kind(HarnessEvidenceGraphNodeKind::Counterexample));
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
