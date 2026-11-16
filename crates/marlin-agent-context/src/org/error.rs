// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Errors from typed Org host-cache operations.

use marlin_context_org_model::OrgNodeId;
use thiserror::Error;

/// Org workspace operation result.
pub type WorkspaceResult<T> = Result<T, WorkspaceError>;

/// Errors surfaced by Org workspace operations.
#[derive(Debug, Error)]
pub enum WorkspaceError {
    #[error("workspace node not found: {}", .0.as_str())]
    NodeNotFound(OrgNodeId),
}
