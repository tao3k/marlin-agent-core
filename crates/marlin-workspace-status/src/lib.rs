//! Status report model for long-running workspace records.

mod report;
mod target;

pub use report::{
    ChecklistStatus, ContractStatus, DecisionTrace, EvidenceStatus, GoalState, GoalStatus,
    MetricTrace, PatchExecutionMode, PatchStatus, SddStatus, WorkspaceStatusReport,
};
pub use target::WorkspaceTarget;
