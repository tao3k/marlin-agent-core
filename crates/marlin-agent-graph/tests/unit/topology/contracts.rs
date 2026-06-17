use marlin_agent_graph::{
    AgentCoordinationDecision, AgentCoordinationEvidenceKind, AgentCoordinationReceipt, AgentEdge,
    AgentEdgeCondition, AgentEdgeKind, AgentElectionReason, AgentElectionReceipt, AgentGraph,
    AgentGraphId, AgentNode, AgentTopologyPolicy, GerbilPolicyScopeRef, GraphLoopEntryRef,
    GraphLoopGraphRef, GraphLoopNodeRef, OrgMemoryScopeRef,
};

use super::support::{capability, edge_id, evidence, graph_id, node, node_id, role};

#[test]
fn linear_handoff_topology_round_trips_as_typed_contract() {
    let graph = AgentGraph {
        graph_id: graph_id("agent-graph.linear"),
        nodes: vec![
            node("research", "researcher", "loop.research", "entry"),
            node(
                "implementation",
                "implementer",
                "loop.implementation",
                "entry",
            ),
        ],
        edges: vec![AgentEdge {
            edge_id: edge_id("research-to-implementation"),
            from: node_id("research"),
            to: node_id("implementation"),
            kind: AgentEdgeKind::Handoff,
            condition: AgentEdgeCondition::Always,
        }],
        topology_policy: AgentTopologyPolicy::Deterministic,
    };

    graph.validate().expect("linear topology should be valid");

    let encoded = serde_json::to_string(&graph).expect("agent graph should serialize");
    let decoded: AgentGraph = serde_json::from_str(&encoded).expect("agent graph should decode");

    assert_eq!(decoded, graph);
}

#[test]
fn fanout_fanin_verification_topology_keeps_coordination_out_of_runtime() {
    let graph = AgentGraph {
        graph_id: graph_id("agent-graph.verify"),
        nodes: vec![
            node("planner", "planner", "loop.plan", "entry"),
            node(
                "implementation",
                "implementer",
                "loop.implementation",
                "entry",
            ),
            node("verification", "verifier", "loop.verify", "entry"),
            node("review", "reviewer", "loop.review", "entry"),
        ],
        edges: vec![
            AgentEdge {
                edge_id: edge_id("planner-to-implementation"),
                from: node_id("planner"),
                to: node_id("implementation"),
                kind: AgentEdgeKind::Fanout,
                condition: AgentEdgeCondition::CapabilityRequired(capability("rust-edit")),
            },
            AgentEdge {
                edge_id: edge_id("planner-to-verification"),
                from: node_id("planner"),
                to: node_id("verification"),
                kind: AgentEdgeKind::Fanout,
                condition: AgentEdgeCondition::CapabilityRequired(capability("verify")),
            },
            AgentEdge {
                edge_id: edge_id("verification-to-review"),
                from: node_id("verification"),
                to: node_id("review"),
                kind: AgentEdgeKind::Fanin,
                condition: AgentEdgeCondition::Always,
            },
        ],
        topology_policy: AgentTopologyPolicy::CapabilityFirst,
    };

    graph
        .validate()
        .expect("fanout/fanin topology should be valid");

    let encoded = serde_json::to_string(&graph).expect("agent graph should serialize");
    assert!(encoded.contains("CapabilityRequired"));
    assert!(encoded.contains("Fanin"));
}

#[test]
fn reviewer_election_receipt_preserves_typed_evidence_refs() {
    let receipt = AgentElectionReceipt {
        graph_id: graph_id("agent-graph.review"),
        candidates: vec![node_id("reviewer-a"), node_id("reviewer-b")],
        selected: node_id("reviewer-b"),
        reason: AgentElectionReason::new("policy.selected.lowest-load").unwrap(),
        evidence: vec![evidence(
            AgentCoordinationEvidenceKind::GerbilPolicyReceipt,
            "policy.receipt.1",
        )],
    };

    let encoded = serde_json::to_string(&receipt).expect("receipt should serialize");
    let decoded: AgentElectionReceipt =
        serde_json::from_str(&encoded).expect("receipt should decode");

    assert_eq!(decoded, receipt);
}

#[test]
fn graph_loop_entry_is_a_reference_not_an_execution_surface() {
    let agent_node = AgentNode {
        node_id: node_id("implementation"),
        role: role("implementer"),
        capabilities: vec![capability("rust-edit")],
        loop_entry: GraphLoopEntryRef {
            graph: GraphLoopGraphRef::new("loop.graph.implementation").unwrap(),
            entry_node: GraphLoopNodeRef::new("loop.node.start").unwrap(),
        },
        memory_scope: Some(OrgMemoryScopeRef::new("memory.scope.project").unwrap()),
        policy_scope: Some(GerbilPolicyScopeRef::new("policy.scope.implementation").unwrap()),
    };

    let encoded = serde_json::to_value(&agent_node).expect("node should serialize");

    assert_eq!(encoded["loop_entry"]["graph"], "loop.graph.implementation");
    assert_eq!(encoded["memory_scope"], "memory.scope.project");
    assert_eq!(encoded["policy_scope"], "policy.scope.implementation");
    assert!(encoded.get("memory_query").is_none());
    assert!(encoded.get("policy_program").is_none());
    assert!(encoded.get("tool").is_none());
    assert!(encoded.get("command").is_none());
    assert!(encoded.get("session").is_none());
    assert!(encoded.get("worktree").is_none());
}

#[test]
fn empty_semantic_ids_are_rejected_before_serialization() {
    let error = AgentGraphId::new(" ").expect_err("blank id should fail");

    assert_eq!(error.to_string(), "AgentGraphId cannot be empty");
}

#[test]
fn coordination_receipt_links_decision_to_graph_without_inline_logs() {
    let receipt = AgentCoordinationReceipt {
        graph_id: graph_id("agent-graph.coordinate"),
        selected_node: node_id("implementation"),
        selected_edge: Some(edge_id("planner-to-implementation")),
        decision: AgentCoordinationDecision::FollowEdge,
        evidence: vec![evidence(
            AgentCoordinationEvidenceKind::LoopReceipt,
            "loop.receipt.42",
        )],
    };

    let encoded = serde_json::to_string(&receipt).expect("receipt should serialize");
    let decoded: AgentCoordinationReceipt =
        serde_json::from_str(&encoded).expect("receipt should decode");

    assert_eq!(decoded, receipt);
}
