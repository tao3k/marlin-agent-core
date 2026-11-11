//! Marlin's typed Context admission requests and Lab scope identities.

#[cfg(feature = "gerbil")]
pub mod gerbil;
/// Org-backed Context source and optional memory projections.
#[cfg(feature = "org-source")]
pub mod org;
mod request;
pub use request::{ContextAdmissionRequest, ContextScope, ContextSession, ContextTurn};

use marlin_agent_protocol::{
    ProjectRuntimeAgentId, ProjectRuntimeProjectId, ProjectRuntimeSessionId,
    ProjectRuntimeWorktreeId,
};

/// Maximum rows in one application export call. The caller can dispatch a partial batch.
pub const CONTEXT_BATCH_MAX_ROWS: usize = 64;
/// Bound on UTF-8 payload bytes, with shared fields counted once.
pub const CONTEXT_BATCH_MAX_TEXT_BYTES: usize = 32 * 1024;
const SCOPE_DIGEST_BYTES: usize = 32;

macro_rules! context_id {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        pub struct $name(String);
        impl $name {
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

context_id!(ContextBundleId);
context_id!(ContextOrganizationId);
context_id!(ContextSourceCutId);
context_id!(ContextTaskId);
context_id!(ContextDestinationId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ContextScopeEpoch(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ContextTurnIndex(pub u64);

/// Common inputs that are materialized once on the Scheme owner.
pub struct ContextScopeKey<'a> {
    pub bundle: &'a ContextBundleId,
    pub organization: &'a ContextOrganizationId,
    pub project: &'a ProjectRuntimeProjectId,
    pub worktree: &'a ProjectRuntimeWorktreeId,
    pub source_cut: &'a ContextSourceCutId,
    pub epoch: ContextScopeEpoch,
}

/// Session and turn travel together when a request names a conversation step.
pub struct ContextSessionTurn<'a> {
    pub session: &'a ProjectRuntimeSessionId,
    pub turn: ContextTurnIndex,
}

pub struct ContextScopeRow<'a> {
    pub actor: &'a ProjectRuntimeAgentId,
    pub task: &'a ContextTaskId,
    pub destination: &'a ContextDestinationId,
    pub session_turn: Option<ContextSessionTurn<'a>>,
}

/// Lab's SHA-256 scope identity, not a Rust permission or execution receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ContextScopeDigest([u8; SCOPE_DIGEST_BYTES]);

impl ContextScopeDigest {
    pub const fn as_bytes(&self) -> &[u8; SCOPE_DIGEST_BYTES] {
        &self.0
    }
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum ContextAdmissionError {
    #[error("Context batch requires between 1 and {CONTEXT_BATCH_MAX_ROWS} rows")]
    RowCount,
    #[error("Context identifiers require between 1 and 256 Unicode scalar values")]
    InvalidIdentifier,
    #[error("Context batch exceeds {CONTEXT_BATCH_MAX_TEXT_BYTES} UTF-8 bytes")]
    TextBudget,
}

/// A single Context scope and a bounded ordered set of rows ready to enter its VM.
pub struct ContextScopeBatch<'a> {
    key: ContextScopeKey<'a>,
    rows: &'a [ContextScopeRow<'a>],
}

impl ContextScopeKey<'_> {
    fn text(&self) -> [&str; 5] {
        [
            self.bundle.as_str(),
            self.organization.as_str(),
            self.project.as_str(),
            self.worktree.as_str(),
            self.source_cut.as_str(),
        ]
    }
}

impl ContextScopeRow<'_> {
    fn text(&self) -> impl Iterator<Item = &str> {
        [
            self.actor.as_str(),
            self.task.as_str(),
            self.destination.as_str(),
        ]
        .into_iter()
        .chain(self.session_turn.as_ref().map(|turn| turn.session.as_str()))
    }
}

impl<'a> ContextScopeBatch<'a> {
    /// Qualify the fixed scope, row count and text budget before entering the VM.
    pub fn new(
        key: ContextScopeKey<'a>,
        rows: &'a [ContextScopeRow<'a>],
    ) -> Result<Self, ContextAdmissionError> {
        if rows.is_empty() || rows.len() > CONTEXT_BATCH_MAX_ROWS {
            return Err(ContextAdmissionError::RowCount);
        }
        let mut bytes = 0;
        for value in key
            .text()
            .into_iter()
            .chain(rows.iter().flat_map(ContextScopeRow::text))
        {
            if value.is_empty()
                || value.len() > 4 * 256
                || (value.len() > 256 && value.chars().nth(256).is_some())
            {
                return Err(ContextAdmissionError::InvalidIdentifier);
            }
            bytes += value.len();
            if bytes > CONTEXT_BATCH_MAX_TEXT_BYTES {
                return Err(ContextAdmissionError::TextBudget);
            }
        }
        Ok(Self { key, rows })
    }

    pub fn key(&self) -> &ContextScopeKey<'a> {
        &self.key
    }

    pub fn rows(&self) -> &[ContextScopeRow<'a>] {
        self.rows
    }

    pub fn text_bytes(&self) -> usize {
        self.key
            .text()
            .into_iter()
            .chain(self.rows.iter().flat_map(ContextScopeRow::text))
            .map(str::len)
            .sum()
    }
}
