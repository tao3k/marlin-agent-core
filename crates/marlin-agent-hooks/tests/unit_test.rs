#[path = "unit/config.rs"]
mod config;
#[path = "unit/hooks/mod.rs"]
mod hooks;

marlin_rust_project_harness_policy::scenario_performance_tests!();
