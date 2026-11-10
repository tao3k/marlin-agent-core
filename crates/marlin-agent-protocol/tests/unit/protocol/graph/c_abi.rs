use marlin_agent_protocol::{
    GraphCAbiReadinessReceipt, GraphCAbiReadinessStatus, GraphCAbiRequirement,
};

#[test]
fn graph_c_abi_readiness_receipt_reports_symbol_coverage() {
    let requirement =
        GraphCAbiRequirement::new("marlin-deck-route-abi", 1).with_required_symbols([
            "marlin_deck_runtime_select_model_route",
            "marlin_deck_runtime_abi_version",
        ]);

    let ready = GraphCAbiReadinessReceipt::evaluate(
        &requirement,
        [
            "marlin_deck_runtime_select_model_route",
            "marlin_deck_runtime_abi_version",
            "marlin_deck_runtime_extra_symbol",
        ],
    );

    assert_eq!(ready.status, GraphCAbiReadinessStatus::Ready);
    assert_eq!(ready.required_symbol_count, 2);
    assert_eq!(ready.available_symbol_count, 3);
    assert_eq!(ready.matched_symbol_count, 2);
    assert!(ready.missing_symbols.is_empty());

    let missing = GraphCAbiReadinessReceipt::evaluate(
        &requirement,
        ["marlin_deck_runtime_select_model_route"],
    );

    assert_eq!(missing.status, GraphCAbiReadinessStatus::MissingSymbols);
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
