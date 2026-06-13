use marlin_gerbil_scheme::{
    GerbilSchemeFieldName, GerbilSchemeSchemaId, GerbilSchemeTypeDecodeError,
    GerbilSchemeTypeFieldSpec, GerbilSchemeTypeId, GerbilSchemeTypeManifest,
    GerbilSchemeTypeRegistry, GerbilSchemeTypeSpec, GerbilSchemeTypedValue,
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
            field_path: "native_abi".to_owned(),
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
            field_path: "native_abi".to_owned(),
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
            field_path: "native_abi.version".to_owned(),
        }
    );
}

#[test]
fn downstream_scheme_graph_policy_rejects_array_element_type_mismatch_without_rust_policy_type() {
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
                "version": 3,
                "required_symbols": [
                    "downstream_graph_policy_rank",
                    42
                ]
            }
        }),
    )
    .with_schema_id(downstream_graph_policy_schema_id());

    let error = registry
        .decode_dynamic_typed_value(&envelope)
        .expect_err("dynamic graph policy should enforce array element types");

    assert_eq!(
        error,
        GerbilSchemeTypeDecodeError::FieldTypeMismatch {
            type_id: downstream_graph_policy_native_abi_type_id(),
            field_name: GerbilSchemeFieldName::new("required_symbols"),
            expected: GerbilSchemeTypeId::new("string"),
            field_path: "native_abi.required_symbols[1]".to_owned(),
        }
    );
}

#[test]
fn downstream_scheme_graph_policy_rejects_recursive_custom_type_without_rust_policy_type() {
    let registry = GerbilSchemeTypeRegistry::new(recursive_graph_policy_manifest())
        .expect("recursive manifest remains structurally valid");
    let envelope = GerbilSchemeTypedValue::new(
        downstream_graph_policy_type_id(),
        json!({
            "schema_id": "downstream.agent.graph-policy.experimental.v1",
            "native_abi": {
                "parent": {
                    "schema_id": "downstream.agent.graph-policy.experimental.v1",
                    "native_abi": {}
                }
            }
        }),
    )
    .with_schema_id(downstream_graph_policy_schema_id());

    let error = registry
        .decode_dynamic_typed_value(&envelope)
        .expect_err("dynamic graph policy should reject recursive manifest traversal");

    assert_eq!(
        error,
        GerbilSchemeTypeDecodeError::RecursiveTypeReference {
            type_id: downstream_graph_policy_type_id(),
            field_path: "native_abi.parent".to_owned(),
        }
    );
}

#[test]
fn downstream_scheme_graph_policy_rejects_excessive_custom_type_depth_without_rust_policy_type() {
    let registry =
        GerbilSchemeTypeRegistry::new(deep_graph_policy_manifest(34)).expect("deep manifest");
    let envelope = GerbilSchemeTypedValue::new(
        GerbilSchemeTypeId::new("downstream.depth.0"),
        deep_graph_policy_value(34),
    );

    let error = registry
        .decode_dynamic_typed_value(&envelope)
        .expect_err("dynamic graph policy should bound recursive validation depth");

    let GerbilSchemeTypeDecodeError::DynamicValidationDepthExceeded {
        max_depth,
        field_path,
        ..
    } = error
    else {
        panic!("unexpected depth error shape: {error}");
    };
    assert_eq!(max_depth, 32);
    assert!(field_path.contains("child.child"));
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
                        {
                            "name": "required_symbols",
                            "type_id": "array",
                            "element_type_id": "string",
                            "required": true
                        }
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

fn recursive_graph_policy_manifest() -> GerbilSchemeTypeManifest {
    decode_gerbil_scheme_type_manifest(
        r#"{
            "schema_id": "marlin.scheme-types.manifest.v1",
            "types": [
                {
                    "type_id": "downstream.agent.graph-policy.native-abi",
                    "fields": [
                        {
                            "name": "parent",
                            "type_id": "downstream.agent.graph-policy.experimental",
                            "required": true
                        }
                    ]
                },
                {
                    "type_id": "downstream.agent.graph-policy.experimental",
                    "schema_id": "downstream.agent.graph-policy.experimental.v1",
                    "fields": [
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
    .expect("decode recursive graph policy manifest")
}

fn deep_graph_policy_manifest(depth: usize) -> GerbilSchemeTypeManifest {
    let types = (0..depth)
        .map(|index| {
            let next = index + 1;
            GerbilSchemeTypeSpec {
                type_id: GerbilSchemeTypeId::new(format!("downstream.depth.{index}")),
                schema_id: None,
                fields: if next < depth {
                    vec![GerbilSchemeTypeFieldSpec {
                        name: GerbilSchemeFieldName::new("child"),
                        type_id: GerbilSchemeTypeId::new(format!("downstream.depth.{next}")),
                        element_type_id: None,
                        required: true,
                        description: None,
                    }]
                } else {
                    Vec::new()
                },
            }
        })
        .collect();

    GerbilSchemeTypeManifest {
        schema_id: GerbilSchemeSchemaId::new("marlin.scheme-types.manifest.v1"),
        types,
    }
}

fn deep_graph_policy_value(depth: usize) -> Value {
    (1..depth).rev().fold(json!({}), |value, _| {
        json!({
            "child": value
        })
    })
}
