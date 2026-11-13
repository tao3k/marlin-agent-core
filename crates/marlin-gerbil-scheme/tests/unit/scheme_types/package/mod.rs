// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

mod abi_projection;
mod dynamic_graph_policy;
mod dynamic_runtime_bridge;
mod failures;
mod happy_path;
mod loop_governor_bridge;
mod policy_pack_projection;

use super::support::strategy_selection_manifest;
use marlin_gerbil_scheme::{
    GERBIL_AGENT_POLICY_ROUTING_ABI_ID, GERBIL_AGENT_POLICY_ROUTING_ABI_VERSION, GerbilAotConfig,
    GerbilSchemeAbiSymbol, GerbilSchemeCAbiContract, GerbilSchemeCAbiId,
    GerbilSchemeCAbiReadinessPlan, GerbilSchemePackageId, GerbilSchemePackageManifest,
};

fn downstream_strategy_package() -> GerbilSchemePackageManifest {
    GerbilSchemePackageManifest::new(
        GerbilSchemePackageId::new("marlin.downstream.strategy-package"),
        strategy_selection_manifest(),
    )
}

fn graph_policy_abi_contract() -> GerbilSchemeCAbiContract {
    let plan = GerbilAotConfig::agent_policy_routing("target/test-downstream-package").plan();
    plan.scheme_c_abi_contract()
}

fn graph_policy_abi_readiness_plan() -> GerbilSchemeCAbiReadinessPlan {
    let plan = GerbilAotConfig::agent_policy_routing("target/test-downstream-package").plan();
    plan.scheme_c_abi_readiness_plan()
}

fn duplicate_abi_symbol_contract() -> GerbilSchemeCAbiContract {
    let symbol = GerbilSchemeAbiSymbol::new("marlin_agent_policy_routing_select_edges");
    GerbilSchemeCAbiContract::new(
        GerbilSchemeCAbiId::new(GERBIL_AGENT_POLICY_ROUTING_ABI_ID),
        GERBIL_AGENT_POLICY_ROUTING_ABI_VERSION,
    )
    .with_exported_symbols([symbol.clone(), symbol])
}
