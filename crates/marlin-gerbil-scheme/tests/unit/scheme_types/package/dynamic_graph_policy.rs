// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use super::graph_policy_abi_readiness_plan;
use marlin_gerbil_scheme::{
    GerbilSchemeSchemaId, GerbilSchemeTypeId, GerbilSchemeTypeRegistry, GerbilSchemeTypedValue,
    GerbilSchemeValue, scheme_type_fixtures::decode_gerbil_scheme_package_manifest_fixture,
    validate_gerbil_scheme_package_abi_readiness, validate_gerbil_scheme_package_manifest,
};

#[test]
fn scheme_package_manifest_decodes_dynamic_graph_policy_payload_through_c_abi_bridge() {
    let manifest = decode_gerbil_scheme_package_manifest_fixture(
        r#"{
            "schema_id": "marlin.scheme-package.manifest.v1",
            "package_id": "marlin.downstream.graph-policy-package",
            "type_manifest": {
                "schema_id": "marlin.scheme-types.manifest.v1",
                "types": [
                    {
                        "type_id": "marlin.downstream.graph-policy.c-abi",
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
                                "name": "c_abi",
                                "type_id": "marlin.downstream.graph-policy.c-abi",
                                "required": true
                            }
                        ]
                    }
                ]
            },
            "c_abi": {
                "abi_id": "marlin.agent.policy-routing.native",
                "version": 1,
                "exported_symbols": [
                    "marlin_agent_policy_routing_initialize",
                    "marlin_agent_policy_routing_select_edges"
                ]
            }
        }"#,
    )
    .expect("decode dynamic graph policy package manifest");
    let receipt = validate_gerbil_scheme_package_manifest(&manifest)
        .expect("dynamic graph policy package manifest should validate");
    let readiness =
        validate_gerbil_scheme_package_abi_readiness(&manifest, &graph_policy_abi_readiness_plan())
            .expect("dynamic graph policy package should match native readiness");
    let registry = GerbilSchemeTypeRegistry::new(manifest.type_manifest.clone())
        .expect("package type manifest should build dynamic registry");
    let envelope = GerbilSchemeTypedValue::new(
        GerbilSchemeTypeId::new("marlin.downstream.graph-policy"),
        GerbilSchemeValue::record([
            ("schema_id", "marlin.downstream.graph-policy.v1".into()),
            (
                "policy",
                GerbilSchemeValue::record([
                    ("strategy", "beam-search-v2".into()),
                    ("ranker", "pure-gerbil-policy".into()),
                ]),
            ),
            (
                "c_abi",
                GerbilSchemeValue::record([
                    ("abi_id", "marlin.agent.policy-routing.native".into()),
                    ("version", 1.into()),
                    (
                        "required_symbols",
                        GerbilSchemeValue::vector([
                            "marlin_agent_policy_routing_initialize".into(),
                            "marlin_agent_policy_routing_select_edges".into(),
                        ]),
                    ),
                ]),
            ),
        ]),
    )
    .with_schema_id(GerbilSchemeSchemaId::new(
        "marlin.downstream.graph-policy.v1",
    ));

    let projection = registry
        .decode_dynamic_typed_value(&envelope)
        .expect("package dynamic graph policy should decode through generic bridge");

    assert_eq!(receipt.type_count, 2);
    assert_eq!(readiness.matched_symbol_count, 2);
    assert_eq!(
        projection
            .get("policy")
            .and_then(|policy| policy.get("ranker"))
            .and_then(GerbilSchemeValue::as_text),
        Some("pure-gerbil-policy")
    );
    assert_eq!(
        projection
            .get("c_abi")
            .and_then(|c_abi| c_abi.get("required_symbols"))
            .and_then(|symbols| symbols.get_index(0))
            .and_then(GerbilSchemeValue::as_text),
        Some("marlin_agent_policy_routing_initialize")
    );
}
