use marlin_rust_project_harness_policy::{
    RustProjectHarnessPackageEvidenceGraphRequest, RustProjectHarnessQualityFindingEvidencePaths,
    build_package_evidence_graph_receipt, evaluate_performance_and_stability_gate,
    run_marlin_rust_project_harness_for_package, rust_project_harness_config_for_project,
};
use rust_lang_project_harness::plan_rust_project_verification_with_config;

use crate::workspace::policy_package_root;

use super::helpers::{workspace_crates, workspace_root};

#[test]
fn package_harness_rejects_workspace_root() {
    let workspace = tempfile::tempdir().expect("temporary workspace should be available");
    std::fs::write(
        workspace.path().join("Cargo.toml"),
        "[workspace]\nmembers = []\nresolver = \"3\"\n",
    )
    .expect("workspace manifest should be writable");
    let config = rust_project_harness_config_for_project(workspace.path());

    let error = run_marlin_rust_project_harness_for_package(workspace.path(), &config)
        .expect_err("package runner must reject a workspace root");

    assert!(error.contains("non-workspace Cargo package root"));
}

#[test]
fn package_evidence_graph_receipt_is_owned_by_build_support() {
    let project_root = policy_package_root();
    let package_name = env!("CARGO_PKG_NAME").to_owned();
    let config = rust_project_harness_config_for_project(project_root);
    let harness_report = run_marlin_rust_project_harness_for_package(project_root, &config)
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
    assert_eq!(
        receipt.verification_policy_receipt.package_name,
        package_name
    );
    assert!(receipt.verification_policy_receipt.performance_task_count > 0);
    assert!(receipt.verification_policy_receipt.stability_task_count > 0);
    assert_eq!(receipt.quality_findings_receipt.hard_error_count(), 0);
    assert!(receipt.improvement_queue_receipt.is_healthy());
    assert_eq!(receipt.improvement_queue_receipt.action_required_count(), 0);
    assert!(receipt.improvement_plan_receipt.is_noop());
    assert_eq!(receipt.improvement_plan_receipt.step_count(), 0);
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
