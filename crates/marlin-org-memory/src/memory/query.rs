//! Query helpers for `MemoryOrgWorkspace`.

use crate::memory::format::todo_state;
use marlin_org_model::{CheckboxState, LinkKind, OrgNode, OrgNodeId, OrgNodeKind};
use marlin_workspace_query::{
    PropertyFilter, QueryFilter, QueryOrder, SourceRange, WorkspaceScope,
};
use std::collections::{BTreeMap, HashSet};

pub(super) fn scope_nodes<'a>(
    nodes: &'a BTreeMap<OrgNodeId, OrgNode>,
    scope: &WorkspaceScope,
) -> Vec<&'a OrgNode> {
    match scope {
        WorkspaceScope::WholeWorkspace => nodes.values().collect(),
        WorkspaceScope::Document(path) => nodes
            .values()
            .filter(|node| node_in_document(node, path))
            .collect(),
        WorkspaceScope::SourceRange(range) => nodes
            .values()
            .filter(|node| node_in_source_range(node, range))
            .collect(),
        WorkspaceScope::Subtree(root) => {
            let mut ids = HashSet::new();
            collect_subtree_ids(nodes, root, &mut ids);
            nodes
                .values()
                .filter(|node| ids.contains(&node.id))
                .collect()
        }
        WorkspaceScope::Nodes(ids) => ids.iter().filter_map(|id| nodes.get(id)).collect(),
    }
}

pub(super) fn matches_filter(node: &OrgNode, filter: &QueryFilter) -> bool {
    match filter {
        QueryFilter::FullText(term) => node_text(node).contains(&term.to_lowercase()),
        QueryFilter::Property(property) => matches_property(node, property),
        QueryFilter::Tag(tag) => node.tags.iter().any(|candidate| candidate == tag),
        QueryFilter::TodoState(state) => node
            .todo
            .as_ref()
            .is_some_and(|todo| todo_state(todo) == *state),
        QueryFilter::Kind(kind) => node_kind(&node.kind) == *kind,
        QueryFilter::OpenCheckbox => node
            .checkboxes
            .iter()
            .any(|checkbox| checkbox.state == CheckboxState::Open),
        QueryFilter::EvidenceLinked => node
            .links
            .iter()
            .any(|link| matches!(&link.kind, LinkKind::Custom(kind) if kind == "evidence")),
        QueryFilter::MemoryDispatch(dispatch) => {
            node.properties.get("MEMORY_DISPATCH") == Some(dispatch)
        }
        QueryFilter::SourceDocument(document) => node_in_document(node, document),
        QueryFilter::SourceRange(range) => node_in_source_range(node, range),
    }
}

pub(super) fn order_nodes(nodes: &mut Vec<&OrgNode>, order: &QueryOrder) {
    match order {
        QueryOrder::DocumentOrder | QueryOrder::RecentlyUpdated | QueryOrder::Priority => {}
        QueryOrder::Explicit(ids) => nodes.sort_by_key(|node| {
            ids.iter()
                .position(|id| id == node.id.as_str())
                .unwrap_or(usize::MAX)
        }),
    }
}

pub(super) fn match_reason(node: &OrgNode) -> String {
    node.source
        .as_ref()
        .map(|source| {
            format!(
                "in-memory match at {}:{}-{}",
                source.document, source.start_line, source.end_line
            )
        })
        .unwrap_or_else(|| "in-memory match".to_string())
}

fn node_in_document(node: &OrgNode, document: &str) -> bool {
    node.source
        .as_ref()
        .is_some_and(|source| source.document == document)
        || node
            .properties
            .get("DOCUMENT")
            .is_some_and(|value| value == document)
}

fn node_in_source_range(node: &OrgNode, range: &SourceRange) -> bool {
    node.source.as_ref().is_some_and(|source| {
        source.document == range.document
            && source.start_line <= range.end_line
            && source.end_line >= range.start_line
    })
}

fn matches_property(node: &OrgNode, property: &PropertyFilter) -> bool {
    match &property.value {
        Some(value) => node.properties.get(&property.key) == Some(value),
        None => node.properties.contains_key(&property.key),
    }
}

fn collect_subtree_ids(
    nodes: &BTreeMap<OrgNodeId, OrgNode>,
    current: &OrgNodeId,
    ids: &mut HashSet<OrgNodeId>,
) {
    if !ids.insert(current.clone()) {
        return;
    }
    if let Some(node) = nodes.get(current) {
        for child in &node.children {
            collect_subtree_ids(nodes, child, ids);
        }
    }
}

fn node_text(node: &OrgNode) -> String {
    let mut text = String::new();
    if let Some(title) = &node.title {
        text.push_str(title);
    }
    if let Some(body) = &node.body {
        text.push(' ');
        text.push_str(body);
    }
    for value in node.properties.values() {
        text.push(' ');
        text.push_str(value);
    }
    text.to_lowercase()
}

fn node_kind(kind: &OrgNodeKind) -> String {
    format!("{kind:?}").to_lowercase()
}
