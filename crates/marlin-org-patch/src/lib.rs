//! Source-aware `Org` text patch planning for workspace mutations.

mod apply;
mod edit;
mod planner;

pub use apply::{OrgPatchApplier, OrgPatchApplyReport, OrgPatchDocumentChange, org_text_hash};
pub use edit::{OrgPatchDiagnostic, OrgPatchPlan, OrgTextEdit};
pub use planner::OrgPatchPlanner;
