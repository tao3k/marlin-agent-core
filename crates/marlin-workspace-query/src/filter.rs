//! Filter model for workspace queries.

use serde::{Deserialize, Serialize};

use crate::{WorkspaceScope, selector::SourceRange};

/// Query request over a workspace scope.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceQuery {
    pub scope: WorkspaceScope,
    pub filters: Vec<QueryFilter>,
    pub order: QueryOrder,
    pub limit: Option<usize>,
}

impl WorkspaceQuery {
    pub fn new(scope: WorkspaceScope) -> Self {
        Self {
            scope,
            filters: Vec::new(),
            order: QueryOrder::DocumentOrder,
            limit: None,
        }
    }
}

/// Predicate used to match workspace nodes.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum QueryFilter {
    FullText(String),
    Property(PropertyFilter),
    Tag(String),
    TodoState(String),
    Kind(String),
    OpenCheckbox,
    EvidenceLinked,
    MemoryDispatch(String),
    SourceDocument(String),
    SourceRange(SourceRange),
}

/// Property key or key/value query predicate.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PropertyFilter {
    pub key: String,
    pub value: Option<String>,
}

/// Ordering mode for query results.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum QueryOrder {
    DocumentOrder,
    RecentlyUpdated,
    Priority,
    Explicit(Vec<String>),
}
