//! Native workspace trait for query, view, patch, validation, and status.

use async_trait::async_trait;
use marlin_org_model::{OrgNode, OrgNodeId};
use marlin_workspace_patch::{WorkspacePatch, WorkspacePatchReceipt, WorkspaceValidationReport};
use marlin_workspace_query::{WorkspaceQuery, WorkspaceQueryResult, WorkspaceScope};
use marlin_workspace_status::{WorkspaceStatusReport, WorkspaceTarget};
use marlin_workspace_view::{RenderedWorkspaceView, WorkspaceViewSpec};

use crate::{WorkspaceCtx, WorkspaceResult};

#[async_trait]
/// Structured workspace interface implemented by native workspace backends.
pub trait AgentWorkspace: Send + Sync {
    async fn query(
        &self,
        query: WorkspaceQuery,
        ctx: WorkspaceCtx,
    ) -> WorkspaceResult<WorkspaceQueryResult>;

    async fn read_node(&self, id: OrgNodeId, ctx: WorkspaceCtx) -> WorkspaceResult<OrgNode>;

    async fn patch(
        &self,
        patch: WorkspacePatch,
        ctx: WorkspaceCtx,
    ) -> WorkspaceResult<WorkspacePatchReceipt>;

    async fn render_view(
        &self,
        view: WorkspaceViewSpec,
        ctx: WorkspaceCtx,
    ) -> WorkspaceResult<RenderedWorkspaceView>;

    async fn validate(
        &self,
        scope: WorkspaceScope,
        ctx: WorkspaceCtx,
    ) -> WorkspaceResult<WorkspaceValidationReport>;

    async fn status(
        &self,
        target: WorkspaceTarget,
        ctx: WorkspaceCtx,
    ) -> WorkspaceResult<WorkspaceStatusReport>;
}
