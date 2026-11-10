#[path = "unit/aot_runtime/mod.rs"]
mod aot_runtime;

#[path = "unit/dependency_boundary.rs"]
mod dependency_boundary;

#[path = "unit/deck_runtime_policy.rs"]
mod deck_runtime_policy;

#[path = "unit/deck_runtime_script.rs"]
mod deck_runtime_script;

#[path = "unit/working_copy_policy.rs"]
mod working_copy_policy;

#[path = "unit/deck_runtime_native/mod.rs"]
mod deck_runtime_native;

#[path = "unit/agent_policy_routing_native/mod.rs"]
mod agent_policy_routing_native;

#[path = "unit/loop_case_driver/mod.rs"]
mod loop_case_driver;

#[path = "unit/scheme_types/mod.rs"]
mod scheme_types;

marlin_rust_project_harness_policy::scenario_performance_tests!();

#[path = "unit/scheme_package.rs"]
mod scheme_package;
