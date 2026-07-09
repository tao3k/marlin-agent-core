//! Public `scenario_benchmark` facade for crate and workspace benchmark gates.

mod assertions;
mod macros;
mod optimization;
mod paths;
mod types;
mod workspace_gate;

pub use assertions::{
    assert_crate_scenario_performance_baseline_receipt_is_stable,
    assert_crate_scenario_performance_contract_gate_accepts_crate_scenarios,
    render_rust_scenario_benchmark_snapshot,
};
pub use optimization::{
    assert_crate_scenario_performance_first_batch_optimization_frontier_is_clear,
    crate_scenario_performance_optimization_receipt, optimization_receipt_from_benchmark_receipt,
};
pub use rust_lang_project_harness::{
    RustScenarioBenchmarkDuration, RustScenarioBenchmarkReceipt, RustScenarioBenchmarkStatus,
    RustScenarioBenchmarkSuiteReceipt, validate_rust_scenario_benchmark,
};
pub use types::{
    RustScenarioPerformanceCrateName, RustScenarioPerformanceDurationMs,
    RustScenarioPerformanceMemoryBytes, RustScenarioPerformanceOptimizationFinding,
    RustScenarioPerformanceOptimizationFindingKind, RustScenarioPerformanceOptimizationPriority,
    RustScenarioPerformanceOptimizationReceipt, RustScenarioPerformanceOptimizationScore,
    RustScenarioPerformanceOptimizationStatus, RustScenarioPerformanceScenarioId,
    RustScenarioWorkspaceBenchmarkCrateReceipt, RustScenarioWorkspaceBenchmarkGateReceipt,
    RustScenarioWorkspaceBenchmarkImprovementKind,
    RustScenarioWorkspaceBenchmarkImprovementSuggestion, RustScenarioWorkspaceBenchmarkStatus,
};
pub use workspace_gate::{
    assert_workspace_scenario_performance_contract_gate_accepts_all_crates,
    validate_workspace_required_scenario_benchmarks,
    workspace_scenario_performance_optimization_receipt,
};
