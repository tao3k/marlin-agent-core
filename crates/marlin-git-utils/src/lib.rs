//! Git utility boundary for repository discovery and native Git execution.

#![forbid(unsafe_code)]

mod diff;
mod operations;
mod read;
mod status;

pub use diff::{
    GitDiff, GitDiffRequest, GitDiffScope, GitRepositoryPath, InvalidGitRepositoryPath,
};
pub use operations::{GitRepositoryRoot, GitToolingError, ProcessGitTooling};
pub use read::GitReadError;
pub use status::{GitFileState, GitPath, GitStatus, GitStatusEntry, GitStatusError};
