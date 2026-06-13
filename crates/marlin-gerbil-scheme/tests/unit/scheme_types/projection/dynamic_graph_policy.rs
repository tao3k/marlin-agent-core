use marlin_gerbil_scheme::{
    GerbilSchemeFieldName, GerbilSchemeSchemaId, GerbilSchemeTypeDecodeError, GerbilSchemeTypeId,
    GerbilSchemeTypeManifest, GerbilSchemeTypeRegistry, GerbilSchemeTypedValue,
    decode_gerbil_scheme_type_manifest,
};
use serde_json::{Value, json};

#[test]
fn downstream_scheme_graph_policy_projects_without_rust_static_binding() {
    let registry = downstream_graph_policy_registry();
    let envelope = GerbilSchemeTypedValue::new(
        downstream_graph_policy_type_id(),
        json!({
            "schema_id": "downstream.agent.graph-policy.experimental.v1",
            "policy": {
                "strategy": "beam-search-v2",
                "ranker": "pure-gerbil-policy",
                "max_candidates": 8
            },
            "native_abi": {
                "abi_id": "downstream.agent.graph-policy.native",
                "version": 3,
                "required_symbols": [
                    "downstream_graph_policy_rank",
                    "downstream_graph_policy_select"
                ]
            }
        }),
    )
    .with_schema_id(downstream_graph_policy_schema_id());

    let projection: Value = registry
        .decode_dynamic_typed_value(&envelope)
        .expect("dynamic downstream graph policy should project without a Rust policy type");

    assert_eq!(registry.validation_receipt().type_count, 2);
    assert_eq!(registry.validation_receipt().field_count, 6);
    assert_eq!(projection["policy"]["strategy"], "beam-search-v2");
    assert_eq!(
        projection["native_abi"]["required_symbols"][1],
        "downstream_graph_policy_select"
    );
}

#[test]
fn downstream_scheme_graph_policy_rejects_missing_required_field_without_rust_policy_type() {
    let registry = downstream_graph_policy_registry();
    let envelope = GerbilSchemeTypedValue::new(
        downstream_graph_policy_type_id(),
        json!({
            "schema_id": "downstream.agent.graph-policy.experimental.v1",
            "policy": {
                "strategy": "beam-search-v2",
                "ranker": "pure-gerbil-policy"
            }
        }),
    )
    .with_schema_id(downstream_graph_policy_schema_id());

    let error = registry
        .decode_dynamic_typed_value(&envelope)
        .expect_err("dynamic graph policy should enforce manifest required fields");

    assert_eq!(
        error,
        GerbilSchemeTypeDecodeError::MissingRequiredField {
            type_id: downstream_graph_policy_type_id(),
            field_name: GerbilSchemeFieldName::new("native_abi"),
        }
    );
}

#[test]
fn downstream_scheme_graph_policy_rejects_builtin_field_type_mismatch_without_rust_policy_type() {
    let registry = downstream_graph_policy_registry();
    let envelope = GerbilSchemeTypedValue::new(
        downstream_graph_policy_type_id(),
        json!({
            "schema_id": "downstream.agent.graph-policy.experimental.v1",
            "policy": {
                "strategy": "beam-search-v2",
                "ranker": "pure-gerbil-policy"
            },
            "native_abi": "not-an-abi-object"
        }),
    )
    .with_schema_id(downstream_graph_policy_schema_id());

    let error = registry
        .decode_dynamic_typed_value(&envelope)
        .expect_err("dynamic graph policy should enforce builtin manifest field types");

    assert_eq!(
        error,
        GerbilSchemeTypeDecodeError::FieldTypeMismatch {
            type_id: downstream_graph_policy_type_id(),
            field_name: GerbilSchemeFieldName::new("native_abi"),
            expected: downstream_graph_policy_native_abi_type_id(),
        }
    );
}

#[test]
fn downstream_scheme_graph_policy_rejects_nested_custom_type_field_mismatch_without_rust_policy_type()
 {
    let registry = downstream_graph_policy_registry();
    let envelope = GerbilSchemeTypedValue::new(
        downstream_graph_policy_type_id(),
        json!({
            "schema_id": "downstream.agent.graph-policy.experimental.v1",
            "policy": {
                "strategy": "beam-search-v2",
                "ranker": "pure-gerbil-policy"
            },
            "native_abi": {
                "abi_id": "downstream.agent.graph-policy.native",
                "version": "3",
                "required_symbols": [
                    "downstream_graph_policy_rank",
                    "downstream_graph_policy_select"
                ]
            }
        }),
    )
    .with_schema_id(downstream_graph_policy_schema_id());

    let error = registry
        .decode_dynamic_typed_value(&envelope)
        .expect_err("dynamic graph policy should recursively enforce custom Scheme type fields");

    assert_eq!(
        error,
        GerbilSchemeTypeDecodeError::FieldTypeMismatch {
            type_id: downstream_graph_policy_native_abi_type_id(),
            field_name: GerbilSchemeFieldName::new("version"),
            expected: GerbilSchemeTypeId::new("integer"),
        }
    );
}

fn downstream_graph_policy_registry() -> GerbilSchemeTypeRegistry {
    GerbilSchemeTypeRegistry::new(downstream_graph_policy_manifest())
        .expect("downstream manifest builds registry")
}

fn downstream_graph_policy_manifest() -> GerbilSchemeTypeManifest {
    decode_gerbil_scheme_type_manifest(
        r#"{
            "schema_id": "marlin.scheme-types.manifest.v1",
            "types": [
                {
                    "type_id": "downstream.agent.graph-policy.native-abi",
                    "schema_id": "downstream.agent.graph-policy.native-abi.v1",
                    "fields": [
                        {"name": "abi_id", "type_id": "string", "required": true},
                        {"name": "version", "type_id": "integer", "required": true},
                        {"name": "required_symbols", "type_id": "array", "required": true}
                    ]
                },
                {
                    "type_id": "downstream.agent.graph-policy.experimental",
                    "schema_id": "downstream.agent.graph-policy.experimental.v1",
                    "fields": [
                        {"name": "schema_id", "type_id": "string", "required": true},
                        {"name": "policy", "type_id": "object", "required": true},
                        {
                            "name": "native_abi",
                            "type_id": "downstream.agent.graph-policy.native-abi",
                            "required": true
                        }
                    ]
                }
            ]
        }"#,
    )
    .expect("decode downstream graph policy manifest")
}

fn downstream_graph_policy_type_id() -> GerbilSchemeTypeId {
    GerbilSchemeTypeId::new("downstream.agent.graph-policy.experimental")
}

fn downstream_graph_policy_native_abi_type_id() -> GerbilSchemeTypeId {
    GerbilSchemeTypeId::new("downstream.agent.graph-policy.native-abi")
}

fn downstream_graph_policy_schema_id() -> GerbilSchemeSchemaId {
    GerbilSchemeSchemaId::new("downstream.agent.graph-policy.experimental.v1")
}
