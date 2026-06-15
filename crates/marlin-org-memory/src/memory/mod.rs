//! In-memory Org workspace backend for protocol tests and local agents.

mod contracts;
mod format;
mod patch;
mod project_graph;
mod query;
mod render;
mod status;
mod workspace;

pub use project_graph::{
    PROJECT_MEMORY_CONTENT_ID_PROPERTY, PROJECT_MEMORY_CONTRACT_VALIDATED_PROPERTY,
    PROJECT_MEMORY_ID_PROPERTY, PROJECT_MEMORY_PROJECT_ID_PROPERTY,
    PROJECT_MEMORY_RECALL_QUERY_PROPERTY, PROJECT_MEMORY_ROOT_SESSION_ID_PROPERTY,
    PROJECT_MEMORY_SESSION_ID_PROPERTY, PROJECT_MEMORY_WORKSPACE_ID_PROPERTY,
    PROJECT_MEMORY_WORKTREE_ID_PROPERTY,
};
pub use workspace::MemoryOrgWorkspace;
