use super::support::{
    StrategySelectionProjection, strategy_selection_manifest, strategy_selection_schema_id,
    strategy_selection_type_id,
};
use marlin_gerbil_scheme::{
    GerbilSchemeProjectionContract, GerbilSchemeSchemaId, GerbilSchemeTypeId,
    GerbilSchemeTypeRegistry, GerbilSchemeTypedValue,
};
use serde_json::json;

#[test]
fn scheme_type_registry_decodes_static_typed_projection_contract() {
    let registry = GerbilSchemeTypeRegistry::new(strategy_selection_manifest())
        .expect("strategy manifest should build registry");
    let envelope = GerbilSchemeTypedValue::new(
        strategy_selection_type_id(),
        json!({
            "schema_id": "marlin.deck-runtime.strategy-selection.v1",
            "matched": true,
            "action": "dynamic-hook-action"
        }),
    )
    .with_schema_id(strategy_selection_schema_id());

    let projection: StrategySelectionProjection = registry
        .decode_projection(&envelope)
        .expect("typed projection contract should accept matching type and schema");

    assert_eq!(
        projection,
        StrategySelectionProjection {
            schema_id: "marlin.deck-runtime.strategy-selection.v1".to_string(),
            matched: true,
            action: "dynamic-hook-action".to_string(),
        }
    );
}

#[test]
fn scheme_type_registry_decodes_runtime_projection_contract() {
    let registry = GerbilSchemeTypeRegistry::new(strategy_selection_manifest())
        .expect("strategy manifest should build registry");
    let contract = GerbilSchemeProjectionContract::new(strategy_selection_type_id())
        .with_schema_id(strategy_selection_schema_id());
    let envelope = GerbilSchemeTypedValue::new(
        strategy_selection_type_id(),
        json!({
            "schema_id": "marlin.deck-runtime.strategy-selection.v1",
            "matched": true,
            "action": "runtime-contract-action"
        }),
    )
    .with_schema_id(strategy_selection_schema_id());

    let projection: StrategySelectionProjection = registry
        .decode_typed_value_with_contract(&envelope, &contract)
        .expect("runtime projection contract should accept matching type and schema");

    assert_eq!(projection.action, "runtime-contract-action");
}

#[test]
fn scheme_projection_contract_round_trips_as_protocol_data() {
    let contract = GerbilSchemeProjectionContract::new(strategy_selection_type_id())
        .with_schema_id(strategy_selection_schema_id());

    let encoded = serde_json::to_string(&contract).expect("encode projection contract");
    let decoded: GerbilSchemeProjectionContract =
        serde_json::from_str(&encoded).expect("decode projection contract");

    assert_eq!(decoded, contract);
}

#[test]
fn scheme_typed_projection_rejects_wrong_schema_before_payload_decode() {
    let envelope = GerbilSchemeTypedValue::new(
        strategy_selection_type_id(),
        json!({
            "schema_id": "marlin.deck-runtime.strategy-selection.v2",
            "matched": "serde should not run before schema check"
        }),
    )
    .with_schema_id(GerbilSchemeSchemaId::new(
        "marlin.deck-runtime.strategy-selection.v2",
    ));

    let error = envelope
        .decode_projection::<StrategySelectionProjection>()
        .expect_err("typed projection should reject schema before payload decode");

    assert_eq!(
        error.to_string(),
        "Scheme typed value marlin.deck-runtime.strategy-selection has schema_id marlin.deck-runtime.strategy-selection.v2, expected marlin.deck-runtime.strategy-selection.v1"
    );
}

#[test]
fn scheme_typed_value_contract_rejects_wrong_schema_before_payload_decode() {
    let contract = GerbilSchemeProjectionContract::new(strategy_selection_type_id())
        .with_schema_id(strategy_selection_schema_id());
    let envelope = GerbilSchemeTypedValue::new(
        strategy_selection_type_id(),
        json!({
            "schema_id": "marlin.deck-runtime.strategy-selection.v2",
            "matched": "serde should not run before schema check"
        }),
    )
    .with_schema_id(GerbilSchemeSchemaId::new(
        "marlin.deck-runtime.strategy-selection.v2",
    ));

    let error = envelope
        .decode_value_with_contract::<StrategySelectionProjection>(&contract)
        .expect_err("runtime projection contract should reject schema before payload decode");

    assert_eq!(
        error.to_string(),
        "Scheme typed value marlin.deck-runtime.strategy-selection has schema_id marlin.deck-runtime.strategy-selection.v2, expected marlin.deck-runtime.strategy-selection.v1"
    );
}

#[test]
fn scheme_typed_projection_rejects_wrong_type_before_payload_decode() {
    let envelope = GerbilSchemeTypedValue::new(
        GerbilSchemeTypeId::new("marlin.dynamic.capability"),
        json!({
            "schema_id": "marlin.dynamic.capability.v1",
            "matched": "serde should not run before type check"
        }),
    );

    let error = envelope
        .decode_projection::<StrategySelectionProjection>()
        .expect_err("typed projection should reject type before payload decode");

    assert_eq!(
        error.to_string(),
        "Scheme typed value has type_id marlin.dynamic.capability, expected marlin.deck-runtime.strategy-selection"
    );
}
