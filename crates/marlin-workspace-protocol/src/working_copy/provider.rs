//! Provider and operation families for working-copy isolation.

use serde::{Deserialize, Serialize};

/// Provider used to isolate agent-owned working copies.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum WorkingCopyIsolationProvider {
    /// Rust-owned baseline over native `git worktree` semantics.
    GitWorktree,
    /// Worktrunk provider for agent-oriented Git worktree orchestration.
    Worktrunk,
}

impl WorkingCopyIsolationProvider {
    /// Returns true when the provider supports the operation family.
    pub fn supports(&self, operation: &WorkingCopyIsolationOperationKind) -> bool {
        match self {
            Self::GitWorktree => matches!(
                operation,
                WorkingCopyIsolationOperationKind::Create
                    | WorkingCopyIsolationOperationKind::Switch
                    | WorkingCopyIsolationOperationKind::List
                    | WorkingCopyIsolationOperationKind::Remove
            ),
            Self::Worktrunk => true,
        }
    }
}

/// Operation family requested from a working-copy isolation provider.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum WorkingCopyIsolationOperationKind {
    Create,
    Switch,
    List,
    Remove,
    Merge,
    PullRequestCheckout,
}
