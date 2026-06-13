use super::assert_rust_projection_decode_error;
use super::support::{
    StrategyDecisionProjection, StrategySelectionProjection, nested_strategy_manifest,
    strategy_decision_schema_id, strategy_decision_type_id, strategy_selection_manifest,
    strategy_selection_schema_id, strategy_selection_type_id,
};
use marlin_gerbil_scheme::{GerbilSchemeTypeRegistry, GerbilSchemeTypedValue, GerbilSchemeValue};
use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq)]
struct RichProjection {
    schema_id: String,
    tags: Vec<String>,
    score: Option<i64>,
    note: Option<String>,
}

#[derive(Debug, Deserialize, PartialEq)]
enum RichEnumProjection {
    Unit,
    Newtype(i64),
    Struct { action: String },
}

#[test]
fn scheme_value_deserializer_projects_vectors_and_options_without_json_value() {
    let envelope = GerbilSchemeTypedValue::new(
        strategy_selection_type_id(),
        GerbilSchemeValue::record([
            (
                "schema_id",
                "marlin.deck-runtime.strategy-selection.v1".into(),
            ),
            (
                "tags",
                GerbilSchemeValue::vector(["alpha".into(), "beta".into()]),
            ),
            ("score", 42.into()),
            ("note", GerbilSchemeValue::null()),
        ]),
    );

    let projection = envelope
        .decode_value::<RichProjection>()
        .expect("Scheme value should project directly through serde");

    assert_eq!(
        projection,
        RichProjection {
            schema_id: "marlin.deck-runtime.strategy-selection.v1".to_owned(),
            tags: vec!["alpha".to_owned(), "beta".to_owned()],
            score: Some(42),
            note: None,
        }
    );
}

#[test]
fn scheme_value_deserializer_projects_enum_variants_without_json_value() {
    let unit = GerbilSchemeTypedValue::new(strategy_selection_type_id(), "Unit".into())
        .decode_value::<RichEnumProjection>()
        .expect("text Scheme value should project to unit enum variant");
    let newtype = GerbilSchemeTypedValue::new(
        strategy_selection_type_id(),
        GerbilSchemeValue::record([("Newtype", 7.into())]),
    )
    .decode_value::<RichEnumProjection>()
    .expect("single-field Scheme record should project to newtype enum variant");
    let structured = GerbilSchemeTypedValue::new(
        strategy_selection_type_id(),
        GerbilSchemeValue::record([(
            "Struct",
            GerbilSchemeValue::record([("action", "select".into())]),
        )]),
    )
    .decode_value::<RichEnumProjection>()
    .expect("single-field Scheme record should project to struct enum variant");

    assert_eq!(unit, RichEnumProjection::Unit);
    assert_eq!(newtype, RichEnumProjection::Newtype(7));
    assert_eq!(
        structured,
        RichEnumProjection::Struct {
            action: "select".to_owned()
        }
    );
}

#[test]
fn scheme_value_deserializer_rejects_unsigned_integer_underflow_without_json_value() {
    let error = GerbilSchemeTypedValue::new(strategy_selection_type_id(), (-1_i64).into())
        .decode_value::<u64>()
        .expect_err("negative Scheme integer must not project to unsigned Rust integer");

    assert_rust_projection_decode_error(error, "invalid type");
}

#[test]
fn scheme_typed_value_payload_shape_errors_are_owned_by_rust_decode() {
    let registry = GerbilSchemeTypeRegistry::new(strategy_selection_manifest())
        .expect("strategy manifest should build registry");
    let envelope = GerbilSchemeTypedValue::new(
        strategy_selection_type_id(),
        GerbilSchemeValue::record([
            (
                "schema_id",
                "marlin.deck-runtime.strategy-selection.v1".into(),
            ),
            ("matched", "yes".into()),
            ("action", "dynamic-hook-action".into()),
        ]),
    )
    .with_schema_id(strategy_selection_schema_id());

    let error = registry
        .decode_projection::<StrategySelectionProjection>(&envelope)
        .expect_err("serde should reject payload shape during Rust projection");

    assert_rust_projection_decode_error(error, "expected a boolean");
}

#[test]
fn scheme_typed_value_missing_field_errors_are_owned_by_rust_decode() {
    let registry = GerbilSchemeTypeRegistry::new(strategy_selection_manifest())
        .expect("strategy manifest should build registry");
    let envelope = GerbilSchemeTypedValue::new(
        strategy_selection_type_id(),
        GerbilSchemeValue::record([
            (
                "schema_id",
                "marlin.deck-runtime.strategy-selection.v1".into(),
            ),
            ("matched", true.into()),
        ]),
    )
    .with_schema_id(strategy_selection_schema_id());

    let error = registry
        .decode_projection::<StrategySelectionProjection>(&envelope)
        .expect_err("serde should reject missing Rust projection field");

    assert_rust_projection_decode_error(error, "missing field");
}

#[test]
fn scheme_typed_value_non_object_payload_errors_are_owned_by_rust_decode() {
    let registry = GerbilSchemeTypeRegistry::new(strategy_selection_manifest())
        .expect("strategy manifest should build registry");
    let envelope = GerbilSchemeTypedValue::new(
        strategy_selection_type_id(),
        GerbilSchemeValue::vector(["not".into(), "an".into(), "object".into()]),
    )
    .with_schema_id(strategy_selection_schema_id());

    let error = registry
        .decode_projection::<StrategySelectionProjection>(&envelope)
        .expect_err("serde should reject non-object Rust projection payload");

    assert_rust_projection_decode_error(error, "invalid type");
}

#[test]
fn scheme_typed_value_nested_shape_errors_are_owned_by_rust_decode() {
    let registry = GerbilSchemeTypeRegistry::new(nested_strategy_manifest())
        .expect("nested strategy manifest should build registry");
    let envelope = GerbilSchemeTypedValue::new(
        strategy_decision_type_id(),
        GerbilSchemeValue::record([
            (
                "schema_id",
                "marlin.deck-runtime.strategy-decision.v1".into(),
            ),
            ("selection", "not-a-strategy-selection".into()),
            ("reason", "nested shape belongs to serde".into()),
        ]),
    )
    .with_schema_id(strategy_decision_schema_id());

    let error = registry
        .decode_projection::<StrategyDecisionProjection>(&envelope)
        .expect_err("serde should reject nested Rust projection payload");

    assert_rust_projection_decode_error(error, "invalid type");
}
