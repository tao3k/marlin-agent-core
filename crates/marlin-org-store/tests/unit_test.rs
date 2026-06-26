#[path = "unit/commit.rs"]
mod commit;
#[path = "unit/discovery.rs"]
mod discovery;
#[path = "unit/file.rs"]
mod file;

marlin_rust_project_harness_policy::scenario_performance_tests!();
