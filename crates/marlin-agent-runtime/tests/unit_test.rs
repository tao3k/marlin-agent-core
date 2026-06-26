#[path = "unit/runtime.rs"]
mod runtime;
#[path = "unit/runtime/agent_flow_loop.rs"]
mod runtime_agent_flow_loop;
#[path = "unit/runtime/agent_graph/mod.rs"]
mod runtime_agent_graph;
#[path = "unit/runtime/cleanup.rs"]
mod runtime_cleanup;
#[path = "unit/runtime/dependency_boundary.rs"]
mod runtime_dependency_boundary;
#[path = "unit/runtime/events.rs"]
mod runtime_events;
#[path = "unit/runtime/graph_loop_run_registry.rs"]
mod runtime_graph_loop_run_registry;
#[path = "unit/runtime/graph_loop_runtime.rs"]
mod runtime_graph_loop_runtime;
#[path = "unit/runtime/model_route/mod.rs"]
mod runtime_model_route;
#[path = "unit/runtime/observability.rs"]
mod runtime_observability;
#[path = "unit/runtime/process/mod.rs"]
mod runtime_process;
#[path = "unit/runtime/resilience.rs"]
mod runtime_resilience;
#[path = "unit/runtime/session.rs"]
mod runtime_session;
#[path = "unit/runtime/spawn_config.rs"]
mod runtime_spawn_config;
#[path = "unit/runtime/task_tracker.rs"]
mod runtime_task_tracker;
#[path = "unit/runtime/working_copy.rs"]
mod runtime_working_copy;

marlin_rust_project_harness_policy::scenario_performance_tests!();
