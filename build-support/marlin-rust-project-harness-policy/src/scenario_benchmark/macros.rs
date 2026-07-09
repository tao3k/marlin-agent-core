//! Standard `scenario_benchmark` test macro expansion.

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
