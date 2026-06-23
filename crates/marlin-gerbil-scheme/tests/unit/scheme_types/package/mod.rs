mod dynamic_graph_policy;
mod dynamic_runtime_bridge;
mod failures;
mod happy_path;
mod loop_governor_bridge;
mod native_projection;
mod policy_pack_projection;

use super::support::strategy_selection_manifest;
use marlin_gerbil_scheme::{
    GERBIL_DECK_RUNTIME_NATIVE_ABI_ID, GERBIL_DECK_RUNTIME_NATIVE_ABI_VERSION,
    GerbilDeckRuntimeNativeAotConfig, GerbilSchemeNativeAbiContract, GerbilSchemeNativeAbiId,
    GerbilSchemeNativeAbiReadinessPlan, GerbilSchemeNativeSymbol, GerbilSchemePackageId,
    GerbilSchemePackageManifest,
};

fn downstream_strategy_package() -> GerbilSchemePackageManifest {
    GerbilSchemePackageManifest::new(
        GerbilSchemePackageId::new("marlin.downstream.strategy-package"),
        strategy_selection_manifest(),
    )
}

fn deck_runtime_native_abi_contract() -> GerbilSchemeNativeAbiContract {
    let plan = GerbilDeckRuntimeNativeAotConfig::new("target/test-downstream-package").plan();
    plan.scheme_native_abi_contract()
}

fn deck_runtime_native_readiness_plan() -> GerbilSchemeNativeAbiReadinessPlan {
    let plan = GerbilDeckRuntimeNativeAotConfig::new("target/test-downstream-package").plan();
    plan.scheme_native_abi_readiness_plan()
}

fn duplicate_native_symbol_contract() -> GerbilSchemeNativeAbiContract {
    let symbol = GerbilSchemeNativeSymbol::new("marlin_deck_runtime_select_model_route");
    GerbilSchemeNativeAbiContract::new(
        GerbilSchemeNativeAbiId::new(GERBIL_DECK_RUNTIME_NATIVE_ABI_ID),
        GERBIL_DECK_RUNTIME_NATIVE_ABI_VERSION,
    )
    .with_exported_symbols([symbol.clone(), symbol])
}
