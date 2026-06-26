//! Build-script gate orchestration for Marlin workspace crates.

use std::path::Path;

use rust_lang_project_harness::{RustHarnessConfig, RustHarnessReport};

use crate::{
    RustProjectHarnessEvidenceReceipt,
    assert_rust_harness_dependency_topology_receipt_from_env_if_strict, config,
    write_evidence_graph_from_env,
};

/// Run Marlin's complete Rust project harness gate for a downstream package.
///
/// Downstream crates should call this wrapper from `build.rs` instead of
/// depending on `rust-lang-project-harness` directly. This keeps the
/// rust-project-harness boundary owned by `marlin-rust-project-harness-policy`.
pub fn assert_marlin_rust_project_harness_gate_from_env(
    project_root: &Path,
) -> RustProjectHarnessEvidenceReceipt {
    let policy = config::rust_project_harness_policy_for_project(project_root);
    let harness_report =
        rust_lang_project_harness::assert_rust_project_harness_downstream_policy_from_env(&policy);
    complete_build_gate(project_root, policy.config(), harness_report)
}

/// Complete Marlin's build-time Rust harness gate after the parser-owned check ran.
pub fn complete_build_gate(
    project_root: &Path,
    config: &RustHarnessConfig,
    harness_report: RustHarnessReport,
) -> RustProjectHarnessEvidenceReceipt {
    let _dependency_topology_receipt =
        assert_rust_harness_dependency_topology_receipt_from_env_if_strict(project_root);
    write_evidence_graph_from_env(config, harness_report)
}
