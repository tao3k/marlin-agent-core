use super::{
    deck_runtime_native_abi_contract, deck_runtime_native_readiness_plan,
    downstream_strategy_package,
};
use crate::scheme_types::support::{strategy_selection_schema_id, strategy_selection_type_id};
use marlin_agent_protocol::{
    GraphLoopStrategy, GraphNativeAbiId, GraphNativeSymbol, GraphPolicyProposal,
    GraphPolicyProposalStatus, LoopGraph, LoopNodeSpec,
};
use marlin_gerbil_scheme::{
    GERBIL_DECK_RUNTIME_NATIVE_ABI_ID, GERBIL_DECK_RUNTIME_NATIVE_ABI_VERSION,
    GerbilDeckRuntimeNativeAotConfig, GerbilSchemeNativeAbiId, GerbilSchemeNativeSymbol,
    GerbilSchemePackageId, GerbilSchemeProjectionContract, decode_gerbil_scheme_package_manifest,
    validate_gerbil_scheme_package_manifest, validate_gerbil_scheme_package_native_readiness,
};
use std::collections::BTreeMap;

#[test]
fn scheme_package_manifest_connects_types_projection_and_native_abi() {
    let manifest = downstream_strategy_package()
        .with_projection_contracts([GerbilSchemeProjectionContract::new(
            strategy_selection_type_id(),
        )
        .with_schema_id(strategy_selection_schema_id())])
        .with_native_abi(deck_runtime_native_abi_contract());

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
        receipt.native_abi_version,
        Some(GERBIL_DECK_RUNTIME_NATIVE_ABI_VERSION)
    );
    assert_eq!(receipt.native_symbol_count, 2);

    let readiness = validate_gerbil_scheme_package_native_readiness(
        &manifest,
        &deck_runtime_native_readiness_plan(),
    )
    .expect("downstream Scheme package should match native readiness plan");

    assert_eq!(readiness.required_symbol_count, 2);
    assert_eq!(readiness.available_symbol_count, 2);
    assert_eq!(readiness.matched_symbol_count, 2);
}

#[test]
fn scheme_package_manifest_round_trips_from_downstream_json() {
    let manifest = decode_gerbil_scheme_package_manifest(
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
    .expect("decode downstream package manifest");

    let receipt = validate_gerbil_scheme_package_manifest(&manifest)
        .expect("decoded downstream package manifest should validate");

    assert_eq!(receipt.projection_contract_count, 1);
    assert_eq!(receipt.native_symbol_count, 2);

    let readiness = validate_gerbil_scheme_package_native_readiness(
        &manifest,
        &deck_runtime_native_readiness_plan(),
    )
    .expect("decoded downstream package manifest should match native readiness plan");

    assert_eq!(readiness.matched_symbol_count, 2);
}

#[test]
fn deck_runtime_native_aot_plan_derives_scheme_readiness_plan() {
    let aot_plan = GerbilDeckRuntimeNativeAotConfig::new("target/test-downstream-package").plan();
    let readiness_plan = aot_plan.scheme_native_abi_readiness_plan();

    assert_eq!(
        readiness_plan.abi_id,
        GerbilSchemeNativeAbiId::new(GERBIL_DECK_RUNTIME_NATIVE_ABI_ID)
    );
    assert_eq!(
        readiness_plan.version,
        GERBIL_DECK_RUNTIME_NATIVE_ABI_VERSION
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
            .map(GerbilSchemeNativeSymbol::from)
            .collect::<Vec<_>>()
    );
}

#[test]
fn deck_runtime_native_aot_plan_derives_scheme_native_abi_contract() {
    let aot_plan = GerbilDeckRuntimeNativeAotConfig::new("target/test-downstream-package").plan();
    let native_abi = aot_plan.scheme_native_abi_contract();

    assert_eq!(
        native_abi.abi_id,
        GerbilSchemeNativeAbiId::new(GERBIL_DECK_RUNTIME_NATIVE_ABI_ID)
    );
    assert_eq!(native_abi.version, GERBIL_DECK_RUNTIME_NATIVE_ABI_VERSION);
    assert_eq!(
        native_abi.exported_symbols.len(),
        aot_plan.exported_symbols.len()
    );
    assert_eq!(
        native_abi.exported_symbols,
        aot_plan
            .exported_symbols
            .iter()
            .map(GerbilSchemeNativeSymbol::from)
            .collect::<Vec<_>>()
    );
}

#[test]
fn scheme_native_abi_contract_projects_to_graph_native_abi_requirement() {
    let native_abi = deck_runtime_native_abi_contract();
    let graph_requirement = native_abi.graph_native_abi_requirement();

    assert_eq!(
        graph_requirement.abi_id,
        GraphNativeAbiId::new(GERBIL_DECK_RUNTIME_NATIVE_ABI_ID)
    );
    assert_eq!(
        graph_requirement.version,
        GERBIL_DECK_RUNTIME_NATIVE_ABI_VERSION
    );
    assert_eq!(
        graph_requirement.required_symbols,
        native_abi
            .exported_symbols
            .iter()
            .map(|symbol| GraphNativeSymbol::new(symbol.as_str()))
            .collect::<Vec<_>>()
    );
}

#[test]
fn deck_runtime_native_aot_plan_derives_graph_native_abi_requirement() {
    let aot_plan = GerbilDeckRuntimeNativeAotConfig::new("target/test-downstream-package").plan();
    let graph_requirement = aot_plan.graph_native_abi_requirement();

    assert_eq!(
        graph_requirement.abi_id,
        GraphNativeAbiId::new(GERBIL_DECK_RUNTIME_NATIVE_ABI_ID)
    );
    assert_eq!(
        graph_requirement.required_symbols,
        aot_plan
            .exported_symbols
            .iter()
            .map(|symbol| GraphNativeSymbol::new(symbol.as_str()))
            .collect::<Vec<_>>()
    );
}

#[test]
fn deck_runtime_native_graph_requirement_satisfies_graph_policy_validation() {
    let aot_plan = GerbilDeckRuntimeNativeAotConfig::new("target/test-downstream-package").plan();
    let proposal = GraphPolicyProposal::new(
        GraphLoopStrategy::native_gerbil("gerbil-deck-runtime", "v1"),
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
    .with_native_abi_requirement(aot_plan.graph_native_abi_requirement());

    let validation = proposal.validate();

    assert_eq!(validation.status, GraphPolicyProposalStatus::Accepted);
    assert_eq!(
        validation
            .native_abi
            .as_ref()
            .expect("native ABI requirement")
            .required_symbols
            .len(),
        aot_plan.exported_symbols.len()
    );
}
