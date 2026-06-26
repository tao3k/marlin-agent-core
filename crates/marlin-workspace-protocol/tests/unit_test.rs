#[path = "unit/project_profile.rs"]
mod project_profile;
#[path = "unit/working_copy/mod.rs"]
mod working_copy;

marlin_rust_project_harness_policy::scenario_performance_tests!();
