// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Errors from Org workspace reads and document projection.

use thiserror::Error;

/// Org workspace operation result.
pub type WorkspaceResult<T> = Result<T, WorkspaceError>;

/// Errors surfaced by Org workspace operations.
#[derive(Debug, Error)]
pub enum WorkspaceError {
    #[error("workspace node not found: {0}")]
    NodeNotFound(String),
    #[error("workspace backend failed: {0}")]
    Backend(String),
}
