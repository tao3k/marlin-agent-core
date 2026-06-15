//! Project-scoped runtime graph identifiers.

use serde::{Deserialize, Serialize};

macro_rules! project_runtime_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash, Serialize, Deserialize)]
        pub struct $name(String);

        impl $name {
            pub fn new(id: impl Into<String>) -> Self {
                Self(id.into())
            }

            pub fn as_str(&self) -> &str {
                self.0.as_str()
            }
        }
    };
}

project_runtime_id! {
    /// Stable identifier for a Marlin project runtime boundary.
    ProjectRuntimeProjectId
}

project_runtime_id! {
    /// Stable identifier for one workspace imported under a project runtime.
    ProjectRuntimeWorkspaceId
}

project_runtime_id! {
    /// Worktree provenance identifier. This is not a memory namespace.
    ProjectRuntimeWorktreeId
}

project_runtime_id! {
    /// Branch or ref provenance attached to the active workspace view.
    ProjectRuntimeBranchRef
}

project_runtime_id! {
    /// Root interactive session that owns transcript isolation.
    ProjectRuntimeRootSessionId
}

project_runtime_id! {
    /// Runtime session identifier under a root session.
    ProjectRuntimeSessionId
}

project_runtime_id! {
    /// Agent identity within a runtime session lineage.
    ProjectRuntimeAgentId
}

project_runtime_id! {
    /// Typed content node identifier addressable by the project graph.
    ProjectRuntimeContentId
}

project_runtime_id! {
    /// Compact context pack identifier produced from bounded session content.
    ProjectRuntimeContextPackId
}

project_runtime_id! {
    /// Typed project memory identifier addressable by the project graph.
    ProjectRuntimeMemoryId
}

project_runtime_id! {
    /// Typed evidence identifier carried by project memory context packs.
    ProjectRuntimeEvidenceId
}

project_runtime_id! {
    /// Source span reference for parser-owned project runtime evidence.
    ProjectRuntimeSourceSpanRef
}

project_runtime_id! {
    /// Typed tool capability identifier addressable by the project graph.
    ProjectRuntimeToolCapabilityId
}

project_runtime_id! {
    /// Receipt identifier for replayable project runtime graph query evidence.
    ProjectRuntimeReceiptId
}

/// Upper bound for a graph query result set.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct GraphQueryLimit(u16);

impl GraphQueryLimit {
    pub fn new(limit: u16) -> Self {
        Self(limit)
    }

    pub fn as_u16(self) -> u16 {
        self.0
    }
}

impl From<u16> for GraphQueryLimit {
    fn from(value: u16) -> Self {
        Self::new(value)
    }
}

/// Rank score encoded as basis points.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct GraphQueryScoreBasisPoints(u16);

impl GraphQueryScoreBasisPoints {
    pub fn new(score: u16) -> Self {
        Self(score)
    }

    pub fn as_u16(self) -> u16 {
        self.0
    }
}

impl From<u16> for GraphQueryScoreBasisPoints {
    fn from(value: u16) -> Self {
        Self::new(value)
    }
}
