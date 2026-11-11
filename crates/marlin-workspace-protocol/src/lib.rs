// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Native workspace protocol facade for agent-core callers.

mod context;
mod error;
mod project;
mod working_copy;
mod workspace;

pub use context::WorkspaceCtx;
pub use error::{WorkspaceError, WorkspaceResult};
pub use project::{
    WorkspaceProjectGitHubOperation, WorkspaceProjectGitHubOps, WorkspaceProjectGitHubRepository,
    WorkspaceProjectId, WorkspaceProjectPersistence, WorkspaceProjectPolicyPlane,
    WorkspaceProjectProfile, WorkspaceProjectRepository, WorkspaceProjectRoot,
    WorkspaceProjectTrust, WorkspaceProjectVcsBackend, WorkspaceProjectVcsExtension,
    WorkspaceProjectVcsExtensionKind,
};
pub use working_copy::{
    WorkingCopyActiveBinding, WorkingCopyBaseRef, WorkingCopyBranchName,
    WorkingCopyCommandInvocation, WorkingCopyCommandProgram, WorkingCopyCommandProjection,
    WorkingCopyCommandProjectionError, WorkingCopyCommandReceipt, WorkingCopyCommandStatus,
    WorkingCopyCreateRequest, WorkingCopyFanoutBenchmarkReceipt, WorkingCopyFinalizeBranchRequest,
    WorkingCopyGitTopLevel, WorkingCopyHandle, WorkingCopyId, WorkingCopyIsolationOperationKind,
    WorkingCopyIsolationPlan, WorkingCopyIsolationPlanError, WorkingCopyIsolationPlanStep,
    WorkingCopyIsolationProvider, WorkingCopyIsolationReceipt, WorkingCopyIsolationRequest,
    WorkingCopyIsolationStatus, WorkingCopyListOptions, WorkingCopyListRequest,
    WorkingCopyParallelIsolationReceipt, WorkingCopyPullRequestCheckoutRequest,
    WorkingCopyPullRequestNumber, WorkingCopyRemovalMode, WorkingCopyRemoveRequest,
    WorkingCopyRepositoryDiscoveryPath, WorkingCopyRetentionActionKind,
    WorkingCopyRetentionActionReceipt, WorkingCopyRetentionPolicy,
    WorkingCopyRetentionSweepReceipt, WorkingCopySwitchMode, WorkingCopySwitchRequest,
};
pub use workspace::AgentWorkspace;

pub use marlin_context_org_model::{
    BlockKind, CheckboxState, LinkKind, OrgBlock, OrgCheckbox, OrgLink, OrgNode, OrgNodeId,
    OrgNodeKind, OrgNodeSourceTokens, OrgProperty, OrgSourceSpan, OrgTable, OrgTableRow,
    OrgTimestamp, TodoState,
};
pub use marlin_workspace_query::{
    NodeSelector, PropertyFilter, QueryFilter, QueryMatch, QueryOrder, SourceRange, WorkspaceQuery,
    WorkspaceQueryResult, WorkspaceScope,
};
pub use marlin_workspace_status::{
    ChecklistStatus, DecisionTrace, EvidenceStatus, GoalState, GoalStatus, MetricTrace,
    ReleaseGateReceipt, ReleaseGateState, ReleaseGateStatus, ReleaseLandingReport, ReleaseStatus,
    ReleaseVisibilityStatus, SddStatus, WorkspaceStatusReport, WorkspaceTarget,
};
pub use marlin_workspace_view::{
    RenderMode, RenderedViewNode, RenderedWorkspaceView, WorkspaceField, WorkspaceViewSpec,
};
