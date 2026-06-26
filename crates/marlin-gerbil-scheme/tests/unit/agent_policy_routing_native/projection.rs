use super::support::{fake_select_edges, routing_request};
use marlin_agent_graph::{AgentCoordinationEvidenceKind, AgentPolicyRoutingDecision};
use marlin_gerbil_scheme::{
    GERBIL_AGENT_POLICY_ROUTING_NATIVE_PROJECTION_ABI_ID,
    GERBIL_AGENT_POLICY_ROUTING_NATIVE_SYMBOL, GERBIL_AGENT_POLICY_ROUTING_SCHEMA_ID,
    GERBIL_AGENT_POLICY_ROUTING_TYPE_ID, GerbilAgentPolicyRoutingNativeSelector,
    GerbilSchemeNativeAbiId, GerbilSchemeNativeProjectionStatus, GerbilSchemeNativeSymbol,
    GerbilSchemeSchemaId, GerbilSchemeTypeId,
};

#[test]
fn agent_policy_routing_native_selector_projects_typed_receipt() {
    let selector = GerbilAgentPolicyRoutingNativeSelector::new(fake_select_edges);
    let request = routing_request();

    let typed_value = selector
        .project_typed_value(&request)
        .expect("native selector should build Rust-owned typed Scheme projection");
    assert_eq!(
        typed_value.type_id,
        GerbilSchemeTypeId::new(GERBIL_AGENT_POLICY_ROUTING_TYPE_ID)
    );
    assert_eq!(
        typed_value.schema_id,
        Some(GerbilSchemeSchemaId::new(
            GERBIL_AGENT_POLICY_ROUTING_SCHEMA_ID
        ))
    );

    let (native_receipt, policy_receipt) = selector
        .project_policy_receipt(&request)
        .expect("native selector should project typed AgentGraph routing receipt");

    assert_eq!(
        native_receipt.status,
        GerbilSchemeNativeProjectionStatus::Projected
    );
    assert_eq!(
        native_receipt.abi_id,
        GerbilSchemeNativeAbiId::new(GERBIL_AGENT_POLICY_ROUTING_NATIVE_PROJECTION_ABI_ID)
    );
    assert_eq!(
        native_receipt.symbol,
        GerbilSchemeNativeSymbol::new(GERBIL_AGENT_POLICY_ROUTING_NATIVE_SYMBOL)
    );
    assert_eq!(policy_receipt.graph_id.as_str(), "agent-graph.policy");
    assert_eq!(
        policy_receipt.policy_scope.as_str(),
        "gerbil.scope.agent-topology"
    );
    assert_eq!(policy_receipt.root_node.as_str(), "planner");
    assert_eq!(
        policy_receipt.decision,
        AgentPolicyRoutingDecision::SelectEdges
    );
    assert_eq!(
        policy_receipt.candidate_edges[0].as_str(),
        "planner-to-custom"
    );
    assert_eq!(
        policy_receipt.evidence[0].kind,
        AgentCoordinationEvidenceKind::GerbilPolicyReceipt
    );
    assert_eq!(
        policy_receipt.evidence[0].evidence_id.as_str(),
        "gerbil.policy.receipt.1"
    );
}

#[test]
fn agent_policy_routing_native_selector_reuses_epoch_match_key_backing() {
    let selector = GerbilAgentPolicyRoutingNativeSelector::new(fake_select_edges);
    let request = routing_request();
    let epoch_backing =
        marlin_gerbil_scheme::GerbilAgentPolicyRoutingNativeEpochBacking::from_request(&request);
    let payload = request.payload();

    let payload_profile = epoch_backing.native_conversion_profile_for_payload(&payload);
    assert_eq!(payload_profile.scalar_string_count, 0);
    assert_eq!(payload_profile.reused_epoch_scalar_string_count, 3);
    assert_eq!(payload_profile.candidate_edge_count, 1);
    assert_eq!(payload_profile.evidence_count, 1);
    assert_eq!(payload_profile.copied_string_count, 3);

    let (_, policy_receipt) = selector
        .project_policy_receipt_with_epoch_backing(&epoch_backing, &payload)
        .expect("epoch-backed selector should project typed policy-routing receipt");

    assert_eq!(policy_receipt.graph_id.as_str(), "agent-graph.policy");
    assert_eq!(
        policy_receipt.policy_scope.as_str(),
        "gerbil.scope.agent-topology"
    );
    assert_eq!(policy_receipt.root_node.as_str(), "planner");
    assert_eq!(
        policy_receipt.decision,
        AgentPolicyRoutingDecision::SelectEdges
    );
    assert_eq!(
        policy_receipt.candidate_edges[0].as_str(),
        "planner-to-custom"
    );
}

#[test]
fn agent_policy_routing_native_request_conversion_profile_counts_copied_backing() {
    let request = routing_request()
        .with_candidate_edge("planner-to-reviewer")
        .with_evidence(
            marlin_gerbil_scheme::GerbilAgentPolicyRoutingEvidenceKind::LoopReceipt,
            "loop.receipt.2",
        );

    let profile = request.native_conversion_profile();

    assert_eq!(profile.scalar_string_count, 3);
    assert_eq!(profile.candidate_edge_count, 2);
    assert_eq!(profile.evidence_count, 2);
    assert_eq!(profile.copied_string_count, 9);
    assert_eq!(profile.raw_utf8_descriptor_count, 9);
    assert_eq!(profile.raw_evidence_descriptor_count, 2);
    assert_eq!(profile.raw_list_descriptor_count, 2);
    assert_eq!(profile.backing_vector_count, 4);
    assert_eq!(profile.reused_epoch_scalar_string_count, 0);
    assert_eq!(
        profile.copied_utf8_bytes,
        "agent-graph.policy".len()
            + "gerbil.scope.agent-topology".len()
            + "planner".len()
            + "planner-to-custom".len()
            + "planner-to-reviewer".len()
            + "gerbil_policy_receipt".len()
            + "gerbil.policy.receipt.1".len()
            + "loop_receipt".len()
            + "loop.receipt.2".len()
    );
}
