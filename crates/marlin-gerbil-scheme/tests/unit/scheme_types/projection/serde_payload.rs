use super::assert_json_decode_error;
use super::support::{
    StrategyDecisionProjection, StrategySelectionProjection, nested_strategy_manifest,
    strategy_decision_schema_id, strategy_decision_type_id, strategy_selection_manifest,
    strategy_selection_schema_id, strategy_selection_type_id,
};
use marlin_gerbil_scheme::{GerbilSchemeTypeRegistry, GerbilSchemeTypedValue};
use serde_json::json;

#[test]
fn scheme_typed_value_payload_shape_errors_are_owned_by_rust_decode() {
    let registry = GerbilSchemeTypeRegistry::new(strategy_selection_manifest())
        .expect("strategy manifest should build registry");
    let envelope = GerbilSchemeTypedValue::new(
        strategy_selection_type_id(),
        json!({
            "schema_id": "marlin.deck-runtime.strategy-selection.v1",
            "matched": "yes",
            "action": "dynamic-hook-action"
        }),
    )
    .with_schema_id(strategy_selection_schema_id());

    let error = registry
        .decode_projection::<StrategySelectionProjection>(&envelope)
        .expect_err("serde should reject payload shape during Rust projection");

    assert_json_decode_error(error, "expected a boolean");
}

#[test]
fn scheme_typed_value_missing_field_errors_are_owned_by_rust_decode() {
    let registry = GerbilSchemeTypeRegistry::new(strategy_selection_manifest())
        .expect("strategy manifest should build registry");
    let envelope = GerbilSchemeTypedValue::new(
        strategy_selection_type_id(),
        json!({
            "schema_id": "marlin.deck-runtime.strategy-selection.v1",
            "matched": true
        }),
    )
    .with_schema_id(strategy_selection_schema_id());

    let error = registry
        .decode_projection::<StrategySelectionProjection>(&envelope)
        .expect_err("serde should reject missing Rust projection field");

    assert_json_decode_error(error, "missing field");
}

#[test]
fn scheme_typed_value_non_object_payload_errors_are_owned_by_rust_decode() {
    let registry = GerbilSchemeTypeRegistry::new(strategy_selection_manifest())
        .expect("strategy manifest should build registry");
    let envelope =
        GerbilSchemeTypedValue::new(strategy_selection_type_id(), json!(["not", "an", "object"]))
            .with_schema_id(strategy_selection_schema_id());

    let error = registry
        .decode_projection::<StrategySelectionProjection>(&envelope)
        .expect_err("serde should reject non-object Rust projection payload");

    assert_json_decode_error(error, "invalid type");
}

#[test]
fn scheme_typed_value_nested_shape_errors_are_owned_by_rust_decode() {
    let registry = GerbilSchemeTypeRegistry::new(nested_strategy_manifest())
        .expect("nested strategy manifest should build registry");
    let envelope = GerbilSchemeTypedValue::new(
        strategy_decision_type_id(),
        json!({
            "schema_id": "marlin.deck-runtime.strategy-decision.v1",
            "selection": "not-a-strategy-selection",
            "reason": "nested shape belongs to serde"
        }),
    )
    .with_schema_id(strategy_decision_schema_id());

    let error = registry
        .decode_projection::<StrategyDecisionProjection>(&envelope)
        .expect_err("serde should reject nested Rust projection payload");

    assert_json_decode_error(error, "invalid type");
}
