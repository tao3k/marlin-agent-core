use std::{
    fs,
    path::{Path, PathBuf},
};

use marlin_rust_project_harness_policy::{
    RustProjectHarnessFindingSeverity, RustProjectHarnessPackageEvidenceGraphRequest,
    RustProjectHarnessQualityFindingEvidencePaths, RustProjectHarnessQualityFindingsInput,
    build_package_evidence_graph_receipt, evaluate_performance_and_stability_gate,
    evaluate_quality_findings_for_gate, rust_project_harness_config_for_project,
};
use rust_lang_project_harness::{
    plan_rust_project_verification_with_config, run_rust_project_harness_with_config,
};

#[test]
fn performance_and_stability_gate_receipt_accepts_marlin_project_policy() {
    let project_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let config = rust_project_harness_config_for_project(project_root);
    let plan = plan_rust_project_verification_with_config(project_root, &config)
        .expect("build-support crate should plan rust harness verification");

    let receipt = evaluate_performance_and_stability_gate(&plan, env!("CARGO_PKG_NAME").to_owned());

    assert!(receipt.is_success());
    assert_eq!(receipt.package_name, env!("CARGO_PKG_NAME"));
    assert!(receipt.performance_gate);
    assert!(receipt.stability_gate);
    assert!(receipt.performance_report_obligation);
    assert!(receipt.stability_report_obligation);
}

#[test]
fn performance_and_stability_gate_receipt_reports_missing_package_gates() {
    let project_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let config = rust_project_harness_config_for_project(project_root);
    let mut plan = plan_rust_project_verification_with_config(project_root, &config)
        .expect("build-support crate should plan rust harness verification");
    plan.tasks.clear();
    plan.report_obligations.clear();

    let receipt = evaluate_performance_and_stability_gate(&plan, env!("CARGO_PKG_NAME").to_owned());

    assert!(!receipt.is_success());
    assert!(!receipt.performance_gate);
    assert!(!receipt.stability_gate);
    assert!(!receipt.performance_report_obligation);
    assert!(!receipt.stability_report_obligation);
}

#[test]
fn quality_findings_turn_missing_gates_into_agent_actionable_hard_errors() {
    let receipt = marlin_rust_project_harness_policy::RustProjectHarnessGateReceipt {
        package_name: "demo".to_owned(),
        performance_gate: false,
        stability_gate: false,
        performance_report_obligation: false,
        stability_report_obligation: false,
    };

    let findings = evaluate_quality_findings_for_gate(RustProjectHarnessQualityFindingsInput {
        package_name: "demo".to_owned(),
        gate_receipt: receipt,
        evidence_paths: RustProjectHarnessQualityFindingEvidencePaths::new(
            "evidence-graph.json",
            "verification_plan.json",
            "task_index.json",
        ),
    });

    assert_eq!(findings.hard_error_count(), 4);
    assert_eq!(findings.warning_count(), 0);
    assert_eq!(findings.advice_count(), 1);
    assert!(findings.findings.iter().any(|finding| {
        finding.severity == RustProjectHarnessFindingSeverity::HardError
            && finding.rule_id == "MARLIN-QUALITY-GATE-PERF"
            && finding.agent_next_action.contains("performance")
    }));
}

#[test]
fn quality_findings_keep_successful_gate_as_agent_evidence_advice() {
    let receipt = marlin_rust_project_harness_policy::RustProjectHarnessGateReceipt {
        package_name: "demo".to_owned(),
        performance_gate: true,
        stability_gate: true,
        performance_report_obligation: true,
        stability_report_obligation: true,
    };

    let findings = evaluate_quality_findings_for_gate(RustProjectHarnessQualityFindingsInput {
        package_name: "demo".to_owned(),
        gate_receipt: receipt,
        evidence_paths: RustProjectHarnessQualityFindingEvidencePaths::new(
            "evidence-graph.json",
            "verification_plan.json",
            "task_index.json",
        ),
    });

    assert_eq!(findings.hard_error_count(), 0);
    assert_eq!(findings.warning_count(), 0);
    assert_eq!(findings.advice_count(), 1);
    assert_eq!(
        findings.findings[0].severity,
        RustProjectHarnessFindingSeverity::Advice
    );
    assert!(
        findings.findings[0]
            .evidence
            .contains(&"evidence-graph.json".to_owned())
    );
}

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

fn workspace_crates() -> Vec<PathBuf> {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("policy crate should live under workspace/build-support");
    let crates_dir = workspace_root.join("crates");
    let mut crates = fs::read_dir(&crates_dir)
        .unwrap_or_else(|error| panic!("read workspace crates dir {crates_dir:?}: {error}"))
        .map(|entry| entry.expect("workspace crate entry").path())
        .filter(|path| path.join("Cargo.toml").is_file() && path.join("src/lib.rs").is_file())
        .collect::<Vec<_>>();

    crates.sort();
    crates
}
