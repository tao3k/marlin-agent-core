//! Status report structures derived from workspace records.

use serde::{Deserialize, Serialize};

/// Combined status projection for a workspace target.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceStatusReport {
    pub goal: Option<GoalStatus>,
    pub sdd: Option<SddStatus>,
    pub checklist: Option<ChecklistStatus>,
    pub evidence: Option<EvidenceStatus>,
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
