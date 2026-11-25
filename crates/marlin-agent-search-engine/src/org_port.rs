// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Marlin Org authority adapter implementing ASP's [`OrgDocumentProjectionPort`].
//!
//! Translates Marlin's [`OrgNode`] trees (projected once by Marlin's
//! `marlin-agent-context` / Scheme `orgize-fold`) into ASP's parser-neutral
//! [`OrgDocumentProjection`] and [`OrgOverlayDocumentProjection`] without
//! embedding a second Orgize/Gambit FFI runtime inside ASP.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use agent_semantic_topology::{
    OrgDocumentProjection, OrgDocumentProjectionPort, OrgOverlayDocumentProjection,
    OrgProjectionPortError, OrgPropertyEntry, OrgSectionProjection,
};
use marlin_context_org_model::{OrgNode, OrgNodeId, OrgNodeKind};

type NodeProjectorFn = dyn Fn(&str) -> Result<Vec<OrgNode>, OrgProjectionPortError> + Send + Sync;

/// Convert a Marlin [`OrgNode`] slice into ASP's [`OrgDocumentProjection`].
#[must_use]
pub fn project_org_nodes_to_document(source: &str, nodes: &[OrgNode]) -> OrgDocumentProjection {
    let parent_by_node_id = collect_parent_links(nodes);
    let document_properties = collect_document_properties(nodes);
    let sections = collect_heading_sections(nodes, &parent_by_node_id);
    OrgDocumentProjection::new(
        OrgDocumentProjection::digest_for_source(source),
        document_properties,
        sections,
    )
}

/// Convert a Marlin [`OrgNode`] slice into ASP's [`OrgOverlayDocumentProjection`].
#[must_use]
pub fn project_org_nodes_to_overlay(
    source: &str,
    nodes: &[OrgNode],
) -> OrgOverlayDocumentProjection {
    let has_headline = nodes.iter().any(|node| node.kind == OrgNodeKind::Heading);
    OrgOverlayDocumentProjection::new(
        source.to_owned(),
        OrgDocumentProjection::digest_for_source(source),
        has_headline,
    )
}

fn collect_parent_links(nodes: &[OrgNode]) -> HashMap<&OrgNodeId, &OrgNodeId> {
    nodes
        .iter()
        .flat_map(|node| {
            node.children
                .iter()
                .map(move |child_id| (child_id, &node.id))
        })
        .collect()
}

fn collect_document_properties(nodes: &[OrgNode]) -> Vec<OrgPropertyEntry> {
    nodes
        .iter()
        .filter(|node| node.kind == OrgNodeKind::Document)
        .flat_map(|node| {
            node.properties
                .iter()
                .map(|(key, value)| OrgPropertyEntry::new(key.clone(), value.clone()))
        })
        .collect()
}

fn collect_heading_sections(
    nodes: &[OrgNode],
    parent_by_node_id: &HashMap<&OrgNodeId, &OrgNodeId>,
) -> Vec<OrgSectionProjection> {
    let mut heading_index_by_id: HashMap<&OrgNodeId, usize> = HashMap::new();
    let mut sections = Vec::new();
    for node in nodes
        .iter()
        .filter(|node| node.kind == OrgNodeKind::Heading)
    {
        let parent_index = find_parent_heading_index(
            &node.id,
            parent_by_node_id,
            &heading_index_by_id,
            nodes.len(),
        );
        let level = parent_index.map_or(1, |idx| {
            sections
                .get(idx)
                .map_or(1, |parent: &OrgSectionProjection| {
                    parent.level.saturating_add(1)
                })
        });
        let properties = node
            .properties
            .iter()
            .map(|(key, value)| OrgPropertyEntry::new(key.clone(), value.clone()))
            .collect();
        let current_index = sections.len();
        heading_index_by_id.insert(&node.id, current_index);
        sections.push(OrgSectionProjection::new(level, parent_index, properties));
    }
    sections
}

fn find_parent_heading_index(
    node_id: &OrgNodeId,
    parent_by_node_id: &HashMap<&OrgNodeId, &OrgNodeId>,
    heading_index_by_id: &HashMap<&OrgNodeId, usize>,
    max_hops: usize,
) -> Option<usize> {
    let mut current = parent_by_node_id.get(node_id).copied();
    let mut hops = 0usize;
    while let Some(parent_id) = current {
        if let Some(&heading_index) = heading_index_by_id.get(parent_id) {
            return Some(heading_index);
        }
        hops += 1;
        if hops > max_hops {
            return None;
        }
        current = parent_by_node_id.get(parent_id).copied();
    }
    None
}

/// Marlin host adapter implementing ASP's [`OrgDocumentProjectionPort`].
pub struct MarlinContextOrgPort {
    projector: Arc<NodeProjectorFn>,
}

impl MarlinContextOrgPort {
    /// Construct an Org port from any Marlin-owned `OrgNode` projector
    /// (for example, wrapping `marlin-agent-context::gerbil_org::ContextOrgModule`).
    #[must_use]
    pub fn from_projector<F>(projector: F) -> Self
    where
        F: Fn(&str) -> Result<Vec<OrgNode>, OrgProjectionPortError> + Send + Sync + 'static,
    {
        Self {
            projector: Arc::new(projector),
        }
    }

    /// Construct a deterministic Org port backed by pre-projected [`OrgNode`] documents.
    #[must_use]
    pub fn from_node_map(documents: BTreeMap<String, Vec<OrgNode>>) -> Self {
        Self::from_projector(move |source| {
            documents.get(source).cloned().ok_or_else(|| {
                OrgProjectionPortError::new(
                    "marlin-org-source-unregistered",
                    "Org source was not pre-projected by Marlin Context",
                )
            })
        })
    }

    /// Construct a live zero-FFI Org port that parses raw Org source directly
    /// into Marlin [`OrgNode`] trees (document drawers, nested headings, and
    /// section `:PROPERTIES:` drawers).
    #[must_use]
    pub fn with_live_org_parser() -> Self {
        Self::from_projector(|source| Ok(parse_live_org_nodes(source)))
    }
}

/// Parse raw Org text directly into Marlin [`OrgNode`] trees without any C/FFI bridge.
#[must_use]
pub fn parse_live_org_nodes(source: &str) -> Vec<OrgNode> {
    let mut parser = LiveOrgParser::new();
    for raw_line in source.lines() {
        parser.apply_line(raw_line);
    }
    parser.finish()
}

struct LiveOrgParser {
    nodes: Vec<OrgNode>,
    heading_stack: Vec<(usize, usize)>,
    active_node_idx: usize,
    in_properties: bool,
}

impl LiveOrgParser {
    fn new() -> Self {
        let mut doc = OrgNode::heading("doc", "Document");
        doc.kind = OrgNodeKind::Document;
        Self {
            nodes: vec![doc],
            heading_stack: Vec::new(),
            active_node_idx: 0,
            in_properties: false,
        }
    }

    fn apply_line(&mut self, raw_line: &str) {
        if let Some((level, title)) = parse_org_headline(raw_line) {
            self.push_heading(level, title);
            return;
        }
        let trimmed = raw_line.trim();
        if trimmed.eq_ignore_ascii_case(":PROPERTIES:") {
            self.in_properties = true;
        } else if self.in_properties && trimmed.eq_ignore_ascii_case(":END:") {
            self.in_properties = false;
        } else if self.in_properties
            && let Some((key, value)) = parse_property_entry(trimmed)
        {
            self.nodes[self.active_node_idx]
                .properties
                .insert(key, value);
        }
    }

    fn push_heading(&mut self, level: usize, title: &str) {
        self.in_properties = false;
        while self
            .heading_stack
            .last()
            .is_some_and(|(parent_level, _)| *parent_level >= level)
        {
            self.heading_stack.pop();
        }
        let parent_idx = self.heading_stack.last().map_or(0, |(_, idx)| *idx);
        let node_idx = self.nodes.len();
        let node_id = format!("heading-{node_idx}");
        self.nodes[parent_idx]
            .children
            .push(OrgNodeId::from(node_id.clone()));
        self.nodes.push(OrgNode::heading(node_id, title));
        self.heading_stack.push((level, node_idx));
        self.active_node_idx = node_idx;
    }

    fn finish(self) -> Vec<OrgNode> {
        self.nodes
    }
}

fn parse_org_headline(line: &str) -> Option<(usize, &str)> {
    let stars = line.bytes().take_while(|byte| *byte == b'*').count();
    if stars == 0 {
        return None;
    }
    let rest = line.get(stars..)?;
    let title = rest.strip_prefix(' ')?.trim();
    Some((stars, title))
}

fn parse_property_entry(trimmed: &str) -> Option<(String, String)> {
    let rest = trimmed.strip_prefix(':')?;
    let (key, value) = rest.split_once(':')?;
    let key = key.trim();
    if key.is_empty() {
        return None;
    }
    Some((key.to_owned(), value.trim().to_owned()))
}

impl OrgDocumentProjectionPort for MarlinContextOrgPort {
    fn project_document(
        &self,
        source: &str,
    ) -> Result<OrgDocumentProjection, OrgProjectionPortError> {
        let nodes = (self.projector)(source)?;
        Ok(project_org_nodes_to_document(source, &nodes))
    }

    fn inspect_overlay(
        &self,
        source: &str,
    ) -> Result<OrgOverlayDocumentProjection, OrgProjectionPortError> {
        let nodes = (self.projector)(source)?;
        Ok(project_org_nodes_to_overlay(source, &nodes))
    }
}
