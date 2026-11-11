//! Native workspace trait for query, view, and status.

use async_trait::async_trait;
use marlin_context_org_model::{OrgNode, OrgNodeId};
use marlin_workspace_query::{WorkspaceQuery, WorkspaceQueryResult};
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

    async fn render_view(
        &self,
        view: WorkspaceViewSpec,
        ctx: WorkspaceCtx,
    ) -> WorkspaceResult<RenderedWorkspaceView>;

    async fn status(
        &self,
        target: WorkspaceTarget,
        ctx: WorkspaceCtx,
    ) -> WorkspaceResult<WorkspaceStatusReport>;
}
