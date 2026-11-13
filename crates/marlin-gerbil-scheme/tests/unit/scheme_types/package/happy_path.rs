// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use super::{
    downstream_strategy_package, graph_policy_abi_contract, graph_policy_abi_readiness_plan,
};
use crate::scheme_types::support::{strategy_selection_schema_id, strategy_selection_type_id};
use marlin_agent_protocol::{
    GraphAbiSymbol, GraphCAbiId, GraphLoopStrategy, GraphPolicyProposal, GraphPolicyProposalStatus,
    LoopGraph, LoopNodeSpec,
};
use marlin_gerbil_scheme::{
    GERBIL_AGENT_POLICY_ROUTING_ABI_ID, GERBIL_AGENT_POLICY_ROUTING_ABI_VERSION, GerbilAotConfig,
    GerbilSchemeAbiSymbol, GerbilSchemeCAbiId, GerbilSchemePackageId,
    GerbilSchemeProjectionContract,
    scheme_type_fixtures::decode_gerbil_scheme_package_manifest_fixture,
    validate_gerbil_scheme_package_abi_readiness, validate_gerbil_scheme_package_manifest,
};
use std::collections::BTreeMap;

#[test]
fn scheme_package_manifest_connects_types_projection_and_c_abi() {
    let manifest = downstream_strategy_package()
        .with_projection_contracts([GerbilSchemeProjectionContract::new(
            strategy_selection_type_id(),
        )
        .with_schema_id(strategy_selection_schema_id())])
        .with_c_abi(graph_policy_abi_contract());

    let receipt = validate_gerbil_scheme_package_manifest(&manifest)
        .expect("downstream Scheme package manifest should validate");

    assert_eq!(
        receipt.package_id,
        GerbilSchemePackageId::new("marlin.downstream.strategy-package")
    );
    assert_eq!(receipt.type_count, 1);
    assert_eq!(receipt.field_count, 3);
    assert_eq!(receipt.projection_contract_count, 1);
    assert_eq!(
        receipt.c_abi_version,
        Some(GERBIL_AGENT_POLICY_ROUTING_ABI_VERSION)
    );
    assert_eq!(receipt.abi_symbol_count, 2);

    let readiness =
        validate_gerbil_scheme_package_abi_readiness(&manifest, &graph_policy_abi_readiness_plan())
            .expect("downstream Scheme package should match native readiness plan");

    assert_eq!(readiness.required_symbol_count, 2);
    assert_eq!(readiness.available_symbol_count, 2);
    assert_eq!(readiness.matched_symbol_count, 2);
}

#[test]
fn scheme_package_manifest_round_trips_from_serialized_fixture() {
    let manifest = decode_gerbil_scheme_package_manifest_fixture(
        r#"{
            "schema_id": "marlin.scheme-package.manifest.v1",
            "package_id": "marlin.downstream.strategy-package",
            "type_manifest": {
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
            },
            "projection_contracts": [
                {
                    "type_id": "marlin.deck-runtime.strategy-selection",
                    "schema_id": "marlin.deck-runtime.strategy-selection.v1"
                }
            ],
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
    .expect("decode downstream package manifest");

    let receipt = validate_gerbil_scheme_package_manifest(&manifest)
        .expect("decoded downstream package manifest should validate");

    assert_eq!(receipt.projection_contract_count, 1);
    assert_eq!(receipt.abi_symbol_count, 2);

    let readiness =
        validate_gerbil_scheme_package_abi_readiness(&manifest, &graph_policy_abi_readiness_plan())
            .expect("decoded downstream package manifest should match native readiness plan");

    assert_eq!(readiness.matched_symbol_count, 2);
}

#[test]
fn graph_aot_plan_derives_scheme_readiness_plan() {
    let aot_plan = GerbilAotConfig::agent_policy_routing("target/test-downstream-package").plan();
    let readiness_plan = aot_plan.scheme_c_abi_readiness_plan();

    assert_eq!(
        readiness_plan.abi_id,
        GerbilSchemeCAbiId::new(GERBIL_AGENT_POLICY_ROUTING_ABI_ID)
    );
    assert_eq!(
        readiness_plan.version,
        GERBIL_AGENT_POLICY_ROUTING_ABI_VERSION
    );
    assert_eq!(
        readiness_plan.exported_symbols.len(),
        aot_plan.exported_symbols.len()
    );
    assert_eq!(
        readiness_plan.exported_symbols,
        aot_plan
            .exported_symbols
            .iter()
            .map(GerbilSchemeAbiSymbol::from)
            .collect::<Vec<_>>()
    );
}

#[test]
fn graph_aot_plan_derives_scheme_c_abi_contract() {
    let aot_plan = GerbilAotConfig::agent_policy_routing("target/test-downstream-package").plan();
    let c_abi = aot_plan.scheme_c_abi_contract();

    assert_eq!(
        c_abi.abi_id,
        GerbilSchemeCAbiId::new(GERBIL_AGENT_POLICY_ROUTING_ABI_ID)
    );
    assert_eq!(c_abi.version, GERBIL_AGENT_POLICY_ROUTING_ABI_VERSION);
    assert_eq!(
        c_abi.exported_symbols.len(),
        aot_plan.exported_symbols.len()
    );
    assert_eq!(
        c_abi.exported_symbols,
        aot_plan
            .exported_symbols
            .iter()
            .map(GerbilSchemeAbiSymbol::from)
            .collect::<Vec<_>>()
    );
}

#[test]
fn scheme_c_abi_contract_projects_to_graph_c_abi_requirement() {
    let c_abi = graph_policy_abi_contract();
    let graph_requirement = c_abi.graph_c_abi_requirement();

    assert_eq!(
        graph_requirement.abi_id,
        GraphCAbiId::new(GERBIL_AGENT_POLICY_ROUTING_ABI_ID)
    );
    assert_eq!(
        graph_requirement.version,
        GERBIL_AGENT_POLICY_ROUTING_ABI_VERSION
    );
    assert_eq!(
        graph_requirement.required_symbols,
        c_abi
            .exported_symbols
            .iter()
            .map(|symbol| GraphAbiSymbol::new(symbol.as_str()))
            .collect::<Vec<_>>()
    );
}

#[test]
fn graph_aot_plan_derives_graph_c_abi_requirement() {
    let aot_plan = GerbilAotConfig::agent_policy_routing("target/test-downstream-package").plan();
    let graph_requirement = aot_plan.graph_c_abi_requirement();

    assert_eq!(
        graph_requirement.abi_id,
        GraphCAbiId::new(GERBIL_AGENT_POLICY_ROUTING_ABI_ID)
    );
    assert_eq!(
        graph_requirement.required_symbols,
        aot_plan
            .exported_symbols
            .iter()
            .map(|symbol| GraphAbiSymbol::new(symbol.as_str()))
            .collect::<Vec<_>>()
    );
}

#[test]
fn deck_route_abi_graph_requirement_satisfies_graph_policy_validation() {
    let aot_plan = GerbilAotConfig::agent_policy_routing("target/test-downstream-package").plan();
    let proposal = GraphPolicyProposal::new(
        GraphLoopStrategy::gerbil_c_abi("gerbil-deck-runtime", "v1"),
        LoopGraph {
            graph_id: "gerbil-deck-runtime-graph".to_string(),
            nodes: vec![LoopNodeSpec {
                id: "select-model-route".to_string(),
                executor: "gerbil.deck-runtime.select-model-route".to_string(),
                config: BTreeMap::new(),
            }],
            edges: Vec::new(),
        },
        "sha256:input",
        "sha256:output",
    )
    .with_c_abi_requirement(aot_plan.graph_c_abi_requirement());

    let validation = proposal.validate();

    assert_eq!(validation.status, GraphPolicyProposalStatus::Accepted);
    assert_eq!(
        validation
            .c_abi
            .as_ref()
            .expect("native ABI requirement")
            .required_symbols
            .len(),
        aot_plan.exported_symbols.len()
    );
}
