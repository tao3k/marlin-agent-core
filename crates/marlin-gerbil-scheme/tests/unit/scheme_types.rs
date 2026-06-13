use marlin_gerbil_scheme::{
    GerbilSchemeTypeId, GerbilSchemeTypedValue, decode_gerbil_scheme_type_manifest,
    decode_gerbil_scheme_typed_value,
};
use serde::Deserialize;
use serde_json::json;

#[derive(Debug, Deserialize, PartialEq)]
struct StrategySelectionProjection {
    schema_id: String,
    matched: bool,
    action: String,
}

#[test]
fn scheme_type_manifest_describes_downstream_value_shape() {
    let manifest = decode_gerbil_scheme_type_manifest(
        r#"{
            "schema_id": "marlin.scheme-types.manifest.v1",
            "types": [
                {
                    "type_id": "marlin.deck-runtime.strategy-selection",
                    "schema_id": "marlin.deck-runtime.strategy-selection.v1",
                    "fields": [
                        {"name": "schema_id", "type_id": "string", "required": true},
                        {"name": "matched", "type_id": "boolean", "required": true},
                        {"name": "action", "type_id": "string", "required": true}
                    ]
                }
            ]
        }"#,
    )
    .expect("decode scheme type manifest");

    let strategy = manifest
        .type_spec(&GerbilSchemeTypeId::new(
            "marlin.deck-runtime.strategy-selection",
        ))
        .expect("strategy selection type is registered");

    assert_eq!(
        manifest.schema_id.as_str(),
        "marlin.scheme-types.manifest.v1"
    );
    assert_eq!(
        strategy.schema_id.as_ref().map(|schema| schema.as_str()),
        Some("marlin.deck-runtime.strategy-selection.v1")
    );
    assert!(strategy.field("matched").expect("matched field").required);
}

#[test]
fn scheme_typed_value_projects_to_rust_without_static_scheme_binding() {
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
        .decode_value_as(&GerbilSchemeTypeId::new(
            "marlin.deck-runtime.strategy-selection",
        ))
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
        .decode_value_as::<StrategySelectionProjection>(&GerbilSchemeTypeId::new(
            "marlin.deck-runtime.strategy-selection",
        ))
        .expect_err("wrong type_id should be rejected before projection");

    assert_eq!(
        error.to_string(),
        "Scheme typed value has type_id marlin.dynamic.capability, expected marlin.deck-runtime.strategy-selection"
    );
}
