#[path = "unit/gerbil_intent/mod.rs"]
mod gerbil_intent;
#[path = "unit/gerbil_release.rs"]
mod gerbil_release;
#[path = "unit/source_commit.rs"]
mod source_commit;

marlin_rust_project_harness_policy::scenario_performance_tests!();
