// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! GitOps for agent workspaces.
//!
//! Owns bounded repository discovery, status and diff reads, and typed
//! working-copy plans and receipts. Environment executes the projected
//! mutations through the selected provider.

#![forbid(unsafe_code)]

mod diff;
mod read;
mod repository;
mod status;
pub mod working_copy;

pub use diff::{
    GitDiff, GitDiffRequest, GitDiffScope, GitRepositoryPath, InvalidGitRepositoryPath,
};
pub use read::{GIT_REPOSITORY_OVERRIDES, GitReadError};
pub use repository::GitOperations;
pub use status::{GitFileState, GitPath, GitStatus, GitStatusEntry, GitStatusError};
