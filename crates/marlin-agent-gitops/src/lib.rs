// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! GitOps for agent workspaces.
//!
//! Owns bounded repository discovery, status and diff reads, and typed
//! working-copy plans and receipts. Environment executes the projected
//! mutations through the selected provider.

#![forbid(unsafe_code)]

mod blob_locator;
mod diff;
mod read;
mod repository;
#[cfg(unix)]
mod repository_owner;
mod repository_route;
#[cfg(unix)]
pub mod retention;
mod status;
pub mod working_copy;

pub use blob_locator::{GitBlobId, GitBlobLocator};
pub use diff::{
    GitDiff, GitDiffRequest, GitDiffScope, GitRepositoryPath, InvalidGitRepositoryPath,
};
pub use read::{DISABLED_FSMONITOR, DISABLED_HOOKS, GIT_REPOSITORY_OVERRIDES, GitReadError};
pub use repository::{
    CommittedCheckout, ContextDocument, GitCheckoutRef, GitCommitId, GitContext, GitOperations,
};
#[cfg(unix)]
pub use repository_owner::{EnrolledRepository, RepositoryEnrollmentError, RepositoryInstanceId};
pub use repository_route::{
    GitRemoteName, NetworkRepositoryRoute, RepositoryRoute, RepositoryRouteError,
};
pub use status::{GitFileState, GitPath, GitStatus, GitStatusEntry, GitStatusError};
