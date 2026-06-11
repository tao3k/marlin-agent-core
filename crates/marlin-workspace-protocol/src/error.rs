//! Error model for workspace protocol operations.

use thiserror::Error;

/// Workspace operation result alias.
pub type WorkspaceResult<T> = Result<T, WorkspaceError>;

/// Error cases surfaced by workspace protocol implementations.
#[derive(Debug, Error)]
pub enum WorkspaceError {
    #[error("workspace node not found: {0}")]
    NodeNotFound(String),
    #[error("workspace query failed: {0}")]
    QueryFailed(String),
    #[error("workspace patch rejected: {0}")]
    PatchRejected(String),
    #[error("workspace validation failed: {0}")]
    ValidationFailed(String),
    #[error("workspace backend failed: {0}")]
    Backend(String),
}
