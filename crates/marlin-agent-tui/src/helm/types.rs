// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Core Helm value types, two-stage Consult/Orderless parser, playbook admission,
//! and candidate projection helpers.

use std::sync::{Arc, RwLock};

use marlin_agent_search_engine::{
    SearchDslChoiceWitness, SearchDslChoiceWitnessKind, SearchDslExecutionMode,
    SearchDslStructuredResult, SearchDslStructuredRow, SearchPlaybookComposition,
    SearchPlaybookLeaf, SearchPlaybookPlanInput, TopologyOwnerMembershipBlock,
    WorkspaceContextLine, WorkspaceSearchHit, WorkspaceSearchIndex, admit_search_playbook_plan,
};
use marlin_agent_ui::{UiBufferId, UiCommand, UiCommandName};

pub(crate) type SharedSearchIndex = Arc<RwLock<WorkspaceSearchIndex>>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum HelmSourceKind {
    All,
    Dsl,
    Search,
    Buffers,
    Commands,
}

impl HelmSourceKind {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Dsl => "dsl",
            Self::Search => "search",
            Self::Buffers => "buffers",
            Self::Commands => "commands",
        }
    }

    pub(crate) fn section_title(self) -> &'static str {
        match self {
            Self::All => "All Sources",
            Self::Dsl => "Ascent / Dusa DSL",
            Self::Search => "Workspace Search",
            Self::Buffers => "Buffers",
            Self::Commands => "Commands (M-x)",
        }
    }

    pub(super) fn next(self) -> Self {
        match self {
            Self::All => Self::Search,
            Self::Search => Self::Dsl,
            Self::Dsl => Self::Buffers,
            Self::Buffers => Self::Commands,
            Self::Commands => Self::All,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct HelmParsedInput {
    pub(crate) async_pattern: String,
    pub(crate) local_tokens: Vec<String>,
    pub(crate) languages: Vec<String>,
    pub(crate) documents: Vec<String>,
    pub(crate) path_prefix: Option<String>,
    pub(crate) extension: Option<String>,
    pub(crate) symbol: Option<String>,
    pub(crate) graph_target: Option<String>,
}

#[derive(Eq, PartialEq)]
pub(super) struct HelmAsyncKey<'a> {
    async_pattern: &'a str,
    languages: &'a [String],
    documents: &'a [String],
    path_prefix: Option<&'a str>,
    extension: Option<&'a str>,
    symbol: Option<&'a str>,
    graph_target: Option<&'a str>,
}

impl HelmParsedInput {
    pub(crate) fn parse(raw: &str) -> Self {
        if raw.trim_start().starts_with('(') {
            return Self {
                async_pattern: raw.trim().to_owned(),
                local_tokens: Vec::new(),
                languages: Vec::new(),
                documents: Vec::new(),
                path_prefix: None,
                extension: None,
                symbol: None,
                graph_target: None,
            };
        }
        let (async_raw, local_raw) = match raw.find('#') {
            Some(idx) => (raw[..idx].trim(), raw[idx + 1..].trim()),
            None => (raw.trim(), ""),
        };
        let mut pattern_parts = Vec::new();
        let mut languages = Vec::new();
        let mut documents = Vec::new();
        let mut path_prefix = None;
        let mut extension = None;
        let mut symbol = None;
        let mut graph_target = None;

        for token in async_raw.split_whitespace() {
            if let Some(val) = token.strip_prefix("@lang:") {
                if !val.is_empty() && !languages.iter().any(|item| item == val) {
                    languages.push(val.to_owned());
                }
            } else if let Some(val) = token.strip_prefix("@doc:") {
                if !val.is_empty() && !documents.iter().any(|item| item == val) {
                    documents.push(val.to_owned());
                }
            } else if let Some(val) = token.strip_prefix("@prefix:") {
                if !val.is_empty() {
                    path_prefix = Some(val.to_owned());
                }
            } else if let Some(val) = token.strip_prefix("@ext:") {
                if !val.is_empty() {
                    extension = Some(val.to_owned());
                }
            } else if let Some(val) = token.strip_prefix("@sym:") {
                if !val.is_empty() {
                    symbol = Some(val.to_owned());
                }
            } else if let Some(val) = token.strip_prefix("@graph:") {
                if !val.is_empty() {
                    graph_target = Some(val.to_owned());
                }
            } else {
                pattern_parts.push(token);
            }
        }

        let local_tokens = local_raw
            .split(|ch: char| ch.is_whitespace() || ch == '#')
            .filter(|tok| !tok.is_empty())
            .map(str::to_owned)
            .collect::<Vec<_>>();

        Self {
            async_pattern: pattern_parts.join(" "),
            local_tokens,
            languages,
            documents,
            path_prefix,
            extension,
            symbol,
            graph_target,
        }
    }

    pub(super) fn async_key(&self) -> HelmAsyncKey<'_> {
        HelmAsyncKey {
            async_pattern: &self.async_pattern,
            languages: &self.languages,
            documents: &self.documents,
            path_prefix: self.path_prefix.as_deref(),
            extension: self.extension.as_deref(),
            symbol: self.symbol.as_deref(),
            graph_target: self.graph_target.as_deref(),
        }
    }

    pub(crate) fn highlight_needles(&self) -> Vec<String> {
        let mut needles = Vec::new();
        if !self.async_pattern.is_empty() {
            needles.push(self.async_pattern.clone());
        }
        if let Some(sym) = self.symbol.as_deref()
            && !sym.is_empty()
        {
            needles.push(sym.to_owned());
        }
        if let Some(graph) = self.graph_target.as_deref()
            && !graph.is_empty()
        {
            needles.push(graph.to_owned());
        }
        needles.extend(
            self.local_tokens
                .iter()
                .filter(|tok| !tok.is_empty() && !tok.starts_with('!'))
                .cloned(),
        );
        needles
    }
}

#[derive(Clone, Debug)]
pub(crate) struct HelmCandidate {
    pub(crate) id: String,
    pub(crate) source: HelmSourceKind,
    pub(crate) display: String,
    pub(crate) path: Option<String>,
    pub(crate) language: Option<String>,
    pub(crate) primary_symbol: Option<String>,
    pub(crate) graph_relations: Vec<String>,
    pub(crate) snippet: String,
    pub(crate) citation: Option<String>,
    pub(crate) badge: String,
    pub(crate) context_lines: Vec<WorkspaceContextLine>,
    pub(crate) buffer_id: Option<UiBufferId>,
    pub(crate) command_name: Option<UiCommandName>,
}

impl HelmCandidate {
    pub(super) fn matches_tokens(&self, tokens: &[String]) -> bool {
        if tokens.is_empty() {
            return true;
        }
        let haystack = format!(
            "{} {} {} {} {}",
            self.display.to_lowercase(),
            self.snippet.to_lowercase(),
            self.badge.to_lowercase(),
            self.graph_relations.join(" ").to_lowercase(),
            self.source.label()
        );
        tokens.iter().all(|token| {
            if token.is_empty() {
                true
            } else if let Some(neg) = token.strip_prefix('!') {
                if neg.is_empty() {
                    true
                } else {
                    !haystack.contains(&neg.to_lowercase())
                }
            } else {
                haystack.contains(&token.to_lowercase())
            }
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum HelmDisplayRow {
    SourceHeader {
        source: HelmSourceKind,
        count: usize,
        meta: String,
    },
    Candidate {
        filtered_pos: usize,
        raw_index: usize,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum HelmMenuActionKind {
    Primary,
    ExportCitations,
    ScopeDirectory,
    ScopeLanguage,
    ScopeSymbol,
    ScopeGraph,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct HelmMenuActionItem {
    pub(crate) kind: HelmMenuActionKind,
    pub(crate) key: &'static str,
    pub(crate) label: String,
    pub(crate) detail: String,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum HelmAction {
    InsertCitations(String),
    SelectBuffer(UiBufferId),
    PrepareCommand(UiCommandName),
}

fn find_matching_choice_witness<'a>(
    row: &SearchDslStructuredRow,
    result: &'a SearchDslStructuredResult,
) -> Option<(usize, &'a SearchDslChoiceWitness)> {
    for world in &result.choice_worlds {
        let prefix = format!("[W#{}] ", world.world_index);
        if !row.display.starts_with(&prefix) {
            continue;
        }
        let matched = world.witnesses.iter().find(|w| {
            w.fact.len() == row.cells.len() + 1 && w.fact.iter().skip(1).eq(row.cells.iter())
        });
        if let Some(witness) = matched {
            return Some((world.world_index, witness));
        }
    }
    None
}

fn append_choice_witness_preview(
    context_lines: &mut Vec<WorkspaceContextLine>,
    world_index: usize,
    witness: &SearchDslChoiceWitness,
) {
    let base_line = context_lines
        .last()
        .map_or(1, |ctx| ctx.line_number.saturating_add(1));
    context_lines.push(WorkspaceContextLine {
        line_number: base_line,
        text: format!(
            "── Dusa World #{world_index} [{} · {} rule] ──",
            witness.kind.as_str(),
            witness.rule_kind.as_str()
        ),
        is_match: true,
    });
    if !witness.premises.is_empty() {
        context_lines.push(WorkspaceContextLine {
            line_number: base_line + 1,
            text: format!("via: {}", witness.premises.join(" ; ")),
            is_match: false,
        });
    }
    if !witness.alternatives.is_empty() {
        context_lines.push(WorkspaceContextLine {
            line_number: base_line + 2,
            text: format!("alts: {}", witness.alternatives.join(", ")),
            is_match: false,
        });
    }
}

pub(super) fn dsl_row_to_candidate(
    idx: usize,
    row: &SearchDslStructuredRow,
    result: &SearchDslStructuredResult,
    index: Option<&WorkspaceSearchIndex>,
    context_radius: usize,
) -> HelmCandidate {
    let (display, citation, mut context_lines) = match (&row.path, row.line) {
        (Some(path), Some(line)) => {
            let ctx = index
                .map(|idx_ref| idx_ref.live_file_context(path, line, context_radius))
                .unwrap_or_default();
            (
                format!("{path}:{line}"),
                Some(format!("@{path}:{line}")),
                ctx,
            )
        }
        (Some(path), None) => {
            let ctx = index
                .map(|idx_ref| idx_ref.live_file_context(path, 1, context_radius))
                .unwrap_or_default();
            (path.clone(), Some(format!("@{path}")), ctx)
        }
        _ => (row.display.clone(), Some(row.display.clone()), Vec::new()),
    };
    let mut badge = format!("{}:{}", result.mode.as_str(), result.relation);
    let mut graph_relations = result.repairs.clone();
    if result.mode == SearchDslExecutionMode::Choice
        && let Some((world_index, witness)) = find_matching_choice_witness(row, result)
    {
        let pill = match witness.kind {
            SearchDslChoiceWitnessKind::Deduction => "deduction",
            SearchDslChoiceWitnessKind::Branch => "branch",
            SearchDslChoiceWitnessKind::Derived => "derived",
        };
        badge = format!("W#{world_index}·{pill}");
        graph_relations.insert(
            0,
            format!(
                "Dusa W#{world_index} [{pill}:{}]",
                witness.rule_kind.as_str()
            ),
        );
        append_choice_witness_preview(&mut context_lines, world_index, witness);
    }
    HelmCandidate {
        id: format!("dsl:{}:{}:{idx}", result.relation, display),
        source: HelmSourceKind::Dsl,
        display,
        path: row.path.clone(),
        language: None,
        primary_symbol: row
            .cells
            .iter()
            .find(|c| Some(*c) != row.path.as_ref() && c.parse::<usize>().is_err())
            .cloned(),
        graph_relations,
        snippet: if row.path.is_some() {
            row.display.clone()
        } else {
            String::new()
        },
        citation,
        badge,
        context_lines,
        buffer_id: None,
        command_name: None,
    }
}

pub(super) fn admit_helm_playbook(parsed: &HelmParsedInput) -> Result<(), String> {
    let raw = if parsed.async_pattern.is_empty() {
        "marlin"
    } else {
        parsed.async_pattern.as_str()
    };
    let rg_pattern = raw
        .strip_prefix("re:")
        .or_else(|| raw.strip_prefix("rg:"))
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(raw);
    let tantivy_expr = if raw.contains("title:") || raw.contains("body:") {
        raw.to_owned()
    } else {
        let safe_phrase: String = rg_pattern
            .chars()
            .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | ' ' | '.'))
            .collect();
        let trimmed = safe_phrase.trim();
        let term = if trimmed.is_empty() {
            "marlin"
        } else {
            trimmed
        };
        format!("title:\"{term}\"^2 OR body:\"{term}\"")
    };
    let mut leaves = vec![
        SearchPlaybookComposition::Leaf(SearchPlaybookLeaf::Rg(vec![
            "-n".into(),
            "-e".into(),
            rg_pattern.into(),
        ])),
        SearchPlaybookComposition::Leaf(SearchPlaybookLeaf::Tantivy(vec![tantivy_expr])),
    ];
    if parsed.path_prefix.is_some() || parsed.extension.is_some() {
        leaves.push(SearchPlaybookComposition::Leaf(
            SearchPlaybookLeaf::Topology(TopologyOwnerMembershipBlock {
                kind: "file".into(),
                exact_path: None,
                path_prefix: parsed.path_prefix.clone(),
                extension: parsed.extension.clone(),
                path_glob: None,
            }),
        ));
    }
    let languages = if parsed.languages.is_empty() && parsed.documents.is_empty() {
        vec!["rust".into()]
    } else {
        parsed.languages.clone()
    };
    let input = SearchPlaybookPlanInput {
        workspace: Some("marlin-tui".into()),
        languages,
        documents: parsed.documents.clone(),
        composition: SearchPlaybookComposition::Intersect(leaves),
    };
    admit_search_playbook_plan(input)
        .map(|_| ())
        .map_err(|err| format!("Playbook rejected: {err}"))
}

pub(super) fn build_static_candidates(command_catalog: &[UiCommand]) -> Vec<HelmCandidate> {
    let mut out = Vec::new();
    let default_buffers = [
        ("conversation", "Active chat transcript buffer"),
        ("messages", "UI diagnostics and system messages"),
        ("help", "Keyboard shortcuts and Meow bindings"),
        ("processes", "Managed Runtime processes"),
        ("models", "Configured model routes and aliases"),
        ("sessions", "Recorded conversation sessions"),
    ];
    for (id, desc) in default_buffers {
        if let Ok(buffer_id) = UiBufferId::new(id) {
            out.push(HelmCandidate {
                id: format!("buf:{id}"),
                source: HelmSourceKind::Buffers,
                display: format!("*{id}*"),
                path: None,
                language: None,
                primary_symbol: None,
                graph_relations: Vec::new(),
                snippet: desc.to_owned(),
                citation: None,
                badge: "buffer".to_owned(),
                context_lines: vec![WorkspaceContextLine {
                    line_number: 1,
                    text: format!("Buffer *{id}* — {desc}"),
                    is_match: true,
                }],
                buffer_id: Some(buffer_id),
                command_name: None,
            });
        }
    }
    for cmd in command_catalog {
        if cmd.name.as_str() == "helm" {
            continue;
        }
        out.push(HelmCandidate {
            id: format!("cmd:{}", cmd.name.as_str()),
            source: HelmSourceKind::Commands,
            display: cmd.name.as_str().to_owned(),
            path: None,
            language: None,
            primary_symbol: None,
            graph_relations: Vec::new(),
            snippet: cmd.description.clone(),
            citation: None,
            badge: "M-x".to_owned(),
            context_lines: vec![
                WorkspaceContextLine {
                    line_number: 1,
                    text: format!("Command: M-x {}", cmd.name.as_str()),
                    is_match: true,
                },
                WorkspaceContextLine {
                    line_number: 2,
                    text: cmd.description.clone(),
                    is_match: false,
                },
            ],
            buffer_id: None,
            command_name: Some(cmd.name.clone()),
        });
    }
    out
}

pub(super) fn hit_to_candidate(hit: WorkspaceSearchHit) -> HelmCandidate {
    let display = match hit.first_line {
        Some(line) if !hit.line_matches.is_empty() || !hit.symbols.is_empty() => {
            format!("{}:{line}", hit.path)
        }
        _ => hit.path.clone(),
    };
    let citation = Some(format!("@{display}"));
    let primary_symbol = hit.symbols.first().map(|s| s.name.clone());
    // Keep regular file/grep/fuzzy/BM25 rows clean without redundant `[lang·rg]`
    // or `[lang·fuzzy-path+bm25]` clutter; only display a concise badge when a
    // non-obvious graph edge relation is active.
    let badge = hit.graph_relations.first().cloned().unwrap_or_default();
    HelmCandidate {
        id: format!("search:{display}"),
        source: HelmSourceKind::Search,
        display,
        path: Some(hit.path),
        language: hit.language,
        primary_symbol,
        graph_relations: hit.graph_relations,
        snippet: hit.first_line_text.trim().to_owned(),
        citation,
        badge,
        context_lines: hit.context_lines,
        buffer_id: None,
        command_name: None,
    }
}
