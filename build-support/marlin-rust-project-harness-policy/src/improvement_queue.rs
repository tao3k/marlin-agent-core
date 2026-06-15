//! Reflection-derived improvement queue for Rust harness evidence.

use serde::Serialize;
use std::fmt;

use crate::{
    quality_findings::{
        RustProjectHarnessExpectedArtifact, RustProjectHarnessFindingSeverity,
        RustProjectHarnessQualityAutofixability, RustProjectHarnessQualityBlockingLevel,
        RustProjectHarnessQualityDomain, RustProjectHarnessQualityFinding,
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

/// Status for a compiled engineering improvement plan.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RustProjectHarnessImprovementPlanStatus {
    Noop,
    Ready,
}

/// Stable identifier for a compiled improvement plan step.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize)]
#[serde(transparent)]
pub struct RustProjectHarnessImprovementStepId(String);

impl RustProjectHarnessImprovementStepId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for RustProjectHarnessImprovementStepId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Stable identifier for one Rust project harness improvement.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize)]
#[serde(transparent)]
pub struct RustProjectHarnessImprovementId(String);

impl RustProjectHarnessImprovementId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for RustProjectHarnessImprovementId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Stable rule identifier for the finding that produced an improvement.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize)]
#[serde(transparent)]
pub struct RustProjectHarnessRuleId(String);

impl RustProjectHarnessRuleId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for RustProjectHarnessRuleId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// One concrete improvement that should be acted on before adding new surface.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RustProjectHarnessImprovementItem {
    pub improvement_id: RustProjectHarnessImprovementId,
    pub priority: RustProjectHarnessImprovementPriority,
    pub owner: String,
    pub crate_role: String,
    pub source_rule_id: RustProjectHarnessRuleId,
    pub quality_domain: RustProjectHarnessQualityDomain,
    pub blocking_level: RustProjectHarnessQualityBlockingLevel,
    pub autofixability: RustProjectHarnessQualityAutofixability,
    pub expected_artifact: RustProjectHarnessExpectedArtifact,
    pub problem: String,
    pub repair_objective: String,
    pub next_action: String,
    pub verification_command: String,
    pub evidence: Vec<String>,
    pub source_authority: String,
}

/// One ordered engineering repair step compiled from the improvement queue.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RustProjectHarnessImprovementPlanStep {
    pub step_id: RustProjectHarnessImprovementStepId,
    pub sequence: usize,
    pub improvement_id: RustProjectHarnessImprovementId,
    pub priority: RustProjectHarnessImprovementPriority,
    pub owner: String,
    pub crate_role: String,
    pub source_rule_id: RustProjectHarnessRuleId,
    pub quality_domain: RustProjectHarnessQualityDomain,
    pub blocking_level: RustProjectHarnessQualityBlockingLevel,
    pub autofixability: RustProjectHarnessQualityAutofixability,
    pub expected_artifact: RustProjectHarnessExpectedArtifact,
    pub patch_boundary: String,
    pub repair_objective: String,
    pub verification_command: String,
    pub depends_on: Vec<RustProjectHarnessImprovementId>,
    pub skip_reason: String,
    pub rollback_reason: String,
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

/// Compiled plan that turns quality findings into ordered Rust engineering work.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RustProjectHarnessImprovementPlanReceipt {
    pub schema_id: String,
    pub schema_version: String,
    pub package_name: String,
    pub crate_role: String,
    pub status: RustProjectHarnessImprovementPlanStatus,
    pub queue_status: RustProjectHarnessImprovementQueueStatus,
    pub plan_sources: Vec<String>,
    pub steps: Vec<RustProjectHarnessImprovementPlanStep>,
}

impl RustProjectHarnessImprovementQueueReceipt {
    pub fn action_required_count(&self) -> usize {
        self.items.len()
    }

    pub fn is_healthy(&self) -> bool {
        self.status == RustProjectHarnessImprovementQueueStatus::Healthy
    }
}

impl RustProjectHarnessImprovementPlanReceipt {
    pub fn step_count(&self) -> usize {
        self.steps.len()
    }

    pub fn is_noop(&self) -> bool {
        self.status == RustProjectHarnessImprovementPlanStatus::Noop
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

/// Compiles an improvement queue into an ordered Rust engineering repair plan.
pub fn build_improvement_plan_receipt(
    queue: &RustProjectHarnessImprovementQueueReceipt,
) -> RustProjectHarnessImprovementPlanReceipt {
    let mut items = queue.items.clone();
    items.sort_by_key(|item| {
        (
            improvement_priority_rank(item.priority),
            expected_artifact_rank(item.expected_artifact),
            item.improvement_id.clone(),
        )
    });

    let steps = items
        .iter()
        .enumerate()
        .map(|(index, item)| improvement_plan_step_from_item(index + 1, item, queue))
        .collect::<Vec<_>>();
    let status = if steps.is_empty() {
        RustProjectHarnessImprovementPlanStatus::Noop
    } else {
        RustProjectHarnessImprovementPlanStatus::Ready
    };

    RustProjectHarnessImprovementPlanReceipt {
        schema_id: "marlin.rust-project-harness.improvement-plan".to_owned(),
        schema_version: "1".to_owned(),
        package_name: queue.package_name.clone(),
        crate_role: queue.crate_role.clone(),
        status,
        queue_status: queue.status,
        plan_sources: vec![
            "quality_findings.json".to_owned(),
            "verification_policy.json".to_owned(),
            "improvement_queue.json".to_owned(),
        ],
        steps,
    }
}

fn improvement_item_from_finding(
    finding: &RustProjectHarnessQualityFinding,
    verification_policy: &RustProjectHarnessVerificationPolicyReceipt,
) -> Option<RustProjectHarnessImprovementItem> {
    let priority = improvement_priority_for_finding(finding)?;
    Some(RustProjectHarnessImprovementItem {
        improvement_id: RustProjectHarnessImprovementId::new(format!(
            "{}:{}",
            finding.owner, finding.rule_id
        )),
        priority,
        owner: finding.owner.clone(),
        crate_role: verification_policy.crate_role.clone(),
        source_rule_id: RustProjectHarnessRuleId::new(finding.rule_id.clone()),
        quality_domain: finding.domain,
        blocking_level: finding.blocking_level,
        autofixability: finding.autofixability,
        expected_artifact: finding.expected_artifact,
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

fn improvement_plan_step_from_item(
    sequence: usize,
    item: &RustProjectHarnessImprovementItem,
    queue: &RustProjectHarnessImprovementQueueReceipt,
) -> RustProjectHarnessImprovementPlanStep {
    RustProjectHarnessImprovementPlanStep {
        step_id: RustProjectHarnessImprovementStepId::new(format!(
            "{}:step:{sequence}",
            queue.package_name
        )),
        sequence,
        improvement_id: item.improvement_id.clone(),
        priority: item.priority,
        owner: item.owner.clone(),
        crate_role: item.crate_role.clone(),
        source_rule_id: item.source_rule_id.clone(),
        quality_domain: item.quality_domain,
        blocking_level: item.blocking_level,
        autofixability: item.autofixability,
        expected_artifact: item.expected_artifact,
        patch_boundary: patch_boundary_for_item(item),
        repair_objective: item.repair_objective.clone(),
        verification_command: item.verification_command.clone(),
        depends_on: dependency_ids_for_item(item, queue),
        skip_reason: skip_reason_for_item(item),
        rollback_reason: rollback_reason_for_item(item),
        evidence: item.evidence.clone(),
        source_authority: item.source_authority.clone(),
    }
}

fn patch_boundary_for_item(item: &RustProjectHarnessImprovementItem) -> String {
    match item.expected_artifact {
        RustProjectHarnessExpectedArtifact::PerformanceVerificationTask
        | RustProjectHarnessExpectedArtifact::StabilityVerificationTask => {
            format!(
                "{} rust-project-harness verification task policy",
                item.owner
            )
        }
        RustProjectHarnessExpectedArtifact::PerformanceReportObligation
        | RustProjectHarnessExpectedArtifact::StabilityReportObligation => {
            format!(
                "{} rust-project-harness verification report obligations",
                item.owner
            )
        }
        RustProjectHarnessExpectedArtifact::EvidenceGraph
        | RustProjectHarnessExpectedArtifact::DeterminismObservation
        | RustProjectHarnessExpectedArtifact::StructuredEvidenceReview => {
            format!("{} rust-project-harness evidence inputs", item.owner)
        }
        RustProjectHarnessExpectedArtifact::GerbilRuntimeAssets => {
            format!("{} gerbil runtime asset root", item.owner)
        }
    }
}

fn dependency_ids_for_item(
    item: &RustProjectHarnessImprovementItem,
    queue: &RustProjectHarnessImprovementQueueReceipt,
) -> Vec<RustProjectHarnessImprovementId> {
    let dependency_artifact = match item.expected_artifact {
        RustProjectHarnessExpectedArtifact::PerformanceReportObligation => {
            Some(RustProjectHarnessExpectedArtifact::PerformanceVerificationTask)
        }
        RustProjectHarnessExpectedArtifact::StabilityReportObligation => {
            Some(RustProjectHarnessExpectedArtifact::StabilityVerificationTask)
        }
        _ => None,
    };

    dependency_artifact
        .into_iter()
        .flat_map(|expected_artifact| {
            queue
                .items
                .iter()
                .filter(move |candidate| candidate.expected_artifact == expected_artifact)
        })
        .map(|candidate| candidate.improvement_id.clone())
        .collect()
}

fn skip_reason_for_item(item: &RustProjectHarnessImprovementItem) -> String {
    match item.autofixability {
        RustProjectHarnessQualityAutofixability::ManualPolicyEdit => format!(
            "skip when {} no longer appears in quality_findings.json",
            item.source_rule_id
        ),
        RustProjectHarnessQualityAutofixability::EvidenceReadOnly => format!(
            "skip when the expected {:?} evidence is already present",
            item.expected_artifact
        ),
    }
}

fn rollback_reason_for_item(item: &RustProjectHarnessImprovementItem) -> String {
    format!(
        "revert edits inside `{}` if `{}` still fails after running `{}`",
        patch_boundary_for_item(item),
        item.source_rule_id,
        item.verification_command,
    )
}

fn improvement_priority_rank(priority: RustProjectHarnessImprovementPriority) -> u8 {
    match priority {
        RustProjectHarnessImprovementPriority::Critical => 0,
        RustProjectHarnessImprovementPriority::High => 1,
        RustProjectHarnessImprovementPriority::Medium => 2,
        RustProjectHarnessImprovementPriority::Low => 3,
    }
}

fn expected_artifact_rank(expected_artifact: RustProjectHarnessExpectedArtifact) -> u8 {
    match expected_artifact {
        RustProjectHarnessExpectedArtifact::PerformanceVerificationTask => 0,
        RustProjectHarnessExpectedArtifact::StabilityVerificationTask => 1,
        RustProjectHarnessExpectedArtifact::PerformanceReportObligation => 2,
        RustProjectHarnessExpectedArtifact::StabilityReportObligation => 3,
        RustProjectHarnessExpectedArtifact::GerbilRuntimeAssets => 4,
        RustProjectHarnessExpectedArtifact::EvidenceGraph => 5,
        RustProjectHarnessExpectedArtifact::DeterminismObservation => 6,
        RustProjectHarnessExpectedArtifact::StructuredEvidenceReview => 7,
    }
}
