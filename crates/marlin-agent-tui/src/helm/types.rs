// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Core Helm value types, two-stage Consult/Orderless parser, playbook admission,
//! and candidate projection helpers.

use std::sync::{Arc, RwLock};

use marlin_agent_search_engine::{
    SearchPlaybookComposition, SearchPlaybookLeaf, SearchPlaybookPlanInput,
    TopologyOwnerMembershipBlock, WorkspaceContextLine, WorkspaceSearchHit, WorkspaceSearchIndex,
    admit_search_playbook_plan,
};
use marlin_agent_ui::{UiBufferId, UiCommand, UiCommandName};

pub(crate) type SharedSearchIndex = Arc<RwLock<WorkspaceSearchIndex>>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum HelmSourceKind {
    All,
    Search,
    Buffers,
    Commands,
}

impl HelmSourceKind {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Search => "search",
            Self::Buffers => "buffers",
            Self::Commands => "commands",
        }
    }

    pub(crate) fn section_title(self) -> &'static str {
        match self {
            Self::All => "All Sources",
            Self::Search => "Workspace Search",
            Self::Buffers => "Buffers",
            Self::Commands => "Commands (M-x)",
        }
    }

    pub(super) fn next(self) -> Self {
        match self {
            Self::All => Self::Search,
            Self::Search => Self::Buffers,
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
            .split_whitespace()
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
    let lang = hit.language.as_deref().unwrap_or("file");
    let axes_tag: String = hit
        .matched_axes
        .iter()
        .filter(|a| *a != "topology" && *a != "index")
        .take(2)
        .cloned()
        .collect::<Vec<_>>()
        .join("+");
    let primary_symbol = hit.symbols.first().map(|s| s.name.clone());
    let rel_suffix = hit
        .graph_relations
        .first()
        .map(|rel| format!("·{rel}"))
        .or_else(|| {
            hit.symbols
                .first()
                .filter(|_| hit.matched_axes.iter().any(|a| a == "symbol"))
                .map(|sym| format!("·{}", sym.kind.as_str()))
        })
        .unwrap_or_default();
    let badge = if axes_tag.is_empty() {
        format!("{lang}{rel_suffix}")
    } else {
        format!("{lang}·{axes_tag}{rel_suffix}")
    };
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
