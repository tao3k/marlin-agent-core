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
mod package_evidence_graph;

pub use config::{
    rust_project_harness_config_for_project, rust_project_harness_policy_for_project,
};
pub use dependency_topology::{
    ExternalDependencyTopologyReceipt, ExternalDependencyTopologyReceiptStatus,
    assert_rust_harness_dependency_topology_receipt_from_env_if_strict,
    consume_external_dependency_topology_receipt,
    observe_rust_harness_dependency_topology_receipt_from_env,
};
pub use evidence::{
    RustProjectHarnessEvidenceReceipt, RustProjectHarnessFindingSeverity,
    RustProjectHarnessGateReceipt, RustProjectHarnessQualityFinding,
    RustProjectHarnessQualityFindingEvidencePaths, RustProjectHarnessQualityFindingsInput,
    RustProjectHarnessQualityFindingsReceipt, evaluate_performance_and_stability_gate,
    evaluate_quality_findings_for_gate, write_evidence_graph_from_env,
};
pub use gate::complete_build_gate;
pub use gerbil_runtime_assets::{
    GerbilRuntimeAssetManifestReceipt, GerbilRuntimeAssetManifestStatus,
    generate_gerbil_runtime_assets, inspect_gerbil_runtime_assets,
};
pub use package_evidence_graph::{
    RustProjectHarnessPackageEvidenceGraphReceipt, RustProjectHarnessPackageEvidenceGraphRequest,
    build_package_evidence_graph_receipt,
};
