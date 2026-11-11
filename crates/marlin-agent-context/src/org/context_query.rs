// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Bind an Org memory query to the same identities as a Context request.

use crate::ContextAdmissionRequest;
use marlin_agent_protocol::{
    GraphQueryFamily, GraphQueryRequest, GraphQueryResponse, ProjectRuntimeReceiptId,
};
use marlin_workspace_protocol::WorkspaceResult;

use super::MemoryOrgWorkspace;

/// An Org memory query whose supplied identities match a Context request.
///
/// This checks input consistency only. It is not Lab scope admission, source
/// authentication, a current-use grant, or a durable projection receipt.
#[derive(Clone, Debug)]
pub struct ContextOrgMemoryQuery {
    context: ContextAdmissionRequest,
    query: GraphQueryRequest,
}

/// Local Org query result correlated with its original Context request.
/// Neither the source cut nor the projection is authenticated by this type.
#[derive(Clone, Debug)]
pub struct ContextOrgMemoryProjection {
    pub context: ContextAdmissionRequest,
    pub response: GraphQueryResponse,
}

/// The Org query family or its common scope identities do not match Context.
#[derive(Debug, Eq, PartialEq, thiserror::Error)]
pub enum ContextOrgQueryError {
    #[error("Org memory query family must be Memory")]
    Family,
    #[error("Org memory query project, worktree, actor or session differs from Context")]
    ScopeMismatch,
}

impl ContextOrgMemoryQuery {
    /// Check the common identities before executing the local Org projection.
    ///
    /// # Errors
    /// Rejects non-memory queries and mismatching project, worktree, actor or
    /// session identities. The query has no source-cut or turn-index field to
    /// compare; retaining those values does not verify them.
    pub fn new(
        context: ContextAdmissionRequest,
        query: GraphQueryRequest,
    ) -> Result<Self, ContextOrgQueryError> {
        if query.family != GraphQueryFamily::Memory {
            return Err(ContextOrgQueryError::Family);
        }
        let scope = context.scope();
        let turn = context.turn();
        if query.context.project_id != scope.project
            || query.context.worktree_id.as_ref() != Some(&scope.worktree)
            || query.context.agent_lineage.last() != Some(&turn.actor)
            || query.context.session_id.as_ref() != turn.session.as_ref().map(|s| &s.session)
        {
            return Err(ContextOrgQueryError::ScopeMismatch);
        }
        Ok(Self { context, query })
    }

    /// Borrow the full Context request, including source cut and turn index.
    pub fn context(&self) -> &ContextAdmissionRequest {
        &self.context
    }

    /// Execute the local workspace projection without conferring authority.
    ///
    /// # Errors
    /// Propagates workspace access errors.
    pub fn project(
        self,
        workspace: &MemoryOrgWorkspace,
        receipt: ProjectRuntimeReceiptId,
    ) -> WorkspaceResult<ContextOrgMemoryProjection> {
        let response = workspace.query_project_memory_graph(receipt.as_str(), self.query)?;
        Ok(ContextOrgMemoryProjection {
            context: self.context,
            response,
        })
    }
}
