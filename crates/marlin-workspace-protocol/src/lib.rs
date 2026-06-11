//! Native workspace protocol facade for agent-core callers.

mod context;
mod error;
mod workspace;

pub use context::WorkspaceCtx;
pub use error::{WorkspaceError, WorkspaceResult};
pub use workspace::AgentWorkspace;

pub use marlin_org_model::{
    BlockKind, CheckboxState, LinkKind, OrgBlock, OrgCheckbox, OrgLink, OrgNode, OrgNodeId,
    OrgNodeKind, OrgNodeSourceTokens, OrgProperty, OrgSourceSpan, OrgTable, OrgTableRow,
    OrgTimestamp, TodoState,
};
pub use marlin_org_patch::{
    OrgPatchApplier, OrgPatchApplyReport, OrgPatchDiagnostic, OrgPatchDocumentChange, OrgPatchPlan,
    OrgPatchPlanner, OrgTextEdit, org_text_hash,
};
pub use marlin_workspace_patch::{
    AffectedNodeSource, DecisionRecord, EvidenceRef, EvidenceTrust, MemoryDispatchReceipt,
    MetricPoint, PatchId, ValidationDiagnostic, ValidationSeverity, WorkspacePatch,
    WorkspacePatchOp, WorkspacePatchReceipt, WorkspaceValidationReport,
};
pub use marlin_workspace_query::{
    NodeSelector, PropertyFilter, QueryFilter, QueryMatch, QueryOrder, SourceRange, WorkspaceQuery,
    WorkspaceQueryResult, WorkspaceScope,
};
pub use marlin_workspace_status::{
    ChecklistStatus, DecisionTrace, EvidenceStatus, GoalState, GoalStatus, MetricTrace,
    ReleaseGateReceipt, ReleaseGateState, ReleaseGateStatus, ReleaseStatus,
    ReleaseVisibilityStatus, SddStatus, WorkspaceStatusReport, WorkspaceTarget,
};
pub use marlin_workspace_view::{
    RenderMode, RenderedViewNode, RenderedWorkspaceView, WorkspaceField, WorkspaceViewSpec,
};
