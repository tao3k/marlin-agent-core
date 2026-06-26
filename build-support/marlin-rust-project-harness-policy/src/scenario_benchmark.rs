//! Shared scenario benchmark test entrypoints for Marlin crates.

use std::path::{Path, PathBuf};

use rust_lang_project_harness::{
    RustScenarioBenchmarkReceipt, RustScenarioBenchmarkStatus,
    assert_rule_fixture_scenario_benchmarks, render_rust_scenario_benchmark_snapshot,
    validate_required_rust_scenario_benchmarks, validate_rust_scenario_benchmark,
};
use serde::Serialize;

/// Status for a crate-local scenario performance optimization frontier.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RustScenarioPerformanceOptimizationStatus {
    Healthy,
    ActionRequired,
}

/// Priority for one scenario performance optimization finding.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RustScenarioPerformanceOptimizationPriority {
    Critical,
    High,
    Medium,
    Low,
}

/// First-batch anomaly class for scenario performance baselines.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RustScenarioPerformanceOptimizationFindingKind {
    PlaceholderBaseline,
    LooseTotalBudget,
    LooseMemoryBudget,
    MissingPhaseTiming,
}

/// Crate name that owns a scenario performance optimization receipt.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct RustScenarioPerformanceCrateName(String);

impl RustScenarioPerformanceCrateName {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

/// Stable scenario id for an optimization finding.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct RustScenarioPerformanceScenarioId(String);

impl RustScenarioPerformanceScenarioId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

/// Relative score used to order first-batch optimization findings.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct RustScenarioPerformanceOptimizationScore(u64);

impl RustScenarioPerformanceOptimizationScore {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

/// Scenario performance duration in milliseconds.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct RustScenarioPerformanceDurationMs(u64);

impl RustScenarioPerformanceDurationMs {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

/// Scenario performance memory size in bytes.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct RustScenarioPerformanceMemoryBytes(u64);

impl RustScenarioPerformanceMemoryBytes {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

/// One actionable scenario performance optimization finding.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RustScenarioPerformanceOptimizationFinding {
    pub kind: RustScenarioPerformanceOptimizationFindingKind,
    pub priority: RustScenarioPerformanceOptimizationPriority,
    pub scenario_id: RustScenarioPerformanceScenarioId,
    pub optimization_score: RustScenarioPerformanceOptimizationScore,
    pub problem: String,
    pub next_action: String,
    pub observed_total_ms: RustScenarioPerformanceDurationMs,
    pub target_total_ms: RustScenarioPerformanceDurationMs,
    pub max_total_ms: RustScenarioPerformanceDurationMs,
    pub regression_budget_ms: RustScenarioPerformanceDurationMs,
    pub observed_memory_bytes: RustScenarioPerformanceMemoryBytes,
    pub memory_budget_bytes: RustScenarioPerformanceMemoryBytes,
}

/// Optimization frontier compiled from a crate-local scenario performance receipt.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RustScenarioPerformanceOptimizationReceipt {
    pub schema_id: String,
    pub schema_version: String,
    pub crate_name: RustScenarioPerformanceCrateName,
    pub scenario_id: RustScenarioPerformanceScenarioId,
    pub status: RustScenarioPerformanceOptimizationStatus,
    pub findings: Vec<RustScenarioPerformanceOptimizationFinding>,
}

/// Assert that a crate-local scenario performance baseline receipt is stable.
pub fn assert_crate_scenario_performance_baseline_receipt_is_stable(
    crate_root: &Path,
    crate_name: &str,
) {
    let receipt = validate_rust_scenario_benchmark(scenario_root(crate_root))
        .expect("validate crate scenario performance benchmark");

    assert_eq!(receipt.status, RustScenarioBenchmarkStatus::Pass);
    assert!(receipt.violations.is_empty(), "{:?}", receipt.violations);
    assert_eq!(
        receipt.scenario.id,
        format!("{crate_name}.scenario-performance")
    );
    assert!(receipt.benchmark.observed_total_ms <= receipt.benchmark.max_total_ms);
    assert!(receipt.benchmark.observed_memory_bytes <= receipt.benchmark.memory_budget_bytes);

    let snapshot = render_rust_scenario_benchmark_snapshot(&receipt);
    assert!(snapshot.contains("status: pass"));
    assert!(snapshot.contains("observed_total_ms: <measured>"));
    assert!(snapshot.contains("observed_memory_bytes: <measured>"));
    assert!(snapshot.contains("timings: fixture_ms=<measured>"));
}

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

/// Assert that all required crate-local scenario benchmarks pass the harness gate.
pub fn assert_crate_scenario_performance_contract_gate_accepts_crate_scenarios(crate_root: &Path) {
    let receipt = validate_required_rust_scenario_benchmarks(crate_root)
        .expect("validate required crate scenario benchmarks");

    assert_eq!(receipt.status, RustScenarioBenchmarkStatus::Pass);
    assert!(receipt.violations.is_empty(), "{:?}", receipt.violations);
    assert!(receipt.requirements.iter().any(|requirement| {
        requirement
            .root
            .strip_prefix(crate_root)
            .map(|path| path == Path::new("tests/unit/scenarios/performance_baseline"))
            .unwrap_or(false)
    }));

    assert_rule_fixture_scenario_benchmarks(crate_root);
}

fn scenario_root(crate_root: &Path) -> PathBuf {
    crate_root
        .join("tests")
        .join("unit")
        .join("scenarios")
        .join("performance_baseline")
}

fn optimization_findings_from_benchmark_receipt(
    receipt: &RustScenarioBenchmarkReceipt,
) -> Vec<RustScenarioPerformanceOptimizationFinding> {
    let benchmark = &receipt.benchmark;
    let observed_total_ms = benchmark.observed_total_ms.as_u64();
    let target_total_ms = benchmark.target_total_ms.as_u64();
    let max_total_ms = benchmark.max_total_ms.as_u64();
    let regression_budget_ms = benchmark.regression_budget_ms.as_u64();
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
        observed_total_ms: RustScenarioPerformanceDurationMs::new(
            receipt.benchmark.observed_total_ms.as_u64(),
        ),
        target_total_ms: RustScenarioPerformanceDurationMs::new(
            receipt.benchmark.target_total_ms.as_u64(),
        ),
        max_total_ms: RustScenarioPerformanceDurationMs::new(
            receipt.benchmark.max_total_ms.as_u64(),
        ),
        regression_budget_ms: RustScenarioPerformanceDurationMs::new(
            receipt.benchmark.regression_budget_ms.as_u64(),
        ),
        observed_memory_bytes: RustScenarioPerformanceMemoryBytes::new(
            receipt.benchmark.observed_memory_bytes.as_u64(),
        ),
        memory_budget_bytes: RustScenarioPerformanceMemoryBytes::new(
            receipt.benchmark.memory_budget_bytes.as_u64(),
        ),
    }
}

/// Generate the standard crate-local scenario performance benchmark tests.
#[macro_export]
macro_rules! scenario_performance_tests {
    () => {
        #[test]
        fn scenario_performance_baseline_receipt_is_stable() {
            $crate::assert_crate_scenario_performance_baseline_receipt_is_stable(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")),
                env!("CARGO_PKG_NAME"),
            );
        }

        #[test]
        fn scenario_performance_contract_gate_accepts_crate_scenarios() {
            $crate::assert_crate_scenario_performance_contract_gate_accepts_crate_scenarios(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")),
            );
        }

        #[test]
        fn scenario_performance_first_batch_optimization_frontier_is_clear() {
            $crate::assert_crate_scenario_performance_first_batch_optimization_frontier_is_clear(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")),
                env!("CARGO_PKG_NAME"),
            );
        }
    };
}
