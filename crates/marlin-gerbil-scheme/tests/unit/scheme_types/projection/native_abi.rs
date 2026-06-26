use marlin_gerbil_scheme::{
    GERBIL_DECK_RUNTIME_NATIVE_PROJECTION_ABI_ID,
    GERBIL_DECK_RUNTIME_POO_POLICY_PROJECTION_SCHEMA_ID,
    GERBIL_DECK_RUNTIME_POO_POLICY_PROJECTION_TYPE_ID,
    GERBIL_DECK_RUNTIME_PROJECT_POO_POLICY_SYMBOL,
    GERBIL_DECK_RUNTIME_PROJECT_RESOLVED_LOOP_POLICY_PACK_SYMBOL,
    GERBIL_RESOLVED_LOOP_POLICY_PACK_SCHEMA_ID, GERBIL_RESOLVED_LOOP_POLICY_PACK_TYPE_ID,
    GerbilDeckRuntimePooPolicyProjection, GerbilSchemeNativeAbiId,
    GerbilSchemeNativeAbiReadinessPlan, GerbilSchemeNativeProjectionRequest,
    GerbilSchemeNativeProjectionStatus, GerbilSchemeNativeSymbol, GerbilSchemeSchemaId,
    GerbilSchemeTypeDecodeError, GerbilSchemeTypeId, GerbilSchemeTypeRegistry,
    GerbilSchemeTypedValue, GerbilSchemeValue, decode_gerbil_deck_runtime_poo_policy_projection,
    decode_gerbil_deck_runtime_resolved_loop_policy_pack_projection,
    decode_gerbil_scheme_native_projection, gerbil_deck_runtime_native_projection_readiness_plan,
    gerbil_deck_runtime_native_projection_type_manifest,
    gerbil_deck_runtime_poo_policy_projection_request,
    gerbil_deck_runtime_poo_policy_projection_type_manifest,
    gerbil_deck_runtime_resolved_loop_policy_pack_projection_request,
};

#[test]
fn native_abi_projection_decodes_poo_policy_typed_value() {
    let registry =
        GerbilSchemeTypeRegistry::new(gerbil_deck_runtime_poo_policy_projection_type_manifest())
            .expect("POO projection manifest should build registry");
    let envelope = poo_projection_envelope("register").with_schema_id(poo_projection_schema_id());

    let (receipt, projection) =
        decode_gerbil_deck_runtime_poo_policy_projection(&registry, &envelope)
            .expect("native ABI projection should validate and decode");

    assert_eq!(
        receipt.status,
        GerbilSchemeNativeProjectionStatus::Projected
    );
    assert_eq!(receipt.abi_id, poo_projection_abi_id());
    assert_eq!(receipt.abi_version, 1);
    assert_eq!(receipt.symbol, poo_projection_symbol());
    assert_eq!(receipt.type_id, poo_projection_type_id());
    assert_eq!(receipt.schema_id, Some(poo_projection_schema_id()));
    assert_eq!(
        projection,
        GerbilDeckRuntimePooPolicyProjection {
            schema_id: GERBIL_DECK_RUNTIME_POO_POLICY_PROJECTION_SCHEMA_ID.to_string(),
            policy_id: "customer-extension".to_string(),
            object_system: "clan/poo".to_string(),
            package: "marlin-deck-runtime".to_string(),
            module: ":marlin/deck-runtime-native-projection".to_string(),
            action: "register".to_string(),
        }
    );
}

#[test]
fn native_abi_projection_decodes_resolved_loop_policy_pack_after_readiness_gate() {
    let registry =
        GerbilSchemeTypeRegistry::new(gerbil_deck_runtime_native_projection_type_manifest())
            .expect("Deck runtime native projection manifest should build registry");
    let envelope = resolved_loop_policy_pack_envelope(1);

    let (receipt, pack) =
        decode_gerbil_deck_runtime_resolved_loop_policy_pack_projection(&registry, &envelope)
            .expect("native ABI projection should validate and decode resolved loop policy pack");

    assert_eq!(
        receipt.status,
        GerbilSchemeNativeProjectionStatus::Projected
    );
    assert_eq!(receipt.abi_id, poo_projection_abi_id());
    assert_eq!(receipt.abi_version, 1);
    assert_eq!(receipt.symbol, resolved_loop_policy_pack_symbol());
    assert_eq!(receipt.type_id, resolved_loop_policy_pack_type_id());
    assert_eq!(
        receipt.schema_id,
        Some(resolved_loop_policy_pack_schema_id())
    );
    assert!(pack.has_current_schema());
    assert_eq!(pack.policy_epoch.get(), 42);
    assert_eq!(pack.hot.capability_mask, 0b101);
    assert_eq!(pack.audit.provenance[0].winner_role.as_str(), "planner");
}

#[test]
fn native_abi_projection_rejects_resolved_loop_policy_schema_version_drift() {
    let registry =
        GerbilSchemeTypeRegistry::new(gerbil_deck_runtime_native_projection_type_manifest())
            .expect("Deck runtime native projection manifest should build registry");
    let envelope = resolved_loop_policy_pack_envelope(2);

    let error =
        decode_gerbil_deck_runtime_resolved_loop_policy_pack_projection(&registry, &envelope)
            .expect_err("native ABI projection should reject resolved pack schema drift");

    assert_eq!(
        error,
        GerbilSchemeTypeDecodeError::RustProjection {
            message: "resolved loop policy pack schema version 2 does not match 1".to_string(),
        }
    );
}

#[test]
fn native_abi_projection_rejects_missing_symbol_before_payload_decode() {
    let registry =
        GerbilSchemeTypeRegistry::new(gerbil_deck_runtime_poo_policy_projection_type_manifest())
            .expect("POO projection manifest should build registry");
    let request = poo_projection_request();
    let readiness_plan = GerbilSchemeNativeAbiReadinessPlan::new(poo_projection_abi_id(), 1)
        .with_exported_symbols([GerbilSchemeNativeSymbol::new("other_symbol")]);
    let envelope =
        poo_projection_envelope_with_bad_payload().with_schema_id(poo_projection_schema_id());

    let error = decode_gerbil_scheme_native_projection::<GerbilDeckRuntimePooPolicyProjection>(
        &registry,
        &readiness_plan,
        &request,
        &envelope,
    )
    .expect_err("native ABI projection should reject missing symbol before payload decode");

    assert_eq!(
        error,
        GerbilSchemeTypeDecodeError::MissingNativeSymbol {
            symbol: poo_projection_symbol(),
        }
    );
}

#[test]
fn native_abi_projection_rejects_wrong_schema_before_payload_decode() {
    let registry =
        GerbilSchemeTypeRegistry::new(gerbil_deck_runtime_poo_policy_projection_type_manifest())
            .expect("POO projection manifest should build registry");
    let request = poo_projection_request();
    let readiness_plan = poo_projection_readiness_plan();
    let envelope = poo_projection_envelope_with_bad_payload().with_schema_id(
        GerbilSchemeSchemaId::new("marlin.deck-runtime.poo-policy-projection.v2"),
    );

    let error = decode_gerbil_scheme_native_projection::<GerbilDeckRuntimePooPolicyProjection>(
        &registry,
        &readiness_plan,
        &request,
        &envelope,
    )
    .expect_err("native ABI projection should reject schema before payload decode");

    assert_eq!(
        error.to_string(),
        format!(
            "Scheme typed value {} has schema_id marlin.deck-runtime.poo-policy-projection.v2, expected {}",
            GERBIL_DECK_RUNTIME_POO_POLICY_PROJECTION_TYPE_ID,
            GERBIL_DECK_RUNTIME_POO_POLICY_PROJECTION_SCHEMA_ID
        )
    );
}

#[test]
fn native_abi_projection_request_round_trips_as_protocol_data() {
    let request = poo_projection_request();

    let encoded = serde_json::to_string(&request).expect("encode projection request");
    let decoded: GerbilSchemeNativeProjectionRequest =
        serde_json::from_str(&encoded).expect("decode projection request");

    assert_eq!(decoded, request);
}

#[test]
fn resolved_loop_policy_pack_projection_request_round_trips_as_protocol_data() {
    let request = resolved_loop_policy_pack_request();

    let encoded = serde_json::to_string(&request).expect("encode resolved pack request");
    let decoded: GerbilSchemeNativeProjectionRequest =
        serde_json::from_str(&encoded).expect("decode resolved pack request");

    assert_eq!(decoded, request);
}

fn poo_projection_request() -> GerbilSchemeNativeProjectionRequest {
    gerbil_deck_runtime_poo_policy_projection_request()
}

fn resolved_loop_policy_pack_request() -> GerbilSchemeNativeProjectionRequest {
    gerbil_deck_runtime_resolved_loop_policy_pack_projection_request()
}

fn poo_projection_readiness_plan() -> GerbilSchemeNativeAbiReadinessPlan {
    gerbil_deck_runtime_native_projection_readiness_plan()
}

fn poo_projection_envelope(action: &str) -> GerbilSchemeTypedValue {
    GerbilSchemeTypedValue::new(
        poo_projection_type_id(),
        GerbilSchemeValue::record([
            (
                "schema_id",
                GERBIL_DECK_RUNTIME_POO_POLICY_PROJECTION_SCHEMA_ID.into(),
            ),
            ("policy_id", "customer-extension".into()),
            ("object_system", "clan/poo".into()),
            ("package", "marlin-deck-runtime".into()),
            ("module", ":marlin/deck-runtime-native-projection".into()),
            ("action", action.into()),
        ]),
    )
}

fn poo_projection_envelope_with_bad_payload() -> GerbilSchemeTypedValue {
    GerbilSchemeTypedValue::new(
        poo_projection_type_id(),
        GerbilSchemeValue::record([
            (
                "schema_id",
                GERBIL_DECK_RUNTIME_POO_POLICY_PROJECTION_SCHEMA_ID.into(),
            ),
            ("policy_id", true.into()),
        ]),
    )
}

fn resolved_loop_policy_pack_envelope(schema_version: u32) -> GerbilSchemeTypedValue {
    GerbilSchemeTypedValue::new(
        resolved_loop_policy_pack_type_id(),
        resolved_loop_policy_pack_payload(schema_version),
    )
    .with_schema_id(resolved_loop_policy_pack_schema_id())
}

fn resolved_loop_policy_pack_payload(schema_version: u32) -> GerbilSchemeValue {
    GerbilSchemeValue::record([
        ("schema_version", i64::from(schema_version).into()),
        ("policy_epoch", 42_i64.into()),
        (
            "policy_digest",
            GerbilSchemeValue::vector((0..32).map(|_| GerbilSchemeValue::from(7_i64))),
        ),
        ("hot", resolved_loop_policy_hot_payload()),
        ("audit", resolved_loop_policy_audit_payload()),
    ])
}

fn resolved_loop_policy_hot_payload() -> GerbilSchemeValue {
    GerbilSchemeValue::record([
        ("capability_mask", 0b101_i64.into()),
        ("human_gate_mask", 0b001_i64.into()),
        (
            "budget_caps",
            GerbilSchemeValue::record([
                ("max_attempts", 3_i64.into()),
                ("max_cost_units", 1000_i64.into()),
                ("max_wall_time_ms", 30000_i64.into()),
            ]),
        ),
        (
            "graph_nodes",
            GerbilSchemeValue::vector([GerbilSchemeValue::record([
                ("node_id", 1_i64.into()),
                ("executor_id", 2_i64.into()),
                ("capability_mask", 0b101_i64.into()),
                ("resource_class_id", 4_i64.into()),
            ])]),
        ),
        (
            "graph_edges",
            GerbilSchemeValue::vector([GerbilSchemeValue::record([
                ("from", 1_i64.into()),
                ("to", 2_i64.into()),
            ])]),
        ),
        (
            "route_index",
            GerbilSchemeValue::record([(
                "buckets",
                GerbilSchemeValue::vector([GerbilSchemeValue::record([
                    ("bucket_id", 1_i64.into()),
                    ("scope_mask", 255_i64.into()),
                    ("target_id", 3_i64.into()),
                ])]),
            )]),
        ),
        (
            "resource_classes",
            GerbilSchemeValue::vector([GerbilSchemeValue::record([
                ("resource_class_id", 4_i64.into()),
                ("exclusive", true.into()),
            ])]),
        ),
        (
            "continuation_table",
            GerbilSchemeValue::vector([GerbilSchemeValue::record([(
                "op",
                "stop_completed".into(),
            )])]),
        ),
        (
            "maker_profiles",
            GerbilSchemeValue::vector([GerbilSchemeValue::from(11_i64)]),
        ),
        (
            "checker_profiles",
            GerbilSchemeValue::vector([GerbilSchemeValue::from(12_i64)]),
        ),
    ])
}

fn resolved_loop_policy_audit_payload() -> GerbilSchemeValue {
    GerbilSchemeValue::record([
        (
            "provenance",
            GerbilSchemeValue::vector([GerbilSchemeValue::record([
                ("slot_id", 9_i64.into()),
                ("winner_role", "planner".into()),
                (
                    "source_role_order",
                    GerbilSchemeValue::vector(["planner".into(), "reviewer".into()]),
                ),
                ("merge", "union".into()),
            ])]),
        ),
        (
            "linearization",
            GerbilSchemeValue::vector(["planner".into(), "reviewer".into()]),
        ),
        (
            "diagnostics",
            GerbilSchemeValue::vector([GerbilSchemeValue::record([
                ("code", "policy-pack-ok".into()),
                ("severity", "info".into()),
            ])]),
        ),
        (
            "source_locations",
            GerbilSchemeValue::vector([GerbilSchemeValue::record([
                ("source_location_id", 1_i64.into()),
                (
                    "path",
                    "gerbil/src/config-interface/modules/policy-pack.ss".into(),
                ),
                ("line", 10_i64.into()),
                ("column", 2_i64.into()),
            ])]),
        ),
        (
            "explanation_strings",
            GerbilSchemeValue::vector(["forced policy pack before native handoff".into()]),
        ),
        (
            "forced_slots",
            GerbilSchemeValue::vector([GerbilSchemeValue::record([
                ("slot_id", 9_i64.into()),
                ("hotness", "hot".into()),
            ])]),
        ),
        (
            "merge_receipts",
            GerbilSchemeValue::vector([GerbilSchemeValue::record([
                ("slot_id", 9_i64.into()),
                ("merge", "union".into()),
                ("status", "applied".into()),
            ])]),
        ),
    ])
}

fn poo_projection_type_id() -> GerbilSchemeTypeId {
    GerbilSchemeTypeId::new(GERBIL_DECK_RUNTIME_POO_POLICY_PROJECTION_TYPE_ID)
}

fn poo_projection_schema_id() -> GerbilSchemeSchemaId {
    GerbilSchemeSchemaId::new(GERBIL_DECK_RUNTIME_POO_POLICY_PROJECTION_SCHEMA_ID)
}

fn resolved_loop_policy_pack_type_id() -> GerbilSchemeTypeId {
    GerbilSchemeTypeId::new(GERBIL_RESOLVED_LOOP_POLICY_PACK_TYPE_ID)
}

fn resolved_loop_policy_pack_schema_id() -> GerbilSchemeSchemaId {
    GerbilSchemeSchemaId::new(GERBIL_RESOLVED_LOOP_POLICY_PACK_SCHEMA_ID)
}

fn poo_projection_abi_id() -> GerbilSchemeNativeAbiId {
    GerbilSchemeNativeAbiId::new(GERBIL_DECK_RUNTIME_NATIVE_PROJECTION_ABI_ID)
}

fn poo_projection_symbol() -> GerbilSchemeNativeSymbol {
    GerbilSchemeNativeSymbol::new(GERBIL_DECK_RUNTIME_PROJECT_POO_POLICY_SYMBOL)
}

fn resolved_loop_policy_pack_symbol() -> GerbilSchemeNativeSymbol {
    GerbilSchemeNativeSymbol::new(GERBIL_DECK_RUNTIME_PROJECT_RESOLVED_LOOP_POLICY_PACK_SYMBOL)
}
