use std::{collections::BTreeMap, path::PathBuf};

use marlin_rust_project_harness_policy::{
    RustScenarioPerformanceOptimizationFindingKind, RustScenarioPerformanceOptimizationPriority,
    RustScenarioPerformanceOptimizationStatus, optimization_receipt_from_benchmark_receipt,
};
use rust_lang_project_harness::{
    RustScenarioBenchmarkContract, RustScenarioBenchmarkDurationMs,
    RustScenarioBenchmarkMemoryBytes, RustScenarioBenchmarkReceipt, RustScenarioBenchmarkStatus,
    RustScenarioMetadata,
};

#[test]
fn placeholder_baseline_produces_first_batch_optimization_findings() {
    let receipt = optimization_receipt_from_benchmark_receipt(
        "marlin-example",
        &benchmark_receipt(100, 8_388_608, "Crate-local scenario performance baseline"),
    );

    assert_eq!(
        receipt.status,
        RustScenarioPerformanceOptimizationStatus::ActionRequired
    );
    assert_eq!(receipt.findings.len(), 3);
    assert_eq!(
        receipt.findings[0].kind,
        RustScenarioPerformanceOptimizationFindingKind::PlaceholderBaseline
    );
    assert_eq!(
        receipt.findings[0].priority,
        RustScenarioPerformanceOptimizationPriority::Critical
    );
    assert_eq!(
        receipt.findings[1].kind,
        RustScenarioPerformanceOptimizationFindingKind::LooseTotalBudget
    );
    assert_eq!(
        receipt.findings[2].kind,
        RustScenarioPerformanceOptimizationFindingKind::LooseMemoryBudget
    );
}

#[test]
fn tuned_first_batch_baseline_has_clear_optimization_frontier() {
    let receipt = optimization_receipt_from_benchmark_receipt(
        "marlin-example",
        &benchmark_receipt(45, 6_291_456, "First-batch tuned scenario baseline"),
    );

    assert_eq!(
        receipt.status,
        RustScenarioPerformanceOptimizationStatus::Healthy
    );
    assert!(receipt.findings.is_empty());
}

fn benchmark_receipt(
    max_total_ms: u64,
    memory_budget_bytes: u64,
    rationale: &str,
) -> RustScenarioBenchmarkReceipt {
    RustScenarioBenchmarkReceipt {
        root: PathBuf::from("tests/unit/scenarios/performance_baseline"),
        scenario: RustScenarioMetadata {
            id: "marlin-example.scenario-performance".to_owned(),
            title: "example scenario performance baseline".to_owned(),
            policy_ids: vec!["MARLIN-SCENARIO-PERFORMANCE".to_owned()],
            agent_goal: "Keep scenario performance contracts stable and replayable.".to_owned(),
            inputs: "inputs".to_owned(),
            expected: "expected".to_owned(),
        },
        benchmark: RustScenarioBenchmarkContract {
            bench_command: "cargo test -p marlin-example --test unit_test scenario_performance"
                .to_owned(),
            target_total_ms: RustScenarioBenchmarkDurationMs(25),
            max_total_ms: RustScenarioBenchmarkDurationMs(max_total_ms),
            observed_total_ms: RustScenarioBenchmarkDurationMs(25),
            regression_budget_ms: RustScenarioBenchmarkDurationMs(20),
            memory_budget_bytes: RustScenarioBenchmarkMemoryBytes(memory_budget_bytes),
            observed_memory_bytes: RustScenarioBenchmarkMemoryBytes(4_194_304),
            target_rationale: rationale.to_owned(),
            observed_timings: BTreeMap::from([(
                "fixture_ms".to_owned(),
                RustScenarioBenchmarkDurationMs(25),
            )]),
        },
        status: RustScenarioBenchmarkStatus::Pass,
        violations: Vec::new(),
    }
}
