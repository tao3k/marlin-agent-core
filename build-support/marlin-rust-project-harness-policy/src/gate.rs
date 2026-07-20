//! Build-script gate orchestration for Marlin workspace crates.

use std::path::Path;

use rust_lang_project_harness::{RustHarnessConfig, RustHarnessReport};

use crate::{
    RustProjectHarnessEvidenceReceipt,
    assert_rust_harness_dependency_topology_receipt_from_env_if_strict, config,
    write_evidence_graph_from_env,
};

/// Run Marlin's package-scoped Rust project harness check during a build.
///
/// Downstream crates should call this wrapper from `build.rs` instead of
/// depending on `rust-lang-project-harness` directly. This keeps the
/// rust-project-harness boundary owned by `marlin-rust-project-harness-policy`.
///
/// This build check deliberately stops before dependency-topology analysis and
/// evidence-graph generation. Those heavier gates belong to explicit test
/// targets, where Cargo can address and cache them independently from ordinary
/// package compilation.
pub fn assert_marlin_rust_project_harness_build_check_from_env(
    project_root: &Path,
) -> RustHarnessReport {
    let policy = config::rust_project_harness_policy_for_project(project_root);
    rust_lang_project_harness::assert_rust_project_harness_downstream_policy_from_env(&policy)
}

/// Run the Rust harness for exactly one Cargo package.
///
/// The pinned upstream runner expands a workspace root to every member.  This
/// boundary rejects workspace manifests before calling it, so package evidence
/// targets cannot accidentally turn into workspace-wide build actions.
pub fn run_marlin_rust_project_harness_for_package(
    package_root: &Path,
    config: &RustHarnessConfig,
) -> Result<RustHarnessReport, String> {
    let manifest_path = package_root.join("Cargo.toml");
    let manifest_body = std::fs::read_to_string(&manifest_path)
        .map_err(|error| format!("failed to read {}: {error}", manifest_path.display()))?;
    let manifest = manifest_body
        .parse::<toml::Table>()
        .map_err(|error| format!("failed to parse {}: {error}", manifest_path.display()))?;
    if !manifest.contains_key("package") || manifest.contains_key("workspace") {
        return Err(format!(
            "package harness requires a non-workspace Cargo package root: {}",
            package_root.display()
        ));
    }

    rust_lang_project_harness::run_rust_project_harness_with_config(package_root, config)
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
