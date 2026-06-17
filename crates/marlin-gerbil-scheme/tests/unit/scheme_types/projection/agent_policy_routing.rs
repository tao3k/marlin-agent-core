use marlin_agent_graph::{AgentCoordinationEvidenceKind, AgentPolicyRoutingDecision};
use marlin_gerbil_scheme::{
    GERBIL_AGENT_POLICY_ROUTING_NATIVE_PROJECTION_ABI_ID,
    GERBIL_AGENT_POLICY_ROUTING_NATIVE_SYMBOL, GERBIL_AGENT_POLICY_ROUTING_SCHEMA_ID,
    GERBIL_AGENT_POLICY_ROUTING_TYPE_ID, GerbilSchemeNativeAbiId,
    GerbilSchemeNativeProjectionRequest, GerbilSchemeNativeProjectionStatus,
    GerbilSchemeNativeSymbol, GerbilSchemeSchemaId, GerbilSchemeTypeDecodeError,
    GerbilSchemeTypeId, GerbilSchemeTypeRegistry, GerbilSchemeTypedValue, GerbilSchemeValue,
    decode_gerbil_agent_policy_routing_native_projection,
    decode_gerbil_agent_policy_routing_projection,
    gerbil_agent_policy_routing_native_projection_request,
    gerbil_agent_policy_routing_type_manifest, project_gerbil_agent_policy_routing_native_receipt,
    project_gerbil_agent_policy_routing_receipt,
};

#[test]
fn gerbil_agent_policy_routing_projection_projects_typed_policy_receipt() {
    let registry = policy_routing_registry();
    let envelope = policy_routing_envelope(policy_routing_payload());

    let projection = decode_gerbil_agent_policy_routing_projection(&registry, &envelope)
        .expect("Gerbil AgentGraph policy routing projection decodes");
    assert!(projection.has_current_schema());
    assert_eq!(projection.candidate_edges, vec!["planner-to-custom"]);

    let receipt = project_gerbil_agent_policy_routing_receipt(&registry, &envelope)
        .expect("Gerbil AgentGraph policy routing projection becomes a typed Rust receipt");
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

#[test]
fn gerbil_agent_policy_routing_native_projection_validates_abi_before_receipt_projection() {
    let registry = policy_routing_registry();
    let envelope = policy_routing_envelope(policy_routing_payload());

    let (native_receipt, projection) =
        decode_gerbil_agent_policy_routing_native_projection(&registry, &envelope)
            .expect("native AgentGraph policy routing projection decodes");
    assert_eq!(
        native_receipt.status,
        GerbilSchemeNativeProjectionStatus::Projected
    );
    assert_eq!(native_receipt.abi_id, policy_routing_abi_id());
    assert_eq!(native_receipt.symbol, policy_routing_symbol());
    assert_eq!(native_receipt.type_id, policy_routing_type_id());
    assert_eq!(native_receipt.schema_id, Some(policy_routing_schema_id()));
    assert_eq!(
        projection.decision,
        marlin_gerbil_scheme::GerbilAgentPolicyRoutingDecision::SelectEdges
    );

    let (native_receipt, policy_receipt) =
        project_gerbil_agent_policy_routing_native_receipt(&registry, &envelope)
            .expect("native AgentGraph policy routing projection becomes a typed Rust receipt");
    assert_eq!(native_receipt.symbol, policy_routing_symbol());
    assert_eq!(
        policy_receipt.decision,
        AgentPolicyRoutingDecision::SelectEdges
    );
}

#[test]
fn gerbil_agent_policy_routing_projection_rejects_payload_schema_drift() {
    let registry = policy_routing_registry();
    let envelope = policy_routing_envelope(policy_routing_payload_with_schema(
        "marlin.agent.policy-routing.v0",
    ));

    let error = decode_gerbil_agent_policy_routing_projection(&registry, &envelope)
        .expect_err("payload schema drift should be rejected by Rust projection");
    assert_rust_projection_error(error, "does not match marlin.agent.policy-routing.v1");
}

#[test]
fn gerbil_agent_policy_routing_projection_rejects_wrong_envelope_schema_before_payload_decode() {
    let registry = policy_routing_registry();
    let envelope = GerbilSchemeTypedValue::new(
        policy_routing_type_id(),
        GerbilSchemeValue::record([
            ("schema_id", "marlin.agent.policy-routing.v0".into()),
            ("graph_id", "payload should not decode".into()),
        ]),
    )
    .with_schema_id(GerbilSchemeSchemaId::new("marlin.agent.policy-routing.v0"));

    let error = decode_gerbil_agent_policy_routing_projection(&registry, &envelope)
        .expect_err("projection contract should reject stale schemas before payload decode");
    assert!(
        error
            .to_string()
            .contains("expected marlin.agent.policy-routing.v1")
    );
}

#[test]
fn gerbil_agent_policy_routing_native_projection_request_declares_typed_contract() {
    let request = gerbil_agent_policy_routing_native_projection_request();

    assert_eq!(
        request,
        GerbilSchemeNativeProjectionRequest::new(
            policy_routing_abi_id(),
            1,
            policy_routing_symbol(),
            marlin_gerbil_scheme::gerbil_agent_policy_routing_projection_contract(),
        )
    );
}

fn policy_routing_registry() -> GerbilSchemeTypeRegistry {
    GerbilSchemeTypeRegistry::new(gerbil_agent_policy_routing_type_manifest())
        .expect("AgentGraph policy routing manifest")
}

fn policy_routing_envelope(payload: GerbilSchemeValue) -> GerbilSchemeTypedValue {
    GerbilSchemeTypedValue::new(policy_routing_type_id(), payload)
        .with_schema_id(policy_routing_schema_id())
}

fn policy_routing_payload() -> GerbilSchemeValue {
    policy_routing_payload_with_schema(GERBIL_AGENT_POLICY_ROUTING_SCHEMA_ID)
}

fn policy_routing_payload_with_schema(schema_id: &str) -> GerbilSchemeValue {
    GerbilSchemeValue::record([
        ("schema_id", schema_id.into()),
        ("graph_id", "agent-graph.policy".into()),
        ("policy_scope", "gerbil.scope.agent-topology".into()),
        ("root_node", "planner".into()),
        ("decision", "select_edges".into()),
        (
            "candidate_edges",
            GerbilSchemeValue::vector(["planner-to-custom".into()]),
        ),
        (
            "evidence",
            GerbilSchemeValue::vector([GerbilSchemeValue::record([
                ("kind", "gerbil_policy_receipt".into()),
                ("evidence_id", "gerbil.policy.receipt.1".into()),
            ])]),
        ),
    ])
}

fn policy_routing_type_id() -> GerbilSchemeTypeId {
    GerbilSchemeTypeId::new(GERBIL_AGENT_POLICY_ROUTING_TYPE_ID)
}

fn policy_routing_schema_id() -> GerbilSchemeSchemaId {
    GerbilSchemeSchemaId::new(GERBIL_AGENT_POLICY_ROUTING_SCHEMA_ID)
}

fn policy_routing_abi_id() -> GerbilSchemeNativeAbiId {
    GerbilSchemeNativeAbiId::new(GERBIL_AGENT_POLICY_ROUTING_NATIVE_PROJECTION_ABI_ID)
}

fn policy_routing_symbol() -> GerbilSchemeNativeSymbol {
    GerbilSchemeNativeSymbol::new(GERBIL_AGENT_POLICY_ROUTING_NATIVE_SYMBOL)
}

fn assert_rust_projection_error(error: GerbilSchemeTypeDecodeError, needle: &str) {
    let GerbilSchemeTypeDecodeError::RustProjection { message } = error else {
        panic!("unexpected non-Rust projection decode error: {error}");
    };

    assert!(
        message.contains(needle),
        "Rust projection error did not contain {needle:?}: {message}"
    );
}
