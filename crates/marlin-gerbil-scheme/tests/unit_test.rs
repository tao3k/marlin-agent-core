#[path = "unit/artifact.rs"]
mod artifact;

#[path = "unit/aot_runtime/mod.rs"]
mod aot_runtime;

#[path = "unit/compiler.rs"]
mod compiler;

#[path = "unit/deps.rs"]
mod deps;

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

#[path = "unit/command/mod.rs"]
mod command;

#[path = "unit/runtime/mod.rs"]
mod runtime;

#[path = "unit/resident_runtime/mod.rs"]
mod resident_runtime;

#[path = "unit/scheme_types/mod.rs"]
mod scheme_types;

marlin_rust_project_harness_policy::scenario_performance_tests!();
