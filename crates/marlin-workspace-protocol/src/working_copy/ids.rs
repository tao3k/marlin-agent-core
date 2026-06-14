//! Domain identifiers for working-copy isolation.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Stable local name for an isolated working copy.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct WorkingCopyId(String);

impl WorkingCopyId {
    /// Creates a working-copy id from a caller-owned name.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Returns the string form used in receipts.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl From<&str> for WorkingCopyId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for WorkingCopyId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

/// Git branch name associated with a Git-backed working copy.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct WorkingCopyBranchName(String);

impl WorkingCopyBranchName {
    /// Creates a branch name newtype.
    pub fn new(branch: impl Into<String>) -> Self {
        Self(branch.into())
    }

    /// Returns the branch string.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl From<&str> for WorkingCopyBranchName {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for WorkingCopyBranchName {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

/// Base revision or ref used to create an isolated working copy.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct WorkingCopyBaseRef(String);

impl WorkingCopyBaseRef {
    /// Creates a base ref newtype.
    pub fn new(base_ref: impl Into<String>) -> Self {
        Self(base_ref.into())
    }

    /// Returns the base ref string.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl From<&str> for WorkingCopyBaseRef {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for WorkingCopyBaseRef {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

/// Absolute Git top-level path resolved from the imported project's `.git`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkingCopyGitTopLevel(PathBuf);

impl WorkingCopyGitTopLevel {
    /// Creates a Git top-level path from a native Git resolution result.
    pub fn from_resolved_path(path: PathBuf) -> Self {
        Self(path)
    }

    /// Returns the Git top-level path.
    pub fn as_path(&self) -> &Path {
        self.0.as_path()
    }

    /// Consumes this value into its owned path.
    pub fn into_path_buf(self) -> PathBuf {
        self.0
    }
}

/// Path used only as input to discover the real Git repository top-level.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkingCopyRepositoryDiscoveryPath(PathBuf);

impl WorkingCopyRepositoryDiscoveryPath {
    /// Creates a discovery path for native Git repository resolution.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self(path.into())
    }

    /// Returns the discovery path.
    pub fn as_path(&self) -> &Path {
        self.0.as_path()
    }
}

impl From<PathBuf> for WorkingCopyRepositoryDiscoveryPath {
    fn from(value: PathBuf) -> Self {
        Self::new(value)
    }
}

impl From<&Path> for WorkingCopyRepositoryDiscoveryPath {
    fn from(value: &Path) -> Self {
        Self::new(value.to_path_buf())
    }
}

impl From<&str> for WorkingCopyRepositoryDiscoveryPath {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for WorkingCopyRepositoryDiscoveryPath {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

/// GitHub pull request number used by provider-level PR checkout.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct WorkingCopyPullRequestNumber(u64);

impl WorkingCopyPullRequestNumber {
    /// Creates a pull request number.
    pub fn new(number: u64) -> Self {
        Self(number)
    }

    /// Returns the raw pull request number.
    pub fn get(&self) -> u64 {
        self.0
    }
}
