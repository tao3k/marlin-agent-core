//! Status report structures derived from workspace records.

use marlin_gerbil_ir::ReleaseTopologySpec;
use serde::{Deserialize, Serialize};

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
    pub validation_receipts: usize,
    pub validation_passed: usize,
    pub validation_failed: usize,
    pub validation_skipped: usize,
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
}

/// Release gate state before execution evidence is attached.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ReleaseGateState {
    Pending,
    RequiresLocalGerbil,
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
