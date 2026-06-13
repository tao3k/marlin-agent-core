use super::support::{strategy_selection_manifest, strategy_selection_type_id};
use marlin_gerbil_scheme::{
    GerbilSchemeFieldName, GerbilSchemeTypeDecodeError, GerbilSchemeTypeId,
    decode_gerbil_scheme_type_manifest, validate_gerbil_scheme_type_manifest,
};

#[test]
fn scheme_type_manifest_describes_downstream_value_shape() {
    let manifest = strategy_selection_manifest();
    let receipt = validate_gerbil_scheme_type_manifest(&manifest)
        .expect("manifest should pass structural validation");

    let strategy = manifest
        .type_spec(&strategy_selection_type_id())
        .expect("strategy selection type is registered");

    assert_eq!(
        manifest.schema_id.as_str(),
        "marlin.scheme-types.manifest.v1"
    );
    assert_eq!(
        strategy.schema_id.as_ref().map(|schema| schema.as_str()),
        Some("marlin.deck-runtime.strategy-selection.v1")
    );
    assert_eq!(receipt.type_count, 1);
    assert_eq!(receipt.field_count, 3);
    assert!(
        strategy
            .field(&GerbilSchemeFieldName::new("matched"))
            .expect("matched field")
            .required
    );
}

#[test]
fn scheme_type_manifest_rejects_duplicate_type_ids() {
    let manifest = decode_gerbil_scheme_type_manifest(
        r#"{
            "schema_id": "marlin.scheme-types.manifest.v1",
            "types": [
                {"type_id": "marlin.duplicate", "fields": []},
                {"type_id": "marlin.duplicate", "fields": []}
            ]
        }"#,
    )
    .expect("decode duplicate manifest");

    let error = validate_gerbil_scheme_type_manifest(&manifest)
        .expect_err("duplicate type ids should fail manifest validation");

    assert_eq!(
        error,
        GerbilSchemeTypeDecodeError::DuplicateType {
            type_id: GerbilSchemeTypeId::new("marlin.duplicate")
        }
    );
}

#[test]
fn scheme_type_manifest_rejects_duplicate_field_names() {
    let manifest = decode_gerbil_scheme_type_manifest(
        r#"{
            "schema_id": "marlin.scheme-types.manifest.v1",
            "types": [
                {
                    "type_id": "marlin.duplicate-fields",
                    "fields": [
                        {"name": "action", "type_id": "string", "required": true},
                        {"name": "action", "type_id": "string", "required": false}
                    ]
                }
            ]
        }"#,
    )
    .expect("decode duplicate field manifest");

    let error = validate_gerbil_scheme_type_manifest(&manifest)
        .expect_err("duplicate field names should fail manifest validation");

    assert_eq!(
        error,
        GerbilSchemeTypeDecodeError::DuplicateField {
            type_id: GerbilSchemeTypeId::new("marlin.duplicate-fields"),
            field_name: GerbilSchemeFieldName::new("action"),
        }
    );
}

#[test]
fn scheme_type_manifest_rejects_unknown_field_type_references() {
    let manifest = decode_gerbil_scheme_type_manifest(
        r#"{
            "schema_id": "marlin.scheme-types.manifest.v1",
            "types": [
                {
                    "type_id": "marlin.bad-field-type",
                    "fields": [
                        {"name": "action", "type_id": "marlin.missing-type", "required": true}
                    ]
                }
            ]
        }"#,
    )
    .expect("decode unknown field type manifest");

    let error = validate_gerbil_scheme_type_manifest(&manifest)
        .expect_err("unknown field type should fail manifest validation");

    assert_eq!(
        error,
        GerbilSchemeTypeDecodeError::UnknownFieldType {
            type_id: GerbilSchemeTypeId::new("marlin.bad-field-type"),
            field_name: GerbilSchemeFieldName::new("action"),
            field_type_id: GerbilSchemeTypeId::new("marlin.missing-type"),
        }
    );
}
