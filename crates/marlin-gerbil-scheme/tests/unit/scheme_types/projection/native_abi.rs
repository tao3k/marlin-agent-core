use marlin_gerbil_scheme::{
    GERBIL_DECK_RUNTIME_NATIVE_PROJECTION_ABI_ID,
    GERBIL_DECK_RUNTIME_POO_POLICY_PROJECTION_SCHEMA_ID,
    GERBIL_DECK_RUNTIME_POO_POLICY_PROJECTION_TYPE_ID,
    GERBIL_DECK_RUNTIME_PROJECT_POO_POLICY_SYMBOL, GerbilDeckRuntimePooPolicyProjection,
    GerbilSchemeNativeAbiId, GerbilSchemeNativeAbiReadinessPlan,
    GerbilSchemeNativeProjectionRequest, GerbilSchemeNativeProjectionStatus,
    GerbilSchemeNativeSymbol, GerbilSchemeSchemaId, GerbilSchemeTypeDecodeError,
    GerbilSchemeTypeId, GerbilSchemeTypeRegistry, GerbilSchemeTypedValue, GerbilSchemeValue,
    decode_gerbil_deck_runtime_poo_policy_projection, decode_gerbil_scheme_native_projection,
    gerbil_deck_runtime_native_projection_readiness_plan,
    gerbil_deck_runtime_poo_policy_projection_request,
    gerbil_deck_runtime_poo_policy_projection_type_manifest,
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

fn poo_projection_request() -> GerbilSchemeNativeProjectionRequest {
    gerbil_deck_runtime_poo_policy_projection_request()
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

fn poo_projection_type_id() -> GerbilSchemeTypeId {
    GerbilSchemeTypeId::new(GERBIL_DECK_RUNTIME_POO_POLICY_PROJECTION_TYPE_ID)
}

fn poo_projection_schema_id() -> GerbilSchemeSchemaId {
    GerbilSchemeSchemaId::new(GERBIL_DECK_RUNTIME_POO_POLICY_PROJECTION_SCHEMA_ID)
}

fn poo_projection_abi_id() -> GerbilSchemeNativeAbiId {
    GerbilSchemeNativeAbiId::new(GERBIL_DECK_RUNTIME_NATIVE_PROJECTION_ABI_ID)
}

fn poo_projection_symbol() -> GerbilSchemeNativeSymbol {
    GerbilSchemeNativeSymbol::new(GERBIL_DECK_RUNTIME_PROJECT_POO_POLICY_SYMBOL)
}
