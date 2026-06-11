//! Document loading adapter from `Org` text into structured workspace nodes.

use marlin_org_model::{
    CheckboxState, LinkKind, OrgCheckbox, OrgContract, OrgContractAssertion, OrgContractBinding,
    OrgContractElementCategory, OrgContractElementKind, OrgContractExpectation, OrgContractKind,
    OrgContractQuery, OrgContractRegistry, OrgContractRelativeScope, OrgContractScope,
    OrgContractSeverity, OrgContractSourceSpan, OrgLink, OrgNode, OrgNodeId, OrgSourceSpan,
    TodoState,
};
use marlin_workspace_protocol::{WorkspaceError, WorkspaceResult};
use orgize::ast::parse_contracts_from_document;
use orgize::export::{Container, Event, TraversalContext, Traverser};
use orgize::syntax_ast::{Headline, SyntaxLink};

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
        OrgDocumentParser::new(document).parse()
    }
}

struct OrgDocumentParser<'a> {
    document: &'a OrgDocument,
}

impl<'a> OrgDocumentParser<'a> {
    fn new(document: &'a OrgDocument) -> Self {
        Self { document }
    }

    fn parse(self) -> WorkspaceResult<OrgDocumentWorkspace> {
        let org = parse_org_with_workspace_todo_keywords(&self.document.text);
        let parsed_document = org.document();
        let contracts =
            project_contract_registry(parse_contracts_from_document(&parsed_document, None));
        let mut projection = OrgAstProjection::new(&self.document.id, &self.document.text);

        for headline in org.syntax_document().headlines() {
            projection.push_headline(headline, None)?;
        }

        Ok(OrgDocumentWorkspace {
            nodes: projection.into_nodes(),
            contracts,
        })
    }
}

struct OrgAstProjection<'a> {
    document_id: &'a OrgDocumentId,
    document_text: &'a str,
    nodes: Vec<OrgNode>,
}

impl<'a> OrgAstProjection<'a> {
    fn new(document_id: &'a OrgDocumentId, document_text: &'a str) -> Self {
        Self {
            document_id,
            document_text,
            nodes: Vec::new(),
        }
    }

    fn push_headline(
        &mut self,
        headline: Headline,
        parent: Option<OrgNodeId>,
    ) -> WorkspaceResult<OrgNodeId> {
        let level = headline.level();
        let title = headline_title(
            &headline,
            self.line_number_at_byte(byte_offset(headline.start())),
        );
        let id = self.node_id(&headline, &title);
        let mut node = OrgNode::heading(id.clone(), title);

        node.source = Some(self.source_span(&headline));
        node.todo = headline_todo(&headline);
        if let Some(keyword) = headline.todo_keyword() {
            node.tokens.todo_keyword = self.todo_keyword_span(&headline, keyword.as_ref());
        }
        node.tags = headline.tags().map(|tag| tag.to_string()).collect();
        node.properties.insert(
            "DOCUMENT".to_string(),
            self.document_id.as_str().to_string(),
        );
        node.properties
            .insert("LEVEL".to_string(), level.to_string());

        if let Some(priority) = headline.priority() {
            node.properties
                .insert("PRIORITY".to_string(), priority.to_string());
        }

        if let Some(properties) = headline.properties() {
            for (key, value) in properties.iter() {
                node.properties.insert(key.to_string(), value.to_string());
                if let Some(span) =
                    self.property_value_span(&headline, key.as_ref(), value.as_ref())
                {
                    node.tokens.property_values.insert(key.to_string(), span);
                }
            }
        }

        if let Some(section) = headline.section() {
            let section_raw = section.raw();
            node.body = section_body(&section_raw);
            let section_start = self.section_start_byte(&headline, &section_raw);
            collect_section_facts(
                self.document_id,
                self.document_text,
                &section_raw,
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

        for child in headline.headlines() {
            self.push_headline(child, Some(id.clone()))?;
        }

        Ok(id)
    }

    fn into_nodes(self) -> Vec<OrgNode> {
        self.nodes
    }

    fn node_id(&self, headline: &Headline, title: &str) -> OrgNodeId {
        let line_number = self.line_number_at_byte(byte_offset(headline.start()));
        OrgNodeId::new(format!(
            "{}:{}:{}",
            self.document_id.as_str(),
            line_number,
            slug(title)
        ))
    }

    fn source_span(&self, headline: &Headline) -> OrgSourceSpan {
        let start_byte = byte_offset(headline.start());
        let end_byte = byte_offset(headline.end());
        source_span_for_range(self.document_id, self.document_text, start_byte, end_byte)
    }

    fn todo_keyword_span(&self, headline: &Headline, keyword: &str) -> Option<OrgSourceSpan> {
        let heading_start = byte_offset(headline.start());
        let heading_end = self.document_text[heading_start..]
            .find('\n')
            .map(|offset| heading_start + offset)
            .unwrap_or_else(|| byte_offset(headline.end()).min(self.document_text.len()));
        let heading_line = self.document_text.get(heading_start..heading_end)?;
        let search_start = headline.level();
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
        headline: &Headline,
        key: &str,
        value: &str,
    ) -> Option<OrgSourceSpan> {
        let headline_start = byte_offset(headline.start());
        let headline_end = byte_offset(headline.end()).min(self.document_text.len());
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

    fn section_start_byte(&self, headline: &Headline, section_raw: &str) -> usize {
        let headline_start = byte_offset(headline.start());
        let headline_end = byte_offset(headline.end()).min(self.document_text.len());
        self.document_text
            .get(headline_start..headline_end)
            .and_then(|raw| raw.find(section_raw))
            .map(|offset| headline_start + offset)
            .unwrap_or(headline_start)
    }

    fn line_number_at_byte(&self, byte_offset: usize) -> usize {
        line_number_at_byte(self.document_text, byte_offset)
    }
}

fn source_span_for_range(
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

fn parse_org_with_workspace_todo_keywords(text: &str) -> orgize::Org {
    let mut config = orgize::ParseConfig::default();
    config.todo_keywords.0 = vec![
        "TODO".to_string(),
        "NEXT".to_string(),
        "WAIT".to_string(),
        "BLOCKED".to_string(),
    ];
    config.todo_keywords.1 = vec!["DONE".to_string()];
    config.parse(text)
}

fn project_contract_registry(registry: orgize::ast::OrgContractRegistry) -> OrgContractRegistry {
    OrgContractRegistry {
        contracts: registry.contracts.iter().map(project_contract).collect(),
    }
}

fn project_contract(contract: &orgize::ast::OrgContract) -> OrgContract {
    OrgContract {
        id: contract.id.clone(),
        aliases: contract.aliases.clone(),
        scope: OrgContractScope::new(debug_label(&contract.scope)),
        kind: OrgContractKind::new(debug_label(&contract.kind)),
        assertions: contract
            .assertions
            .iter()
            .map(project_contract_assertion)
            .collect(),
    }
}

fn project_contract_assertion(
    assertion: &orgize::ast::OrgContractAssertion,
) -> OrgContractAssertion {
    OrgContractAssertion {
        id: assertion.id.clone(),
        severity: OrgContractSeverity::new(debug_label(&assertion.severity)),
        bindings: assertion
            .bindings
            .iter()
            .map(|binding| OrgContractBinding {
                name: binding.name.clone(),
                query: project_contract_query(&binding.query),
            })
            .collect(),
        query: project_contract_query(&assertion.query),
        expectation: OrgContractExpectation::new(debug_label(&assertion.expectation)),
        message: assertion.message.clone(),
        fix: assertion.fix.clone(),
        query_source: assertion
            .query_source
            .as_ref()
            .map(|source| OrgContractSourceSpan {
                start_line: source.start.line,
                start_column: source.start.column,
                end_line: source.end.line,
                end_column: source.end.column,
                start_byte: source.range_start as usize,
                end_byte: source.range_end as usize,
            }),
        expect_source: assertion
            .expect_source
            .as_ref()
            .map(|source| OrgContractSourceSpan {
                start_line: source.start.line,
                start_column: source.start.column,
                end_line: source.end.line,
                end_column: source.end.column,
                start_byte: source.range_start as usize,
                end_byte: source.range_end as usize,
            }),
    }
}

fn project_contract_query(query: &orgize::ast::OrgContractQuery) -> OrgContractQuery {
    OrgContractQuery {
        category: query
            .category
            .as_ref()
            .map(|category| OrgContractElementCategory::new(debug_label(category))),
        kind: query
            .kind
            .as_ref()
            .map(|kind| OrgContractElementKind::new(debug_label(kind))),
        affiliated_name: query.affiliated_name.clone(),
        context: query.context.clone(),
        outline_path_prefix: query.outline_path_prefix.clone(),
        outline_path_exact_len: query.outline_path_exact_len,
        property_equals: query.property_equals.clone(),
        property_contains: query.property_contains.clone(),
        summary_equals: query.summary_equals.clone(),
        summary_contains: query.summary_contains.clone(),
        limit: query.limit,
        use_scope_outline_path: query.use_scope_outline_path,
        has_outline_path_prefix: query.has_outline_path_prefix,
        scope_outline_depth: query.scope_outline_depth,
        relative_to: query
            .relative_to
            .as_ref()
            .map(|scope| OrgContractRelativeScope::new(debug_label(scope))),
    }
}

fn debug_label(value: &impl std::fmt::Debug) -> String {
    format!("{value:?}")
}

fn headline_title(headline: &Headline, line_number: usize) -> String {
    let title = headline.title_raw().trim().to_string();
    if title.is_empty() {
        format!("Untitled heading {line_number}")
    } else {
        title
    }
}

fn headline_todo(headline: &Headline) -> Option<TodoState> {
    headline
        .todo_keyword()
        .map(|keyword| todo_state_from_keyword(keyword.as_ref()))
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
    document_id: &OrgDocumentId,
    document_text: &str,
    section_raw: &str,
    section_start: usize,
    node: &mut OrgNode,
) {
    let mut collector = SectionFactCollector::new(node);
    orgize::Org::parse(section_raw).traverse(&mut collector);
    collect_checkbox_facts(document_id, document_text, section_raw, section_start, node);
}

struct SectionFactCollector<'a> {
    node: &'a mut OrgNode,
}

impl<'a> SectionFactCollector<'a> {
    fn new(node: &'a mut OrgNode) -> Self {
        Self { node }
    }
}

impl Traverser for SectionFactCollector<'_> {
    fn event(&mut self, event: Event, _ctx: &mut TraversalContext) {
        if let Event::Enter(Container::Link(link)) = event {
            self.node.links.push(link_from_syntax(link));
        }
    }
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

fn link_from_syntax(link: SyntaxLink) -> OrgLink {
    let target = link.path().to_string();
    let description = if link.has_description() {
        let raw_description = link.description_raw();
        (!raw_description.is_empty()).then_some(raw_description)
    } else {
        None
    };

    OrgLink {
        kind: link_kind(&target),
        target,
        description,
    }
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
