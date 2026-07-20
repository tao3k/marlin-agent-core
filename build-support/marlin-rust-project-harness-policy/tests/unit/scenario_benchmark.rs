use std::{fs, path::Path};

use marlin_rust_project_harness_policy::{
    RustScenarioPerformanceOptimizationFindingKind, RustScenarioPerformanceOptimizationPriority,
    RustScenarioPerformanceOptimizationStatus, RustScenarioWorkspaceBenchmarkImprovementKind,
    RustScenarioWorkspaceBenchmarkStatus, optimization_receipt_from_benchmark_receipt,
    validate_workspace_required_scenario_benchmarks,
};
use rust_lang_project_harness::{RustScenarioBenchmarkReceipt, validate_rust_scenario_benchmark};
use tempfile::tempdir;

use crate::workspace::workspace_root;

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

#[test]
fn workspace_gate_accepts_all_crates_using_upstream_benchmark_schema() {
    let tempdir = tempdir().expect("create workspace benchmark fixture tempdir");
    write_workspace_manifest(tempdir.path(), &["crates/marlin-one", "crates/marlin-two"]);
    write_crate_scenario_fixture(tempdir.path(), "crates/marlin-one", true);
    write_crate_scenario_fixture(tempdir.path(), "crates/marlin-two", true);
    fs::create_dir_all(tempdir.path().join("build-support/ignored"))
        .expect("write ignored build-support member");

    let receipt = validate_workspace_required_scenario_benchmarks(tempdir.path());

    assert_eq!(receipt.status, RustScenarioWorkspaceBenchmarkStatus::Pass);
    assert_eq!(receipt.crate_count, 2);
    assert_eq!(receipt.passed_crate_count, 2);
    assert_eq!(receipt.failed_crate_count, 0);
    assert!(receipt.improvements.is_empty(), "{receipt:?}");
}

#[test]
fn current_workspace_gate_accepts_all_crate_scenario_benchmarks() {
    let workspace_root = workspace_root();

    let receipt = validate_workspace_required_scenario_benchmarks(workspace_root);

    assert_eq!(
        receipt.status,
        RustScenarioWorkspaceBenchmarkStatus::Pass,
        "{receipt:?}"
    );
    assert_ne!(receipt.crate_count, 0, "{receipt:?}");
    assert!(receipt.improvements.is_empty(), "{receipt:?}");
}

#[test]
fn workspace_gate_turns_schema_drift_into_actionable_improvement() {
    let tempdir = tempdir().expect("create workspace benchmark fixture tempdir");
    write_workspace_manifest(
        tempdir.path(),
        &["crates/marlin-good", "crates/marlin-legacy"],
    );
    write_crate_scenario_fixture(tempdir.path(), "crates/marlin-good", true);
    write_crate_scenario_fixture(tempdir.path(), "crates/marlin-legacy", false);

    let receipt = validate_workspace_required_scenario_benchmarks(tempdir.path());

    assert_eq!(receipt.status, RustScenarioWorkspaceBenchmarkStatus::Fail);
    assert_eq!(receipt.crate_count, 2);
    assert_eq!(receipt.failed_crate_count, 1);
    assert!(receipt.improvement_count >= 1, "{receipt:?}");
    assert!(
        receipt.improvements.iter().any(|improvement| {
            improvement.kind
                == RustScenarioWorkspaceBenchmarkImprovementKind::InvalidScenarioBenchmarkSchema
                && improvement
                    .next_action
                    .contains("upstream rust-lang-project-harness scenario schema")
                && improvement
                    .verification_command
                    .contains("scenario_performance_contract_gate_accepts_crate_scenarios")
        }),
        "{receipt:?}"
    );
}

fn benchmark_receipt(
    max_total_ms: u64,
    memory_budget_bytes: u64,
    rationale: &str,
) -> RustScenarioBenchmarkReceipt {
    let tempdir = tempdir().expect("create scenario benchmark fixture tempdir");
    let root = tempdir.path();
    fs::write(
        root.join("scenario.toml"),
        r#"id = "marlin-example.scenario-performance"
title = "example scenario performance baseline"
policy_ids = ["RUST-AGENT-SCENARIO-PERFORMANCE-001"]
agent_goal = "Keep scenario performance contracts stable and replayable."
reference_repositories = ["rust-lang/rust", "tokio-rs/tokio"]
reference_patterns = ["scenario benchmark contract", "bounded runtime performance"]
inputs = "inputs"
expected = "expected"
"#,
    )
    .expect("write scenario benchmark metadata fixture");

    fs::write(
        root.join("benchmark.toml"),
        format!(
            r#"harness = "libtest"
test = "scenario_performance"
target_total = "25ms"
max_total = "{max_total_ms}ms"
observed_total = "25ms"
regression_budget = "20ms"
memory_budget_bytes = {memory_budget_bytes}
observed_memory_bytes = 4194304
target_rationale = "{rationale}"

[observed_timings]
fixture = "25ms"
"#
        ),
    )
    .expect("write scenario benchmark contract fixture");

    validate_rust_scenario_benchmark(root).expect("upstream harness validates benchmark fixture")
}

fn write_workspace_manifest(root: &Path, members: &[&str]) {
    let members = members
        .iter()
        .map(|member| format!("    \"{member}\","))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(
        root.join("Cargo.toml"),
        format!(
            r#"[workspace]
members = [
{members}
    "build-support/ignored",
]
"#
        ),
    )
    .expect("write workspace Cargo.toml fixture");
}

fn write_crate_scenario_fixture(workspace_root: &Path, crate_rel: &str, upstream_schema: bool) {
    let crate_root = workspace_root.join(crate_rel);
    let scenario_root = crate_root
        .join("tests")
        .join("unit")
        .join("scenarios")
        .join("performance_baseline");
    fs::create_dir_all(&scenario_root).expect("create crate scenario fixture root");
    let crate_name = crate_rel.rsplit('/').next().expect("crate name");
    fs::write(
        crate_root.join("Cargo.toml"),
        format!(
            r#"[package]
name = "{crate_name}"
edition = "2024"
version = "0.0.0"
"#
        ),
    )
    .expect("write crate manifest fixture");
    fs::write(
        scenario_root.join("scenario.toml"),
        format!(
            r#"id = "{crate_name}.scenario-performance"
title = "{crate_name} scenario performance baseline"
policy_ids = ["RUST-AGENT-SCENARIO-PERFORMANCE-001"]
agent_goal = "Keep scenario performance contracts stable and replayable."
reference_repositories = ["rust-lang/rust", "tokio-rs/tokio"]
reference_patterns = ["scenario benchmark contract", "bounded runtime performance"]
inputs = "inputs"
expected = "expected"
"#
        ),
    )
    .expect("write scenario metadata fixture");

    let benchmark = if upstream_schema {
        r#"harness = "libtest"
test = "scenario_performance"
target_total = "25ms"
max_total = "45ms"
observed_total = "25ms"
regression_budget = "20ms"
memory_budget_bytes = 6291456
observed_memory_bytes = 4194304
target_rationale = "First-batch tuned scenario baseline"

[observed_timings]
fixture = "25ms"
"#
    } else {
        r#"max_total_ms = 100
memory_budget = "8MiB"
"#
    };
    fs::write(scenario_root.join("benchmark.toml"), benchmark).expect("write benchmark fixture");
}
