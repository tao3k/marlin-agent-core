//! Owned Context messages suitable for a runtime admission channel.

use crate::{
    ContextAdmissionError, ContextBundleId, ContextDestinationId, ContextOrganizationId,
    ContextScopeBatch, ContextScopeEpoch, ContextScopeKey, ContextScopeRow, ContextSessionTurn,
    ContextSourceCutId, ContextTaskId, ContextTurnIndex,
};
use marlin_agent_protocol::{
    ProjectRuntimeAgentId, ProjectRuntimeProjectId, ProjectRuntimeSessionId,
    ProjectRuntimeWorktreeId,
};
use std::sync::Arc;

/// Immutable common scope shared by requests in a conversation or task.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextScope {
    pub bundle: ContextBundleId,
    pub organization: ContextOrganizationId,
    pub project: ProjectRuntimeProjectId,
    pub worktree: ProjectRuntimeWorktreeId,
    pub source_cut: ContextSourceCutId,
    pub epoch: ContextScopeEpoch,
}

impl ContextScope {
    pub fn as_key(&self) -> ContextScopeKey<'_> {
        ContextScopeKey {
            bundle: &self.bundle,
            organization: &self.organization,
            project: &self.project,
            worktree: &self.worktree,
            source_cut: &self.source_cut,
            epoch: self.epoch,
        }
    }
}

/// Owned session identity and turn number, using the existing Context semantics.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextSession {
    pub session: ProjectRuntimeSessionId,
    pub turn: ContextTurnIndex,
}

/// The per-request portion of a Context scope admission.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextTurn {
    pub actor: ProjectRuntimeAgentId,
    pub task: ContextTaskId,
    pub destination: ContextDestinationId,
    pub session: Option<ContextSession>,
}

impl ContextTurn {
    pub fn as_row(&self) -> ContextScopeRow<'_> {
        ContextScopeRow {
            actor: &self.actor,
            task: &self.task,
            destination: &self.destination,
            session_turn: self.session.as_ref().map(|session| ContextSessionTurn {
                session: &session.session,
                turn: session.turn,
            }),
        }
    }
}

/// Validated owned input. Its shared scope stays immutable while queued.
#[derive(Clone, Debug)]
pub struct ContextAdmissionRequest {
    scope: Arc<ContextScope>,
    turn: ContextTurn,
    text_bytes: usize,
}

impl ContextAdmissionRequest {
    pub fn new(scope: Arc<ContextScope>, turn: ContextTurn) -> Result<Self, ContextAdmissionError> {
        let rows = [turn.as_row()];
        let batch = ContextScopeBatch::new(scope.as_key(), &rows)?;
        let text_bytes = batch.text_bytes();
        Ok(Self {
            scope,
            turn,
            text_bytes,
        })
    }

    pub fn scope(&self) -> &Arc<ContextScope> {
        &self.scope
    }
    pub fn turn(&self) -> &ContextTurn {
        &self.turn
    }
    pub fn text_bytes(&self) -> usize {
        self.text_bytes
    }
}
