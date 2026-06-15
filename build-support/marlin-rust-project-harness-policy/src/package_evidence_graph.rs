//! Package evidence graph receipt owned by Marlin build support.

use std::path::PathBuf;

use rust_lang_project_harness::{
    RustDeterminismReadinessInput, RustEvidenceGraphInput, RustEvidenceGraphSummary,
    RustHarnessConfig, RustHarnessReport, RustReviewPacketInput, build_rust_determinism_readiness,
    build_rust_evidence_graph, build_rust_review_packet,
    plan_rust_project_verification_with_config,
};

use crate::{
    RustProjectHarnessExpectedArtifact, RustProjectHarnessFindingSeverity,
    RustProjectHarnessGateReceipt, RustProjectHarnessImprovementPlanReceipt,
    RustProjectHarnessImprovementQueueReceipt, RustProjectHarnessQualityAutofixability,
    RustProjectHarnessQualityBlockingLevel, RustProjectHarnessQualityDomain,
    RustProjectHarnessQualityFinding, RustProjectHarnessQualityFindingEvidencePaths,
    RustProjectHarnessQualityFindingsInput, RustProjectHarnessQualityFindingsReceipt,
    RustProjectHarnessVerificationPolicyReceipt, build_improvement_plan_receipt,
    build_improvement_queue_receipt, build_verification_policy_receipt,
    evaluate_performance_and_stability_gate, evaluate_quality_findings_for_gate,
};

/// Named request for building a no-write package evidence graph receipt.
#[derive(Clone, Debug)]
pub struct RustProjectHarnessPackageEvidenceGraphRequest<'a> {
    pub config: &'a RustHarnessConfig,
    pub harness_report: RustHarnessReport,
    pub project_root: PathBuf,
    pub package_name: String,
    pub evidence_paths: RustProjectHarnessQualityFindingEvidencePaths,
}

/// No-write package evidence graph receipt owned by Marlin build support.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustProjectHarnessPackageEvidenceGraphReceipt {
    pub package_name: String,
    pub evidence_graph_summary: RustEvidenceGraphSummary,
    pub gate_receipt: RustProjectHarnessGateReceipt,
    pub verification_policy_receipt: RustProjectHarnessVerificationPolicyReceipt,
    pub quality_findings_receipt: RustProjectHarnessQualityFindingsReceipt,
    pub improvement_queue_receipt: RustProjectHarnessImprovementQueueReceipt,
    pub improvement_plan_receipt: RustProjectHarnessImprovementPlanReceipt,
}

impl RustProjectHarnessPackageEvidenceGraphReceipt {
    /// Returns true when the package has inspectable graph evidence and passing package gates.
    pub fn is_success(&self) -> bool {
        self.evidence_graph_summary.nodes > 0
            && self.gate_receipt.is_success()
            && self.quality_findings_receipt.hard_error_count() == 0
            && self.improvement_queue_receipt.is_healthy()
            && self.improvement_plan_receipt.is_noop()
    }
}

/// Builds a package evidence graph receipt without writing artifacts from crate source.
pub fn build_package_evidence_graph_receipt(
    request: RustProjectHarnessPackageEvidenceGraphRequest<'_>,
) -> RustProjectHarnessPackageEvidenceGraphReceipt {
    let determinism_readiness = build_rust_determinism_readiness(RustDeterminismReadinessInput {
        project_root: request.project_root.clone(),
        include_tests: request.config.include_tests,
    })
    .unwrap_or_else(|error| panic!("rust determinism readiness failed: {error}"));
    let determinism_observation_count = determinism_readiness.observations.len();
    let review_packet = build_rust_review_packet(RustReviewPacketInput {
        project_root: request.project_root.clone(),
        report: request.harness_report,
        receipts: Vec::new(),
        behavior_snapshots: Vec::new(),
        determinism_readiness: vec![determinism_readiness],
        proof_pilots: Vec::new(),
        waivers: Vec::new(),
    });
    let evidence_graph = build_rust_evidence_graph(RustEvidenceGraphInput {
        project_root: request.project_root.clone(),
        review_packets: vec![review_packet],
    });
    let verification_plan =
        plan_rust_project_verification_with_config(&request.project_root, request.config)
            .unwrap_or_else(|error| panic!("rust verification plan failed: {error}"));
    let gate_receipt =
        evaluate_performance_and_stability_gate(&verification_plan, request.package_name.clone());
    let mut quality_findings_receipt =
        evaluate_quality_findings_for_gate(RustProjectHarnessQualityFindingsInput {
            package_name: request.package_name.clone(),
            gate_receipt: gate_receipt.clone(),
            evidence_paths: request.evidence_paths,
        });

    append_artifact_findings(
        &mut quality_findings_receipt,
        &request.package_name,
        evidence_graph.summary.nodes,
        determinism_observation_count,
    );

    let verification_policy_receipt = build_verification_policy_receipt(
        request.package_name.clone(),
        &request.project_root,
        request.config,
        &verification_plan,
    );
    let improvement_queue_receipt =
        build_improvement_queue_receipt(&quality_findings_receipt, &verification_policy_receipt);
    let improvement_plan_receipt = build_improvement_plan_receipt(&improvement_queue_receipt);

    RustProjectHarnessPackageEvidenceGraphReceipt {
        package_name: request.package_name,
        evidence_graph_summary: evidence_graph.summary,
        gate_receipt,
        verification_policy_receipt,
        quality_findings_receipt,
        improvement_queue_receipt,
        improvement_plan_receipt,
    }
}

fn append_artifact_findings(
    receipt: &mut RustProjectHarnessQualityFindingsReceipt,
    package_name: &str,
    evidence_graph_nodes: usize,
    determinism_observation_count: usize,
) {
    if evidence_graph_nodes == 0 {
        receipt.findings.push(artifact_warning(
            package_name,
            "evidence-graph-empty",
            "MARLIN-QUALITY-EVIDENCE-GRAPH",
            "evidence-graph",
            "the emitted evidence graph has no nodes for the agent to inspect",
            "inspect upstream rust-harness graph inputs before editing Marlin policy",
            RustProjectHarnessExpectedArtifact::EvidenceGraph,
        ));
    }
    if determinism_observation_count == 0 {
        receipt.findings.push(artifact_warning(
            package_name,
            "determinism-observations-empty",
            "MARLIN-QUALITY-DETERMINISM",
            "determinism-readiness",
            "the determinism readiness packet contains no observations",
            "inspect language harness determinism inputs and package ownership boundaries",
            RustProjectHarnessExpectedArtifact::DeterminismObservation,
        ));
    }
}

fn artifact_warning(
    package_name: &str,
    finding_suffix: &str,
    rule_id: &str,
    evidence: &str,
    why: &str,
    agent_next_action: &str,
    expected_artifact: RustProjectHarnessExpectedArtifact,
) -> RustProjectHarnessQualityFinding {
    RustProjectHarnessQualityFinding {
        finding_id: format!("{package_name}:{finding_suffix}"),
        severity: RustProjectHarnessFindingSeverity::Warning,
        domain: RustProjectHarnessQualityDomain::RepairEvidence,
        blocking_level: RustProjectHarnessQualityBlockingLevel::NonBlockingAdvice,
        autofixability: RustProjectHarnessQualityAutofixability::EvidenceReadOnly,
        expected_artifact,
        rule_id: rule_id.to_owned(),
        owner: package_name.to_owned(),
        evidence: vec![evidence.to_owned()],
        why: why.to_owned(),
        agent_next_action: agent_next_action.to_owned(),
        verification_command: "cargo test -p marlin-rust-project-harness-policy --quiet".to_owned(),
        source_authority: "marlin-rust-project-harness-policy".to_owned(),
    }
}
