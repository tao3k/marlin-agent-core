#[path = "unit/loop_graph.rs"]
mod loop_graph;
#[path = "unit/workspace_policy.rs"]
mod workspace_policy;

marlin_rust_project_harness_policy::scenario_performance_tests!();
