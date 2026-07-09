//! Agent-actionable quality findings derived from Rust harness gates.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use serde::Serialize;

use crate::quality_gate::RustProjectHarnessGateReceipt;

/// Severity taxonomy for agent-actionable quality findings.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RustProjectHarnessFindingSeverity {
    HardError,
    Warning,
    Advice,
}

/// Engineering domain for one Rust project harness quality finding.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RustProjectHarnessQualityDomain {
    VerificationGate,
    ReportObligation,
    RepairEvidence,
}

/// Whether the finding blocks the Rust engineering gate.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RustProjectHarnessQualityBlockingLevel {
    BuildBlocking,
    NonBlockingAdvice,
}

/// How the finding should be repaired by the Rust engineering plane.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RustProjectHarnessQualityAutofixability {
    ManualPolicyEdit,
    EvidenceReadOnly,
}

/// Expected artifact that closes one quality finding.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RustProjectHarnessExpectedArtifact {
    PerformanceVerificationTask,
    StabilityVerificationTask,
    PerformanceReportObligation,
    StabilityReportObligation,
    EvidenceGraph,
    DeterminismObservation,
    GerbilRuntimeAssets,
    StructuredEvidenceReview,
}

/// One structured finding that an agent can reason over without parsing prose.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RustProjectHarnessQualityFinding {
    pub finding_id: String,
    pub severity: RustProjectHarnessFindingSeverity,
    pub domain: RustProjectHarnessQualityDomain,
    pub blocking_level: RustProjectHarnessQualityBlockingLevel,
    pub autofixability: RustProjectHarnessQualityAutofixability,
    pub expected_artifact: RustProjectHarnessExpectedArtifact,
    pub rule_id: String,
    pub owner: String,
    pub evidence: Vec<String>,
    pub why: String,
    pub agent_next_action: String,
    pub verification_command: String,
    pub source_authority: String,
}

/// JSON receipt containing hard errors, warnings, and advice for the package gate.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RustProjectHarnessQualityFindingsReceipt {
    pub schema_id: String,
    pub schema_version: String,
    pub package_name: String,
    pub findings: Vec<RustProjectHarnessQualityFinding>,
    pub policy_projection: RustProjectHarnessPolicyProjectionReceipt,
}

/// How Marlin projected one upstream Rust agent policy into its package gate.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RustProjectHarnessPolicyProjectionDecision {
    Emitted,
    Suppressed,
    NotApplicable,
}

/// Projection evidence for one upstream `RUST-AGENT-*` rule.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RustProjectHarnessPolicyProjection {
    pub rule_id: String,
    pub decision: RustProjectHarnessPolicyProjectionDecision,
    pub quality_finding_id: Option<String>,
    pub reason: String,
    pub source_authority: String,
}

/// Complete upstream Rust policy catalog projection for one Marlin package gate.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RustProjectHarnessPolicyProjectionReceipt {
    pub schema_id: String,
    pub schema_version: String,
    pub package_name: String,
    pub upstream_rule_count: usize,
    pub emitted_count: usize,
    pub suppressed_count: usize,
    pub not_applicable_count: usize,
    pub missing_rule_ids: Vec<String>,
    pub projections: Vec<RustProjectHarnessPolicyProjection>,
}

/// Named input for evaluating package-level gate state into structured findings.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustProjectHarnessQualityFindingsInput {
    pub package_name: String,
    pub gate_receipt: RustProjectHarnessGateReceipt,
    pub evidence_paths: RustProjectHarnessQualityFindingEvidencePaths,
}

/// Artifact paths exposed to agents as structured evidence handles.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustProjectHarnessQualityFindingEvidencePaths {
    evidence_graph_path: PathBuf,
    verification_plan_path: PathBuf,
    task_index_path: PathBuf,
    verification_policy_path: PathBuf,
}

impl RustProjectHarnessQualityFindingEvidencePaths {
    pub fn new(
        evidence_graph_path: impl Into<PathBuf>,
        verification_plan_path: impl Into<PathBuf>,
        task_index_path: impl Into<PathBuf>,
        verification_policy_path: impl Into<PathBuf>,
    ) -> Self {
        Self {
            evidence_graph_path: evidence_graph_path.into(),
            verification_plan_path: verification_plan_path.into(),
            task_index_path: task_index_path.into(),
            verification_policy_path: verification_policy_path.into(),
        }
    }

    pub fn agent_evidence(&self) -> Vec<String> {
        vec![
            self.evidence_graph_path.display().to_string(),
            self.verification_plan_path.display().to_string(),
            self.task_index_path.display().to_string(),
            self.verification_policy_path.display().to_string(),
        ]
    }
}

impl RustProjectHarnessQualityFindingsReceipt {
    pub fn hard_error_count(&self) -> usize {
        self.findings
            .iter()
            .filter(|finding| finding.severity == RustProjectHarnessFindingSeverity::HardError)
            .count()
    }

    pub fn warning_count(&self) -> usize {
        self.findings
            .iter()
            .filter(|finding| finding.severity == RustProjectHarnessFindingSeverity::Warning)
            .count()
    }

    pub fn advice_count(&self) -> usize {
        self.findings
            .iter()
            .filter(|finding| finding.severity == RustProjectHarnessFindingSeverity::Advice)
            .count()
    }
}

impl RustProjectHarnessPolicyProjectionReceipt {
    pub fn has_complete_upstream_projection(&self) -> bool {
        self.missing_rule_ids.is_empty() && self.upstream_rule_count == self.projections.len()
    }
}

/// Projects the current upstream Rust agent policy catalog into Marlin gate evidence.
#[must_use]
pub fn project_current_rust_agent_policy_catalog(
    package_name: impl Into<String>,
    findings: &[RustProjectHarnessQualityFinding],
) -> RustProjectHarnessPolicyProjectionReceipt {
    let package_name = package_name.into();
    let upstream_rules = rust_lang_project_harness::rust_agent_policy_rules();
    let finding_by_rule_id = findings
        .iter()
        .map(|finding| (finding.rule_id.as_str(), finding.finding_id.as_str()))
        .collect::<BTreeMap<_, _>>();

    let mut projected_rule_ids = BTreeSet::new();
    let projections = upstream_rules
        .iter()
        .map(|rule| {
            projected_rule_ids.insert(rule.rule_id.to_owned());
            if let Some(finding_id) = finding_by_rule_id.get(rule.rule_id) {
                RustProjectHarnessPolicyProjection {
                    rule_id: rule.rule_id.to_owned(),
                    decision: RustProjectHarnessPolicyProjectionDecision::Emitted,
                    quality_finding_id: Some((*finding_id).to_owned()),
                    reason: "projected from an emitted Marlin quality finding".to_owned(),
                    source_authority: "rust-lang-project-harness".to_owned(),
                }
            } else {
                RustProjectHarnessPolicyProjection {
                    rule_id: rule.rule_id.to_owned(),
                    decision: RustProjectHarnessPolicyProjectionDecision::Suppressed,
                    quality_finding_id: None,
                    reason: "available in upstream Rust agent policy catalog but not emitted by the package-level Marlin gate"
                        .to_owned(),
                    source_authority: "rust-lang-project-harness".to_owned(),
                }
            }
        })
        .collect::<Vec<_>>();

    let missing_rule_ids = upstream_rules
        .iter()
        .map(|rule| rule.rule_id.to_owned())
        .filter(|rule_id| !projected_rule_ids.contains(rule_id))
        .collect::<Vec<_>>();

    let emitted_count = projections
        .iter()
        .filter(|projection| {
            projection.decision == RustProjectHarnessPolicyProjectionDecision::Emitted
        })
        .count();
    let suppressed_count = projections
        .iter()
        .filter(|projection| {
            projection.decision == RustProjectHarnessPolicyProjectionDecision::Suppressed
        })
        .count();
    let not_applicable_count = projections
        .iter()
        .filter(|projection| {
            projection.decision == RustProjectHarnessPolicyProjectionDecision::NotApplicable
        })
        .count();

    RustProjectHarnessPolicyProjectionReceipt {
        schema_id: "marlin.rust-project-harness.policy-projection".to_owned(),
        schema_version: "1".to_owned(),
        package_name,
        upstream_rule_count: upstream_rules.len(),
        emitted_count,
        suppressed_count,
        not_applicable_count,
        missing_rule_ids,
        projections,
    }
}

/// Evaluates package-level gate state into structured findings for agent repair.
pub fn evaluate_quality_findings_for_gate(
    input: RustProjectHarnessQualityFindingsInput,
) -> RustProjectHarnessQualityFindingsReceipt {
    let RustProjectHarnessQualityFindingsInput {
        package_name,
        gate_receipt,
        evidence_paths,
    } = input;
    let mut findings = Vec::new();

    push_gate_finding(
        &mut findings,
        &package_name,
        gate_receipt.performance_gate,
        GateFindingSpec {
            rule_id: "MARLIN-QUALITY-GATE-PERF",
            finding_suffix: "performance-gate",
            evidence_label: "active performance verification task",
            agent_next_action: "add or enable a performance verification task for this package policy",
            domain: RustProjectHarnessQualityDomain::VerificationGate,
            expected_artifact: RustProjectHarnessExpectedArtifact::PerformanceVerificationTask,
        },
    );
    push_gate_finding(
        &mut findings,
        &package_name,
        gate_receipt.stability_gate,
        GateFindingSpec {
            rule_id: "MARLIN-QUALITY-GATE-STABILITY",
            finding_suffix: "stability-gate",
            evidence_label: "active stability verification task",
            agent_next_action: "add or enable a stability verification task for this package policy",
            domain: RustProjectHarnessQualityDomain::VerificationGate,
            expected_artifact: RustProjectHarnessExpectedArtifact::StabilityVerificationTask,
        },
    );
    push_gate_finding(
        &mut findings,
        &package_name,
        gate_receipt.performance_report_obligation,
        GateFindingSpec {
            rule_id: "MARLIN-QUALITY-REPORT-PERF",
            finding_suffix: "performance-report-obligation",
            evidence_label: "performance_index_json report obligation",
            agent_next_action: "add performance_index_json to the package verification report obligations",
            domain: RustProjectHarnessQualityDomain::ReportObligation,
            expected_artifact: RustProjectHarnessExpectedArtifact::PerformanceReportObligation,
        },
    );
    push_gate_finding(
        &mut findings,
        &package_name,
        gate_receipt.stability_report_obligation,
        GateFindingSpec {
            rule_id: "MARLIN-QUALITY-REPORT-STABILITY",
            finding_suffix: "stability-report-obligation",
            evidence_label: "stability_index_json report obligation",
            agent_next_action: "add stability_index_json to the package verification report obligations",
            domain: RustProjectHarnessQualityDomain::ReportObligation,
            expected_artifact: RustProjectHarnessExpectedArtifact::StabilityReportObligation,
        },
    );

    findings.push(RustProjectHarnessQualityFinding {
        finding_id: format!("{package_name}:agent-read-evidence"),
        severity: RustProjectHarnessFindingSeverity::Advice,
        domain: RustProjectHarnessQualityDomain::RepairEvidence,
        blocking_level: RustProjectHarnessQualityBlockingLevel::NonBlockingAdvice,
        autofixability: RustProjectHarnessQualityAutofixability::EvidenceReadOnly,
        expected_artifact: RustProjectHarnessExpectedArtifact::StructuredEvidenceReview,
        rule_id: "MARLIN-QUALITY-AGENT-EVIDENCE".to_owned(),
        owner: package_name.clone(),
        evidence: evidence_paths.agent_evidence(),
        why: "agent repair should reason from structured evidence before editing package code"
            .to_owned(),
        agent_next_action:
            "read evidence-graph.json, verification_plan.json, task_index.json, and verification_policy.json before selecting an edit boundary"
                .to_owned(),
        verification_command: "cargo test --workspace --no-fail-fast --quiet".to_owned(),
        source_authority: "marlin-rust-project-harness-policy".to_owned(),
    });

    let policy_projection = project_current_rust_agent_policy_catalog(&package_name, &findings);

    RustProjectHarnessQualityFindingsReceipt {
        schema_id: "marlin.rust-project-harness.quality-findings".to_owned(),
        schema_version: "1".to_owned(),
        package_name,
        findings,
        policy_projection,
    }
}

struct GateFindingSpec<'a> {
    rule_id: &'a str,
    finding_suffix: &'a str,
    evidence_label: &'a str,
    agent_next_action: &'a str,
    domain: RustProjectHarnessQualityDomain,
    expected_artifact: RustProjectHarnessExpectedArtifact,
}

fn push_gate_finding(
    findings: &mut Vec<RustProjectHarnessQualityFinding>,
    package_name: &str,
    gate_present: bool,
    spec: GateFindingSpec<'_>,
) {
    if gate_present {
        return;
    }

    findings.push(RustProjectHarnessQualityFinding {
        finding_id: format!("{}:{}", package_name, spec.finding_suffix),
        severity: RustProjectHarnessFindingSeverity::HardError,
        domain: spec.domain,
        blocking_level: RustProjectHarnessQualityBlockingLevel::BuildBlocking,
        autofixability: RustProjectHarnessQualityAutofixability::ManualPolicyEdit,
        expected_artifact: spec.expected_artifact,
        rule_id: spec.rule_id.to_owned(),
        owner: package_name.to_owned(),
        evidence: vec![spec.evidence_label.to_owned()],
        why: format!("package quality gate is missing {}", spec.evidence_label),
        agent_next_action: spec.agent_next_action.to_owned(),
        verification_command: "cargo test -p marlin-rust-project-harness-policy --quiet".to_owned(),
        source_authority: "marlin-rust-project-harness-policy".to_owned(),
    });
}
