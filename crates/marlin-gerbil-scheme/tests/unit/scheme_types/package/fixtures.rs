// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use crate::scheme_types::support::strategy_selection_manifest;
use marlin_gerbil_scheme::{
    GERBIL_AGENT_POLICY_ROUTING_ABI_ID, GERBIL_AGENT_POLICY_ROUTING_ABI_VERSION, GerbilAotConfig,
    GerbilSchemeAbiSymbol, GerbilSchemeCAbiContract, GerbilSchemeCAbiId,
    GerbilSchemeCAbiReadinessPlan, GerbilSchemePackageId, GerbilSchemePackageManifest,
};

pub(super) fn downstream_strategy_package() -> GerbilSchemePackageManifest {
    GerbilSchemePackageManifest::new(
        GerbilSchemePackageId::new("marlin.downstream.strategy-package"),
        strategy_selection_manifest(),
    )
}

pub(super) fn graph_policy_abi_contract() -> GerbilSchemeCAbiContract {
    let plan = GerbilAotConfig::agent_policy_routing("target/test-downstream-package").plan();
    plan.scheme_c_abi_contract()
}

pub(super) fn graph_policy_abi_readiness_plan() -> GerbilSchemeCAbiReadinessPlan {
    let plan = GerbilAotConfig::agent_policy_routing("target/test-downstream-package").plan();
    plan.scheme_c_abi_readiness_plan()
}

pub(super) fn duplicate_abi_symbol_contract() -> GerbilSchemeCAbiContract {
    let symbol = GerbilSchemeAbiSymbol::new("marlin_agent_policy_routing_select_edges");
    GerbilSchemeCAbiContract::new(
        GerbilSchemeCAbiId::new(GERBIL_AGENT_POLICY_ROUTING_ABI_ID),
        GERBIL_AGENT_POLICY_ROUTING_ABI_VERSION,
    )
    .with_exported_symbols([symbol.clone(), symbol])
}
