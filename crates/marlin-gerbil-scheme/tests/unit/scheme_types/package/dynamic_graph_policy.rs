use super::deck_runtime_native_readiness_plan;
use marlin_gerbil_scheme::{
    GerbilSchemeSchemaId, GerbilSchemeTypeId, GerbilSchemeTypeRegistry, GerbilSchemeTypedValue,
    decode_gerbil_scheme_package_manifest, validate_gerbil_scheme_package_manifest,
    validate_gerbil_scheme_package_native_readiness,
};
use serde_json::json;

#[test]
fn scheme_package_manifest_decodes_dynamic_graph_policy_payload_through_native_abi_bridge() {
    let manifest = decode_gerbil_scheme_package_manifest(
        r#"{
            "schema_id": "marlin.scheme-package.manifest.v1",
            "package_id": "marlin.downstream.graph-policy-package",
            "type_manifest": {
                "schema_id": "marlin.scheme-types.manifest.v1",
                "types": [
                    {
                        "type_id": "marlin.downstream.graph-policy.native-abi",
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
                        "type_id": "marlin.downstream.graph-policy",
                        "schema_id": "marlin.downstream.graph-policy.v1",
                        "fields": [
                            {"name": "schema_id", "type_id": "string", "required": true},
                            {"name": "policy", "type_id": "object", "required": true},
                            {
                                "name": "native_abi",
                                "type_id": "marlin.downstream.graph-policy.native-abi",
                                "required": true
                            }
                        ]
                    }
                ]
            },
            "native_abi": {
                "abi_id": "marlin.deck-runtime.native",
                "version": 1,
                "exported_symbols": [
                    "marlin_deck_runtime_initialize",
                    "marlin_deck_runtime_select_model_route"
                ]
            }
        }"#,
    )
    .expect("decode dynamic graph policy package manifest");
    let receipt = validate_gerbil_scheme_package_manifest(&manifest)
        .expect("dynamic graph policy package manifest should validate");
    let readiness = validate_gerbil_scheme_package_native_readiness(
        &manifest,
        &deck_runtime_native_readiness_plan(),
    )
    .expect("dynamic graph policy package should match native readiness");
    let registry = GerbilSchemeTypeRegistry::new(manifest.type_manifest.clone())
        .expect("package type manifest should build dynamic registry");
    let envelope = GerbilSchemeTypedValue::new(
        GerbilSchemeTypeId::new("marlin.downstream.graph-policy"),
        json!({
            "schema_id": "marlin.downstream.graph-policy.v1",
            "policy": {
                "strategy": "beam-search-v2",
                "ranker": "pure-gerbil-policy"
            },
            "native_abi": {
                "abi_id": "marlin.deck-runtime.native",
                "version": 1,
                "required_symbols": [
                    "marlin_deck_runtime_initialize",
                    "marlin_deck_runtime_select_model_route"
                ]
            }
        }),
    )
    .with_schema_id(GerbilSchemeSchemaId::new(
        "marlin.downstream.graph-policy.v1",
    ));

    let projection = registry
        .decode_dynamic_typed_value(&envelope)
        .expect("package dynamic graph policy should decode through generic bridge");

    assert_eq!(receipt.type_count, 2);
    assert_eq!(readiness.matched_symbol_count, 2);
    assert_eq!(projection["policy"]["ranker"], "pure-gerbil-policy");
    assert_eq!(
        projection["native_abi"]["required_symbols"][0],
        "marlin_deck_runtime_initialize"
    );
}
