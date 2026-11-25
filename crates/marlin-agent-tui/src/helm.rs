// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Interactive Modern Helm overlay powered by in-process ASP search playbooks.
//!
//! Mirrors `scheme/ui-helm/funcs.ss` and `dev/quint/ui-helm.qnt` 1:1:
//! - Two-stage Consult/Orderless prompt (`@lang:`, `@doc:`, `@prefix:`, `@ext:`, `#local !neg`)
//! - Strict exact-epoch fencing (`reply.epoch == self.epoch`) so stale async hits never overwrite
//! - Multi-source candidate merging (`search`, `buffers`, `commands`)
//! - Bounded O(viewport_height) virtual viewport, split Preview pane, multi-mark (`TAB`),
//!   and decoupled Embark actions (`RET`, `C-e`, `C-o`, `C-s`, `C-a`).

use std::path::PathBuf;

use marlin_agent_search_engine::{
    ConvergedSearchHit, SearchPlaybookComposition, SearchPlaybookLeaf, SearchPlaybookPlanInput,
    TopologyOwnerMembershipBlock, admit_search_playbook_plan, execute_admitted_playbook,
};
use marlin_agent_ui::{UiBufferId, UiCommand, UiCommandName, UiRgb, UiTheme};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};
use tokio::sync::oneshot;

const MAX_HELM_HITS: usize = 64;
const MAX_PREVIEW_LINES: usize = 6;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum HelmSourceKind {
    All,
    Search,
    Buffers,
    Commands,
}

impl HelmSourceKind {
    fn label(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Search => "search",
            Self::Buffers => "buffers",
            Self::Commands => "commands",
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
}

#[derive(Clone, Debug)]
pub(crate) struct HelmCandidate {
    pub(crate) id: String,
    pub(crate) source: HelmSourceKind,
    pub(crate) display: String,
    pub(crate) citation: Option<String>,
    pub(crate) annotation: String,
    pub(crate) preview_lines: Vec<String>,
    pub(crate) buffer_id: Option<UiBufferId>,
    pub(crate) command_name: Option<UiCommandName>,
}

impl HelmCandidate {
    fn matches_tokens(&self, tokens: &[String]) -> bool {
        if tokens.is_empty() {
            return true;
        }
        let haystack = format!(
            "{} {} {}",
            self.display.to_lowercase(),
            self.annotation.to_lowercase(),
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

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum HelmAction {
    InsertCitations(String),
    SelectBuffer(UiBufferId),
    PrepareCommand(UiCommandName),
}

struct HelmSearchReply {
    epoch: u64,
    generation_digest: Option<String>,
    candidates: Vec<HelmCandidate>,
    error: Option<String>,
}

pub(crate) struct HelmOverlay {
    input: String,
    parsed: HelmParsedInput,
    epoch: u64,
    active_source: HelmSourceKind,
    static_candidates: Vec<HelmCandidate>,
    search_candidates: Vec<HelmCandidate>,
    filtered_indices: Vec<usize>,
    selected: usize,
    viewport_offset: usize,
    marked_ids: Vec<String>,
    split_preview: bool,
    action_menu_open: bool,
    generation_digest: Option<String>,
    last_error: Option<String>,
    searching: bool,
    workspace_root: PathBuf,
    inflight: Option<oneshot::Receiver<HelmSearchReply>>,
}

impl HelmOverlay {
    pub(crate) fn open(
        initial_query: Option<&str>,
        workspace_root: PathBuf,
        command_catalog: &[UiCommand],
        runtime: &tokio::runtime::Handle,
    ) -> Self {
        let input = initial_query.unwrap_or("").trim().to_owned();
        let parsed = HelmParsedInput::parse(&input);
        let static_candidates = build_static_candidates(command_catalog);
        let mut overlay = Self {
            input,
            parsed,
            epoch: 1,
            active_source: HelmSourceKind::All,
            static_candidates,
            search_candidates: Vec::new(),
            filtered_indices: Vec::new(),
            selected: 0,
            viewport_offset: 0,
            marked_ids: Vec::new(),
            split_preview: true,
            action_menu_open: false,
            generation_digest: None,
            last_error: None,
            searching: false,
            workspace_root,
            inflight: None,
        };
        overlay.rebuild_view();
        if !overlay.parsed.async_pattern.is_empty() {
            overlay.spawn_search(runtime);
        }
        overlay
    }

    pub(crate) fn is_searching(&self) -> bool {
        self.searching
    }

    pub(crate) fn insert_text(&mut self, text: &str, runtime: &tokio::runtime::Handle) {
        let clean: String = text.chars().filter(|ch| !ch.is_control()).collect();
        if clean.is_empty() {
            return;
        }
        self.input.push_str(&clean);
        self.on_input_changed(runtime);
    }

    pub(crate) fn backspace(&mut self, runtime: &tokio::runtime::Handle) {
        if self.input.pop().is_some() {
            self.on_input_changed(runtime);
        }
    }

    fn on_input_changed(&mut self, runtime: &tokio::runtime::Handle) {
        let next_parsed = HelmParsedInput::parse(&self.input);
        let async_changed = self.parsed.async_key() != next_parsed.async_key();
        self.parsed = next_parsed;
        self.action_menu_open = false;
        if async_changed {
            self.epoch = self.epoch.saturating_add(1);
            if self.parsed.async_pattern.is_empty() {
                self.inflight = None;
                self.searching = false;
                self.search_candidates.clear();
                self.generation_digest = None;
                self.last_error = None;
            } else {
                self.spawn_search(runtime);
            }
        }
        self.rebuild_view();
    }

    fn spawn_search(&mut self, runtime: &tokio::runtime::Handle) {
        let epoch = self.epoch;
        let parsed = self.parsed.clone();
        let workspace_root = self.workspace_root.clone();
        let (tx, rx) = oneshot::channel();
        self.searching = true;
        self.last_error = None;
        self.inflight = Some(rx);
        runtime.spawn_blocking(move || {
            let reply = execute_helm_search(epoch, &workspace_root, &parsed);
            let _ = tx.send(reply);
        });
    }

    pub(crate) fn poll(&mut self) -> bool {
        let Some(rx) = self.inflight.as_mut() else {
            return false;
        };
        match rx.try_recv() {
            Ok(reply) => {
                self.inflight = None;
                self.searching = false;
                // Fail-closed exact-epoch fence matching `dev/quint/ui-helm.qnt`.
                if reply.epoch != self.epoch {
                    return false;
                }
                self.generation_digest = reply.generation_digest;
                self.last_error = reply.error;
                self.search_candidates = reply.candidates;
                self.rebuild_view();
                true
            }
            Err(oneshot::error::TryRecvError::Empty) => false,
            Err(oneshot::error::TryRecvError::Closed) => {
                self.inflight = None;
                self.searching = false;
                true
            }
        }
    }

    pub(crate) fn move_selection(&mut self, delta: isize) {
        let total = self.filtered_indices.len();
        if total == 0 {
            self.selected = 0;
            self.viewport_offset = 0;
            return;
        }
        let next = self
            .selected
            .saturating_add_signed(delta)
            .min(total.saturating_sub(1));
        self.selected = next;
    }

    pub(crate) fn toggle_mark(&mut self) {
        let Some(candidate) = self.selected_candidate() else {
            return;
        };
        let id = candidate.id.clone();
        if let Some(pos) = self.marked_ids.iter().position(|existing| existing == &id) {
            self.marked_ids.remove(pos);
        } else {
            self.marked_ids.push(id);
        }
    }

    pub(crate) fn next_source(&mut self) {
        self.active_source = self.active_source.next();
        self.rebuild_view();
    }

    pub(crate) fn toggle_preview(&mut self) {
        self.split_preview = !self.split_preview;
    }

    pub(crate) fn toggle_actions(&mut self) {
        self.action_menu_open = !self.action_menu_open;
    }

    pub(crate) fn accept(&mut self, export_citations_only: bool) -> Option<HelmAction> {
        self.action_menu_open = false;
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

    fn all_candidates(&self) -> impl Iterator<Item = &HelmCandidate> {
        self.search_candidates
            .iter()
            .chain(self.static_candidates.iter())
    }

    fn candidate_at(&self, raw_index: usize) -> Option<&HelmCandidate> {
        if raw_index < self.search_candidates.len() {
            self.search_candidates.get(raw_index)
        } else {
            self.static_candidates
                .get(raw_index - self.search_candidates.len())
        }
    }

    fn selected_candidate(&self) -> Option<&HelmCandidate> {
        let raw_idx = *self.filtered_indices.get(self.selected)?;
        self.candidate_at(raw_idx)
    }

    fn rebuild_view(&mut self) {
        let local_tokens = &self.parsed.local_tokens;
        let active = self.active_source;
        self.filtered_indices = self
            .all_candidates()
            .enumerate()
            .filter(|(_, cand)| {
                (active == HelmSourceKind::All || cand.source == active)
                    && cand.matches_tokens(local_tokens)
            })
            .map(|(idx, _)| idx)
            .collect();

        if self.filtered_indices.is_empty() {
            self.selected = 0;
            self.viewport_offset = 0;
        } else if self.selected >= self.filtered_indices.len() {
            self.selected = self.filtered_indices.len() - 1;
        }

        // Prune dangling marks (invariant `s.marks.subseteq(s.candidates)`).
        let valid_ids: Vec<String> = self
            .filtered_indices
            .iter()
            .filter_map(|idx| self.candidate_at(*idx).map(|cand| cand.id.clone()))
            .collect();
        self.marked_ids
            .retain(|mid| valid_ids.iter().any(|valid| valid == mid));
    }

    pub(crate) fn render(&self, frame: &mut Frame<'_>, area: Rect, theme: &UiTheme) {
        if area.width < 20 || area.height < 8 {
            return;
        }
        let margin_x = area.width / 12;
        let margin_y = area.height / 10;
        let overlay_area = Rect {
            x: area.x + margin_x,
            y: area.y + margin_y,
            width: area.width.saturating_sub(margin_x * 2).max(20),
            height: area.height.saturating_sub(margin_y * 2).max(8),
        };
        frame.render_widget(Clear, overlay_area);

        let status = if self.searching {
            "searching..."
        } else if self.last_error.is_some() {
            "error"
        } else {
            "ready"
        };
        let gen_short = self
            .generation_digest
            .as_deref()
            .map(|digest| &digest[..digest.len().min(18)])
            .unwrap_or("local");
        let title = format!(
            " HELM [{}] epoch={} hits={} marked={} gen={} [{}] ",
            self.active_source.label(),
            self.epoch,
            self.filtered_indices.len(),
            self.marked_ids.len(),
            gen_short,
            status
        );
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(rgb(theme.accent)))
            .style(Style::default().bg(rgb(theme.canvas)).fg(rgb(theme.ink)))
            .title(Span::styled(
                title,
                Style::default()
                    .fg(rgb(theme.accent))
                    .add_modifier(Modifier::BOLD),
            ));
        let inner = block.inner(overlay_area);
        frame.render_widget(block, overlay_area);

        if inner.height < 4 {
            return;
        }
        let footer_height = if self.action_menu_open { 2 } else { 1 };
        let sections = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(2),
                Constraint::Length(footer_height),
            ])
            .split(inner);

        let prompt_line = Line::from(vec![
            Span::styled(
                "Helm> ",
                Style::default()
                    .fg(rgb(theme.info))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(self.input.as_str(), Style::default().fg(rgb(theme.ink))),
            Span::styled("▏", Style::default().fg(rgb(theme.accent))),
        ]);
        frame.render_widget(Paragraph::new(prompt_line), sections[0]);

        let selected_preview = self
            .selected_candidate()
            .filter(|cand| !cand.preview_lines.is_empty())
            .map(|cand| (cand.display.as_str(), cand.preview_lines.as_slice()));
        let show_preview =
            self.split_preview && selected_preview.is_some() && sections[1].height >= 6;
        let (list_area, preview_area) = if show_preview {
            let split = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
                .split(sections[1]);
            (split[0], Some(split[1]))
        } else {
            (sections[1], None)
        };

        let visible_rows = usize::from(list_area.height).max(1);
        let offset = compute_viewport_offset(
            self.selected,
            self.viewport_offset,
            visible_rows,
            self.filtered_indices.len(),
        );
        let mut list_lines = Vec::with_capacity(visible_rows);
        if self.filtered_indices.is_empty() {
            let msg = self
                .last_error
                .as_deref()
                .unwrap_or("  (no matches — type a query or @lang:rust pattern #local !neg)");
            list_lines.push(Line::from(Span::styled(
                msg,
                Style::default().fg(rgb(theme.muted)),
            )));
        } else {
            for (rel_idx, raw_idx) in self
                .filtered_indices
                .iter()
                .enumerate()
                .skip(offset)
                .take(visible_rows)
            {
                let Some(cand) = self.candidate_at(*raw_idx) else {
                    continue;
                };
                let is_selected = rel_idx == self.selected;
                let is_marked = self.marked_ids.iter().any(|mid| mid == &cand.id);
                let cursor = if is_selected { "›" } else { " " };
                let mark = if is_marked { "*" } else { " " };
                let row_style = if is_selected {
                    Style::default().bg(rgb(theme.bar))
                } else {
                    Style::default()
                };
                let mut spans = vec![
                    Span::styled(
                        format!("{cursor}{mark} "),
                        row_style.fg(rgb(if is_selected {
                            theme.accent
                        } else {
                            theme.muted
                        })),
                    ),
                    Span::styled(
                        cand.display.as_str(),
                        row_style.fg(rgb(if is_selected { theme.info } else { theme.ink })),
                    ),
                ];
                if !cand.annotation.is_empty() {
                    spans.push(Span::styled(
                        format!("  [{}]", cand.annotation),
                        row_style.fg(rgb(theme.muted)),
                    ));
                }
                list_lines.push(Line::from(spans));
            }
        }
        frame.render_widget(Paragraph::new(list_lines), list_area);

        if let (Some(p_area), Some((label, preview_lines))) = (preview_area, selected_preview) {
            let mut lines = Vec::with_capacity(preview_lines.len() + 1);
            lines.push(Line::from(Span::styled(
                format!("─── Preview: {label} ───"),
                Style::default().fg(rgb(theme.info)),
            )));
            for line in preview_lines {
                lines.push(Line::from(Span::styled(
                    line.as_str(),
                    Style::default().fg(rgb(theme.ink)),
                )));
            }
            frame.render_widget(Paragraph::new(lines), p_area);
        }

        let mut footer_lines = Vec::new();
        if self.action_menu_open {
            footer_lines.push(Line::from(Span::styled(
                "Actions: [RET] Open/Cite  [C-e] Export Citations  [TAB] Mark  [C-o] Preview  [C-s] Source",
                Style::default().fg(rgb(theme.warning)),
            )));
        }
        footer_lines.push(Line::from(Span::styled(
            "RET Open/Cite · TAB Mark · C-o Preview · C-s Source · C-a Actions · #filter !neg · ESC Close",
            Style::default().fg(rgb(theme.muted)),
        )));
        frame.render_widget(Paragraph::new(footer_lines), sections[2]);
    }
}

fn compute_viewport_offset(selected: usize, current: usize, height: usize, total: usize) -> usize {
    if total == 0 || height == 0 {
        return 0;
    }
    if selected < current {
        selected
    } else if selected >= current + height {
        selected + 1 - height
    } else {
        current.min(total.saturating_sub(height))
    }
}

fn build_static_candidates(command_catalog: &[UiCommand]) -> Vec<HelmCandidate> {
    let mut out = Vec::new();
    let default_buffers = [
        ("conversation", "*Conversation* active chat transcript"),
        ("messages", "*Messages* UI diagnostics"),
        ("help", "*Help* keyboard shortcuts"),
        ("processes", "*Processes* managed Runtime processes"),
        ("models", "*Models* configured model routes"),
        ("sessions", "*Sessions* recorded conversations"),
    ];
    for (id, desc) in default_buffers {
        if let Ok(buffer_id) = UiBufferId::new(id) {
            out.push(HelmCandidate {
                id: format!("buf:{id}"),
                source: HelmSourceKind::Buffers,
                display: format!("*{id}*"),
                citation: None,
                annotation: format!("buffer · {desc}"),
                preview_lines: vec![desc.to_owned()],
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
            citation: None,
            annotation: format!("M-x · {}", cmd.description),
            preview_lines: vec![
                format!("Command: M-x {}", cmd.name.as_str()),
                cmd.description.clone(),
            ],
            buffer_id: None,
            command_name: Some(cmd.name.clone()),
        });
    }
    out
}

fn execute_helm_search(
    epoch: u64,
    workspace_root: &std::path::Path,
    parsed: &HelmParsedInput,
) -> HelmSearchReply {
    let pattern = parsed.async_pattern.as_str();
    let rg_leaf =
        SearchPlaybookComposition::Leaf(SearchPlaybookLeaf::Rg(vec!["-n".into(), pattern.into()]));
    let tantivy_query = format!("title:\"{pattern}\"^2 OR body:{pattern}");
    let tantivy_leaf =
        SearchPlaybookComposition::Leaf(SearchPlaybookLeaf::Tantivy(vec![tantivy_query]));
    let mut leaves = vec![rg_leaf, tantivy_leaf];
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
    let admitted = match admit_search_playbook_plan(input) {
        Ok(request) => request,
        Err(err) => {
            return HelmSearchReply {
                epoch,
                generation_digest: None,
                candidates: Vec::new(),
                error: Some(format!("Helm playbook rejected: {err}")),
            };
        }
    };
    match execute_admitted_playbook(workspace_root, &admitted) {
        Ok(report) => {
            let candidates = report
                .hits
                .into_iter()
                .take(MAX_HELM_HITS)
                .map(hit_to_candidate)
                .collect();
            HelmSearchReply {
                epoch,
                generation_digest: Some(report.generation_digest.as_str().to_owned()),
                candidates,
                error: None,
            }
        }
        Err(err) => HelmSearchReply {
            epoch,
            generation_digest: None,
            candidates: Vec::new(),
            error: Some(format!("Helm search failed: {err}")),
        },
    }
}

fn hit_to_candidate(hit: ConvergedSearchHit) -> HelmCandidate {
    let first_line = hit.line_matches.first().map(|m| m.line_number);
    let display = match first_line {
        Some(line) => format!("{}:{line}", hit.path),
        None => hit.path.clone(),
    };
    let citation = Some(format!("@{display}"));
    let lang = hit.language.as_deref().unwrap_or("text");
    let axes = if hit.matched_axes.is_empty() {
        "search".to_owned()
    } else {
        hit.matched_axes.join("+")
    };
    let annotation = format!("{lang} · {axes} · {}", hit.score);
    let preview_lines = hit
        .line_matches
        .into_iter()
        .take(MAX_PREVIEW_LINES)
        .map(|m| format!("{}: {}", m.line_number, m.line_text.trim_end()))
        .collect();
    HelmCandidate {
        id: format!("search:{display}"),
        source: HelmSourceKind::Search,
        display,
        citation,
        annotation,
        preview_lines,
        buffer_id: None,
        command_name: None,
    }
}

fn rgb(value: UiRgb) -> Color {
    Color::Rgb(value.red, value.green, value.blue)
}

#[cfg(test)]
#[path = "../tests/unit/helm_source.rs"]
mod tests;
