//! Reflection-derived improvement queue for Rust harness evidence.

use serde::Serialize;

use crate::{
    quality_findings::{
        RustProjectHarnessFindingSeverity, RustProjectHarnessQualityFinding,
        RustProjectHarnessQualityFindingsReceipt,
    },
    verification_policy::RustProjectHarnessVerificationPolicyReceipt,
};

/// Agent-facing status for reflection-derived engineering improvements.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RustProjectHarnessImprovementQueueStatus {
    Healthy,
    ActionRequired,
}

/// Priority assigned to one reflection-derived improvement item.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RustProjectHarnessImprovementPriority {
    Critical,
    High,
    Medium,
    Low,
}

/// One concrete improvement that should be acted on before adding new surface.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RustProjectHarnessImprovementItem {
    pub improvement_id: String,
    pub priority: RustProjectHarnessImprovementPriority,
    pub owner: String,
    pub crate_role: String,
    pub source_rule_id: String,
    pub problem: String,
    pub repair_objective: String,
    pub next_action: String,
    pub verification_command: String,
    pub evidence: Vec<String>,
    pub source_authority: String,
}

/// Queue of concrete engineering improvements discovered from reflection evidence.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RustProjectHarnessImprovementQueueReceipt {
    pub schema_id: String,
    pub schema_version: String,
    pub package_name: String,
    pub crate_role: String,
    pub status: RustProjectHarnessImprovementQueueStatus,
    pub reflection_sources: Vec<String>,
    pub items: Vec<RustProjectHarnessImprovementItem>,
}

impl RustProjectHarnessImprovementQueueReceipt {
    pub fn action_required_count(&self) -> usize {
        self.items.len()
    }

    pub fn is_healthy(&self) -> bool {
        self.status == RustProjectHarnessImprovementQueueStatus::Healthy
    }
}

/// Converts quality findings and role policy reflection into an improvement queue.
pub fn build_improvement_queue_receipt(
    quality_findings: &RustProjectHarnessQualityFindingsReceipt,
    verification_policy: &RustProjectHarnessVerificationPolicyReceipt,
) -> RustProjectHarnessImprovementQueueReceipt {
    let items = quality_findings
        .findings
        .iter()
        .filter_map(|finding| improvement_item_from_finding(finding, verification_policy))
        .collect::<Vec<_>>();
    let status = if items.is_empty() {
        RustProjectHarnessImprovementQueueStatus::Healthy
    } else {
        RustProjectHarnessImprovementQueueStatus::ActionRequired
    };

    RustProjectHarnessImprovementQueueReceipt {
        schema_id: "marlin.rust-project-harness.improvement-queue".to_owned(),
        schema_version: "1".to_owned(),
        package_name: quality_findings.package_name.clone(),
        crate_role: verification_policy.crate_role.clone(),
        status,
        reflection_sources: vec![
            "quality_findings.json".to_owned(),
            "verification_policy.json".to_owned(),
        ],
        items,
    }
}

fn improvement_item_from_finding(
    finding: &RustProjectHarnessQualityFinding,
    verification_policy: &RustProjectHarnessVerificationPolicyReceipt,
) -> Option<RustProjectHarnessImprovementItem> {
    let priority = improvement_priority_for_finding(finding)?;
    Some(RustProjectHarnessImprovementItem {
        improvement_id: format!("{}:{}", finding.owner, finding.rule_id),
        priority,
        owner: finding.owner.clone(),
        crate_role: verification_policy.crate_role.clone(),
        source_rule_id: finding.rule_id.clone(),
        problem: finding.why.clone(),
        repair_objective: finding.agent_next_action.clone(),
        next_action: format!(
            "repair {owner} for {role} policy, then run: {command}",
            owner = finding.owner,
            role = verification_policy.crate_role,
            command = finding.verification_command,
        ),
        verification_command: finding.verification_command.clone(),
        evidence: finding.evidence.clone(),
        source_authority: finding.source_authority.clone(),
    })
}

fn improvement_priority_for_finding(
    finding: &RustProjectHarnessQualityFinding,
) -> Option<RustProjectHarnessImprovementPriority> {
    match finding.severity {
        RustProjectHarnessFindingSeverity::HardError => {
            Some(RustProjectHarnessImprovementPriority::Critical)
        }
        RustProjectHarnessFindingSeverity::Warning => {
            Some(RustProjectHarnessImprovementPriority::High)
        }
        RustProjectHarnessFindingSeverity::Advice => actionable_advice_priority(finding),
    }
}

fn actionable_advice_priority(
    finding: &RustProjectHarnessQualityFinding,
) -> Option<RustProjectHarnessImprovementPriority> {
    (finding.rule_id != "MARLIN-QUALITY-AGENT-EVIDENCE")
        .then_some(RustProjectHarnessImprovementPriority::Low)
}
