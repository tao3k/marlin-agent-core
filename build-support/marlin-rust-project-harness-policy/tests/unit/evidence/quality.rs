use std::collections::BTreeSet;

use marlin_rust_project_harness_policy::{
    RustProjectHarnessExpectedArtifact, RustProjectHarnessFindingSeverity,
    RustProjectHarnessImprovementPlanStatus, RustProjectHarnessImprovementPriority,
    RustProjectHarnessImprovementQueueStatus, RustProjectHarnessPolicyProjectionDecision,
    RustProjectHarnessQualityAutofixability, RustProjectHarnessQualityBlockingLevel,
    RustProjectHarnessQualityDomain, RustProjectHarnessQualityFindingEvidencePaths,
    RustProjectHarnessQualityFindingsInput, build_improvement_plan_receipt,
    build_improvement_queue_receipt, build_verification_policy_receipt,
    evaluate_quality_findings_for_gate, project_current_rust_agent_policy_catalog,
    rust_project_harness_config_for_project,
};
use rust_lang_project_harness::{
    plan_rust_project_verification_with_config, rust_agent_policy_rules,
};

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
            && finding.domain == RustProjectHarnessQualityDomain::VerificationGate
            && finding.blocking_level == RustProjectHarnessQualityBlockingLevel::BuildBlocking
            && finding.autofixability == RustProjectHarnessQualityAutofixability::ManualPolicyEdit
            && finding.expected_artifact
                == RustProjectHarnessExpectedArtifact::PerformanceVerificationTask
            && finding.rule_id == "MARLIN-QUALITY-GATE-PERF"
            && finding.agent_next_action.contains("performance")
    }));
}

#[test]
fn policy_projection_covers_current_upstream_rust_agent_catalog() {
    let gate_receipt = marlin_rust_project_harness_policy::RustProjectHarnessGateReceipt {
        package_name: "demo".to_owned(),
        performance_gate: false,
        stability_gate: false,
        performance_report_obligation: false,
        stability_report_obligation: false,
    };
    let findings = evaluate_quality_findings_for_gate(RustProjectHarnessQualityFindingsInput {
        package_name: "demo".to_owned(),
        gate_receipt,
        evidence_paths: RustProjectHarnessQualityFindingEvidencePaths::new(
            "evidence-graph.json",
            "verification_plan.json",
            "task_index.json",
            "verification_policy.json",
        ),
    });

    let projection = &findings.policy_projection;
    assert_eq!(
        *projection,
        project_current_rust_agent_policy_catalog("demo", &findings.findings)
    );
    let upstream_rule_ids = rust_agent_policy_rules()
        .into_iter()
        .map(|rule| rule.rule_id.to_owned())
        .collect::<BTreeSet<_>>();
    let projection_rule_ids = projection
        .projections
        .iter()
        .map(|projection| projection.rule_id.clone())
        .collect::<BTreeSet<_>>();

    assert!(projection.has_complete_upstream_projection());
    assert_eq!(projection.upstream_rule_count, upstream_rule_ids.len());
    assert_eq!(projection_rule_ids, upstream_rule_ids);
    assert_eq!(projection.emitted_count, 0);
    assert_eq!(projection.suppressed_count, projection.upstream_rule_count);
    assert_eq!(projection.not_applicable_count, 0);
    assert!(
        projection
            .projections
            .iter()
            .all(|projection| projection.rule_id.starts_with("RUST-AGENT-"))
    );
    assert!(
        !projection
            .projections
            .iter()
            .any(|projection| projection.rule_id.starts_with("AGENT-R"))
    );

    for rule_id in [
        "RUST-AGENT-ASYNC-BACKPRESSURE-032",
        "RUST-AGENT-TOKIO-RUNTIME-002",
        "RUST-AGENT-NATIVE-ABI-001",
    ] {
        let rule_projection = projection
            .projections
            .iter()
            .find(|projection| projection.rule_id == rule_id)
            .expect("high-value upstream policy should be projected");
        assert_eq!(
            rule_projection.decision,
            RustProjectHarnessPolicyProjectionDecision::Suppressed
        );
        assert!(
            rule_projection
                .reason
                .contains("upstream Rust agent policy catalog")
        );
    }
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
            && item.blocking_level == RustProjectHarnessQualityBlockingLevel::BuildBlocking
            && item.autofixability == RustProjectHarnessQualityAutofixability::ManualPolicyEdit
            && item.next_action.contains("agent-runtime")
            && item.verification_command.contains("cargo test")
    }));
    assert!(queue.items.iter().any(|item| {
        item.quality_domain == RustProjectHarnessQualityDomain::ReportObligation
            && item.expected_artifact
                == RustProjectHarnessExpectedArtifact::PerformanceReportObligation
    }));

    let plan = build_improvement_plan_receipt(&queue);

    assert_eq!(plan.status, RustProjectHarnessImprovementPlanStatus::Ready);
    assert_eq!(plan.step_count(), 4);
    assert_eq!(plan.crate_role, "agent-runtime");
    assert!(
        plan.plan_sources
            .contains(&"improvement_queue.json".to_owned())
    );
    assert_eq!(
        plan.steps[0].expected_artifact,
        RustProjectHarnessExpectedArtifact::PerformanceVerificationTask
    );
    assert!(plan.steps.iter().all(|step| {
        step.patch_boundary.contains("rust-project-harness")
            && step.skip_reason.contains(step.source_rule_id.as_str())
            && step.rollback_reason.contains(&step.verification_command)
    }));

    let performance_gate_step = plan
        .steps
        .iter()
        .find(|step| {
            step.expected_artifact
                == RustProjectHarnessExpectedArtifact::PerformanceVerificationTask
        })
        .expect("plan should include performance verification task step");
    let performance_report_step = plan
        .steps
        .iter()
        .find(|step| {
            step.expected_artifact
                == RustProjectHarnessExpectedArtifact::PerformanceReportObligation
        })
        .expect("plan should include performance report obligation step");
    assert!(
        performance_report_step
            .depends_on
            .contains(&performance_gate_step.improvement_id)
    );
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
    assert_eq!(
        findings.findings[0].domain,
        RustProjectHarnessQualityDomain::RepairEvidence
    );
    assert_eq!(
        findings.findings[0].blocking_level,
        RustProjectHarnessQualityBlockingLevel::NonBlockingAdvice
    );
    assert_eq!(
        findings.findings[0].autofixability,
        RustProjectHarnessQualityAutofixability::EvidenceReadOnly
    );
    assert_eq!(
        findings.findings[0].expected_artifact,
        RustProjectHarnessExpectedArtifact::StructuredEvidenceReview
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

    let plan = build_improvement_plan_receipt(&queue);

    assert!(plan.is_noop());
    assert_eq!(plan.step_count(), 0);
    assert!(plan.steps.is_empty());
}
