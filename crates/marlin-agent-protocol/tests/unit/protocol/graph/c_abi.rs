// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use marlin_agent_protocol::{
    GraphCAbiReadinessReceipt, GraphCAbiReadinessStatus, GraphCAbiRequirement,
};

#[test]
fn graph_c_abi_readiness_receipt_reports_symbol_coverage() {
    let requirement =
        GraphCAbiRequirement::new("marlin-deck-route-abi", 1).with_required_symbols([
            "marlin_deck_runtime_evaluate",
            "marlin_deck_runtime_abi_version",
        ]);

    let ready = GraphCAbiReadinessReceipt::evaluate(
        &requirement,
        [
            "marlin_deck_runtime_evaluate",
            "marlin_deck_runtime_abi_version",
            "marlin_deck_runtime_extra_symbol",
        ],
    );

    assert_eq!(ready.status, GraphCAbiReadinessStatus::Ready);
    assert_eq!(ready.required_symbol_count, 2);
    assert_eq!(ready.available_symbol_count, 3);
    assert_eq!(ready.matched_symbol_count, 2);
    assert_eq!(ready.required_symbols, requirement.required_symbols);
    assert!(ready.missing_symbols.is_empty());

    let missing =
        GraphCAbiReadinessReceipt::evaluate(&requirement, ["marlin_deck_runtime_evaluate"]);

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

#[test]
fn readiness_encoding_requires_the_exact_symbol_requirement() {
    let requirement = GraphCAbiRequirement::new("abi", 1).with_required_symbols(["a", "b"]);
    let receipt = GraphCAbiReadinessReceipt::evaluate(&requirement, ["a", "b"]);
    let mut value = serde_json::to_value(&receipt).unwrap();
    let decoded: GraphCAbiReadinessReceipt = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(decoded, receipt);
    value.as_object_mut().unwrap().remove("required_symbols");
    assert!(serde_json::from_value::<GraphCAbiReadinessReceipt>(value).is_err());
}
