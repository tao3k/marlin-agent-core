//! Status report model for long-running workspace records.

mod report;
mod target;

pub use report::{
    ChecklistStatus, DecisionTrace, EvidenceStatus, GoalState, GoalStatus, MetricTrace, SddStatus,
    WorkspaceStatusReport,
};
pub use target::WorkspaceTarget;
