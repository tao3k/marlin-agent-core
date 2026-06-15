use super::support::{fake_select_model_route, fake_select_no_match, route_request};
use marlin_agent_protocol::GraphNativeAbiReadinessStatus;
use marlin_gerbil_scheme::{
    GERBIL_DECK_RUNTIME_NATIVE_ABI_ID, GERBIL_DECK_RUNTIME_NATIVE_ABI_VERSION,
    GerbilDeckRuntimeNativeAotBuildReceipt, GerbilDeckRuntimeNativeAotBuildStatus,
    GerbilDeckRuntimeNativeAotCommandPlan, GerbilDeckRuntimeNativeAotPlan,
    GerbilDeckRuntimeNativeAotStatus, GerbilDeckRuntimeNativeModelRouteSelector,
    GerbilDeckRuntimeNativeSymbol, GerbilNativeLinkLibrary, GerbilNativeSymbolAuditor,
};
use std::path::PathBuf;

#[test]
fn gerbil_deck_runtime_native_selector_uses_typed_c_abi_selection() {
    let selector = GerbilDeckRuntimeNativeModelRouteSelector::new(fake_select_model_route);
    let request = route_request("cargo test", "sub-agent");

    let policy_index = selector
        .select_policy_index(&request)
        .expect("native C ABI selector should return a typed policy index");
    assert_eq!(policy_index, Some(0));

    let receipt = selector
        .evaluate(&request)
        .expect("native C ABI selector should project typed runtime selection");
    let selected = receipt.selected_policy().expect("selected policy");

    assert!(receipt.matched);
    assert_eq!(selected.name, "cheap-test-runner");
    assert_eq!(selected.provider, "openai");
    assert_eq!(selected.model, "gpt-5-mini");
}

#[test]
fn gerbil_deck_runtime_native_selector_projects_no_match_selection() {
    let selector = GerbilDeckRuntimeNativeModelRouteSelector::new(fake_select_no_match);
    let request = route_request("cargo clippy", "sub-agent");

    let policy_index = selector
        .select_policy_index(&request)
        .expect("native C ABI selector should return unmatched policy index");
    assert_eq!(policy_index, None);

    let receipt = selector
        .evaluate(&request)
        .expect("native C ABI selector should project unmatched selection");

    assert!(!receipt.matched);
    assert!(receipt.selected_policy().is_none());
    assert_eq!(receipt.command, "cargo clippy");
    assert_eq!(receipt.agent_scope, "sub-agent");
}

#[test]
fn native_aot_build_receipt_projects_ready_graph_abi_readiness() {
    let receipt = native_build_receipt(
        GerbilDeckRuntimeNativeAotBuildStatus::LinkUnitReady,
        Vec::new(),
    );
    let readiness = receipt.graph_native_abi_readiness_receipt();

    assert_eq!(readiness.abi_id.as_str(), GERBIL_DECK_RUNTIME_NATIVE_ABI_ID);
    assert_eq!(readiness.version, GERBIL_DECK_RUNTIME_NATIVE_ABI_VERSION);
    assert_eq!(readiness.status, GraphNativeAbiReadinessStatus::Ready);
    assert_eq!(readiness.required_symbol_count, 2);
    assert_eq!(readiness.available_symbol_count, 2);
    assert_eq!(readiness.matched_symbol_count, 2);
    assert!(readiness.missing_symbols.is_empty());
}

#[test]
fn native_aot_build_receipt_projects_missing_graph_abi_symbols() {
    let receipt = native_build_receipt(
        GerbilDeckRuntimeNativeAotBuildStatus::RequiredSymbolsMissing,
        vec![GerbilDeckRuntimeNativeSymbol::new(
            "marlin_deck_runtime_initialize",
        )],
    );
    let readiness = receipt.graph_native_abi_readiness_receipt();

    assert_eq!(
        readiness.status,
        GraphNativeAbiReadinessStatus::MissingSymbols
    );
    assert_eq!(readiness.available_symbol_count, 1);
    assert_eq!(readiness.matched_symbol_count, 1);
    assert_eq!(readiness.missing_symbols.len(), 1);
    assert_eq!(
        readiness.missing_symbols[0].as_str(),
        "marlin_deck_runtime_initialize"
    );
}

#[test]
fn native_aot_failed_build_never_projects_ready_graph_abi() {
    let receipt = native_build_receipt(
        GerbilDeckRuntimeNativeAotBuildStatus::SymbolAuditFailed,
        Vec::new(),
    );
    let readiness = receipt.graph_native_abi_readiness_receipt();

    assert_eq!(
        readiness.status,
        GraphNativeAbiReadinessStatus::MissingSymbols
    );
    assert_eq!(readiness.available_symbol_count, 0);
    assert_eq!(readiness.matched_symbol_count, 0);
    assert_eq!(readiness.missing_symbols.len(), 2);
}

fn native_build_receipt(
    status: GerbilDeckRuntimeNativeAotBuildStatus,
    missing_symbols: Vec<GerbilDeckRuntimeNativeSymbol>,
) -> GerbilDeckRuntimeNativeAotBuildReceipt {
    GerbilDeckRuntimeNativeAotBuildReceipt {
        status,
        plan: native_aot_plan(),
        detail: None,
        gsc_compile_object: None,
        gsc_generate_link_source: None,
        gsc_compile_link_object: None,
        symbol_audit_method: None,
        symbol_audit: None,
        missing_symbols,
    }
}

fn native_aot_plan() -> GerbilDeckRuntimeNativeAotPlan {
    GerbilDeckRuntimeNativeAotPlan {
        status: GerbilDeckRuntimeNativeAotStatus::ReadyToBuildLinkUnit,
        root: PathBuf::from("/tmp/marlin-native"),
        output_dir: PathBuf::from("/tmp/marlin-native/out"),
        compiled_runtime_scm: PathBuf::from("/tmp/marlin-native/out/runtime.scm"),
        header: PathBuf::from("/tmp/marlin-native/runtime.h"),
        object: PathBuf::from("/tmp/marlin-native/out/runtime.o"),
        link_c_source: PathBuf::from("/tmp/marlin-native/out/runtime_.c"),
        link_object: PathBuf::from("/tmp/marlin-native/out/runtime_.o"),
        exported_symbols: vec![
            GerbilDeckRuntimeNativeSymbol::new("marlin_deck_runtime_initialize"),
            GerbilDeckRuntimeNativeSymbol::new("marlin_deck_runtime_select_model_route"),
        ],
        c_compiler: None,
        symbol_auditor: GerbilNativeSymbolAuditor::new("nm"),
        gambit_link_library: GerbilNativeLinkLibrary::new("gambit"),
        gambit_link_search_dir: None,
        gsc_compile_object: command_plan("gsc"),
        gsc_generate_link_source: command_plan("gsc"),
        gsc_compile_link_object: command_plan("gsc"),
        audit_symbols: command_plan("nm"),
        detail: None,
    }
}

fn command_plan(program: &str) -> GerbilDeckRuntimeNativeAotCommandPlan {
    GerbilDeckRuntimeNativeAotCommandPlan {
        program: PathBuf::from(program),
        args: Vec::new(),
    }
}
