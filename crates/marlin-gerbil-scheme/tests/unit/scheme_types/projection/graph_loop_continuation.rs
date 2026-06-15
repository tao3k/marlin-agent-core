use marlin_agent_protocol::{
    GERBIL_LOOP_GRAPH_CONTINUATION_SCHEMA_ID, GerbilLoopGraphContinuationAction,
    GraphLoopNextAction,
};
use marlin_gerbil_scheme::{
    GERBIL_LOOP_GRAPH_CONTINUATION_NATIVE_PROJECTION_ABI_ID,
    GERBIL_LOOP_GRAPH_CONTINUATION_NATIVE_SYMBOL, GERBIL_LOOP_GRAPH_CONTINUATION_TYPE_ID,
    GerbilSchemeNativeAbiId, GerbilSchemeNativeProjectionRequest,
    GerbilSchemeNativeProjectionStatus, GerbilSchemeNativeSymbol, GerbilSchemeSchemaId,
    GerbilSchemeTypeId, GerbilSchemeTypeRegistry, GerbilSchemeTypedValue, GerbilSchemeValue,
    decode_gerbil_loop_graph_continuation_native_projection,
    decode_gerbil_loop_graph_continuation_projection,
    gerbil_loop_graph_continuation_native_projection_request,
    gerbil_loop_graph_continuation_type_manifest, project_gerbil_loop_graph_continuation_action,
    project_gerbil_loop_graph_continuation_native_action,
};

#[test]
fn gerbil_poo_continuation_projection_drives_graph_loop_next_action() {
    let registry = continuation_registry();
    let envelope = GerbilSchemeTypedValue::new(
        continuation_type_id(),
        continuation_payload(continue_with_graph_action_payload()),
    )
    .with_schema_id(continuation_schema_id());

    let projection = decode_gerbil_loop_graph_continuation_projection(&registry, &envelope)
        .expect("Gerbil continuation projection decodes");
    assert!(projection.has_current_schema());
    assert!(projection.diagnostics.is_empty());
    assert!(matches!(
        projection.action,
        GerbilLoopGraphContinuationAction::ContinueWithGraph { .. }
    ));

    let next_action = project_gerbil_loop_graph_continuation_action(&registry, &envelope)
        .expect("Gerbil continuation projection compiles into controller action");
    let GraphLoopNextAction::ContinueWithGraph(graph) = next_action else {
        panic!("expected continuation graph");
    };
    assert_eq!(graph.graph_id, "poo-continuation-graph");
    assert_eq!(graph.nodes[0].executor, "gerbil.poo.policy");
    assert_eq!(
        graph.nodes[0].config.get("source").map(String::as_str),
        Some("poo")
    );
}

#[test]
fn gerbil_poo_continuation_native_projection_validates_abi_before_action_compile() {
    let registry = continuation_registry();
    let envelope = GerbilSchemeTypedValue::new(
        continuation_type_id(),
        continuation_payload(continue_with_graph_action_payload()),
    )
    .with_schema_id(continuation_schema_id());

    let (receipt, projection) =
        decode_gerbil_loop_graph_continuation_native_projection(&registry, &envelope)
            .expect("native continuation projection decodes");

    assert_eq!(
        receipt.status,
        GerbilSchemeNativeProjectionStatus::Projected
    );
    assert_eq!(receipt.abi_id, continuation_abi_id());
    assert_eq!(receipt.symbol, continuation_symbol());
    assert_eq!(receipt.type_id, continuation_type_id());
    assert_eq!(receipt.schema_id, Some(continuation_schema_id()));
    assert!(matches!(
        projection.action,
        GerbilLoopGraphContinuationAction::ContinueWithGraph { .. }
    ));

    let (receipt, next_action) =
        project_gerbil_loop_graph_continuation_native_action(&registry, &envelope)
            .expect("native continuation projection compiles into controller action");
    assert_eq!(receipt.symbol, continuation_symbol());
    assert!(matches!(
        next_action,
        GraphLoopNextAction::ContinueWithGraph(_)
    ));
}

#[test]
fn gerbil_poo_continuation_terminal_projection_stays_typed() {
    let registry = continuation_registry();
    let envelope = GerbilSchemeTypedValue::new(
        continuation_type_id(),
        continuation_payload(GerbilSchemeValue::record([(
            "kind",
            "stop_completed".into(),
        )])),
    )
    .with_schema_id(continuation_schema_id());

    let next_action = project_gerbil_loop_graph_continuation_action(&registry, &envelope)
        .expect("terminal continuation projection compiles");
    assert_eq!(next_action, GraphLoopNextAction::StopCompleted);
}

#[test]
fn gerbil_poo_continuation_projection_rejects_unpreserved_diagnostics() {
    let registry = continuation_registry();
    let envelope = GerbilSchemeTypedValue::new(
        continuation_type_id(),
        diagnostic_continuation_payload(continue_with_graph_action_payload()),
    )
    .with_schema_id(continuation_schema_id());

    let projection = decode_gerbil_loop_graph_continuation_projection(&registry, &envelope)
        .expect("diagnostic-bearing projection still decodes");
    assert_eq!(projection.diagnostics, vec!["poo_continuation=continue"]);

    let error = project_gerbil_loop_graph_continuation_action(&registry, &envelope).expect_err(
        "diagnostic-bearing projection should not compile without receipt preservation",
    );
    assert!(
        error
            .to_string()
            .contains("DiagnosticRejected([\"poo_continuation=continue\"])")
    );
}

#[test]
fn gerbil_poo_continuation_projection_rejects_wrong_schema_before_payload_decode() {
    let registry = continuation_registry();
    let envelope = GerbilSchemeTypedValue::new(
        continuation_type_id(),
        GerbilSchemeValue::record([
            (
                "schema_id",
                "marlin.agent.gerbil_loop_graph_continuation.v0".into(),
            ),
            ("action", "payload should not decode".into()),
        ]),
    )
    .with_schema_id(GerbilSchemeSchemaId::new(
        "marlin.agent.gerbil_loop_graph_continuation.v0",
    ));

    let error = decode_gerbil_loop_graph_continuation_projection(&registry, &envelope)
        .expect_err("projection contract should reject stale schemas before serde decode");
    assert!(
        error
            .to_string()
            .contains("expected marlin.agent.gerbil_loop_graph_continuation.v1")
    );
}

#[test]
fn gerbil_poo_continuation_native_projection_request_round_trips_as_protocol_data() {
    let request = gerbil_loop_graph_continuation_native_projection_request();

    let encoded = serde_json::to_string(&request).expect("encode continuation request");
    let decoded: GerbilSchemeNativeProjectionRequest =
        serde_json::from_str(&encoded).expect("decode continuation request");

    assert_eq!(decoded, request);
    assert_eq!(decoded.abi_id, continuation_abi_id());
    assert_eq!(decoded.symbol, continuation_symbol());
}

fn continuation_registry() -> GerbilSchemeTypeRegistry {
    GerbilSchemeTypeRegistry::new(continuation_manifest()).expect("continuation manifest")
}

fn continuation_manifest() -> marlin_gerbil_scheme::GerbilSchemeTypeManifest {
    gerbil_loop_graph_continuation_type_manifest()
}

fn continuation_payload(action: GerbilSchemeValue) -> GerbilSchemeValue {
    GerbilSchemeValue::record([
        ("schema_id", GERBIL_LOOP_GRAPH_CONTINUATION_SCHEMA_ID.into()),
        ("action", action),
    ])
}

fn diagnostic_continuation_payload(action: GerbilSchemeValue) -> GerbilSchemeValue {
    GerbilSchemeValue::record([
        ("schema_id", GERBIL_LOOP_GRAPH_CONTINUATION_SCHEMA_ID.into()),
        ("action", action),
        (
            "diagnostics",
            GerbilSchemeValue::vector(["poo_continuation=continue".into()]),
        ),
    ])
}

fn continue_with_graph_action_payload() -> GerbilSchemeValue {
    GerbilSchemeValue::record([
        ("kind", "continue_with_graph".into()),
        (
            "compiled_graph",
            GerbilSchemeValue::record([
                ("graph_id", "poo-continuation-graph".into()),
                (
                    "nodes",
                    GerbilSchemeValue::vector([GerbilSchemeValue::record([
                        ("id", "policy".into()),
                        ("executor", "gerbil.poo.policy".into()),
                        (
                            "config",
                            GerbilSchemeValue::record([("source", "poo".into())]),
                        ),
                    ])]),
                ),
                ("edges", GerbilSchemeValue::vector([])),
            ]),
        ),
    ])
}

fn continuation_type_id() -> GerbilSchemeTypeId {
    GerbilSchemeTypeId::new(GERBIL_LOOP_GRAPH_CONTINUATION_TYPE_ID)
}

fn continuation_schema_id() -> GerbilSchemeSchemaId {
    GerbilSchemeSchemaId::new(GERBIL_LOOP_GRAPH_CONTINUATION_SCHEMA_ID)
}

fn continuation_abi_id() -> GerbilSchemeNativeAbiId {
    GerbilSchemeNativeAbiId::new(GERBIL_LOOP_GRAPH_CONTINUATION_NATIVE_PROJECTION_ABI_ID)
}

fn continuation_symbol() -> GerbilSchemeNativeSymbol {
    GerbilSchemeNativeSymbol::new(GERBIL_LOOP_GRAPH_CONTINUATION_NATIVE_SYMBOL)
}
