#[path = "unit/contract_facts.rs"]
mod contract_facts;
#[path = "unit/spec.rs"]
mod spec;

marlin_rust_project_harness_policy::scenario_performance_tests!();
