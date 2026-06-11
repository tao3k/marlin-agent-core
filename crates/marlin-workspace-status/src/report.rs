//! Status report structures derived from workspace records.

use marlin_agent_protocol::{LoopEvidence, LoopEvidenceKind};
use marlin_gerbil_ir::ReleaseTopologySpec;
use marlin_org_model::{
    OrgContractDiagnostic, OrgContractRegistry, OrgContractResolution, OrgContractTemplate,
    OrgContractValidationReport, OrgNodeId,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Combined status projection for a workspace target.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceStatusReport {
    pub goal: Option<GoalStatus>,
    pub sdd: Option<SddStatus>,
    pub checklist: Option<ChecklistStatus>,
    pub evidence: Option<EvidenceStatus>,
    pub contracts: Option<ContractStatus>,
    pub patch: Option<PatchStatus>,
    #[serde(default)]
    pub release: Option<ReleaseStatus>,
    pub metrics: Vec<MetricTrace>,
    pub decisions: DecisionTrace,
    pub next_actions: Vec<String>,
}

/// Goal-level progress and blockers.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GoalStatus {
    pub title: String,
    pub state: GoalState,
    pub open_blockers: Vec<String>,
}

/// Lifecycle state for a workspace goal.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GoalState {
    Todo,
    Next,
    Active,
    Waiting,
    Blocked,
    Done,
    Archived,
    Custom(String),
}

/// Specification-driven-development status.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SddStatus {
    pub title: String,
    pub accepted: bool,
    pub missing_evidence: usize,
}

/// Checklist completion summary.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ChecklistStatus {
    pub done: usize,
    pub open: usize,
    pub blocked: usize,
}

/// Evidence coverage and quarantine summary.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EvidenceStatus {
    pub linked: usize,
    pub missing: usize,
    pub quarantined: usize,
}

/// Contract projection and validation summary.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ContractStatus {
    pub resolved_references: usize,
    pub unresolved_references: usize,
    pub diagnostics: usize,
    pub templates: usize,
    #[serde(default)]
    pub contract_assertions: usize,
    pub validation_receipts: usize,
    pub validation_passed: usize,
    pub validation_failed: usize,
    pub validation_skipped: usize,
    pub validation_matched_nodes: usize,
    #[serde(default)]
    pub validation_matched_node_ids: Vec<OrgNodeId>,
    #[serde(default)]
    pub reference_resolutions: Vec<OrgContractResolution>,
    #[serde(default)]
    pub diagnostic_records: Vec<OrgContractDiagnostic>,
    #[serde(default)]
    pub template_records: Vec<OrgContractTemplate>,
    #[serde(default)]
    pub registry: OrgContractRegistry,
    #[serde(default)]
    pub validation_report: OrgContractValidationReport,
    #[serde(default)]
    pub contract_expectation_summaries: Vec<String>,
    pub rendered_summary: Vec<String>,
}

/// Latest workspace patch receipt summary.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PatchStatus {
    pub latest_patch_id: String,
    pub execution_mode: PatchExecutionMode,
    pub policy_accepted: bool,
    pub policy_reason: Option<String>,
    pub affected_nodes: usize,
    pub affected_sources: usize,
    pub affected_source_documents: Vec<String>,
    pub validation_accepted: bool,
    pub validation_diagnostics: usize,
    pub memory_dispatches: usize,
    pub memory_dispatch_accepted: usize,
    pub memory_dispatch_failed: usize,
}

/// Release topology status derived from a `Gerbil` release artifact.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReleaseStatus {
    /// Stable release topology identifier.
    pub topology_id: String,
    /// Crate or artifact family covered by this topology.
    pub crate_name: String,
    /// Whether publishing is enabled for this topology.
    pub publish_enabled: bool,
    /// Command used to audit package contents.
    pub asset_audit_command: String,
    /// Package assets that must be present before release.
    pub package_assets: Vec<String>,
    /// Runtime dependency chain that must remain coherent.
    pub runtime_dependency_chain: Vec<String>,
    /// Workflow dependency chain that must remain coherent.
    pub workflow_dependency_chain: Vec<String>,
    /// Gate-level status projections.
    pub gates: Vec<ReleaseGateStatus>,
    /// Flattened visibility reports expected from every gate.
    pub visibility_reports: Vec<ReleaseVisibilityStatus>,
}

impl ReleaseStatus {
    /// Build a pending release status projection from a `Gerbil` topology artifact.
    pub fn pending_from_topology(topology: &ReleaseTopologySpec) -> Self {
        let gates = topology
            .gates
            .iter()
            .map(|gate| ReleaseGateStatus {
                gate_id: gate.gate_id.clone(),
                command: gate.command.clone(),
                requires_local_gerbil: gate.requires_local_gerbil,
                required_artifacts: gate.required_artifacts.clone(),
                state: if gate.requires_local_gerbil {
                    ReleaseGateState::RequiresLocalGerbil
                } else {
                    ReleaseGateState::Pending
                },
                last_receipt: None,
            })
            .collect();
        let visibility_reports = topology
            .gates
            .iter()
            .flat_map(|gate| {
                gate.visibility
                    .iter()
                    .map(|visibility| ReleaseVisibilityStatus {
                        gate_id: gate.gate_id.clone(),
                        report_key: visibility.report_key.clone(),
                        evidence_keys: visibility.evidence_keys.clone(),
                        artifact_paths: visibility.artifact_paths.clone(),
                        observed: false,
                    })
            })
            .collect();

        Self {
            topology_id: topology.topology_id.clone(),
            crate_name: topology.crate_name.clone(),
            publish_enabled: topology.publish_enabled,
            asset_audit_command: topology.asset_audit_command.clone(),
            package_assets: topology.package_assets.clone(),
            runtime_dependency_chain: topology.runtime_dependency_chain.clone(),
            workflow_dependency_chain: topology.workflow_dependency_chain.clone(),
            gates,
            visibility_reports,
        }
    }

    /// Return a status copy with one gate updated from an execution receipt.
    pub fn with_gate_receipt(mut self, receipt: ReleaseGateReceipt) -> Self {
        self.record_gate_receipt(receipt);
        self
    }

    /// Record execution evidence for a release gate.
    pub fn record_gate_receipt(&mut self, receipt: ReleaseGateReceipt) -> bool {
        let Some(gate_index) = self
            .gates
            .iter()
            .position(|gate| gate.gate_id == receipt.gate_id)
        else {
            return false;
        };

        self.gates[gate_index].state = receipt.state.clone();
        self.gates[gate_index].last_receipt = Some(receipt.clone());
        if receipt.state == ReleaseGateState::Passed {
            mark_receipt_visibility_observed(&mut self.visibility_reports, &receipt);
        }
        true
    }

    /// Build a release status projection and attach already-captured visibility evidence.
    pub fn from_topology_and_evidence(
        topology: &ReleaseTopologySpec,
        evidence: &[LoopEvidence],
    ) -> Self {
        let mut status = Self::pending_from_topology(topology);
        status.apply_visibility_evidence(evidence);
        status
    }

    /// Mark release visibility reports from harness-visible evidence subjects.
    pub fn apply_visibility_evidence(&mut self, evidence: &[LoopEvidence]) {
        let failed_gates = mark_visibility_reports_observed(
            self.topology_id.as_str(),
            &mut self.visibility_reports,
            evidence,
        );
        refresh_gate_states_from_visibility(
            &mut self.gates,
            &self.visibility_reports,
            &failed_gates,
        );
    }

    /// Build a compact landing report for humans and thin CLI/query facades.
    pub fn landing_report(&self) -> ReleaseLandingReport {
        ReleaseLandingReport::from_status(self)
    }
}

/// Compact, query-friendly release landing summary.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReleaseLandingReport {
    /// Stable release topology identifier.
    pub topology_id: String,
    /// Crate or artifact family covered by this report.
    pub crate_name: String,
    /// Whether publishing is enabled for this topology.
    pub publish_enabled: bool,
    /// True when all gates passed and all visibility reports were observed.
    pub landing_complete: bool,
    /// Number of release gates in the topology.
    pub gate_count: usize,
    /// Number of gates with a passing receipt.
    pub passed_gates: usize,
    /// Number of gates still pending.
    pub pending_gates: usize,
    /// Number of gates waiting for local Gerbil.
    pub local_gerbil_gates: usize,
    /// Number of failed gates.
    pub failed_gates: usize,
    /// Number of skipped gates.
    pub skipped_gates: usize,
    /// Number of expected visibility reports.
    pub visibility_report_count: usize,
    /// Number of observed visibility reports.
    pub observed_visibility_reports: usize,
    /// Evidence keys observed by passing release visibility reports.
    pub observed_evidence_keys: Vec<String>,
    /// Artifact paths observed by passing release visibility reports.
    pub observed_artifact_paths: Vec<String>,
    /// Artifact paths still missing from unobserved visibility reports.
    pub missing_artifact_paths: Vec<String>,
    /// Gate identifiers that still block landing completion.
    pub blocking_gates: Vec<String>,
    /// Visibility report keys that have not been observed.
    pub missing_visibility_reports: Vec<String>,
}

impl ReleaseLandingReport {
    /// Build a compact landing report from the full release status.
    pub fn from_status(status: &ReleaseStatus) -> Self {
        let gates = release_gate_summary(&status.gates);
        let visibility = release_visibility_summary(&status.visibility_reports);

        Self {
            topology_id: status.topology_id.clone(),
            crate_name: status.crate_name.clone(),
            publish_enabled: status.publish_enabled,
            landing_complete: gates.blocking_gates.is_empty()
                && visibility.missing_visibility_reports.is_empty(),
            gate_count: status.gates.len(),
            passed_gates: gates.passed,
            pending_gates: gates.pending,
            local_gerbil_gates: gates.local_gerbil,
            failed_gates: gates.failed,
            skipped_gates: gates.skipped,
            visibility_report_count: status.visibility_reports.len(),
            observed_visibility_reports: visibility.observed,
            observed_evidence_keys: observed_visibility_evidence_keys(&status.visibility_reports),
            observed_artifact_paths: observed_visibility_artifact_paths(&status.visibility_reports),
            missing_artifact_paths: missing_visibility_artifact_paths(&status.visibility_reports),
            blocking_gates: gates.blocking_gates,
            missing_visibility_reports: visibility.missing_visibility_reports,
        }
    }
}

#[derive(Debug, Default, Eq, PartialEq)]
struct ReleaseGateSummary {
    passed: usize,
    pending: usize,
    local_gerbil: usize,
    failed: usize,
    skipped: usize,
    blocking_gates: Vec<String>,
}

#[derive(Debug, Default, Eq, PartialEq)]
struct ReleaseVisibilitySummary {
    observed: usize,
    missing_visibility_reports: Vec<String>,
}

fn release_gate_summary(gates: &[ReleaseGateStatus]) -> ReleaseGateSummary {
    let mut summary = ReleaseGateSummary::default();

    for gate in gates {
        update_release_gate_summary(&mut summary, gate);
    }

    summary
}

fn update_release_gate_summary(summary: &mut ReleaseGateSummary, gate: &ReleaseGateStatus) {
    match gate.state {
        ReleaseGateState::Passed => summary.passed += 1,
        ReleaseGateState::Pending => {
            summary.pending += 1;
            summary.blocking_gates.push(gate.gate_id.clone());
        }
        ReleaseGateState::RequiresLocalGerbil => {
            summary.local_gerbil += 1;
            summary.blocking_gates.push(gate.gate_id.clone());
        }
        ReleaseGateState::Failed => {
            summary.failed += 1;
            summary.blocking_gates.push(gate.gate_id.clone());
        }
        ReleaseGateState::Skipped => {
            summary.skipped += 1;
            summary.blocking_gates.push(gate.gate_id.clone());
        }
    }
}

fn release_visibility_summary(reports: &[ReleaseVisibilityStatus]) -> ReleaseVisibilitySummary {
    let mut summary = ReleaseVisibilitySummary::default();

    for report in reports {
        update_release_visibility_summary(&mut summary, report);
    }

    summary
}

fn update_release_visibility_summary(
    summary: &mut ReleaseVisibilitySummary,
    report: &ReleaseVisibilityStatus,
) {
    if report.observed {
        summary.observed += 1;
    } else {
        summary
            .missing_visibility_reports
            .push(format!("{}:{}", report.gate_id, report.report_key));
    }
}

fn observed_visibility_evidence_keys(reports: &[ReleaseVisibilityStatus]) -> Vec<String> {
    reports
        .iter()
        .filter(|report| report.observed)
        .flat_map(|report| report.evidence_keys.iter().cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn observed_visibility_artifact_paths(reports: &[ReleaseVisibilityStatus]) -> Vec<String> {
    reports
        .iter()
        .filter(|report| report.observed)
        .flat_map(|report| report.artifact_paths.iter().cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn missing_visibility_artifact_paths(reports: &[ReleaseVisibilityStatus]) -> Vec<String> {
    reports
        .iter()
        .filter(|report| !report.observed)
        .flat_map(|report| report.artifact_paths.iter().cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn mark_visibility_reports_observed(
    topology_id: &str,
    reports: &mut [ReleaseVisibilityStatus],
    evidence: &[LoopEvidence],
) -> BTreeSet<String> {
    let mut failed_gates = BTreeSet::new();

    for report in reports {
        mark_visibility_report_observed(topology_id, report, evidence, &mut failed_gates);
    }

    failed_gates
}

fn mark_visibility_report_observed(
    topology_id: &str,
    report: &mut ReleaseVisibilityStatus,
    evidence: &[LoopEvidence],
    failed_gates: &mut BTreeSet<String>,
) {
    let subject = release_visibility_subject(
        topology_id,
        report.gate_id.as_str(),
        report.report_key.as_str(),
    );

    for evidence in matching_visibility_evidence(evidence, subject.as_str()) {
        if evidence.present {
            report.observed = true;
        } else {
            failed_gates.insert(report.gate_id.clone());
        }
    }
}

fn matching_visibility_evidence<'a>(
    evidence: &'a [LoopEvidence],
    subject: &'a str,
) -> impl Iterator<Item = &'a LoopEvidence> {
    evidence.iter().filter(move |evidence| {
        evidence.kind == LoopEvidenceKind::Visibility && evidence.subject == subject
    })
}

fn refresh_gate_states_from_visibility(
    gates: &mut [ReleaseGateStatus],
    reports: &[ReleaseVisibilityStatus],
    failed_gates: &BTreeSet<String>,
) {
    for gate in gates {
        let gate_reports = visibility_reports_for_gate(reports, gate.gate_id.as_str());
        if let Some(receipt) =
            visibility_gate_receipt(gate.gate_id.as_str(), &gate_reports, failed_gates)
        {
            gate.state = receipt.state.clone();
            gate.last_receipt = Some(receipt);
        }
    }
}

fn visibility_reports_for_gate<'a>(
    reports: &'a [ReleaseVisibilityStatus],
    gate_id: &str,
) -> Vec<&'a ReleaseVisibilityStatus> {
    reports
        .iter()
        .filter(|report| report.gate_id == gate_id)
        .collect()
}

fn visibility_gate_receipt(
    gate_id: &str,
    reports: &[&ReleaseVisibilityStatus],
    failed_gates: &BTreeSet<String>,
) -> Option<ReleaseGateReceipt> {
    if failed_gates.contains(gate_id) {
        return Some(ReleaseGateReceipt::failed(
            gate_id.to_owned(),
            vec!["release visibility evidence was reported missing".to_owned()],
        ));
    }

    if reports.is_empty() || !reports.iter().all(|report| report.observed) {
        return None;
    }

    Some(ReleaseGateReceipt::passed(
        gate_id.to_owned(),
        release_report_evidence_keys(reports),
        release_report_artifact_paths(reports),
    ))
}

fn release_report_evidence_keys(reports: &[&ReleaseVisibilityStatus]) -> Vec<String> {
    reports
        .iter()
        .flat_map(|report| report.evidence_keys.iter().cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn release_report_artifact_paths(reports: &[&ReleaseVisibilityStatus]) -> Vec<String> {
    reports
        .iter()
        .flat_map(|report| report.artifact_paths.iter().cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn mark_receipt_visibility_observed(
    reports: &mut [ReleaseVisibilityStatus],
    receipt: &ReleaseGateReceipt,
) {
    for report in reports
        .iter_mut()
        .filter(|report| report.gate_id == receipt.gate_id)
    {
        if receipt_satisfies_visibility_report(receipt, report) {
            report.observed = true;
        }
    }
}

fn receipt_satisfies_visibility_report(
    receipt: &ReleaseGateReceipt,
    report: &ReleaseVisibilityStatus,
) -> bool {
    let evidence_keys_present = report
        .evidence_keys
        .iter()
        .all(|key| receipt.evidence_keys.contains(key));
    let artifact_paths_present = report
        .artifact_paths
        .iter()
        .all(|path| receipt.artifact_paths.contains(path));
    evidence_keys_present && artifact_paths_present
}

fn release_visibility_subject(topology_id: &str, gate_id: &str, report_key: &str) -> String {
    format!("release-visibility:{topology_id}:{gate_id}:{report_key}")
}

/// Status for one release gate command.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReleaseGateStatus {
    /// Release gate identifier.
    pub gate_id: String,
    /// Command that should be run to satisfy the gate.
    pub command: String,
    /// Whether this gate requires a local Gerbil installation.
    pub requires_local_gerbil: bool,
    /// Artifacts the gate must prove.
    pub required_artifacts: Vec<String>,
    /// Current known gate state.
    pub state: ReleaseGateState,
    /// Latest execution receipt recorded for this gate.
    #[serde(default)]
    pub last_receipt: Option<ReleaseGateReceipt>,
}

/// Release gate state derived from topology requirements and execution receipts.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ReleaseGateState {
    Pending,
    RequiresLocalGerbil,
    Passed,
    Failed,
    Skipped,
}

/// Execution receipt for one release gate.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReleaseGateReceipt {
    /// Release gate identifier.
    pub gate_id: String,
    /// State proven by this receipt.
    pub state: ReleaseGateState,
    /// Evidence keys observed while running the gate.
    pub evidence_keys: Vec<String>,
    /// Artifact paths observed while running the gate.
    pub artifact_paths: Vec<String>,
    /// Diagnostics captured for failed or skipped gates.
    pub diagnostics: Vec<String>,
}

impl ReleaseGateReceipt {
    /// Build a passing release gate receipt.
    pub fn passed(
        gate_id: impl Into<String>,
        evidence_keys: Vec<String>,
        artifact_paths: Vec<String>,
    ) -> Self {
        Self {
            gate_id: gate_id.into(),
            state: ReleaseGateState::Passed,
            evidence_keys,
            artifact_paths,
            diagnostics: Vec::new(),
        }
    }

    /// Build a failed release gate receipt.
    pub fn failed(gate_id: impl Into<String>, diagnostics: Vec<String>) -> Self {
        Self {
            gate_id: gate_id.into(),
            state: ReleaseGateState::Failed,
            evidence_keys: Vec::new(),
            artifact_paths: Vec::new(),
            diagnostics,
        }
    }

    /// Build a skipped release gate receipt.
    pub fn skipped(gate_id: impl Into<String>, diagnostics: Vec<String>) -> Self {
        Self {
            gate_id: gate_id.into(),
            state: ReleaseGateState::Skipped,
            evidence_keys: Vec::new(),
            artifact_paths: Vec::new(),
            diagnostics,
        }
    }
}

/// Visibility report that should be emitted by a release gate.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReleaseVisibilityStatus {
    /// Gate that owns this visibility report.
    pub gate_id: String,
    /// Report key expected in the release status surface.
    pub report_key: String,
    /// Evidence keys required by this report.
    pub evidence_keys: Vec<String>,
    /// Artifact paths that should be visible in the report.
    pub artifact_paths: Vec<String>,
    /// Whether matching release visibility evidence has been observed.
    #[serde(default)]
    pub observed: bool,
}

/// Execution boundary proven by the latest patch receipt.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum PatchExecutionMode {
    #[default]
    DryRun,
    Commit,
}

/// Metric trace latest value and target.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MetricTrace {
    pub name: String,
    pub latest: Option<f64>,
    pub target: Option<f64>,
}

/// Recent decisions attached to a workspace target.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DecisionTrace {
    pub recent: Vec<String>,
}
