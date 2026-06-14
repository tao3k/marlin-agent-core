//! Git utility boundary for repository discovery and native Git execution.

#![forbid(unsafe_code)]

mod operations;

pub use operations::{GitRepositoryRoot, GitToolingError, ProcessGitTooling};
