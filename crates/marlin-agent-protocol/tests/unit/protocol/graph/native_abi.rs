use marlin_agent_protocol::{
    GraphNativeAbiReadinessReceipt, GraphNativeAbiReadinessStatus, GraphNativeAbiRequirement,
};

#[test]
fn graph_native_abi_readiness_receipt_reports_symbol_coverage() {
    let requirement = GraphNativeAbiRequirement::new("marlin-deck-runtime-native", 1)
        .with_required_symbols([
            "marlin_deck_runtime_select_model_route",
            "marlin_deck_runtime_abi_version",
        ]);

    let ready = GraphNativeAbiReadinessReceipt::evaluate(
        &requirement,
        [
            "marlin_deck_runtime_select_model_route",
            "marlin_deck_runtime_abi_version",
            "marlin_deck_runtime_extra_symbol",
        ],
    );

    assert_eq!(ready.status, GraphNativeAbiReadinessStatus::Ready);
    assert_eq!(ready.required_symbol_count, 2);
    assert_eq!(ready.available_symbol_count, 3);
    assert_eq!(ready.matched_symbol_count, 2);
    assert!(ready.missing_symbols.is_empty());

    let missing = GraphNativeAbiReadinessReceipt::evaluate(
        &requirement,
        ["marlin_deck_runtime_select_model_route"],
    );

    assert_eq!(
        missing.status,
        GraphNativeAbiReadinessStatus::MissingSymbols
    );
    assert_eq!(missing.required_symbol_count, 2);
    assert_eq!(missing.available_symbol_count, 1);
    assert_eq!(missing.matched_symbol_count, 1);
    assert_eq!(
        missing
            .missing_symbols
            .iter()
            .map(|symbol| symbol.as_str())
            .collect::<Vec<_>>(),
        vec!["marlin_deck_runtime_abi_version"]
    );
}
