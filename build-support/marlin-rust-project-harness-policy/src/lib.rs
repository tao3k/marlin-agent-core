//! Build-script helper for Marlin's Rust project harness policy.

mod config;
mod dependency_topology;
mod evidence;
mod gate;
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
pub use package_evidence_graph::{
    RustProjectHarnessPackageEvidenceGraphReceipt, RustProjectHarnessPackageEvidenceGraphRequest,
    build_package_evidence_graph_receipt,
};
