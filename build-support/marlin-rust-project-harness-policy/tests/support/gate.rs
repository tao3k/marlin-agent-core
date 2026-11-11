// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! ASP Rust development gate orchestration for Marlin workspace crates.

use std::path::Path;

use asp_rust::{AspRustConfig, AspRustReport, AspRustRunScope};

use crate::policy::{
    RustProjectHarnessEvidenceReceipt,
    assert_rust_harness_dependency_topology_receipt_from_env_if_strict, config,
    write_evidence_graph_from_env,
};

/// Run Marlin's package-scoped ASP Rust development gate.
pub fn assert_marlin_rust_dev_gate(project_root: &Path) -> AspRustReport {
    let policy = config::rust_project_harness_policy_for_project(project_root);
    asp_rust::assert_asp_rust_downstream_policy(project_root, &policy)
}

/// Run the Rust harness for exactly one Cargo package.
///
/// The pinned upstream runner expands a workspace root to every member.  This
/// boundary rejects workspace manifests before calling it, so package evidence
/// targets cannot accidentally turn into workspace-wide build actions.
pub fn run_marlin_rust_project_harness_for_package(
    package_root: &Path,
    config: &AspRustConfig,
) -> Result<AspRustReport, String> {
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

    asp_rust::run_asp_rust_with_config_for_scope(package_root, config, AspRustRunScope::Package)
}

/// Complete Marlin's build-time Rust harness gate after the parser-owned check ran.
pub fn complete_build_gate(
    project_root: &Path,
    config: &AspRustConfig,
    harness_report: AspRustReport,
) -> RustProjectHarnessEvidenceReceipt {
    let _dependency_topology_receipt =
        assert_rust_harness_dependency_topology_receipt_from_env_if_strict(project_root);
    write_evidence_graph_from_env(config, harness_report)
}
