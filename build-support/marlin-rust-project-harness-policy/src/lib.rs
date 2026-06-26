//! Build-script helper for Marlin's Rust project harness policy.
//!
//! This crate belongs to the Rust project harness boundary. It is responsible
//! for package-level engineering quality gates, build-script receipts,
//! performance/stability gates, CI evidence, and Rust project evidence graphs.
//! It must not model agent runtime loops, sub-agents, hook replay, sessions, or
//! agent harness scenarios.

mod config;
mod dependency_topology;
mod evidence;
mod gate;
mod gerbil_runtime_assets;
mod improvement_queue;
mod package_evidence_graph;
mod quality_findings;
mod quality_gate;
mod scenario_benchmark;
mod verification_policy;

pub use config::{
    MarlinCrateVerificationRole, marlin_crate_verification_role_for_project,
    rust_project_harness_config_for_project, rust_project_harness_policy_for_project,
};
pub use dependency_topology::{
    ExternalDependencyTopologyReceipt, ExternalDependencyTopologyReceiptStatus,
    assert_rust_harness_dependency_topology_receipt_from_env_if_strict,
    consume_external_dependency_topology_receipt,
    observe_rust_harness_dependency_topology_receipt_from_env,
};
pub use evidence::{RustProjectHarnessEvidenceReceipt, write_evidence_graph_from_env};
pub use gate::{assert_marlin_rust_project_harness_gate_from_env, complete_build_gate};
pub use gerbil_runtime_assets::{
    GerbilRuntimeAssetManifestReceipt, GerbilRuntimeAssetManifestStatus,
    generate_gerbil_runtime_assets, inspect_gerbil_runtime_assets,
};
pub use improvement_queue::{
    RustProjectHarnessImprovementId, RustProjectHarnessImprovementItem,
    RustProjectHarnessImprovementPlanReceipt, RustProjectHarnessImprovementPlanStatus,
    RustProjectHarnessImprovementPlanStep, RustProjectHarnessImprovementPriority,
    RustProjectHarnessImprovementQueueReceipt, RustProjectHarnessImprovementQueueStatus,
    RustProjectHarnessImprovementStepId, RustProjectHarnessRuleId, build_improvement_plan_receipt,
    build_improvement_queue_receipt,
};
pub use package_evidence_graph::{
    RustProjectHarnessPackageEvidenceGraphReceipt, RustProjectHarnessPackageEvidenceGraphRequest,
    build_package_evidence_graph_receipt,
};
pub use quality_findings::{
    RustProjectHarnessExpectedArtifact, RustProjectHarnessFindingSeverity,
    RustProjectHarnessQualityAutofixability, RustProjectHarnessQualityBlockingLevel,
    RustProjectHarnessQualityDomain, RustProjectHarnessQualityFinding,
    RustProjectHarnessQualityFindingEvidencePaths, RustProjectHarnessQualityFindingsInput,
    RustProjectHarnessQualityFindingsReceipt, evaluate_quality_findings_for_gate,
};
pub use quality_gate::{RustProjectHarnessGateReceipt, evaluate_performance_and_stability_gate};
pub use scenario_benchmark::{
    RustScenarioBenchmarkReceipt, RustScenarioBenchmarkStatus, RustScenarioPerformanceCrateName,
    RustScenarioPerformanceDurationMs, RustScenarioPerformanceMemoryBytes,
    RustScenarioPerformanceOptimizationFinding, RustScenarioPerformanceOptimizationFindingKind,
    RustScenarioPerformanceOptimizationPriority, RustScenarioPerformanceOptimizationReceipt,
    RustScenarioPerformanceOptimizationScore, RustScenarioPerformanceOptimizationStatus,
    RustScenarioPerformanceScenarioId,
    assert_crate_scenario_performance_baseline_receipt_is_stable,
    assert_crate_scenario_performance_contract_gate_accepts_crate_scenarios,
    assert_crate_scenario_performance_first_batch_optimization_frontier_is_clear,
    crate_scenario_performance_optimization_receipt, optimization_receipt_from_benchmark_receipt,
    render_rust_scenario_benchmark_snapshot, validate_rust_scenario_benchmark,
};
pub use verification_policy::{
    RustProjectHarnessVerificationOwnerProfileReceipt, RustProjectHarnessVerificationPolicyReceipt,
    build_verification_policy_receipt,
};
