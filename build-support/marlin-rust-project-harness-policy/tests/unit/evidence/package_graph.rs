use std::path::Path;

use marlin_rust_project_harness_policy::{
    RustProjectHarnessPackageEvidenceGraphRequest, RustProjectHarnessQualityFindingEvidencePaths,
    build_package_evidence_graph_receipt, evaluate_performance_and_stability_gate,
    rust_project_harness_config_for_project,
};
use rust_lang_project_harness::{
    plan_rust_project_verification_with_config, run_rust_project_harness_with_config,
};

use super::helpers::{workspace_crates, workspace_root};

#[test]
fn package_evidence_graph_receipt_is_owned_by_build_support() {
    let project_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let package_name = env!("CARGO_PKG_NAME").to_owned();
    let config = rust_project_harness_config_for_project(project_root);
    let harness_report = run_rust_project_harness_with_config(project_root, &config)
        .expect("build-support crate should produce rust harness report");

    let receipt =
        build_package_evidence_graph_receipt(RustProjectHarnessPackageEvidenceGraphRequest {
            config: &config,
            harness_report,
            project_root: project_root.to_path_buf(),
            package_name: package_name.clone(),
            evidence_paths: RustProjectHarnessQualityFindingEvidencePaths::new(
                "evidence-graph.json",
                "verification_plan.json",
                "task_index.json",
                "verification_policy.json",
            ),
        });

    assert_eq!(receipt.package_name, package_name);
    assert!(receipt.evidence_graph_summary.nodes > 0);
    assert!(receipt.gate_receipt.is_success());
    assert_eq!(receipt.quality_findings_receipt.hard_error_count(), 0);
    assert!(receipt.is_success());
}

#[test]
fn workspace_packages_emit_successful_performance_and_stability_gate_receipts() {
    let crates = workspace_crates();
    assert!(
        crates.len() >= 20,
        "workspace gate receipt coverage expected at least 20 crates, got {}",
        crates.len()
    );

    let receipts = crates
        .iter()
        .map(|crate_dir| {
            let crate_name = crate_dir
                .file_name()
                .and_then(|name| name.to_str())
                .expect("workspace crate should have a utf-8 directory name")
                .to_owned();
            let config = rust_project_harness_config_for_project(crate_dir);
            let plan = plan_rust_project_verification_with_config(crate_dir, &config)
                .unwrap_or_else(|error| panic!("{crate_name} verification plan: {error}"));

            evaluate_performance_and_stability_gate(&plan, crate_name)
        })
        .collect::<Vec<_>>();

    let missing = receipts
        .iter()
        .filter(|receipt| !receipt.is_success())
        .map(|receipt| {
            format!(
                "{}:perf={} stability={} perf_report={} stability_report={}",
                receipt.package_name,
                receipt.performance_gate,
                receipt.stability_gate,
                receipt.performance_report_obligation,
                receipt.stability_report_obligation
            )
        })
        .collect::<Vec<_>>();

    assert!(
        missing.is_empty(),
        "workspace packages missing quality gate receipts: {}",
        missing.join(", ")
    );
    assert_eq!(receipts.len(), crates.len());
}

#[test]
fn workspace_root_points_at_repository_root() {
    assert!(workspace_root().join("Cargo.toml").is_file());
}
