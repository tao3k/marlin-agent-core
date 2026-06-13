use super::support::{
    strategy_selection_manifest, strategy_selection_schema_id, strategy_selection_type_id,
};
use marlin_gerbil_scheme::{
    GerbilSchemeSchemaId, GerbilSchemeTypeDecodeError, GerbilSchemeTypeId, GerbilSchemeTypedValue,
    validate_gerbil_scheme_typed_value,
};
use serde_json::json;

#[test]
fn scheme_typed_value_validation_leaves_payload_shape_to_rust_decode() {
    let manifest = strategy_selection_manifest();
    let envelope = GerbilSchemeTypedValue::new(
        strategy_selection_type_id(),
        json!(["payload", "shape", "belongs", "to", "serde"]),
    )
    .with_schema_id(strategy_selection_schema_id());

    let receipt = validate_gerbil_scheme_typed_value(&manifest, &envelope)
        .expect("typed value validation should only check manifest and envelope identity");

    assert_eq!(receipt.type_id, strategy_selection_type_id());
    assert_eq!(receipt.schema_id, Some(strategy_selection_schema_id()));
    assert_eq!(receipt.declared_field_count, 3);
}

#[test]
fn scheme_typed_value_rejects_schema_mismatch() {
    let manifest = strategy_selection_manifest();
    let envelope = GerbilSchemeTypedValue::new(
        strategy_selection_type_id(),
        json!({
            "schema_id": "marlin.deck-runtime.strategy-selection.v2",
            "matched": true,
            "action": "dynamic-hook-action"
        }),
    )
    .with_schema_id(GerbilSchemeSchemaId::new(
        "marlin.deck-runtime.strategy-selection.v2",
    ));

    let error = validate_gerbil_scheme_typed_value(&manifest, &envelope)
        .expect_err("schema mismatch should fail validation");

    assert_eq!(
        error.to_string(),
        "Scheme typed value marlin.deck-runtime.strategy-selection has schema_id marlin.deck-runtime.strategy-selection.v2, expected marlin.deck-runtime.strategy-selection.v1"
    );
}

#[test]
fn scheme_typed_value_rejects_unknown_type() {
    let manifest = strategy_selection_manifest();
    let envelope = GerbilSchemeTypedValue::new(
        GerbilSchemeTypeId::new("marlin.unknown"),
        json!({
            "schema_id": "marlin.unknown.v1"
        }),
    );

    let error = validate_gerbil_scheme_typed_value(&manifest, &envelope)
        .expect_err("unknown type should fail validation");

    assert_eq!(
        error,
        GerbilSchemeTypeDecodeError::UnknownType {
            type_id: GerbilSchemeTypeId::new("marlin.unknown")
        }
    );
}
