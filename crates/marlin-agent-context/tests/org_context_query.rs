// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later
#![cfg(feature = "org-memory")]

use std::sync::Arc;

use marlin_agent_context::{
    ContextAdmissionRequest, ContextBundleId, ContextDestinationId, ContextOrganizationId,
    ContextScope, ContextScopeEpoch, ContextSession, ContextSourceCutId, ContextTaskId,
    ContextTurn, ContextTurnIndex,
    org::{ContextOrgMemoryQuery, ContextOrgQueryError, MemoryOrgWorkspace},
};
use marlin_agent_protocol::{
    GraphQueryContext, GraphQueryFamily, GraphQueryRequest, ProjectRuntimeAgentId,
    ProjectRuntimeProjectId, ProjectRuntimeReceiptId, ProjectRuntimeSessionId,
    ProjectRuntimeWorktreeId,
};

fn request() -> ContextAdmissionRequest {
    ContextAdmissionRequest::new(
        Arc::new(ContextScope {
            bundle: ContextBundleId::new("bundle"),
            organization: ContextOrganizationId::new("org"),
            project: ProjectRuntimeProjectId::new("project"),
            worktree: ProjectRuntimeWorktreeId::new("worktree"),
            source_cut: ContextSourceCutId::new("source-cut"),
            epoch: ContextScopeEpoch(1),
        }),
        ContextTurn {
            actor: ProjectRuntimeAgentId::new("actor"),
            task: ContextTaskId::new("task"),
            destination: ContextDestinationId::new("model"),
            session: Some(ContextSession {
                session: ProjectRuntimeSessionId::new("session"),
                turn: ContextTurnIndex(3),
            }),
        },
    )
    .expect("valid request")
}

fn query() -> GraphQueryRequest {
    GraphQueryRequest::new(
        GraphQueryContext::new("project")
            .with_worktree("worktree")
            .with_session("session")
            .with_agent("actor"),
        GraphQueryFamily::Memory,
        "decision",
    )
}

#[test]
fn context_query_requires_matching_scope_and_preserves_turn() {
    let bound = ContextOrgMemoryQuery::new(request(), query()).expect("matching identities");
    assert_eq!(
        bound.context().turn().session.as_ref().unwrap().turn,
        ContextTurnIndex(3)
    );
    let response = bound
        .project(
            &MemoryOrgWorkspace::new(),
            ProjectRuntimeReceiptId::new("receipt"),
        )
        .expect("empty local workspace can be queried");
    assert_eq!(
        response.context.turn().session.as_ref().unwrap().turn,
        ContextTurnIndex(3)
    );
    assert_eq!(response.response.receipt_id.as_str(), "receipt");
    assert!(response.response.matches.is_empty());

    let mut mismatch = query();
    mismatch.context.worktree_id = Some(ProjectRuntimeWorktreeId::new("other"));
    assert!(matches!(
        ContextOrgMemoryQuery::new(request(), mismatch),
        Err(ContextOrgQueryError::ScopeMismatch)
    ));
    let mut mismatch = query();
    mismatch.context.session_id = None;
    assert!(matches!(
        ContextOrgMemoryQuery::new(request(), mismatch),
        Err(ContextOrgQueryError::ScopeMismatch)
    ));
    let mut mismatch = query();
    mismatch.context.agent_lineage.clear();
    assert!(matches!(
        ContextOrgMemoryQuery::new(request(), mismatch),
        Err(ContextOrgQueryError::ScopeMismatch)
    ));
    let mut wrong_family = query();
    wrong_family.family = GraphQueryFamily::Tool;
    assert!(matches!(
        ContextOrgMemoryQuery::new(request(), wrong_family),
        Err(ContextOrgQueryError::Family)
    ));
}
