// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use asp_rust::{
    RustScenarioBenchmarkStatus, assert_rule_fixture_scenario_benchmarks,
    validate_rust_scenario_benchmark,
};

use crate::paths::crate_root;

#[test]
fn turso_0_7_scenario_performance_contract_is_stable() {
    let root = crate_root();
    assert_rule_fixture_scenario_benchmarks(&root);
    let receipt =
        validate_rust_scenario_benchmark(root.join("tests/unit/scenarios/performance_baseline"))
            .expect("validate Turso performance benchmark");
    assert_eq!(receipt.status, RustScenarioBenchmarkStatus::Pass);
    assert!(receipt.violations.is_empty(), "{:?}", receipt.violations);
    assert_eq!(
        receipt.scenario.id,
        "marlin-agent-storage.scenario-performance"
    );
    assert!(receipt.benchmark.observed_total <= receipt.benchmark.max_total);
    assert!(receipt.benchmark.observed_memory_bytes <= receipt.benchmark.memory_budget_bytes);
}

#[test]
fn turso_0_7_scenario_performance_contract_has_no_violations() {
    let scenario_root = crate_root().join("tests/unit/scenarios/performance_baseline");
    let benchmark = validate_rust_scenario_benchmark(&scenario_root)
        .expect("validate Turso 0.7 scenario benchmark");
    assert_eq!(benchmark.status, RustScenarioBenchmarkStatus::Pass);
    assert!(
        benchmark.violations.is_empty(),
        "{:?}",
        benchmark.violations
    );
}
