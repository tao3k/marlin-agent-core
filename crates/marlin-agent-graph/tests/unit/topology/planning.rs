use marlin_agent_graph::{
    AgentCoordinationDecision, AgentCoordinationEvidenceKind, AgentEdge, AgentEdgeCondition,
    AgentEdgeKind, AgentGraph, AgentGraphPlanningRejection, AgentGraphPlanningStatus,
    AgentGraphPlanningTarget, AgentPolicyDecisionRef, AgentPolicyRoutingDecision,
    AgentPolicyRoutingReceipt, AgentTopologyPolicy, GerbilPolicyScopeRef, plan_agent_coordination,
    plan_agent_coordination_with_policy_receipt,
};

use super::support::{capability, edge_id, evidence, graph_id, node, node_id};

#[test]
fn deterministic_planning_projects_root_node_and_outgoing_edges() {
    let graph = AgentGraph {
        graph_id: graph_id("agent-graph.planning"),
        nodes: vec![
            node("planner", "planner", "loop.plan", "entry"),
            node(
                "implementation",
                "implementer",
                "loop.implementation",
                "entry",
            ),
            node("verification", "verifier", "loop.verify", "entry"),
        ],
        edges: vec![
            AgentEdge {
                edge_id: edge_id("planner-to-implementation"),
                from: node_id("planner"),
                to: node_id("implementation"),
                kind: AgentEdgeKind::Handoff,
                condition: AgentEdgeCondition::Always,
            },
            AgentEdge {
                edge_id: edge_id("planner-to-verification"),
                from: node_id("planner"),
                to: node_id("verification"),
                kind: AgentEdgeKind::Review,
                condition: AgentEdgeCondition::Always,
            },
        ],
        topology_policy: AgentTopologyPolicy::Deterministic,
    };
    let target =
        AgentGraphPlanningTarget::new(graph_id("agent-graph.planning"), node_id("planner"))
            .with_evidence(vec![evidence(
                AgentCoordinationEvidenceKind::OrgMemoryReceipt,
                "memory.receipt.1",
            )]);

    let receipt = plan_agent_coordination(&graph, target);

    assert_eq!(receipt.status, AgentGraphPlanningStatus::Planned);
    let plan = receipt.plan.expect("planning should produce plan");
    assert_eq!(
        plan.candidate_edges,
        vec![
            edge_id("planner-to-implementation"),
            edge_id("planner-to-verification")
        ]
    );
    let coordination = receipt.coordination.expect("coordination receipt");
    assert_eq!(coordination.decision, AgentCoordinationDecision::SelectNode);
    assert_eq!(coordination.selected_node, node_id("planner"));
    assert_eq!(coordination.evidence.len(), 1);
}

#[test]
fn capability_first_planning_filters_edges_by_target_capability() {
    let graph = AgentGraph {
        graph_id: graph_id("agent-graph.capability"),
        nodes: vec![
            node("planner", "planner", "loop.plan", "entry"),
            node(
                "implementation",
                "rust-edit",
                "loop.implementation",
                "entry",
            ),
            node("verification", "verify", "loop.verify", "entry"),
        ],
        edges: vec![
            AgentEdge {
                edge_id: edge_id("planner-to-implementation"),
                from: node_id("planner"),
                to: node_id("implementation"),
                kind: AgentEdgeKind::Delegate,
                condition: AgentEdgeCondition::CapabilityRequired(capability("rust-edit")),
            },
            AgentEdge {
                edge_id: edge_id("planner-to-verification"),
                from: node_id("planner"),
                to: node_id("verification"),
                kind: AgentEdgeKind::Delegate,
                condition: AgentEdgeCondition::CapabilityRequired(capability("verify")),
            },
        ],
        topology_policy: AgentTopologyPolicy::CapabilityFirst,
    };
    let target =
        AgentGraphPlanningTarget::new(graph_id("agent-graph.capability"), node_id("planner"))
            .with_required_capability(capability("verify"));

    let receipt = plan_agent_coordination(&graph, target);

    assert_eq!(
        receipt
            .plan
            .expect("planning should produce plan")
            .candidate_edges,
        vec![edge_id("planner-to-verification")]
    );
}

#[test]
fn policy_scoped_planning_rejects_without_typed_gerbil_receipt() {
    let graph = AgentGraph {
        graph_id: graph_id("agent-graph.policy"),
        nodes: vec![
            node("planner", "planner", "loop.plan", "entry"),
            node("custom", "custom-agent", "loop.custom", "entry"),
        ],
        edges: vec![AgentEdge {
            edge_id: edge_id("planner-to-custom"),
            from: node_id("planner"),
            to: node_id("custom"),
            kind: AgentEdgeKind::Delegate,
            condition: AgentEdgeCondition::PolicyDecision(
                AgentPolicyDecisionRef::new("gerbil.policy.route.custom").unwrap(),
            ),
        }],
        topology_policy: AgentTopologyPolicy::PolicyScoped(
            GerbilPolicyScopeRef::new("gerbil.scope.agent-topology").unwrap(),
        ),
    };
    let target = AgentGraphPlanningTarget::new(graph_id("agent-graph.policy"), node_id("planner"));

    let receipt = plan_agent_coordination(&graph, target);

    assert_eq!(
        receipt.rejection,
        Some(AgentGraphPlanningRejection::MissingPolicyReceipt {
            policy_scope: GerbilPolicyScopeRef::new("gerbil.scope.agent-topology").unwrap()
        })
    );
    assert!(receipt.plan.is_none());
}

#[test]
fn policy_scoped_planning_consumes_typed_gerbil_policy_receipt() {
    let graph = AgentGraph {
        graph_id: graph_id("agent-graph.policy"),
        nodes: vec![
            node("planner", "planner", "loop.plan", "entry"),
            node("custom", "custom-agent", "loop.custom", "entry"),
            node("review", "reviewer", "loop.review", "entry"),
        ],
        edges: vec![
            AgentEdge {
                edge_id: edge_id("planner-to-custom"),
                from: node_id("planner"),
                to: node_id("custom"),
                kind: AgentEdgeKind::Delegate,
                condition: AgentEdgeCondition::PolicyDecision(
                    AgentPolicyDecisionRef::new("gerbil.policy.route.custom").unwrap(),
                ),
            },
            AgentEdge {
                edge_id: edge_id("planner-to-review"),
                from: node_id("planner"),
                to: node_id("review"),
                kind: AgentEdgeKind::Review,
                condition: AgentEdgeCondition::Always,
            },
        ],
        topology_policy: AgentTopologyPolicy::PolicyScoped(
            GerbilPolicyScopeRef::new("gerbil.scope.agent-topology").unwrap(),
        ),
    };
    let target = AgentGraphPlanningTarget::new(graph_id("agent-graph.policy"), node_id("planner"));
    let policy_receipt = AgentPolicyRoutingReceipt {
        graph_id: graph_id("agent-graph.policy"),
        policy_scope: GerbilPolicyScopeRef::new("gerbil.scope.agent-topology").unwrap(),
        root_node: node_id("planner"),
        decision: AgentPolicyRoutingDecision::SelectEdges,
        candidate_edges: vec![edge_id("planner-to-custom")],
        evidence: vec![evidence(
            AgentCoordinationEvidenceKind::GerbilPolicyReceipt,
            "gerbil.policy.receipt.1",
        )],
    };

    let receipt = plan_agent_coordination_with_policy_receipt(&graph, target, policy_receipt);

    assert_eq!(receipt.status, AgentGraphPlanningStatus::Planned);
    assert_eq!(
        receipt
            .plan
            .expect("planning should produce plan")
            .candidate_edges,
        vec![edge_id("planner-to-custom")]
    );
    assert_eq!(
        receipt
            .coordination
            .expect("coordination receipt")
            .evidence
            .len(),
        1
    );
}

#[test]
fn planning_rejects_graph_mismatch_before_runtime_admission() {
    let graph = AgentGraph {
        graph_id: graph_id("agent-graph.actual"),
        nodes: vec![node("planner", "planner", "loop.plan", "entry")],
        edges: Vec::new(),
        topology_policy: AgentTopologyPolicy::Deterministic,
    };
    let target =
        AgentGraphPlanningTarget::new(graph_id("agent-graph.expected"), node_id("planner"));

    let receipt = plan_agent_coordination(&graph, target);

    assert_eq!(receipt.status, AgentGraphPlanningStatus::Rejected);
    assert_eq!(
        receipt.rejection,
        Some(AgentGraphPlanningRejection::GraphMismatch {
            target_graph_id: graph_id("agent-graph.expected"),
            graph_id: graph_id("agent-graph.actual")
        })
    );
}
