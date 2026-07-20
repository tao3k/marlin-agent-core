use marlin_rust_project_harness_policy::{
    evaluate_performance_and_stability_gate, rust_project_harness_config_for_project,
};
use rust_lang_project_harness::plan_rust_project_verification_with_config;

use crate::workspace::policy_package_root;

#[test]
fn performance_and_stability_gate_receipt_accepts_marlin_project_policy() {
    let project_root = policy_package_root();
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
    let project_root = policy_package_root();
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
