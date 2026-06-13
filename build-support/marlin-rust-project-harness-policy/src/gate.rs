//! Build-script gate orchestration for Marlin workspace crates.

use std::path::Path;

use rust_lang_project_harness::{RustHarnessConfig, RustHarnessReport};

use crate::{
    RustProjectHarnessEvidenceReceipt,
    assert_rust_harness_dependency_topology_receipt_from_env_if_strict,
    write_evidence_graph_from_env,
};

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
