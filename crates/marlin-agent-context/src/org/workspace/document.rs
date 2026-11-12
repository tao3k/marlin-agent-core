// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Document loading adapter from `Org` text into structured workspace nodes.

use marlin_context_org_model::{
    CheckboxState, LinkKind, OrgCheckbox, OrgContractReference, OrgContractReferenceScope,
    OrgContractRegistry, OrgContractResolutionReport, OrgContractValidationReport, OrgLink,
    OrgNode, OrgNodeId, OrgSourceSpan, TodoState,
};
use marlin_workspace_protocol::{WorkspaceError, WorkspaceResult};
use orgize::ast::{ParsedAnnotation, Section, parse_contracts_from_document};

use crate::org::workspace::contract::{
    contract_reference, document_contract_reference, project_contract_registry,
    resolve_contract_references,
};
use crate::org::workspace::validation::validate_contract_references;

/// Stable identifier for an imported `Org` document.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrgDocumentId(String);

impl OrgDocumentId {
    /// Create a document identifier from caller-owned text.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Borrow the document identifier as text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for OrgDocumentId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for OrgDocumentId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

/// Raw `Org` document payload accepted by the workspace backend.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrgDocument {
    pub id: OrgDocumentId,
    pub text: String,
}

/// Parser-owned workspace projection for one `Org` document.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OrgDocumentWorkspace {
    pub nodes: Vec<OrgNode>,
    pub contracts: OrgContractRegistry,
    pub contract_resolutions: OrgContractResolutionReport,
    pub contract_validations: OrgContractValidationReport,
}

impl OrgDocument {
    /// Create a document payload from an id and raw `Org` text.
    pub fn new(id: impl Into<OrgDocumentId>, text: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
        }
    }
}

/// Loader that projects a useful `Org` subset into `OrgNode` records.
pub struct OrgDocumentLoader;

impl OrgDocumentLoader {
    /// Parse document text into workspace nodes.
    pub fn load(document: &OrgDocument) -> WorkspaceResult<Vec<OrgNode>> {
        Self::load_workspace(document).map(|workspace| workspace.nodes)
    }

    /// Parse document text into workspace nodes and parser-owned contract facts.
    pub fn load_workspace(document: &OrgDocument) -> WorkspaceResult<OrgDocumentWorkspace> {
        OrgDocumentParser::new(document).parse(&[])
    }

    /// Parse document text with additional explicit contract registry documents.
    pub fn load_workspace_with_contracts(
        document: &OrgDocument,
        contract_documents: &[OrgDocument],
    ) -> WorkspaceResult<OrgDocumentWorkspace> {
        OrgDocumentParser::new(document).parse(contract_documents)
    }

    /// Select candidate documents that contain parser-owned Org contract registries.
    pub fn discover_contract_documents(
        candidates: &[OrgDocument],
    ) -> WorkspaceResult<Vec<OrgDocument>> {
        candidates
            .iter()
            .map(|document| {
                let org = parse_org_with_workspace_todo_keywords(&document.text)?;
                Ok((!parse_contracts_from_document(&org.document(), None)
                    .contracts
                    .is_empty())
                .then(|| document.clone()))
            })
            .collect::<WorkspaceResult<Vec<_>>>()
            .map(|documents| documents.into_iter().flatten().collect())
    }
}

struct OrgDocumentParser<'a> {
    document: &'a OrgDocument,
}

impl<'a> OrgDocumentParser<'a> {
    fn new(document: &'a OrgDocument) -> Self {
        Self { document }
    }

    fn parse(self, contract_documents: &[OrgDocument]) -> WorkspaceResult<OrgDocumentWorkspace> {
        let org = parse_org_with_workspace_todo_keywords(&self.document.text)?;
        let parsed_document = org.document();
        let mut parsed_contracts = parse_contracts_from_document(&parsed_document, None);
        for contract_document in contract_documents {
            let contract_org = parse_org_with_workspace_todo_keywords(&contract_document.text)?;
            parsed_contracts
                .contracts
                .extend(parse_contracts_from_document(&contract_org.document(), None).contracts);
        }
        let contracts = project_contract_registry(parsed_contracts.clone());
        let document_contract_reference = document_contract_reference(
            &parsed_document,
            self.document.id.as_str(),
            &self.document.text,
        );
        let mut projection = OrgNodeProjection::new(&self.document.id, &self.document.text);

        for section in &parsed_document.sections {
            projection.push_section(section, None, &org)?;
        }

        let (nodes, subtree_contract_references) = projection.into_parts();
        let contract_resolutions = resolve_contract_references(
            document_contract_reference
                .into_iter()
                .chain(subtree_contract_references)
                .collect(),
            &contracts,
        );
        let contract_validations = validate_contract_references(
            &parsed_document,
            &nodes,
            &parsed_contracts,
            &contract_resolutions,
        );

        Ok(OrgDocumentWorkspace {
            nodes,
            contracts,
            contract_resolutions,
            contract_validations,
        })
    }
}

struct OrgNodeProjection<'a> {
    document_id: &'a OrgDocumentId,
    document_text: &'a str,
    nodes: Vec<OrgNode>,
    contract_references: Vec<OrgContractReference>,
}

impl<'a> OrgNodeProjection<'a> {
    fn new(document_id: &'a OrgDocumentId, document_text: &'a str) -> Self {
        Self {
            document_id,
            document_text,
            nodes: Vec::new(),
            contract_references: Vec::new(),
        }
    }

    fn push_section(
        &mut self,
        headline: &Section<ParsedAnnotation>,
        parent: Option<OrgNodeId>,
        org: &orgize::Org,
    ) -> WorkspaceResult<OrgNodeId> {
        let level = headline.level;
        let title = headline_title(
            headline,
            self.line_number_at_byte(byte_offset(headline.ann.range.start())),
        );
        let id = self.node_id(headline, &title);
        let mut node = OrgNode::heading(id.clone(), title);

        node.source = Some(self.source_span(headline));
        node.todo = headline_todo(headline);
        if let Some(keyword) = &headline.todo {
            node.tokens.todo_keyword = self.todo_keyword_span(headline, &keyword.name);
        }
        node.tags.clone_from(&headline.tags);
        node.properties.insert(
            "DOCUMENT".to_string(),
            self.document_id.as_str().to_string(),
        );
        node.properties
            .insert("LEVEL".to_string(), level.to_string());

        if let Some(priority) = headline.priority.raw_cookie() {
            node.properties
                .insert("PRIORITY".to_string(), priority.to_string());
        }

        for property in &headline.properties {
            let key = property.key.as_str();
            let value = property.value.as_str();
            node.properties.insert(key.to_string(), value.to_string());
            if let Some(span) = self.property_value_span(headline, key, value) {
                if key.eq_ignore_ascii_case("CONTRACT_ORG") {
                    self.contract_references.push(contract_reference(
                        value,
                        OrgContractReferenceScope::Subtree,
                        Some(id.clone()),
                        Some(span.clone()),
                    ));
                }
                node.tokens.property_values.insert(key.to_string(), span);
            }
        }

        if let Some(section) = &headline.body_ann {
            let section_raw = section.raw.as_str();
            node.body = section_body(section_raw);
            let section_start = byte_offset(section.range.start());
            collect_section_facts(
                org,
                self.document_id,
                self.document_text,
                section_raw,
                section_start,
                &mut node,
            );
        }

        if let Some(parent_id) = parent.as_ref() {
            let parent_node = self
                .nodes
                .iter_mut()
                .find(|candidate| candidate.id == *parent_id)
                .ok_or_else(|| {
                    WorkspaceError::Backend(format!(
                        "missing parent heading {}",
                        parent_id.as_str()
                    ))
                })?;
            parent_node.children.push(id.clone());
        }

        self.nodes.push(node);

        for child in &headline.subsections {
            self.push_section(child, Some(id.clone()), org)?;
        }

        Ok(id)
    }

    fn into_parts(self) -> (Vec<OrgNode>, Vec<OrgContractReference>) {
        (self.nodes, self.contract_references)
    }

    fn node_id(&self, headline: &Section<ParsedAnnotation>, title: &str) -> OrgNodeId {
        let line_number = self.line_number_at_byte(byte_offset(headline.ann.range.start()));
        OrgNodeId::new(format!(
            "{}:{}:{}",
            self.document_id.as_str(),
            line_number,
            slug(title)
        ))
    }

    fn source_span(&self, headline: &Section<ParsedAnnotation>) -> OrgSourceSpan {
        let start_byte = byte_offset(headline.ann.range.start());
        let end_byte = byte_offset(headline.ann.range.end());
        source_span_for_range(self.document_id, self.document_text, start_byte, end_byte)
    }

    fn todo_keyword_span(
        &self,
        headline: &Section<ParsedAnnotation>,
        keyword: &str,
    ) -> Option<OrgSourceSpan> {
        let heading_start = byte_offset(headline.ann.range.start());
        let heading_end = self.document_text[heading_start..]
            .find('\n')
            .map(|offset| heading_start + offset)
            .unwrap_or_else(|| byte_offset(headline.ann.range.end()).min(self.document_text.len()));
        let heading_line = self.document_text.get(heading_start..heading_end)?;
        let search_start = headline.level;
        let keyword_start = heading_line.get(search_start..)?.find(keyword)? + search_start;
        let start_byte = heading_start + keyword_start;
        Some(source_span_for_range(
            self.document_id,
            self.document_text,
            start_byte,
            start_byte + keyword.len(),
        ))
    }

    fn property_value_span(
        &self,
        headline: &Section<ParsedAnnotation>,
        key: &str,
        value: &str,
    ) -> Option<OrgSourceSpan> {
        let headline_start = byte_offset(headline.ann.range.start());
        let headline_end = byte_offset(headline.ann.range.end()).min(self.document_text.len());
        let raw = self.document_text.get(headline_start..headline_end)?;
        let key_marker = format!(":{key}:");
        let mut found = None;

        for_line_with_offset(raw, headline_start, |line_start, line| {
            if found.is_some() {
                return;
            }
            let trimmed = line.trim_start();
            let indent = line.len() - trimmed.len();
            let Some(rest) = trimmed.strip_prefix(&key_marker) else {
                return;
            };
            let value_prefix = rest.len() - rest.trim_start().len();
            let value_start = line_start + indent + key_marker.len() + value_prefix;
            let value_end = if value.is_empty() {
                value_start
            } else {
                value_start + value.len()
            };
            found = Some(source_span_for_range(
                self.document_id,
                self.document_text,
                value_start,
                value_end,
            ));
        });

        found
    }

    fn line_number_at_byte(&self, byte_offset: usize) -> usize {
        line_number_at_byte(self.document_text, byte_offset)
    }
}

pub(crate) fn source_span_for_range(
    document_id: &OrgDocumentId,
    document_text: &str,
    start_byte: usize,
    end_byte: usize,
) -> OrgSourceSpan {
    OrgSourceSpan {
        document: document_id.as_str().to_string(),
        start_byte,
        end_byte,
        start_line: line_number_at_byte(document_text, start_byte),
        end_line: line_number_at_byte(document_text, end_byte.saturating_sub(1)),
    }
}

fn line_number_at_byte(document_text: &str, byte_offset: usize) -> usize {
    document_text
        .get(..byte_offset.min(document_text.len()))
        .unwrap_or_default()
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
        + 1
}

fn for_line_with_offset(raw: &str, base: usize, mut visit: impl FnMut(usize, &str)) {
    if raw.is_empty() {
        visit(base, raw);
        return;
    }

    raw.split_inclusive('\n')
        .scan(base, |line_start, segment| {
            let current = *line_start;
            *line_start += segment.len();
            Some((current, segment.strip_suffix('\n').unwrap_or(segment)))
        })
        .for_each(|(line_start, line)| visit(line_start, line));
}

fn byte_offset(offset: orgize::TextSize) -> usize {
    u32::from(offset) as usize
}

fn parse_org_with_workspace_todo_keywords(text: &str) -> WorkspaceResult<orgize::Org> {
    let mut config = orgize::ParseConfig::default();
    config.todo_keywords.0 = vec![
        "TODO".to_string(),
        "NEXT".to_string(),
        "WAIT".to_string(),
        "BLOCKED".to_string(),
    ];
    config.todo_keywords.1 = vec!["DONE".to_string()];
    orgize::org_aot::parse_org_aot_with_config(text, &config)
        .map_err(|error| WorkspaceError::Backend(format!("Org document parse failed: {error:?}")))
}

fn headline_title(headline: &Section<ParsedAnnotation>, line_number: usize) -> String {
    let title = headline.raw_title.trim().to_string();
    if title.is_empty() {
        format!("Untitled heading {line_number}")
    } else {
        title
    }
}

fn headline_todo(headline: &Section<ParsedAnnotation>) -> Option<TodoState> {
    headline
        .todo
        .as_ref()
        .map(|keyword| todo_state_from_keyword(&keyword.name))
}

fn todo_state_from_keyword(value: &str) -> TodoState {
    match value {
        "TODO" => TodoState::Todo,
        "NEXT" => TodoState::Next,
        "WAIT" => TodoState::Wait,
        "BLOCKED" => TodoState::Blocked,
        "DONE" => TodoState::Done,
        custom => TodoState::Custom(custom.to_string()),
    }
}

fn collect_section_facts(
    org: &orgize::Org,
    document_id: &OrgDocumentId,
    document_text: &str,
    section_raw: &str,
    section_start: usize,
    node: &mut OrgNode,
) {
    let section_end = section_start + section_raw.len();
    node.links.extend(
        org.records()
            .iter()
            .filter(|record| {
                record.kind == "link"
                    && byte_offset(record.range.start()) >= section_start
                    && byte_offset(record.range.end()) <= section_end
            })
            .map(|record| {
                let target = record.field("path").unwrap_or_default();
                OrgLink {
                    target: target.to_owned(),
                    kind: link_kind(target),
                    description: record
                        .field("description")
                        .filter(|description| !description.is_empty())
                        .map(str::to_owned),
                }
            }),
    );
    collect_checkbox_facts(document_id, document_text, section_raw, section_start, node);
}

fn collect_checkbox_facts(
    document_id: &OrgDocumentId,
    document_text: &str,
    section_raw: &str,
    section_start: usize,
    node: &mut OrgNode,
) {
    for_line_with_offset(section_raw, section_start, |line_start, line| {
        if let Some(checkbox) = checkbox_from_line(document_id, document_text, line_start, line) {
            node.tokens
                .checkbox_markers
                .push(checkbox.marker.clone().expect("projected checkbox marker"));
            node.checkboxes.push(checkbox);
        }
    });
}

fn checkbox_from_line(
    document_id: &OrgDocumentId,
    document_text: &str,
    line_start: usize,
    line: &str,
) -> Option<OrgCheckbox> {
    let trimmed = line.trim_start();
    let indent = line.len() - trimmed.len();
    let bullet_len = list_bullet_len(trimmed)?;
    let after_bullet = trimmed.get(bullet_len..)?;
    let spaces_after_bullet = after_bullet.len() - after_bullet.trim_start().len();
    let checkbox_start = indent + bullet_len + spaces_after_bullet;
    let marker = line.get(checkbox_start + 1..checkbox_start + 2)?;
    let state = match marker {
        " " => CheckboxState::Open,
        "X" | "x" => CheckboxState::Checked,
        "-" => CheckboxState::Partial,
        _ => return None,
    };
    if line.get(checkbox_start..checkbox_start + 3)? != format!("[{marker}]") {
        return None;
    }

    let source = source_span_for_range(
        document_id,
        document_text,
        line_start,
        line_start + line.len(),
    );
    let marker_start = line_start + checkbox_start + 1;
    let marker_span =
        source_span_for_range(document_id, document_text, marker_start, marker_start + 1);
    let text = line
        .get(checkbox_start + 3..)
        .unwrap_or_default()
        .trim()
        .to_string();

    Some(OrgCheckbox {
        text,
        state,
        source: Some(source),
        marker: Some(marker_span),
    })
}

fn list_bullet_len(trimmed: &str) -> Option<usize> {
    let bytes = trimmed.as_bytes();
    if matches!(bytes, [b'-' | b'+' | b'*', b' ', ..]) {
        return Some(2);
    }

    let marker = trimmed
        .char_indices()
        .find(|(_, character)| matches!(character, '.' | ')'))?;
    let after_marker = marker.0 + marker.1.len_utf8();
    trimmed
        .get(after_marker..)
        .is_some_and(|rest| rest.starts_with(' '))
        .then_some(after_marker + 1)
}

fn link_kind(target: &str) -> LinkKind {
    if target.starts_with("id:") {
        LinkKind::Id
    } else if target.starts_with("file:") {
        LinkKind::File
    } else if target.starts_with("http://") || target.starts_with("https://") {
        LinkKind::Url
    } else {
        LinkKind::Custom("org".to_string())
    }
}

fn section_body(raw: &str) -> Option<String> {
    let body = raw
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    (!body.is_empty()).then_some(body)
}

fn slug(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect()
}
