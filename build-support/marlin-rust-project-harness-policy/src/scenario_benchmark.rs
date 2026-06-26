//! Shared scenario benchmark test entrypoints for Marlin crates.

use std::{
    fs,
    path::{Path, PathBuf},
};

pub use rust_lang_project_harness::{
    RustScenarioBenchmarkDuration, RustScenarioBenchmarkReceipt, RustScenarioBenchmarkStatus,
    render_rust_scenario_benchmark_snapshot, validate_rust_scenario_benchmark,
};
use rust_lang_project_harness::{
    assert_rule_fixture_scenario_benchmarks, validate_required_rust_scenario_benchmarks,
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

/// Workspace-wide scenario benchmark gate status.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RustScenarioWorkspaceBenchmarkStatus {
    Pass,
    Fail,
}

/// Actionable workspace-wide improvement class.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RustScenarioWorkspaceBenchmarkImprovementKind {
    InvalidScenarioBenchmarkSchema,
    ScenarioPerformanceOptimization,
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

/// One crate-local result inside the workspace scenario benchmark gate.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RustScenarioWorkspaceBenchmarkCrateReceipt {
    pub crate_name: RustScenarioPerformanceCrateName,
    pub crate_root: PathBuf,
    pub status: RustScenarioWorkspaceBenchmarkStatus,
    pub required_scenario_count: usize,
    pub violation_count: usize,
    pub optimization_status: Option<RustScenarioPerformanceOptimizationStatus>,
    pub optimization_finding_count: usize,
    pub error: Option<String>,
    pub improvements: Vec<RustScenarioWorkspaceBenchmarkImprovementSuggestion>,
}

/// One actionable suggestion compiled from benchmark validation/reflection.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RustScenarioWorkspaceBenchmarkImprovementSuggestion {
    pub kind: RustScenarioWorkspaceBenchmarkImprovementKind,
    pub priority: RustScenarioPerformanceOptimizationPriority,
    pub crate_name: RustScenarioPerformanceCrateName,
    pub crate_root: PathBuf,
    pub scenario_id: Option<RustScenarioPerformanceScenarioId>,
    pub problem: String,
    pub next_action: String,
    pub verification_command: String,
    pub evidence: Vec<String>,
}

/// Workspace-wide scenario benchmark gate receipt.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RustScenarioWorkspaceBenchmarkGateReceipt {
    pub schema_id: String,
    pub schema_version: String,
    pub workspace_root: PathBuf,
    pub status: RustScenarioWorkspaceBenchmarkStatus,
    pub crate_count: usize,
    pub passed_crate_count: usize,
    pub failed_crate_count: usize,
    pub improvement_count: usize,
    pub crates: Vec<RustScenarioWorkspaceBenchmarkCrateReceipt>,
    pub improvements: Vec<RustScenarioWorkspaceBenchmarkImprovementSuggestion>,
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
    assert!(receipt.benchmark.observed_total <= receipt.benchmark.max_total);
    assert!(receipt.benchmark.observed_memory_bytes <= receipt.benchmark.memory_budget_bytes);

    let snapshot = render_rust_scenario_benchmark_snapshot(&receipt);
    assert!(snapshot.contains("status: pass"));
    assert!(snapshot.contains("observed_total: <measured>"));
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

/// Build the workspace-wide gate receipt for all first-party crates under `crates/`.
pub fn validate_workspace_required_scenario_benchmarks(
    workspace_root: &Path,
) -> RustScenarioWorkspaceBenchmarkGateReceipt {
    let workspace_root = workspace_root.to_path_buf();
    let mut crate_receipts = workspace_crate_members(&workspace_root)
        .into_iter()
        .map(|crate_root| validate_workspace_crate_scenario_benchmark(&workspace_root, crate_root))
        .collect::<Vec<_>>();

    crate_receipts.sort_by(|left, right| left.crate_name.cmp(&right.crate_name));

    let improvements = crate_receipts
        .iter()
        .flat_map(|receipt| receipt.improvements.iter().cloned())
        .collect::<Vec<_>>();
    let failed_crate_count = crate_receipts
        .iter()
        .filter(|receipt| receipt.status == RustScenarioWorkspaceBenchmarkStatus::Fail)
        .count();
    let passed_crate_count = crate_receipts.len().saturating_sub(failed_crate_count);
    let status = if failed_crate_count == 0 && improvements.is_empty() {
        RustScenarioWorkspaceBenchmarkStatus::Pass
    } else {
        RustScenarioWorkspaceBenchmarkStatus::Fail
    };

    RustScenarioWorkspaceBenchmarkGateReceipt {
        schema_id: "marlin.rust-project-harness.workspace-scenario-benchmark-gate".to_owned(),
        schema_version: "1".to_owned(),
        workspace_root,
        status,
        crate_count: crate_receipts.len(),
        passed_crate_count,
        failed_crate_count,
        improvement_count: improvements.len(),
        crates: crate_receipts,
        improvements,
    }
}

/// Alias for callers that use the gate as a reflection/optimization receipt.
pub fn workspace_scenario_performance_optimization_receipt(
    workspace_root: &Path,
) -> RustScenarioWorkspaceBenchmarkGateReceipt {
    validate_workspace_required_scenario_benchmarks(workspace_root)
}

/// Assert that every first-party crate uses the upstream scenario benchmark schema.
pub fn assert_workspace_scenario_performance_contract_gate_accepts_all_crates(
    workspace_root: &Path,
) {
    let receipt = validate_workspace_required_scenario_benchmarks(workspace_root);
    assert_ne!(receipt.crate_count, 0, "{receipt:?}");
    assert_eq!(
        receipt.status,
        RustScenarioWorkspaceBenchmarkStatus::Pass,
        "{receipt:?}"
    );
    assert!(receipt.improvements.is_empty(), "{receipt:?}");
}

fn scenario_root(crate_root: &Path) -> PathBuf {
    crate_root
        .join("tests")
        .join("unit")
        .join("scenarios")
        .join("performance_baseline")
}

fn validate_workspace_crate_scenario_benchmark(
    workspace_root: &Path,
    crate_root: PathBuf,
) -> RustScenarioWorkspaceBenchmarkCrateReceipt {
    let crate_name = workspace_crate_name(&crate_root);
    match validate_required_rust_scenario_benchmarks(&crate_root) {
        Ok(required_receipt) => {
            let mut improvements = Vec::new();
            if required_receipt.status != RustScenarioBenchmarkStatus::Pass
                || !required_receipt.violations.is_empty()
            {
                let schema_error = if required_receipt.violations.is_empty() {
                    format!(
                        "status={:?}; required_scenario_count={}",
                        required_receipt.status,
                        required_receipt.requirements.len()
                    )
                } else {
                    format!("{:?}", required_receipt.violations)
                };
                improvements.push(invalid_schema_suggestion(
                    &crate_root,
                    crate_name.as_str(),
                    schema_error,
                ));
            }

            let optimization_receipt = validate_rust_scenario_benchmark(scenario_root(&crate_root))
                .ok()
                .map(|receipt| {
                    optimization_receipt_from_benchmark_receipt(crate_name.as_str(), &receipt)
                });

            if let Some(optimization_receipt) = &optimization_receipt {
                improvements.extend(
                    optimization_receipt
                        .findings
                        .iter()
                        .map(|finding| optimization_suggestion_from_finding(&crate_root, finding)),
                );
            }

            let status = if required_receipt.status == RustScenarioBenchmarkStatus::Pass
                && required_receipt.violations.is_empty()
                && improvements.is_empty()
            {
                RustScenarioWorkspaceBenchmarkStatus::Pass
            } else {
                RustScenarioWorkspaceBenchmarkStatus::Fail
            };

            RustScenarioWorkspaceBenchmarkCrateReceipt {
                crate_name: RustScenarioPerformanceCrateName::new(crate_name),
                crate_root: crate_root
                    .strip_prefix(workspace_root)
                    .unwrap_or(crate_root.as_path())
                    .to_path_buf(),
                status,
                required_scenario_count: required_receipt.requirements.len(),
                violation_count: required_receipt.violations.len(),
                optimization_status: optimization_receipt.map(|receipt| receipt.status),
                optimization_finding_count: improvements.len(),
                error: None,
                improvements,
            }
        }
        Err(error) => {
            let suggestion = invalid_schema_suggestion(&crate_root, crate_name.as_str(), error);
            RustScenarioWorkspaceBenchmarkCrateReceipt {
                crate_name: RustScenarioPerformanceCrateName::new(crate_name),
                crate_root: crate_root
                    .strip_prefix(workspace_root)
                    .unwrap_or(crate_root.as_path())
                    .to_path_buf(),
                status: RustScenarioWorkspaceBenchmarkStatus::Fail,
                required_scenario_count: 0,
                violation_count: 1,
                optimization_status: None,
                optimization_finding_count: 0,
                error: Some(suggestion.problem.clone()),
                improvements: vec![suggestion],
            }
        }
    }
}

fn workspace_crate_members(workspace_root: &Path) -> Vec<PathBuf> {
    let manifest = read_toml(workspace_root.join("Cargo.toml"));
    let Some(members) = manifest
        .get("workspace")
        .and_then(|workspace| workspace.get("members"))
        .and_then(|members| members.as_array())
    else {
        return Vec::new();
    };

    let mut crate_roots = members
        .iter()
        .filter_map(|member| member.as_str())
        .flat_map(|member| resolve_workspace_member(workspace_root, member))
        .filter(|path| {
            path.strip_prefix(workspace_root)
                .is_ok_and(is_product_crate_member)
        })
        .filter(|path| path.join("Cargo.toml").is_file())
        .collect::<Vec<_>>();
    crate_roots.sort();
    crate_roots.dedup();
    crate_roots
}

fn resolve_workspace_member(workspace_root: &Path, member: &str) -> Vec<PathBuf> {
    if let Some(parent) = member.strip_suffix("/*") {
        let parent = workspace_root.join(parent);
        return fs::read_dir(parent)
            .ok()
            .into_iter()
            .flat_map(|entries| entries.filter_map(Result::ok))
            .map(|entry| entry.path())
            .filter(|path| path.is_dir())
            .collect();
    }

    vec![workspace_root.join(member)]
}

fn is_product_crate_member(path: &Path) -> bool {
    path.starts_with("crates")
}

fn workspace_crate_name(crate_root: &Path) -> String {
    read_toml(crate_root.join("Cargo.toml"))
        .get("package")
        .and_then(|package| package.get("name"))
        .and_then(|name| name.as_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| {
            panic!(
                "crate Cargo.toml at {} must define [package].name",
                crate_root.display()
            )
        })
}

fn read_toml(path: PathBuf) -> toml::Value {
    let content = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    toml::from_str::<toml::Value>(&content)
        .unwrap_or_else(|error| panic!("parse {}: {error}", path.display()))
}

fn invalid_schema_suggestion(
    crate_root: &Path,
    crate_name: &str,
    error: impl std::fmt::Display,
) -> RustScenarioWorkspaceBenchmarkImprovementSuggestion {
    RustScenarioWorkspaceBenchmarkImprovementSuggestion {
        kind: RustScenarioWorkspaceBenchmarkImprovementKind::InvalidScenarioBenchmarkSchema,
        priority: RustScenarioPerformanceOptimizationPriority::Critical,
        crate_name: RustScenarioPerformanceCrateName::new(crate_name),
        crate_root: crate_root.to_path_buf(),
        scenario_id: None,
        problem: format!("crate scenario benchmark does not satisfy upstream schema: {error}"),
        next_action:
            "migrate benchmark.toml to the upstream rust-lang-project-harness scenario schema"
                .to_owned(),
        verification_command: format!(
            "cargo test -p {crate_name} scenario_performance_contract_gate_accepts_crate_scenarios"
        ),
        evidence: vec![
            scenario_root(crate_root)
                .join("benchmark.toml")
                .display()
                .to_string(),
        ],
    }
}

fn optimization_suggestion_from_finding(
    crate_root: &Path,
    finding: &RustScenarioPerformanceOptimizationFinding,
) -> RustScenarioWorkspaceBenchmarkImprovementSuggestion {
    let crate_name = workspace_crate_name(crate_root);
    RustScenarioWorkspaceBenchmarkImprovementSuggestion {
        kind: RustScenarioWorkspaceBenchmarkImprovementKind::ScenarioPerformanceOptimization,
        priority: finding.priority,
        crate_name: RustScenarioPerformanceCrateName::new(crate_name.clone()),
        crate_root: crate_root.to_path_buf(),
        scenario_id: Some(finding.scenario_id.clone()),
        problem: finding.problem.clone(),
        next_action: finding.next_action.clone(),
        verification_command: format!(
            "cargo test -p {crate_name} scenario_performance_first_batch_optimization_frontier_is_clear"
        ),
        evidence: vec![
            scenario_root(crate_root)
                .join("benchmark.toml")
                .display()
                .to_string(),
        ],
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
