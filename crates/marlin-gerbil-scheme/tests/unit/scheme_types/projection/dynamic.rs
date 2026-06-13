use super::support::{
    StrategyDecisionProjection, StrategySelectionProjection, nested_strategy_manifest,
    strategy_decision_schema_id, strategy_decision_type_id, strategy_selection_manifest,
    strategy_selection_schema_id, strategy_selection_type_id,
};
use marlin_gerbil_scheme::{
    GerbilSchemeTypeId, GerbilSchemeTypeRegistry, GerbilSchemeTypedValue, GerbilSchemeValue,
    scheme_type_fixtures::decode_gerbil_scheme_typed_value_fixture,
};

#[test]
fn scheme_typed_value_projects_to_rust_without_static_scheme_binding() {
    let manifest = strategy_selection_manifest();
    let registry =
        GerbilSchemeTypeRegistry::new(manifest).expect("strategy manifest builds registry");
    let envelope = decode_gerbil_scheme_typed_value_fixture(
        r#"{
            "type_id": "marlin.deck-runtime.strategy-selection",
            "schema_id": "marlin.deck-runtime.strategy-selection.v1",
            "value": {
                "schema_id": "marlin.deck-runtime.strategy-selection.v1",
                "matched": true,
                "action": "dynamic-hook-action"
            }
        }"#,
    )
    .expect("decode typed value envelope");

    let projection: StrategySelectionProjection = registry
        .decode_typed_value(&envelope)
        .expect("project envelope payload into Rust type");

    assert_eq!(
        envelope.schema_id().map(|schema| schema.as_str()),
        Some("marlin.deck-runtime.strategy-selection.v1")
    );
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
fn scheme_type_registry_reuses_validated_manifest_index() {
    let registry = GerbilSchemeTypeRegistry::new(strategy_selection_manifest())
        .expect("strategy manifest should build registry");

    assert_eq!(registry.validation_receipt().type_count, 1);
    assert_eq!(registry.validation_receipt().field_count, 3);
    assert!(
        registry
            .type_spec(&strategy_selection_type_id())
            .expect("strategy type is indexed")
            .schema_id
            .as_ref()
            .is_some_and(|schema| schema == &strategy_selection_schema_id())
    );
}

#[test]
fn scheme_typed_value_projects_nested_custom_scheme_types_to_rust() {
    let registry = GerbilSchemeTypeRegistry::new(nested_strategy_manifest())
        .expect("nested strategy manifest should build registry");
    let envelope = GerbilSchemeTypedValue::new(
        strategy_decision_type_id(),
        GerbilSchemeValue::record([
            (
                "schema_id",
                "marlin.deck-runtime.strategy-decision.v1".into(),
            ),
            (
                "selection",
                GerbilSchemeValue::record([
                    (
                        "schema_id",
                        "marlin.deck-runtime.strategy-selection.v1".into(),
                    ),
                    ("matched", true.into()),
                    ("action", "dynamic-hook-action".into()),
                ]),
            ),
            ("reason", "nested manifest projection".into()),
        ]),
    )
    .with_schema_id(strategy_decision_schema_id());

    let projection: StrategyDecisionProjection = registry
        .decode_typed_value(&envelope)
        .expect("nested Scheme typed value should project into Rust type");

    assert_eq!(
        projection,
        StrategyDecisionProjection {
            schema_id: "marlin.deck-runtime.strategy-decision.v1".to_string(),
            selection: StrategySelectionProjection {
                schema_id: "marlin.deck-runtime.strategy-selection.v1".to_string(),
                matched: true,
                action: "dynamic-hook-action".to_string(),
            },
            reason: "nested manifest projection".to_string(),
        }
    );
}

#[test]
fn scheme_typed_value_rejects_wrong_projection_type() {
    let envelope = GerbilSchemeTypedValue::new(
        GerbilSchemeTypeId::new("marlin.dynamic.capability"),
        GerbilSchemeValue::record([
            ("schema_id", "marlin.dynamic.capability.v1".into()),
            ("matched", false.into()),
            ("action", "observe".into()),
        ]),
    );

    let error = envelope
        .decode_value_as::<StrategySelectionProjection>(&strategy_selection_type_id())
        .expect_err("wrong type_id should be rejected before projection");

    assert_eq!(
        error.to_string(),
        "Scheme typed value has type_id marlin.dynamic.capability, expected marlin.deck-runtime.strategy-selection"
    );
}
