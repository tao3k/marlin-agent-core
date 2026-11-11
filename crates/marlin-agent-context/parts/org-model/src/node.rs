//! `Org` node model for structured workspace records.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::{OrgBlock, OrgCheckbox, OrgLink, OrgTable, TodoState};

/// Stable identifier for an `Org` workspace node.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct OrgNodeId(String);

impl OrgNodeId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for OrgNodeId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for OrgNodeId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

/// Structured `Org` node projected from parser-owned document facts.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OrgNode {
    pub id: OrgNodeId,
    pub kind: OrgNodeKind,
    pub source: Option<OrgSourceSpan>,
    pub tokens: OrgNodeSourceTokens,
    pub title: Option<String>,
    pub body: Option<String>,
    pub todo: Option<TodoState>,
    pub tags: Vec<String>,
    pub properties: BTreeMap<String, String>,
    pub checkboxes: Vec<OrgCheckbox>,
    pub planning: Vec<OrgTimestamp>,
    pub links: Vec<OrgLink>,
    pub blocks: Vec<OrgBlock>,
    pub tables: Vec<OrgTable>,
    pub children: Vec<OrgNodeId>,
}

impl OrgNode {
    pub fn heading(id: impl Into<OrgNodeId>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            kind: OrgNodeKind::Heading,
            source: None,
            tokens: OrgNodeSourceTokens::default(),
            title: Some(title.into()),
            body: None,
            todo: None,
            tags: Vec::new(),
            properties: BTreeMap::new(),
            checkboxes: Vec::new(),
            planning: Vec::new(),
            links: Vec::new(),
            blocks: Vec::new(),
            tables: Vec::new(),
            children: Vec::new(),
        }
    }
}

/// Parser-owned source span for a projected `Org` node.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OrgSourceSpan {
    pub document: String,
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_line: usize,
    pub end_line: usize,
}

/// Token-level source spans attached to a projected `Org` node.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct OrgNodeSourceTokens {
    pub todo_keyword: Option<OrgSourceSpan>,
    pub property_values: BTreeMap<String, OrgSourceSpan>,
    pub checkbox_markers: Vec<OrgSourceSpan>,
}

/// High-level node kind used by the workspace protocol.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum OrgNodeKind {
    Document,
    Heading,
    Section,
    List,
    ListItem,
    Paragraph,
    Drawer,
    Block,
    Table,
}

/// Property drawer entry attached to an `Org` node.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OrgProperty {
    pub key: String,
    pub value: String,
}

/// Timestamp value associated with planning metadata.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OrgTimestamp {
    pub kind: TimestampKind,
    pub value: String,
}

/// Planning timestamp class.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum TimestampKind {
    Scheduled,
    Deadline,
    Closed,
    Active,
    Inactive,
}
