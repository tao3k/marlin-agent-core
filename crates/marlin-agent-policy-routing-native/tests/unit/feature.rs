#[test]
fn package_identity_marks_agent_policy_routing_native_bridge() {
    assert_eq!(env!("CARGO_PKG_NAME"), "marlin-agent-policy-routing-native");
}

#[cfg(feature = "linked-native")]
#[test]
fn linked_native_selector_projects_policy_routing_receipt() {
    use marlin_agent_graph::{AgentCoordinationEvidenceKind, AgentPolicyRoutingDecision};
    use marlin_agent_policy_routing_native::linked_agent_policy_routing_native_selector;
    use marlin_gerbil_scheme::{
        GerbilAgentPolicyRoutingEvidenceKind, GerbilAgentPolicyRoutingNativeSelectEdgesRequest,
    };

    let request = GerbilAgentPolicyRoutingNativeSelectEdgesRequest::new(
        "agent-graph.policy",
        "gerbil.scope.agent-topology",
        "planner",
    )
    .with_candidate_edge("planner-to-custom")
    .with_evidence(
        GerbilAgentPolicyRoutingEvidenceKind::GerbilPolicyReceipt,
        "gerbil.policy.receipt.1",
    );

    let (_, receipt) = linked_agent_policy_routing_native_selector()
        .project_policy_receipt(&request)
        .expect("linked Gerbil selector should project a typed policy-routing receipt");

    assert_eq!(receipt.graph_id.as_str(), "agent-graph.policy");
    assert_eq!(receipt.policy_scope.as_str(), "gerbil.scope.agent-topology");
    assert_eq!(receipt.root_node.as_str(), "planner");
    assert_eq!(receipt.decision, AgentPolicyRoutingDecision::SelectEdges);
    assert_eq!(receipt.candidate_edges[0].as_str(), "planner-to-custom");
    assert_eq!(
        receipt.evidence[0].kind,
        AgentCoordinationEvidenceKind::GerbilPolicyReceipt
    );
    assert_eq!(
        receipt.evidence[0].evidence_id.as_str(),
        "gerbil.policy.receipt.1"
    );
}
