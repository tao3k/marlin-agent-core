use marlin_rust_project_harness_policy::{
    RustProjectHarnessFindingSeverity, RustProjectHarnessImprovementPriority,
    RustProjectHarnessImprovementQueueStatus, RustProjectHarnessQualityFindingEvidencePaths,
    RustProjectHarnessQualityFindingsInput, build_improvement_queue_receipt,
    build_verification_policy_receipt, evaluate_quality_findings_for_gate,
    rust_project_harness_config_for_project,
};
use rust_lang_project_harness::plan_rust_project_verification_with_config;

use super::helpers::{runtime_verification_policy_receipt, workspace_root};

#[test]
fn verification_policy_receipt_exposes_crate_role_and_owner_profiles() {
    let project_root = workspace_root().join("crates/marlin-agent-runtime");
    let config = rust_project_harness_config_for_project(&project_root);
    let plan = plan_rust_project_verification_with_config(&project_root, &config)
        .expect("runtime crate should plan rust harness verification");

    let receipt =
        build_verification_policy_receipt("marlin-agent-runtime", &project_root, &config, &plan);

    assert_eq!(
        receipt.schema_id,
        "marlin.rust-project-harness.verification-policy"
    );
    assert_eq!(receipt.crate_role, "agent-runtime");
    assert!(receipt.performance_task_count > 0);
    assert!(receipt.stability_task_count > 0);
    assert!(receipt.performance_report_obligation);
    assert!(receipt.stability_report_obligation);
    assert!(receipt.owner_profiles.iter().any(|profile| {
        profile.owner_path == "src/graph_loop.rs"
            && profile
                .responsibilities
                .contains(&"latency-sensitive".to_owned())
            && profile.task_kinds.contains(&"performance".to_owned())
            && profile.task_kinds.contains(&"stability".to_owned())
    }));
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
            "verification_policy.json",
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
fn reflection_turns_quality_findings_into_improvement_queue() {
    let gate_receipt = marlin_rust_project_harness_policy::RustProjectHarnessGateReceipt {
        package_name: "marlin-agent-runtime".to_owned(),
        performance_gate: false,
        stability_gate: false,
        performance_report_obligation: false,
        stability_report_obligation: false,
    };
    let findings = evaluate_quality_findings_for_gate(RustProjectHarnessQualityFindingsInput {
        package_name: "marlin-agent-runtime".to_owned(),
        gate_receipt,
        evidence_paths: RustProjectHarnessQualityFindingEvidencePaths::new(
            "evidence-graph.json",
            "verification_plan.json",
            "task_index.json",
            "verification_policy.json",
        ),
    });
    let queue = build_improvement_queue_receipt(&findings, &runtime_verification_policy_receipt());

    assert_eq!(
        queue.status,
        RustProjectHarnessImprovementQueueStatus::ActionRequired
    );
    assert_eq!(queue.action_required_count(), 4);
    assert_eq!(queue.crate_role, "agent-runtime");
    assert!(
        queue
            .reflection_sources
            .contains(&"quality_findings.json".to_owned())
    );
    assert!(
        queue
            .reflection_sources
            .contains(&"verification_policy.json".to_owned())
    );
    assert!(queue.items.iter().all(|item| {
        item.priority == RustProjectHarnessImprovementPriority::Critical
            && item.next_action.contains("agent-runtime")
            && item.verification_command.contains("cargo test")
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
            "verification_policy.json",
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
    assert!(
        findings.findings[0]
            .evidence
            .contains(&"verification_policy.json".to_owned())
    );
}

#[test]
fn reflection_keeps_successful_quality_findings_as_healthy_queue() {
    let gate_receipt = marlin_rust_project_harness_policy::RustProjectHarnessGateReceipt {
        package_name: "marlin-agent-runtime".to_owned(),
        performance_gate: true,
        stability_gate: true,
        performance_report_obligation: true,
        stability_report_obligation: true,
    };
    let findings = evaluate_quality_findings_for_gate(RustProjectHarnessQualityFindingsInput {
        package_name: "marlin-agent-runtime".to_owned(),
        gate_receipt,
        evidence_paths: RustProjectHarnessQualityFindingEvidencePaths::new(
            "evidence-graph.json",
            "verification_plan.json",
            "task_index.json",
            "verification_policy.json",
        ),
    });
    let queue = build_improvement_queue_receipt(&findings, &runtime_verification_policy_receipt());

    assert!(queue.is_healthy());
    assert_eq!(queue.action_required_count(), 0);
    assert!(queue.items.is_empty());
}
