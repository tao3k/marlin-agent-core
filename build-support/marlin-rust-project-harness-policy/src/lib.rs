//! Build-script helper for Marlin's Rust project harness policy.

mod config;
mod dependency_topology;
mod evidence;
mod gate;

pub use config::{
    rust_project_harness_config_for_project, rust_project_harness_policy_for_project,
};
pub use dependency_topology::{
    ExternalDependencyTopologyReceipt, ExternalDependencyTopologyReceiptStatus,
    assert_rust_harness_dependency_topology_receipt_from_env,
    consume_external_dependency_topology_receipt,
    consume_rust_harness_dependency_topology_receipt_from_env,
};
pub use evidence::{
    RustProjectHarnessEvidenceReceipt, RustProjectHarnessGateReceipt,
    evaluate_performance_and_stability_gate, write_evidence_graph_from_env,
};
pub use gate::complete_build_gate;
