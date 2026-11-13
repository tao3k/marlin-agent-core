// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use super::{
    downstream_strategy_package, duplicate_abi_symbol_contract, graph_policy_abi_readiness_plan,
};
use crate::scheme_types::support::{strategy_selection_schema_id, strategy_selection_type_id};
use marlin_gerbil_scheme::{
    GERBIL_AGENT_POLICY_ROUTING_ABI_ID, GERBIL_AGENT_POLICY_ROUTING_ABI_VERSION,
    GerbilSchemeAbiSymbol, GerbilSchemeCAbiContract, GerbilSchemeCAbiId,
    GerbilSchemeCAbiReadinessPlan, GerbilSchemePackageId, GerbilSchemeProjectionContract,
    GerbilSchemeTypeDecodeError, GerbilSchemeTypeId, validate_gerbil_scheme_package_abi_readiness,
    validate_gerbil_scheme_package_manifest,
};

#[test]
fn scheme_package_manifest_rejects_unknown_projection_type() {
    let manifest = downstream_strategy_package().with_projection_contracts([
        GerbilSchemeProjectionContract::new(GerbilSchemeTypeId::new(
            "marlin.missing-projection-type",
        )),
    ]);

    let error = validate_gerbil_scheme_package_manifest(&manifest)
        .expect_err("unknown projection contract type should fail package validation");

    assert_eq!(
        error,
        GerbilSchemeTypeDecodeError::UnknownType {
            type_id: GerbilSchemeTypeId::new("marlin.missing-projection-type")
        }
    );
}

#[test]
fn scheme_package_manifest_rejects_duplicate_projection_contracts() {
    let contract = GerbilSchemeProjectionContract::new(strategy_selection_type_id())
        .with_schema_id(strategy_selection_schema_id());
    let manifest =
        downstream_strategy_package().with_projection_contracts([contract.clone(), contract]);

    let error = validate_gerbil_scheme_package_manifest(&manifest)
        .expect_err("duplicate projection contracts should fail package validation");

    assert_eq!(
        error,
        GerbilSchemeTypeDecodeError::DuplicateProjectionContract {
            type_id: strategy_selection_type_id(),
            schema_id: Some(strategy_selection_schema_id()),
        }
    );
}

#[test]
fn scheme_package_manifest_rejects_empty_c_abi_exports() {
    let manifest = downstream_strategy_package().with_c_abi(GerbilSchemeCAbiContract::new(
        GerbilSchemeCAbiId::new(GERBIL_AGENT_POLICY_ROUTING_ABI_ID),
        GERBIL_AGENT_POLICY_ROUTING_ABI_VERSION,
    ));

    let error = validate_gerbil_scheme_package_manifest(&manifest)
        .expect_err("native ABI without exported symbols should fail package validation");

    assert_eq!(
        error,
        GerbilSchemeTypeDecodeError::MissingAbiSymbols {
            abi_id: GerbilSchemeCAbiId::new(GERBIL_AGENT_POLICY_ROUTING_ABI_ID),
        }
    );
}

#[test]
fn scheme_package_manifest_rejects_duplicate_c_abi_exports() {
    let manifest = downstream_strategy_package().with_c_abi(duplicate_abi_symbol_contract());

    let error = validate_gerbil_scheme_package_manifest(&manifest)
        .expect_err("duplicate native ABI symbols should fail package validation");

    assert_eq!(
        error,
        GerbilSchemeTypeDecodeError::DuplicateAbiSymbol {
            symbol: GerbilSchemeAbiSymbol::new("marlin_agent_policy_routing_select_edges"),
        }
    );
}

#[test]
fn scheme_package_native_readiness_rejects_missing_c_abi_contract() {
    let manifest = downstream_strategy_package();

    let error =
        validate_gerbil_scheme_package_abi_readiness(&manifest, &graph_policy_abi_readiness_plan())
            .expect_err("readiness should fail when package has no native ABI contract");

    assert_eq!(
        error,
        GerbilSchemeTypeDecodeError::MissingCAbi {
            package_id: GerbilSchemePackageId::new("marlin.downstream.strategy-package"),
        }
    );
}

#[test]
fn scheme_package_native_readiness_rejects_wrong_exported_symbol() {
    let missing_symbol = GerbilSchemeAbiSymbol::new("marlin_downstream_missing_export");
    let manifest = downstream_strategy_package().with_c_abi(
        GerbilSchemeCAbiContract::new(
            GerbilSchemeCAbiId::new(GERBIL_AGENT_POLICY_ROUTING_ABI_ID),
            GERBIL_AGENT_POLICY_ROUTING_ABI_VERSION,
        )
        .with_exported_symbols([missing_symbol.clone()]),
    );

    let error =
        validate_gerbil_scheme_package_abi_readiness(&manifest, &graph_policy_abi_readiness_plan())
            .expect_err("readiness should fail when manifest requires a missing native symbol");

    assert_eq!(
        error,
        GerbilSchemeTypeDecodeError::MissingAbiSymbol {
            symbol: missing_symbol
        }
    );
}

#[test]
fn scheme_package_native_readiness_rejects_abi_version_mismatch() {
    let manifest = downstream_strategy_package().with_c_abi(
        GerbilSchemeCAbiContract::new(
            GerbilSchemeCAbiId::new(GERBIL_AGENT_POLICY_ROUTING_ABI_ID),
            GERBIL_AGENT_POLICY_ROUTING_ABI_VERSION,
        )
        .with_exported_symbols([GerbilSchemeAbiSymbol::new(
            "marlin_agent_policy_routing_select_edges",
        )]),
    );
    let readiness_plan = GerbilSchemeCAbiReadinessPlan::new(
        GerbilSchemeCAbiId::new(GERBIL_AGENT_POLICY_ROUTING_ABI_ID),
        GERBIL_AGENT_POLICY_ROUTING_ABI_VERSION + 1,
    )
    .with_exported_symbols([GerbilSchemeAbiSymbol::new(
        "marlin_agent_policy_routing_select_edges",
    )]);

    let error = validate_gerbil_scheme_package_abi_readiness(&manifest, &readiness_plan)
        .expect_err("readiness should fail when native ABI versions differ");

    assert_eq!(
        error,
        GerbilSchemeTypeDecodeError::CAbiVersionMismatch {
            abi_id: GerbilSchemeCAbiId::new(GERBIL_AGENT_POLICY_ROUTING_ABI_ID),
            expected: GERBIL_AGENT_POLICY_ROUTING_ABI_VERSION,
            actual: GERBIL_AGENT_POLICY_ROUTING_ABI_VERSION + 1,
        }
    );
}

#[test]
fn package_manifest_rejects_a_retired_abi_key_instead_of_losing_the_contract() {
    let manifest = downstream_strategy_package().with_c_abi(super::graph_policy_abi_contract());
    let mut encoded = serde_json::to_value(manifest).unwrap();
    let fields = encoded.as_object_mut().unwrap();
    let abi = fields.remove("c_abi").unwrap();
    fields.insert("native_abi".into(), abi);
    let error =
        serde_json::from_value::<marlin_gerbil_scheme::GerbilSchemePackageManifest>(encoded)
            .unwrap_err();
    assert!(error.to_string().contains("unknown field"));
}
