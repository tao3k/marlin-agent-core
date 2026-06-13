use super::support::{
    StrategySelectionProjection, strategy_selection_manifest, strategy_selection_schema_id,
    strategy_selection_type_id,
};
use marlin_gerbil_scheme::{
    GerbilSchemeTypeId, GerbilSchemeTypedValue, decode_gerbil_scheme_typed_value,
};
use serde_json::json;
use std::time::Instant;

#[test]
fn scheme_typed_value_projects_to_rust_without_static_scheme_binding() {
    let manifest = strategy_selection_manifest();
    let envelope = decode_gerbil_scheme_typed_value(
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

    let projection: StrategySelectionProjection = envelope
        .decode_value_with_manifest(&manifest)
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
fn scheme_typed_value_rejects_wrong_projection_type() {
    let envelope = GerbilSchemeTypedValue::new(
        GerbilSchemeTypeId::new("marlin.dynamic.capability"),
        json!({
            "schema_id": "marlin.dynamic.capability.v1",
            "matched": false,
            "action": "observe"
        }),
    );

    let error = envelope
        .decode_value_as::<StrategySelectionProjection>(&strategy_selection_type_id())
        .expect_err("wrong type_id should be rejected before projection");

    assert_eq!(
        error.to_string(),
        "Scheme typed value has type_id marlin.dynamic.capability, expected marlin.deck-runtime.strategy-selection"
    );
}

#[test]
fn scheme_typed_value_projection_performance_gate_stays_in_process() {
    let manifest = strategy_selection_manifest();
    let envelopes = (0..2_000)
        .map(|index| {
            GerbilSchemeTypedValue::new(
                strategy_selection_type_id(),
                json!({
                    "schema_id": "marlin.deck-runtime.strategy-selection.v1",
                    "matched": true,
                    "action": format!("dynamic-hook-action-{index}")
                }),
            )
            .with_schema_id(strategy_selection_schema_id())
        })
        .collect::<Vec<_>>();

    let started = Instant::now();
    for envelope in &envelopes {
        let projection: StrategySelectionProjection = envelope
            .decode_value_with_manifest(&manifest)
            .expect("validated Scheme typed value should project into Rust type");
        assert!(projection.matched);
    }
    let elapsed = started.elapsed();

    assert!(
        elapsed.as_secs_f64() < 3.0,
        "Scheme typed value projection gate exceeded in-process budget: {elapsed:?}"
    );
}
