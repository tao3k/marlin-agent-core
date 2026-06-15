use marlin_gerbil_scheme::{
    GERBIL_DECK_RUNTIME_POO_POLICY_PROJECTION_SCHEMA_ID,
    GERBIL_DECK_RUNTIME_POO_POLICY_PROJECTION_TYPE_ID,
    GERBIL_DECK_RUNTIME_SCRIPT_BATCH_DEFAULT_ITERATIONS,
    GERBIL_DECK_RUNTIME_SCRIPT_BATCH_MAX_ELAPSED_US,
    GERBIL_DECK_RUNTIME_SCRIPT_BATCH_METRICS_SCHEMA_ID,
    GERBIL_DECK_RUNTIME_SCRIPT_BATCH_METRICS_TYPE_ID, GERBIL_DECK_RUNTIME_SCRIPT_INTERFACE_KIND,
    GERBIL_DECK_RUNTIME_SCRIPT_INTERFACE_RECEIPT_SCHEMA_ID,
    GERBIL_DECK_RUNTIME_SCRIPT_INTERFACE_RECEIPT_TYPE_ID, GERBIL_DECK_RUNTIME_SCRIPT_KIND,
    GERBIL_MARLIN_DECK_RUNTIME_SCRIPT_SOURCE, GerbilDeckRuntimeScriptBatchPerformanceBudget,
    GerbilDeckRuntimeScriptBatchPerformanceStatus, GerbilSchemeSchemaId, GerbilSchemeTypeId,
    GerbilSchemeTypeRegistry, GerbilSchemeTypedValue, GerbilSchemeValue,
    decode_gerbil_deck_runtime_script_batch_metrics,
    decode_gerbil_deck_runtime_script_interface_receipt,
    evaluate_gerbil_deck_runtime_script_batch_performance,
    gerbil_deck_runtime_script_interface_type_manifest,
};
use std::time::Instant;

#[test]
fn gerbil_deck_runtime_script_contract_matches_scheme_surface() {
    let manifest = gerbil_deck_runtime_script_interface_type_manifest();
    let registry = GerbilSchemeTypeRegistry::new(manifest)
        .expect("script interface type manifest should validate");

    assert!(
        registry
            .type_spec(&GerbilSchemeTypeId::new(
                GERBIL_DECK_RUNTIME_SCRIPT_INTERFACE_RECEIPT_TYPE_ID
            ))
            .is_some()
    );
    assert!(
        registry
            .type_spec(&GerbilSchemeTypeId::new(
                GERBIL_DECK_RUNTIME_SCRIPT_BATCH_METRICS_TYPE_ID
            ))
            .is_some()
    );
    assert!(
        registry
            .type_spec(&GerbilSchemeTypeId::new(
                GERBIL_DECK_RUNTIME_POO_POLICY_PROJECTION_TYPE_ID
            ))
            .is_some()
    );

    for expected in [
        GERBIL_DECK_RUNTIME_SCRIPT_KIND,
        GERBIL_DECK_RUNTIME_SCRIPT_INTERFACE_KIND,
        GERBIL_DECK_RUNTIME_SCRIPT_INTERFACE_RECEIPT_SCHEMA_ID,
        GERBIL_DECK_RUNTIME_SCRIPT_BATCH_METRICS_SCHEMA_ID,
        "defmarlin-deck-runtime-script",
        "marlin-deck-runtime-script-interface-receipt",
        "marlin-deck-runtime-script-batch-metrics",
    ] {
        assert!(
            GERBIL_MARLIN_DECK_RUNTIME_SCRIPT_SOURCE.contains(expected),
            "script Scheme source should contain {expected}"
        );
    }
    assert!(
        !GERBIL_MARLIN_DECK_RUNTIME_SCRIPT_SOURCE.contains("json"),
        "script Scheme interface should not own JSON serialization"
    );
}

#[test]
fn gerbil_deck_runtime_script_interface_receipt_decodes_native_projection() {
    let registry =
        GerbilSchemeTypeRegistry::new(gerbil_deck_runtime_script_interface_type_manifest())
            .expect("script interface type manifest should validate");
    let receipt = decode_gerbil_deck_runtime_script_interface_receipt(
        &registry,
        &script_interface_typed_value(),
    )
    .expect("script interface receipt should decode");

    assert_eq!(
        receipt.kind.as_str(),
        GERBIL_DECK_RUNTIME_SCRIPT_INTERFACE_RECEIPT_SCHEMA_ID
    );
    assert_eq!(receipt.script_id.as_str(), "downstream-ui-script");
    assert_eq!(
        receipt.interface.as_str(),
        GERBIL_DECK_RUNTIME_SCRIPT_INTERFACE_KIND
    );
    assert_eq!(receipt.action.as_str(), "register");
    assert_eq!(receipt.extension_id.as_str(), "downstream-ui-extension");
    assert_eq!(
        receipt
            .metadata
            .get("entry")
            .and_then(GerbilSchemeValue::as_text),
        Some("user-interface")
    );
    assert_eq!(
        receipt.native_projection().schema_id,
        GERBIL_DECK_RUNTIME_POO_POLICY_PROJECTION_SCHEMA_ID
    );
    assert_eq!(
        receipt.native_projection().policy_id,
        "downstream-ui-script"
    );
}

#[test]
fn gerbil_deck_runtime_script_batch_performance_budget_is_rust_owned() {
    let registry =
        GerbilSchemeTypeRegistry::new(gerbil_deck_runtime_script_interface_type_manifest())
            .expect("script interface type manifest should validate");
    let budget = GerbilDeckRuntimeScriptBatchPerformanceBudget::default();

    let metrics = decode_gerbil_deck_runtime_script_batch_metrics(
        &registry,
        &script_batch_metrics_typed_value(
            GERBIL_DECK_RUNTIME_SCRIPT_BATCH_DEFAULT_ITERATIONS,
            GERBIL_DECK_RUNTIME_SCRIPT_BATCH_DEFAULT_ITERATIONS,
            GERBIL_DECK_RUNTIME_SCRIPT_BATCH_MAX_ELAPSED_US,
        ),
    )
    .expect("script batch metrics should decode");
    let receipt = evaluate_gerbil_deck_runtime_script_batch_performance(metrics, &budget);
    assert_eq!(
        receipt.status,
        GerbilDeckRuntimeScriptBatchPerformanceStatus::WithinBudget
    );
    assert!(receipt.within_budget());

    let over_budget = decode_gerbil_deck_runtime_script_batch_metrics(
        &registry,
        &script_batch_metrics_typed_value(
            GERBIL_DECK_RUNTIME_SCRIPT_BATCH_DEFAULT_ITERATIONS,
            GERBIL_DECK_RUNTIME_SCRIPT_BATCH_DEFAULT_ITERATIONS,
            GERBIL_DECK_RUNTIME_SCRIPT_BATCH_MAX_ELAPSED_US + 1,
        ),
    )
    .expect("script batch metrics should decode");
    let over_budget_receipt =
        evaluate_gerbil_deck_runtime_script_batch_performance(over_budget, &budget);
    assert_eq!(
        over_budget_receipt.status,
        GerbilDeckRuntimeScriptBatchPerformanceStatus::OverBudget
    );

    let under_sampled = decode_gerbil_deck_runtime_script_batch_metrics(
        &registry,
        &script_batch_metrics_typed_value(
            GERBIL_DECK_RUNTIME_SCRIPT_BATCH_DEFAULT_ITERATIONS - 1,
            GERBIL_DECK_RUNTIME_SCRIPT_BATCH_DEFAULT_ITERATIONS - 1,
            1,
        ),
    )
    .expect("script batch metrics should decode");
    let under_sampled_receipt =
        evaluate_gerbil_deck_runtime_script_batch_performance(under_sampled, &budget);
    assert_eq!(
        under_sampled_receipt.status,
        GerbilDeckRuntimeScriptBatchPerformanceStatus::UnderSampled
    );
}

#[test]
fn gerbil_deck_runtime_script_batch_projection_gate_stays_in_process() {
    let registry =
        GerbilSchemeTypeRegistry::new(gerbil_deck_runtime_script_interface_type_manifest())
            .expect("script interface type manifest should validate");
    let budget = GerbilDeckRuntimeScriptBatchPerformanceBudget::default();
    let iterations = 2_000;
    let typed_value = script_batch_metrics_typed_value(
        GERBIL_DECK_RUNTIME_SCRIPT_BATCH_DEFAULT_ITERATIONS,
        GERBIL_DECK_RUNTIME_SCRIPT_BATCH_DEFAULT_ITERATIONS,
        1,
    );

    let started = Instant::now();
    for _ in 0..iterations {
        let metrics = decode_gerbil_deck_runtime_script_batch_metrics(&registry, &typed_value)
            .expect("script batch metrics should decode in the Rust projection gate");
        let receipt = evaluate_gerbil_deck_runtime_script_batch_performance(metrics, &budget);
        assert!(receipt.within_budget());
    }
    let elapsed = started.elapsed();

    assert!(
        elapsed.as_secs_f64() < 3.0,
        "script batch projection gate exceeded in-process budget: {elapsed:?} for {iterations} iterations"
    );
}

fn script_interface_typed_value() -> GerbilSchemeTypedValue {
    GerbilSchemeTypedValue::new(
        GerbilSchemeTypeId::new(GERBIL_DECK_RUNTIME_SCRIPT_INTERFACE_RECEIPT_TYPE_ID),
        GerbilSchemeValue::record([
            (
                "kind",
                GERBIL_DECK_RUNTIME_SCRIPT_INTERFACE_RECEIPT_SCHEMA_ID.into(),
            ),
            ("script-id", "downstream-ui-script".into()),
            (
                "interface",
                GERBIL_DECK_RUNTIME_SCRIPT_INTERFACE_KIND.into(),
            ),
            ("action", "register".into()),
            ("extension-id", "downstream-ui-extension".into()),
            (
                "metadata",
                GerbilSchemeValue::record([
                    ("owner", "downstream".into()),
                    ("entry", "user-interface".into()),
                ]),
            ),
            ("native-projection", poo_policy_projection_value()),
        ]),
    )
    .with_schema_id(GerbilSchemeSchemaId::new(
        GERBIL_DECK_RUNTIME_SCRIPT_INTERFACE_RECEIPT_SCHEMA_ID,
    ))
}

fn script_batch_metrics_typed_value(
    iterations: u64,
    runs: u64,
    elapsed_us: u64,
) -> GerbilSchemeTypedValue {
    GerbilSchemeTypedValue::new(
        GerbilSchemeTypeId::new(GERBIL_DECK_RUNTIME_SCRIPT_BATCH_METRICS_TYPE_ID),
        GerbilSchemeValue::record([
            (
                "kind",
                GERBIL_DECK_RUNTIME_SCRIPT_BATCH_METRICS_SCHEMA_ID.into(),
            ),
            ("script-id", "performance-script".into()),
            (
                "interface",
                GERBIL_DECK_RUNTIME_SCRIPT_INTERFACE_KIND.into(),
            ),
            ("iterations", GerbilSchemeValue::integer(iterations as i64)),
            ("runs", GerbilSchemeValue::integer(runs as i64)),
            ("elapsed-us", GerbilSchemeValue::integer(elapsed_us as i64)),
        ]),
    )
    .with_schema_id(GerbilSchemeSchemaId::new(
        GERBIL_DECK_RUNTIME_SCRIPT_BATCH_METRICS_SCHEMA_ID,
    ))
}

fn poo_policy_projection_value() -> GerbilSchemeValue {
    GerbilSchemeValue::record([
        (
            "schema_id",
            GERBIL_DECK_RUNTIME_POO_POLICY_PROJECTION_SCHEMA_ID.into(),
        ),
        ("policy_id", "downstream-ui-script".into()),
        ("object_system", "gerbil-poo".into()),
        ("package", "marlin".into()),
        ("module", "deck-runtime-script".into()),
        ("action", "register".into()),
    ])
}
