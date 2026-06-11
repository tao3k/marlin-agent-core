//! Selector model for workspace scopes and nodes.

use marlin_org_model::OrgNodeId;
use serde::{Deserialize, Serialize};

/// Workspace region considered by a query or validation run.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum WorkspaceScope {
    WholeWorkspace,
    Document(String),
    SourceRange(SourceRange),
    Subtree(OrgNodeId),
    Nodes(Vec<OrgNodeId>),
}

/// Source document line range used by provenance-aware queries.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SourceRange {
    pub document: String,
    pub start_line: usize,
    pub end_line: usize,
}

/// Stable selector for addressing workspace nodes.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum NodeSelector {
    Id(OrgNodeId),
    Path(String),
    Property { key: String, value: String },
    Tag(String),
    Kind(String),
}
