//! First-batch `scenario_benchmark` optimization frontier receipts.

use std::path::Path;

use super::{
    RustScenarioBenchmarkDuration, RustScenarioBenchmarkReceipt, RustScenarioPerformanceCrateName,
    RustScenarioPerformanceDurationMs, RustScenarioPerformanceMemoryBytes,
    RustScenarioPerformanceOptimizationFinding, RustScenarioPerformanceOptimizationFindingKind,
    RustScenarioPerformanceOptimizationPriority, RustScenarioPerformanceOptimizationReceipt,
    RustScenarioPerformanceOptimizationScore, RustScenarioPerformanceOptimizationStatus,
    RustScenarioPerformanceScenarioId, paths::scenario_root, validate_rust_scenario_benchmark,
};

/// Build the first-batch optimization frontier for a crate-local scenario benchmark.
pub fn crate_scenario_performance_optimization_receipt(
    crate_root: &Path,
    crate_name: &str,
) -> RustScenarioPerformanceOptimizationReceipt {
    let receipt = validate_rust_scenario_benchmark(scenario_root(crate_root))
        .expect("validate crate scenario performance benchmark");
    optimization_receipt_from_benchmark_receipt(crate_name, &receipt)
}

/// Assert that first-batch scenario performance optimization anomalies are cleared.
pub fn assert_crate_scenario_performance_first_batch_optimization_frontier_is_clear(
    crate_root: &Path,
    crate_name: &str,
) {
    let receipt = crate_scenario_performance_optimization_receipt(crate_root, crate_name);
    assert_eq!(
        receipt.status,
        RustScenarioPerformanceOptimizationStatus::Healthy,
        "{receipt:?}"
    );
}

/// Build the first-batch optimization frontier from an already validated receipt.
pub fn optimization_receipt_from_benchmark_receipt(
    crate_name: &str,
    receipt: &RustScenarioBenchmarkReceipt,
) -> RustScenarioPerformanceOptimizationReceipt {
    let mut findings = optimization_findings_from_benchmark_receipt(receipt);
    findings.sort_by(|left, right| {
        left.priority
            .cmp(&right.priority)
            .then_with(|| right.optimization_score.cmp(&left.optimization_score))
            .then_with(|| left.scenario_id.cmp(&right.scenario_id))
    });

    let status = if findings.is_empty() {
        RustScenarioPerformanceOptimizationStatus::Healthy
    } else {
        RustScenarioPerformanceOptimizationStatus::ActionRequired
    };

    RustScenarioPerformanceOptimizationReceipt {
        schema_id: "marlin.rust-project-harness.scenario-performance-optimization".to_owned(),
        schema_version: "1".to_owned(),
        crate_name: RustScenarioPerformanceCrateName::new(crate_name),
        scenario_id: RustScenarioPerformanceScenarioId::new(receipt.scenario.id.clone()),
        status,
        findings,
    }
}

fn optimization_findings_from_benchmark_receipt(
    receipt: &RustScenarioBenchmarkReceipt,
) -> Vec<RustScenarioPerformanceOptimizationFinding> {
    let benchmark = &receipt.benchmark;
    let observed_total_ms = duration_millis_u64(benchmark.observed_total);
    let target_total_ms = duration_millis_u64(benchmark.target_total);
    let max_total_ms = duration_millis_u64(benchmark.max_total);
    let regression_budget_ms = duration_millis_u64(benchmark.regression_budget);
    let observed_memory_bytes = benchmark.observed_memory_bytes.as_u64();
    let memory_budget_bytes = benchmark.memory_budget_bytes.as_u64();
    let mut findings = Vec::new();

    if target_total_ms == 25
        && observed_total_ms == 25
        && max_total_ms == 100
        && regression_budget_ms == 20
        && observed_memory_bytes == 4_194_304
        && memory_budget_bytes == 8_388_608
        && benchmark
            .target_rationale
            .contains("Crate-local scenario performance baseline")
    {
        findings.push(optimization_finding(
            receipt,
            RustScenarioPerformanceOptimizationFindingKind::PlaceholderBaseline,
            RustScenarioPerformanceOptimizationPriority::Critical,
            10_000,
            "scenario performance baseline is still the generated placeholder".to_owned(),
            "replace placeholder timing and memory budget with a tuned first-batch baseline"
                .to_owned(),
        ));
    }

    let tuned_max_total_ms = observed_total_ms.saturating_add(regression_budget_ms);
    if max_total_ms > tuned_max_total_ms {
        findings.push(optimization_finding(
            receipt,
            RustScenarioPerformanceOptimizationFindingKind::LooseTotalBudget,
            RustScenarioPerformanceOptimizationPriority::High,
            max_total_ms.saturating_sub(tuned_max_total_ms),
            format!(
                "max_total_ms={max_total_ms} leaves more headroom than observed_total_ms + regression_budget_ms={tuned_max_total_ms}"
            ),
            format!("set max_total_ms to {tuned_max_total_ms} for the first-batch tuned gate"),
        ));
    }

    let tuned_memory_budget_bytes =
        observed_memory_bytes.saturating_add(observed_memory_bytes.saturating_div(2));
    if memory_budget_bytes > tuned_memory_budget_bytes {
        findings.push(optimization_finding(
            receipt,
            RustScenarioPerformanceOptimizationFindingKind::LooseMemoryBudget,
            RustScenarioPerformanceOptimizationPriority::Medium,
            memory_budget_bytes
                .saturating_sub(tuned_memory_budget_bytes)
                .saturating_div(1024),
            format!(
                "memory_budget_bytes={memory_budget_bytes} leaves more than 50% headroom over observed_memory_bytes={observed_memory_bytes}"
            ),
            format!(
                "set memory_budget_bytes to {tuned_memory_budget_bytes} for the first-batch tuned gate"
            ),
        ));
    }

    if benchmark.observed_timings.is_empty() {
        findings.push(optimization_finding(
            receipt,
            RustScenarioPerformanceOptimizationFindingKind::MissingPhaseTiming,
            RustScenarioPerformanceOptimizationPriority::Medium,
            1,
            "observed_timings is empty, so the next optimization cannot identify a hot phase"
                .to_owned(),
            "record at least one phase-level timing entry in benchmark.toml".to_owned(),
        ));
    }

    findings
}

fn optimization_finding(
    receipt: &RustScenarioBenchmarkReceipt,
    kind: RustScenarioPerformanceOptimizationFindingKind,
    priority: RustScenarioPerformanceOptimizationPriority,
    optimization_score: u64,
    problem: String,
    next_action: String,
) -> RustScenarioPerformanceOptimizationFinding {
    RustScenarioPerformanceOptimizationFinding {
        kind,
        priority,
        scenario_id: RustScenarioPerformanceScenarioId::new(receipt.scenario.id.clone()),
        optimization_score: RustScenarioPerformanceOptimizationScore::new(optimization_score),
        problem,
        next_action,
        observed_total_ms: RustScenarioPerformanceDurationMs::new(duration_millis_u64(
            receipt.benchmark.observed_total,
        )),
        target_total_ms: RustScenarioPerformanceDurationMs::new(duration_millis_u64(
            receipt.benchmark.target_total,
        )),
        max_total_ms: RustScenarioPerformanceDurationMs::new(duration_millis_u64(
            receipt.benchmark.max_total,
        )),
        regression_budget_ms: RustScenarioPerformanceDurationMs::new(duration_millis_u64(
            receipt.benchmark.regression_budget,
        )),
        observed_memory_bytes: RustScenarioPerformanceMemoryBytes::new(
            receipt.benchmark.observed_memory_bytes.as_u64(),
        ),
        memory_budget_bytes: RustScenarioPerformanceMemoryBytes::new(
            receipt.benchmark.memory_budget_bytes.as_u64(),
        ),
    }
}

fn duration_millis_u64(duration: RustScenarioBenchmarkDuration) -> u64 {
    u64::try_from(duration.as_duration().as_millis()).unwrap_or(u64::MAX)
}
