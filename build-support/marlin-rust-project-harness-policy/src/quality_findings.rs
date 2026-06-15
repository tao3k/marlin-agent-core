//! Agent-actionable quality findings derived from Rust harness gates.

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

/// One structured finding that an agent can reason over without parsing prose.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RustProjectHarnessQualityFinding {
    pub finding_id: String,
    pub severity: RustProjectHarnessFindingSeverity,
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
        "MARLIN-QUALITY-GATE-PERF",
        "performance-gate",
        "active performance verification task",
        "add or enable a performance verification task for this package policy",
    );
    push_gate_finding(
        &mut findings,
        &package_name,
        gate_receipt.stability_gate,
        "MARLIN-QUALITY-GATE-STABILITY",
        "stability-gate",
        "active stability verification task",
        "add or enable a stability verification task for this package policy",
    );
    push_gate_finding(
        &mut findings,
        &package_name,
        gate_receipt.performance_report_obligation,
        "MARLIN-QUALITY-REPORT-PERF",
        "performance-report-obligation",
        "performance_index_json report obligation",
        "add performance_index_json to the package verification report obligations",
    );
    push_gate_finding(
        &mut findings,
        &package_name,
        gate_receipt.stability_report_obligation,
        "MARLIN-QUALITY-REPORT-STABILITY",
        "stability-report-obligation",
        "stability_index_json report obligation",
        "add stability_index_json to the package verification report obligations",
    );

    findings.push(RustProjectHarnessQualityFinding {
        finding_id: format!("{package_name}:agent-read-evidence"),
        severity: RustProjectHarnessFindingSeverity::Advice,
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

    RustProjectHarnessQualityFindingsReceipt {
        schema_id: "marlin.rust-project-harness.quality-findings".to_owned(),
        schema_version: "1".to_owned(),
        package_name,
        findings,
    }
}

fn push_gate_finding(
    findings: &mut Vec<RustProjectHarnessQualityFinding>,
    package_name: &str,
    gate_present: bool,
    rule_id: &str,
    finding_suffix: &str,
    evidence_label: &str,
    agent_next_action: &str,
) {
    if gate_present {
        return;
    }

    findings.push(RustProjectHarnessQualityFinding {
        finding_id: format!("{package_name}:{finding_suffix}"),
        severity: RustProjectHarnessFindingSeverity::HardError,
        rule_id: rule_id.to_owned(),
        owner: package_name.to_owned(),
        evidence: vec![evidence_label.to_owned()],
        why: format!("package quality gate is missing {evidence_label}"),
        agent_next_action: agent_next_action.to_owned(),
        verification_command: "cargo test -p marlin-rust-project-harness-policy --quiet".to_owned(),
        source_authority: "marlin-rust-project-harness-policy".to_owned(),
    });
}
