#[path = "unit/kernel.rs"]
mod kernel;
#[path = "unit/runtime/mod.rs"]
mod runtime;

marlin_rust_project_harness_policy::scenario_performance_tests!();
