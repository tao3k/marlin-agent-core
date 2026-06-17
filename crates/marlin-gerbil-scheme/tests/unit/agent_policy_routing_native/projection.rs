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
