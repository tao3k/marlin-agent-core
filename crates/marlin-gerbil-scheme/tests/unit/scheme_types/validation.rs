use super::support::{
    strategy_selection_manifest, strategy_selection_schema_id, strategy_selection_type_id,
};
use marlin_gerbil_scheme::{
    GerbilSchemeFieldName, GerbilSchemeSchemaId, GerbilSchemeTypeDecodeError, GerbilSchemeTypeId,
    GerbilSchemeTypedValue,
};
use serde_json::json;

#[test]
fn scheme_typed_value_rejects_missing_required_field() {
    let manifest = strategy_selection_manifest();
    let envelope = GerbilSchemeTypedValue::new(
        strategy_selection_type_id(),
        json!({
            "schema_id": "marlin.deck-runtime.strategy-selection.v1",
            "matched": true
        }),
    )
    .with_schema_id(strategy_selection_schema_id());

    let error = manifest
        .validate_typed_value(&envelope)
        .expect_err("missing action should fail validation");

    assert_eq!(
        error,
        GerbilSchemeTypeDecodeError::MissingRequiredField {
            type_id: strategy_selection_type_id(),
            field_name: GerbilSchemeFieldName::new("action"),
        }
    );
}

#[test]
fn scheme_typed_value_rejects_field_type_mismatch() {
    let manifest = strategy_selection_manifest();
    let envelope = GerbilSchemeTypedValue::new(
        strategy_selection_type_id(),
        json!({
            "schema_id": "marlin.deck-runtime.strategy-selection.v1",
            "matched": "yes",
            "action": "dynamic-hook-action"
        }),
    )
    .with_schema_id(strategy_selection_schema_id());

    let error = manifest
        .validate_typed_value(&envelope)
        .expect_err("string matched value should fail boolean validation");

    assert_eq!(
        error.to_string(),
        "Scheme typed value marlin.deck-runtime.strategy-selection field matched has string payload, expected boolean"
    );
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

    let error = manifest
        .validate_typed_value(&envelope)
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

    let error = manifest
        .validate_typed_value(&envelope)
        .expect_err("unknown type should fail validation");

    assert_eq!(
        error,
        GerbilSchemeTypeDecodeError::UnknownType {
            type_id: GerbilSchemeTypeId::new("marlin.unknown")
        }
    );
}

#[test]
fn scheme_typed_value_rejects_non_object_payload() {
    let manifest = strategy_selection_manifest();
    let envelope =
        GerbilSchemeTypedValue::new(strategy_selection_type_id(), json!(["not", "an", "object"]))
            .with_schema_id(strategy_selection_schema_id());

    let error = manifest
        .validate_typed_value(&envelope)
        .expect_err("array payload should fail object validation");

    assert_eq!(
        error.to_string(),
        "Scheme typed value marlin.deck-runtime.strategy-selection has array payload, expected object"
    );
}
