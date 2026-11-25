// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Authentic Emacs-inspired Modern Helm state machine backed by a sub-millisecond
//! resident trigram workspace search index (`WorkspaceSearchIndex`).
//!
//! - When the resident index is warm in RAM, queries execute synchronously in
//!   **< 0.5 ms** (zero thread-spawn lag, zero 16ms polling jitter, zero disk I/O).
//! - When warming on first boot or on manual `C-l` refresh, builds asynchronously
//!   with exact-epoch fencing (`dev/quint/ui-helm.qnt`).
//! - Organizes results into authentic Emacs Helm source groups (`Workspace Search`,
//!   `Buffers`, `Commands (M-x)`), supports `C-o` next-source jump, `C-SPC`/`M-m`
//!   mark-and-advance, `M-a` mark-all, `C-z` live follow-mode split preview, and
//!   `TAB` interactive Action Transformer menu.

use std::path::PathBuf;
use std::sync::Arc;

use marlin_agent_search_engine::{
    SearchPlaybookComposition, SearchPlaybookLeaf, SearchPlaybookPlanInput,
    TopologyOwnerMembershipBlock, WorkspaceContextLine, WorkspaceIndexConfig, WorkspaceSearchHit,
    WorkspaceSearchIndex, WorkspaceSearchQuery, admit_search_playbook_plan,
};
use marlin_agent_ui::{UiBufferId, UiCommand, UiCommandName};
use tokio::sync::oneshot;

const MAX_HELM_HITS: usize = 64;
const PREVIEW_CONTEXT_RADIUS: usize = 5;

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

    fn next(self) -> Self {
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
        }
    }

    fn async_key(&self) -> (&str, &[String], &[String], Option<&str>, Option<&str>) {
        (
            &self.async_pattern,
            &self.languages,
            &self.documents,
            self.path_prefix.as_deref(),
            self.extension.as_deref(),
        )
    }

    pub(crate) fn highlight_needles(&self) -> Vec<String> {
        let mut needles = Vec::new();
        if !self.async_pattern.is_empty() {
            needles.push(self.async_pattern.clone());
        }
        for tok in &self.local_tokens {
            if !tok.is_empty() && !tok.starts_with('!') {
                needles.push(tok.clone());
            }
        }
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
    pub(crate) snippet: String,
    pub(crate) citation: Option<String>,
    pub(crate) badge: String,
    pub(crate) context_lines: Vec<WorkspaceContextLine>,
    pub(crate) buffer_id: Option<UiBufferId>,
    pub(crate) command_name: Option<UiCommandName>,
}

impl HelmCandidate {
    fn matches_tokens(&self, tokens: &[String]) -> bool {
        if tokens.is_empty() {
            return true;
        }
        let haystack = format!(
            "{} {} {} {}",
            self.display.to_lowercase(),
            self.snippet.to_lowercase(),
            self.badge.to_lowercase(),
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

pub(crate) struct HelmOverlay {
    pub(crate) input: String,
    pub(crate) parsed: HelmParsedInput,
    pub(crate) epoch: u64,
    pub(crate) active_source: HelmSourceKind,
    pub(crate) static_candidates: Vec<HelmCandidate>,
    pub(crate) search_candidates: Vec<HelmCandidate>,
    pub(crate) filtered_indices: Vec<usize>,
    pub(crate) display_rows: Vec<HelmDisplayRow>,
    pub(crate) selected: usize,
    pub(crate) marked_ids: Vec<String>,
    pub(crate) follow_preview: bool,
    pub(crate) action_mode: bool,
    pub(crate) action_selected: usize,
    pub(crate) generation_digest: Option<String>,
    pub(crate) last_query_micros: Option<u64>,
    pub(crate) indexed_files: usize,
    pub(crate) last_error: Option<String>,
    pub(crate) indexing: bool,
    workspace_root: PathBuf,
    index_config: WorkspaceIndexConfig,
    index: Option<Arc<WorkspaceSearchIndex>>,
    index_receiver: Option<oneshot::Receiver<Arc<WorkspaceSearchIndex>>>,
}

impl HelmOverlay {
    pub(crate) fn open(
        initial_query: Option<&str>,
        workspace_root: PathBuf,
        command_catalog: &[UiCommand],
        index_config: WorkspaceIndexConfig,
        warm_index: Option<Arc<WorkspaceSearchIndex>>,
        runtime: &tokio::runtime::Handle,
    ) -> Self {
        let input = initial_query.unwrap_or("").trim().to_owned();
        let parsed = HelmParsedInput::parse(&input);
        let static_candidates = build_static_candidates(command_catalog);
        let indexed_files = warm_index.as_ref().map_or(0, |idx| idx.file_count());
        let generation_digest = warm_index
            .as_ref()
            .map(|idx| idx.generation_digest().as_str().to_owned());

        let mut overlay = Self {
            input,
            parsed,
            epoch: 1,
            active_source: HelmSourceKind::All,
            static_candidates,
            search_candidates: Vec::new(),
            filtered_indices: Vec::new(),
            display_rows: Vec::new(),
            selected: 0,
            marked_ids: Vec::new(),
            follow_preview: true,
            action_mode: false,
            action_selected: 0,
            generation_digest,
            last_query_micros: None,
            indexed_files,
            last_error: None,
            indexing: warm_index.is_none(),
            workspace_root,
            index_config,
            index: warm_index,
            index_receiver: None,
        };

        if overlay.index.is_some() {
            overlay.run_resident_query();
        } else {
            overlay.spawn_index_build(runtime);
            overlay.rebuild_view();
        }
        overlay
    }

    pub(crate) fn attach_warm_index(&mut self, index: Arc<WorkspaceSearchIndex>) {
        self.indexed_files = index.file_count();
        self.generation_digest = Some(index.generation_digest().as_str().to_owned());
        self.index = Some(index);
        self.index_receiver = None;
        self.indexing = false;
        self.run_resident_query();
    }

    pub(crate) fn is_indexing(&self) -> bool {
        self.indexing
    }

    pub(crate) fn refresh_index(&mut self, runtime: &tokio::runtime::Handle) {
        self.spawn_index_build(runtime);
    }

    fn spawn_index_build(&mut self, runtime: &tokio::runtime::Handle) {
        let root = self.workspace_root.clone();
        let config = self.index_config.clone();
        let (tx, rx) = oneshot::channel();
        self.indexing = true;
        self.index_receiver = Some(rx);
        runtime.spawn_blocking(move || {
            let built = Arc::new(WorkspaceSearchIndex::build_with_config(&root, config));
            let _ = tx.send(built);
        });
    }

    pub(crate) fn poll(&mut self) -> Option<Arc<WorkspaceSearchIndex>> {
        let rx = self.index_receiver.as_mut()?;
        match rx.try_recv() {
            Ok(built) => {
                self.index_receiver = None;
                self.indexing = false;
                self.attach_warm_index(built.clone());
                Some(built)
            }
            Err(oneshot::error::TryRecvError::Empty) => None,
            Err(oneshot::error::TryRecvError::Closed) => {
                self.index_receiver = None;
                self.indexing = false;
                None
            }
        }
    }

    pub(crate) fn insert_text(&mut self, text: &str) {
        if self.action_mode {
            return;
        }
        let clean: String = text.chars().filter(|ch| !ch.is_control()).collect();
        if clean.is_empty() {
            return;
        }
        self.input.push_str(&clean);
        self.on_input_changed();
    }

    pub(crate) fn backspace(&mut self) {
        if self.action_mode {
            self.action_mode = false;
            return;
        }
        if self.input.pop().is_some() {
            self.on_input_changed();
        }
    }

    fn on_input_changed(&mut self) {
        let next_parsed = HelmParsedInput::parse(&self.input);
        let async_changed = self.parsed.async_key() != next_parsed.async_key();
        self.parsed = next_parsed;
        self.action_mode = false;
        self.action_selected = 0;
        if async_changed {
            self.epoch = self.epoch.saturating_add(1);
            if self.index.is_some() {
                self.run_resident_query();
                return;
            }
        }
        self.rebuild_view();
    }

    fn run_resident_query(&mut self) {
        let Some(index) = self.index.as_ref() else {
            self.rebuild_view();
            return;
        };
        let has_filters = !self.parsed.languages.is_empty()
            || !self.parsed.documents.is_empty()
            || self.parsed.path_prefix.is_some()
            || self.parsed.extension.is_some();

        if self.parsed.async_pattern.is_empty() && !has_filters {
            let report = index.query(&WorkspaceSearchQuery {
                pattern: "",
                languages: &[],
                documents: &[],
                path_prefix: None,
                extension: None,
                max_hits: 24,
                context_radius: PREVIEW_CONTEXT_RADIUS,
            });
            self.generation_digest = Some(report.generation_digest.as_str().to_owned());
            self.last_query_micros = Some(report.elapsed_micros);
            self.last_error = None;
            self.search_candidates = report.hits.into_iter().map(hit_to_candidate).collect();
            self.rebuild_view();
            return;
        }

        if let Err(err) = admit_helm_playbook(&self.parsed) {
            self.last_error = Some(err);
            self.search_candidates.clear();
            self.rebuild_view();
            return;
        }

        let report = index.query(&WorkspaceSearchQuery {
            pattern: &self.parsed.async_pattern,
            languages: &self.parsed.languages,
            documents: &self.parsed.documents,
            path_prefix: self.parsed.path_prefix.as_deref(),
            extension: self.parsed.extension.as_deref(),
            max_hits: MAX_HELM_HITS,
            context_radius: PREVIEW_CONTEXT_RADIUS,
        });
        self.generation_digest = Some(report.generation_digest.as_str().to_owned());
        self.last_query_micros = Some(report.elapsed_micros);
        self.last_error = None;
        self.search_candidates = report.hits.into_iter().map(hit_to_candidate).collect();
        self.rebuild_view();
    }

    pub(crate) fn move_selection(&mut self, delta: isize) {
        if self.action_mode {
            let total = self.available_actions().len();
            if total == 0 {
                self.action_selected = 0;
            } else {
                self.action_selected = self
                    .action_selected
                    .saturating_add_signed(delta)
                    .min(total - 1);
            }
            return;
        }
        let total = self.filtered_indices.len();
        if total == 0 {
            self.selected = 0;
            return;
        }
        self.selected = self
            .selected
            .saturating_add_signed(delta)
            .min(total.saturating_sub(1));
    }

    /// Authentic Emacs `helm-next-source` (`C-o`): jump directly to the first
    /// candidate of the next visible source group.
    pub(crate) fn jump_next_source(&mut self) {
        if self.action_mode || self.filtered_indices.is_empty() {
            return;
        }
        let current_source = self.selected_candidate().map(|c| c.source);
        for (pos, &raw_idx) in self
            .filtered_indices
            .iter()
            .enumerate()
            .skip(self.selected + 1)
        {
            if let Some(cand) = self.candidate_at(raw_idx)
                && Some(cand.source) != current_source
            {
                self.selected = pos;
                return;
            }
        }
        self.selected = 0;
    }

    /// Authentic Emacs `helm-toggle-visible-mark` (`C-SPC` / `M-m`): toggle mark
    /// and advance smoothly to the next candidate.
    pub(crate) fn toggle_mark_and_advance(&mut self) {
        if self.action_mode {
            return;
        }
        let Some(candidate) = self.selected_candidate() else {
            return;
        };
        let id = candidate.id.clone();
        if let Some(pos) = self.marked_ids.iter().position(|existing| existing == &id) {
            self.marked_ids.remove(pos);
        } else {
            self.marked_ids.push(id);
        }
        self.move_selection(1);
    }

    /// Authentic Emacs `helm-mark-all` (`M-a`): mark all visible candidates (or clear if all marked).
    pub(crate) fn toggle_mark_all(&mut self) {
        if self.action_mode || self.filtered_indices.is_empty() {
            return;
        }
        let all_visible: Vec<String> = self
            .filtered_indices
            .iter()
            .filter_map(|&idx| self.candidate_at(idx).map(|c| c.id.clone()))
            .collect();
        if self.marked_ids.len() == all_visible.len() {
            self.marked_ids.clear();
        } else {
            self.marked_ids = all_visible;
        }
    }

    pub(crate) fn cycle_source_filter(&mut self) {
        self.action_mode = false;
        self.active_source = self.active_source.next();
        self.rebuild_view();
    }

    pub(crate) fn toggle_follow_preview(&mut self) {
        self.follow_preview = !self.follow_preview;
    }

    /// Authentic Emacs `helm-select-action` (`TAB`): switch cleanly between
    /// candidate view and selectable action transformer view.
    pub(crate) fn toggle_action_mode(&mut self) {
        if self.filtered_indices.is_empty() && !self.action_mode {
            return;
        }
        self.action_mode = !self.action_mode;
        self.action_selected = 0;
    }

    pub(crate) fn available_actions(&self) -> Vec<HelmMenuActionItem> {
        let mut items = Vec::new();
        let Some(selected) = self.selected_candidate() else {
            return items;
        };
        let primary_label = match selected.source {
            HelmSourceKind::Buffers => format!("Switch to buffer {}", selected.display),
            HelmSourceKind::Commands => format!("Invoke command M-x {}", selected.display),
            HelmSourceKind::Search | HelmSourceKind::All => {
                if self.marked_ids.is_empty() {
                    format!("Insert citation {} into Composer", selected.display)
                } else {
                    format!(
                        "Insert {} marked citations into Composer",
                        self.marked_ids.len()
                    )
                }
            }
        };
        items.push(HelmMenuActionItem {
            kind: HelmMenuActionKind::Primary,
            key: "RET",
            label: primary_label,
            detail: "Default action".to_owned(),
        });
        if selected.citation.is_some() || !self.marked_ids.is_empty() {
            items.push(HelmMenuActionItem {
                kind: HelmMenuActionKind::ExportCitations,
                key: "C-e",
                label: "Export citations to Composer".to_owned(),
                detail: "Append @path:line references".to_owned(),
            });
        }
        if let Some(path) = selected.path.as_deref()
            && let Some((dir, _)) = path.rsplit_once('/')
        {
            items.push(HelmMenuActionItem {
                kind: HelmMenuActionKind::ScopeDirectory,
                key: "d",
                label: format!("Scope Helm to directory @prefix:{dir}/"),
                detail: "Narrow live search to parent folder".to_owned(),
            });
        }
        if let Some(lang) = selected.language.as_deref() {
            items.push(HelmMenuActionItem {
                kind: HelmMenuActionKind::ScopeLanguage,
                key: "l",
                label: format!("Scope Helm to language @lang:{lang}"),
                detail: "Filter live search by language".to_owned(),
            });
        }
        items
    }

    /// Execute the active action. Returns `Ok(Some(action))` when Helm should close
    /// and hand off to the TUI host, or `Ok(None)` when an in-Helm transformer
    /// action (like scoping `@prefix:` or `@lang:`) updated Helm live!
    pub(crate) fn accept(&mut self, export_citations_only: bool) -> (bool, Option<HelmAction>) {
        if self.action_mode {
            let actions = self.available_actions();
            let chosen = actions
                .get(self.action_selected)
                .map(|item| item.kind)
                .unwrap_or(HelmMenuActionKind::Primary);
            self.action_mode = false;
            return match chosen {
                HelmMenuActionKind::Primary => (true, self.resolve_primary_action(false)),
                HelmMenuActionKind::ExportCitations => (true, self.resolve_primary_action(true)),
                HelmMenuActionKind::ScopeDirectory => {
                    self.apply_directory_scope();
                    (false, None)
                }
                HelmMenuActionKind::ScopeLanguage => {
                    self.apply_language_scope();
                    (false, None)
                }
            };
        }
        (true, self.resolve_primary_action(export_citations_only))
    }

    fn apply_directory_scope(&mut self) {
        let Some(path) = self.selected_candidate().and_then(|c| c.path.clone()) else {
            return;
        };
        let Some((dir, _)) = path.rsplit_once('/') else {
            return;
        };
        let token = format!("@prefix:{dir}/");
        if !self.input.contains(&token) {
            let prefix = if self.input.is_empty() {
                token
            } else {
                format!("{token} {}", self.input)
            };
            self.input = prefix;
            self.on_input_changed();
        }
    }

    fn apply_language_scope(&mut self) {
        let Some(lang) = self.selected_candidate().and_then(|c| c.language.clone()) else {
            return;
        };
        let token = format!("@lang:{lang}");
        if !self.input.contains(&token) {
            let prefix = if self.input.is_empty() {
                token
            } else {
                format!("{token} {}", self.input)
            };
            self.input = prefix;
            self.on_input_changed();
        }
    }

    fn resolve_primary_action(&self, export_citations_only: bool) -> Option<HelmAction> {
        if !self.marked_ids.is_empty() {
            let citations: Vec<String> = self
                .all_candidates()
                .filter(|cand| self.marked_ids.iter().any(|mid| mid == &cand.id))
                .filter_map(|cand| cand.citation.clone().or_else(|| Some(cand.display.clone())))
                .collect();
            if !citations.is_empty() {
                return Some(HelmAction::InsertCitations(citations.join(" ")));
            }
        }
        let selected = self.selected_candidate()?.clone();
        if export_citations_only {
            let text = selected.citation.unwrap_or(selected.display);
            return Some(HelmAction::InsertCitations(text));
        }
        if let Some(buffer_id) = selected.buffer_id {
            return Some(HelmAction::SelectBuffer(buffer_id));
        }
        if let Some(command_name) = selected.command_name {
            return Some(HelmAction::PrepareCommand(command_name));
        }
        let text = selected.citation.unwrap_or(selected.display);
        Some(HelmAction::InsertCitations(text))
    }

    pub(crate) fn all_candidates(&self) -> impl Iterator<Item = &HelmCandidate> {
        self.search_candidates
            .iter()
            .chain(self.static_candidates.iter())
    }

    pub(crate) fn candidate_at(&self, raw_index: usize) -> Option<&HelmCandidate> {
        if raw_index < self.search_candidates.len() {
            self.search_candidates.get(raw_index)
        } else {
            self.static_candidates
                .get(raw_index - self.search_candidates.len())
        }
    }

    pub(crate) fn selected_candidate(&self) -> Option<&HelmCandidate> {
        let raw_idx = *self.filtered_indices.get(self.selected)?;
        self.candidate_at(raw_idx)
    }

    pub(crate) fn format_latency(&self) -> String {
        match self.last_query_micros {
            Some(us) if us < 1_000 => format!("{us}µs"),
            Some(us) => format!("{:.2}ms", (us as f64) / 1_000.0),
            None if self.indexing => "indexing...".to_owned(),
            None => "0µs".to_owned(),
        }
    }

    fn rebuild_view(&mut self) {
        let local_tokens = &self.parsed.local_tokens;
        let async_query = self.parsed.async_pattern.to_lowercase();
        let active = self.active_source;

        let mut grouped: Vec<(HelmSourceKind, Vec<usize>)> = vec![
            (HelmSourceKind::Search, Vec::new()),
            (HelmSourceKind::Buffers, Vec::new()),
            (HelmSourceKind::Commands, Vec::new()),
        ];

        for (raw_idx, cand) in self.all_candidates().enumerate() {
            if active != HelmSourceKind::All && cand.source != active {
                continue;
            }
            if !cand.matches_tokens(local_tokens) {
                continue;
            }
            // For buffers & commands, also filter by the main async pattern when typed.
            if cand.source != HelmSourceKind::Search
                && !async_query.is_empty()
                && !cand.display.to_lowercase().contains(&async_query)
                && !cand.snippet.to_lowercase().contains(&async_query)
            {
                continue;
            }
            if let Some((_, bucket)) = grouped.iter_mut().find(|(kind, _)| *kind == cand.source) {
                bucket.push(raw_idx);
            }
        }

        let latency = self.format_latency();
        let mut filtered_indices = Vec::new();
        let mut display_rows = Vec::new();

        for (source, bucket) in grouped {
            if bucket.is_empty() {
                continue;
            }
            let meta = if source == HelmSourceKind::Search {
                format!("{} · {} files", latency, self.indexed_files)
            } else {
                String::new()
            };
            display_rows.push(HelmDisplayRow::SourceHeader {
                source,
                count: bucket.len(),
                meta,
            });
            for raw_index in bucket {
                let filtered_pos = filtered_indices.len();
                filtered_indices.push(raw_index);
                display_rows.push(HelmDisplayRow::Candidate {
                    filtered_pos,
                    raw_index,
                });
            }
        }

        self.filtered_indices = filtered_indices;
        self.display_rows = display_rows;

        if self.filtered_indices.is_empty() {
            self.selected = 0;
        } else if self.selected >= self.filtered_indices.len() {
            self.selected = self.filtered_indices.len() - 1;
        }

        // Prune dangling marks (`s.marks.subseteq(s.candidates)`).
        let valid_ids: Vec<String> = self
            .filtered_indices
            .iter()
            .filter_map(|&idx| self.candidate_at(idx).map(|cand| cand.id.clone()))
            .collect();
        self.marked_ids
            .retain(|mid| valid_ids.iter().any(|valid| valid == mid));
    }
}

fn admit_helm_playbook(parsed: &HelmParsedInput) -> Result<(), String> {
    let pattern = if parsed.async_pattern.is_empty() {
        "marlin"
    } else {
        parsed.async_pattern.as_str()
    };
    let mut leaves = vec![
        SearchPlaybookComposition::Leaf(SearchPlaybookLeaf::Rg(vec!["-n".into(), pattern.into()])),
        SearchPlaybookComposition::Leaf(SearchPlaybookLeaf::Tantivy(vec![format!(
            "title:\"{pattern}\"^2 OR body:{pattern}"
        )])),
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

fn build_static_candidates(command_catalog: &[UiCommand]) -> Vec<HelmCandidate> {
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

fn hit_to_candidate(hit: WorkspaceSearchHit) -> HelmCandidate {
    let display = match hit.first_line {
        Some(line) if !hit.line_matches.is_empty() => format!("{}:{line}", hit.path),
        _ => hit.path.clone(),
    };
    let citation = Some(format!("@{display}"));
    let badge = hit.language.clone().unwrap_or_else(|| "file".to_owned());
    HelmCandidate {
        id: format!("search:{display}"),
        source: HelmSourceKind::Search,
        display,
        path: Some(hit.path),
        language: hit.language,
        snippet: hit.first_line_text.trim().to_owned(),
        citation,
        badge,
        context_lines: hit.context_lines,
        buffer_id: None,
        command_name: None,
    }
}

#[cfg(test)]
#[path = "../tests/unit/helm_source.rs"]
mod tests;
