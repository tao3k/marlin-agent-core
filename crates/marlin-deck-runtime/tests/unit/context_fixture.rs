// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use marlin_agent_context::{
    ContextAdmissionRequest, ContextScope, ContextScopeEpoch, ContextSession, ContextTurn,
    ContextTurnIndex,
};
use std::sync::Arc;

pub(super) fn scope(label: &str) -> Arc<ContextScope> {
    Arc::new(ContextScope {
        bundle: marlin_agent_context::ContextBundleId::new("marlin"),
        organization: marlin_agent_context::ContextOrganizationId::new("example"),
        project: marlin_agent_protocol::ProjectRuntimeProjectId::new("project"),
        worktree: marlin_agent_protocol::ProjectRuntimeWorktreeId::new("worktree"),
        source_cut: marlin_agent_context::ContextSourceCutId::new(label),
        epoch: ContextScopeEpoch(1),
    })
}

pub(super) fn request(scope: &Arc<ContextScope>, index: usize) -> ContextAdmissionRequest {
    ContextAdmissionRequest::new(
        Arc::clone(scope),
        ContextTurn {
            actor: marlin_agent_protocol::ProjectRuntimeAgentId::new("assistant"),
            task: marlin_agent_context::ContextTaskId::new(format!("任务-{index}")),
            destination: marlin_agent_context::ContextDestinationId::new("panel"),
            session: Some(ContextSession {
                session: marlin_agent_protocol::ProjectRuntimeSessionId::new("conversation"),
                turn: ContextTurnIndex(index as u64),
            }),
        },
    )
    .unwrap()
}
