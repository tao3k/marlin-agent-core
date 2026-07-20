use crate::paths::crate_root;

#[test]
fn turso_0_7_scenario_performance_contract_is_stable() {
    marlin_rust_project_harness_policy::assert_crate_scenario_performance_baseline_receipt_is_stable(
        &crate_root(),
        "marlin-agent-storage",
    );
}

#[test]
fn turso_0_7_scenario_performance_contract_has_no_optimization_findings() {
    let scenario_root = crate_root().join("tests/unit/scenarios/performance_baseline");
    let benchmark =
        marlin_rust_project_harness_policy::validate_rust_scenario_benchmark(&scenario_root)
            .expect("validate Turso 0.7 scenario benchmark");
    let optimization =
        marlin_rust_project_harness_policy::optimization_receipt_from_benchmark_receipt(
            "marlin-agent-storage",
            &benchmark,
        );

    assert_eq!(
        optimization.status,
        marlin_rust_project_harness_policy::RustScenarioPerformanceOptimizationStatus::Healthy
    );
    assert!(
        optimization.findings.is_empty(),
        "{:#?}",
        optimization.findings
    );
}
