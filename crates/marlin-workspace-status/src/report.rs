//! Status report structures derived from workspace records.

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
