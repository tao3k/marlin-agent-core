// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Crate-local `scenario_benchmark` assertions and stable evidence snapshots.

use std::path::Path;

use asp_rust::{
    assert_rule_fixture_scenario_benchmarks, validate_required_rust_scenario_benchmarks,
};

use super::{RustScenarioBenchmarkStatus, paths::scenario_root, validate_rust_scenario_benchmark};

use crate::policy::scenario_snapshot::render_rust_scenario_benchmark_snapshot;

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
