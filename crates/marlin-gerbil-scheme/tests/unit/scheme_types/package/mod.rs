mod abi_projection;
mod dynamic_graph_policy;
mod dynamic_runtime_bridge;
mod failures;
mod happy_path;
mod loop_governor_bridge;
mod policy_pack_projection;

use super::support::strategy_selection_manifest;
use marlin_gerbil_scheme::{
    GERBIL_DECK_ROUTE_ABI_ID, GERBIL_DECK_ROUTE_ABI_VERSION, GerbilDeckRuntimeAotConfig,
    GerbilSchemeAbiSymbol, GerbilSchemeCAbiContract, GerbilSchemeCAbiId,
    GerbilSchemeCAbiReadinessPlan, GerbilSchemePackageId, GerbilSchemePackageManifest,
};

fn downstream_strategy_package() -> GerbilSchemePackageManifest {
    GerbilSchemePackageManifest::new(
        GerbilSchemePackageId::new("marlin.downstream.strategy-package"),
        strategy_selection_manifest(),
    )
}

fn deck_route_abi_abi_contract() -> GerbilSchemeCAbiContract {
    let plan = GerbilDeckRuntimeAotConfig::new("target/test-downstream-package").plan();
    plan.scheme_c_abi_contract()
}

fn deck_route_abi_readiness_plan() -> GerbilSchemeCAbiReadinessPlan {
    let plan = GerbilDeckRuntimeAotConfig::new("target/test-downstream-package").plan();
    plan.scheme_c_abi_readiness_plan()
}

fn duplicate_native_symbol_contract() -> GerbilSchemeCAbiContract {
    let symbol = GerbilSchemeAbiSymbol::new("marlin_deck_runtime_select_model_route");
    GerbilSchemeCAbiContract::new(
        GerbilSchemeCAbiId::new(GERBIL_DECK_ROUTE_ABI_ID),
        GERBIL_DECK_ROUTE_ABI_VERSION,
    )
    .with_exported_symbols([symbol.clone(), symbol])
}
