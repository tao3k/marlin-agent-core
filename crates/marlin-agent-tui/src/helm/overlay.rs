// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Authentic Emacs-inspired Modern Helm state machine backed by a sub-millisecond
//! resident trigram workspace search index (`WorkspaceSearchIndex`).
//!
//! - When the resident index is warm in RAM, queries execute synchronously in
//!   **< 0.5 ms** (zero thread-spawn lag, zero 16ms polling jitter, zero disk I/O).
//! - On first boot, loads via zero-copy `mmap` cached snapshots (`open_or_build_cached`),
//!   and on manual `C-l` refresh runs in-place incremental disk sync
//!   (`sync_workspace_from_disk`) with exact-epoch fencing (`dev/quint/ui-helm.qnt`).
//! - Organizes results into authentic Emacs Helm source groups (`Workspace Search`,
//!   `Buffers`, `Commands (M-x)`), supports `C-o` next-source jump, `C-SPC`/`M-m`
//!   mark-and-advance, `M-a` mark-all, `C-z` live follow-mode split preview, and
//!   `TAB` interactive Action Transformer menu.

use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use marlin_agent_search_engine::{
    HelmSearchPresetSpec, SearchDslCompletionEntry, SearchDslExecutionMode,
    SearchDslStructuredResult, SearchDslStructuredRunner, WorkspaceIndexConfig,
    WorkspaceSearchQuery, default_search_dsl_structured_runner,
    default_workspace_edb_cache_path, default_workspace_snapshot_path,
    hydrate_workspace_open_edb_org_and_cache,
};
use marlin_agent_ui::UiCommand;
use tokio::sync::oneshot;

use super::presets::preset_index_by_alias;
use super::types::{
    HelmCandidate, HelmDisplayRow, HelmParsedInput, HelmSourceKind, SharedSearchIndex,
    admit_helm_playbook, build_static_candidates, dsl_row_to_candidate, hit_to_candidate,
};
use crate::composer::Composer;

const MAX_HELM_HITS: usize = 64;
const PREVIEW_CONTEXT_RADIUS: usize = 5;

pub(crate) struct HelmOverlay {
    pub(crate) editor: Composer,
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
    pub(crate) completion_mode: bool,
    pub(crate) completion_selected: usize,
    pub(crate) completion_catalog: Vec<SearchDslCompletionEntry>,
    pub(crate) completion_entries: Vec<SearchDslCompletionEntry>,
    pub(crate) generation_digest: Option<String>,
    pub(crate) last_query_micros: Option<u64>,
    pub(crate) indexed_files: usize,
    pub(crate) last_error: Option<String>,
    pub(crate) last_dsl_repairs: Vec<String>,
    pub(crate) last_dsl_mode: Option<SearchDslExecutionMode>,
    pub(crate) last_dsl_relation: Option<String>,
    pub(crate) last_choice_worlds: usize,
    pub(crate) last_budget_exhausted: Option<String>,
    pub(crate) asp_status_badge: String,
    pub(crate) search_presets: Vec<HelmSearchPresetSpec>,
    pub(crate) active_preset_label: Option<String>,
    pub(crate) preset_cursor: Option<usize>,
    pub(crate) indexing: bool,
    workspace_root: PathBuf,
    snapshot_path: PathBuf,
    edb_cache_path: PathBuf,
    index_config: WorkspaceIndexConfig,
    index: Option<SharedSearchIndex>,
    index_receiver: Option<oneshot::Receiver<SharedSearchIndex>>,
    dsl_runner: Option<SearchDslStructuredRunner>,
    #[cfg(feature = "linked-ui")]
    pending_dsl_query: Option<String>,
}

impl HelmOverlay {
    pub(crate) fn open(
        initial_query: Option<&str>,
        workspace_root: PathBuf,
        command_catalog: &[UiCommand],
        index_config: WorkspaceIndexConfig,
        warm_index: Option<SharedSearchIndex>,
        runtime: &tokio::runtime::Handle,
    ) -> Self {
        let input = initial_query.unwrap_or("").trim().to_owned();
        let mut editor = Composer::default();
        if !input.is_empty() {
            editor.insert_text(&input);
        }
        let parsed = HelmParsedInput::parse(&input);
        let static_candidates = build_static_candidates(command_catalog);
        let snapshot_path = default_workspace_snapshot_path(&workspace_root);
        let edb_cache_path = default_workspace_edb_cache_path(&workspace_root);
        let (indexed_files, generation_digest) = warm_index
            .as_ref()
            .and_then(|shared| shared.read().ok())
            .map_or((0, None), |idx| {
                (
                    idx.file_count(),
                    Some(idx.generation_digest().as_str().to_owned()),
                )
            });

        let mut overlay = Self {
            editor,
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
            completion_mode: false,
            completion_selected: 0,
            completion_catalog: Vec::new(),
            completion_entries: Vec::new(),
            generation_digest,
            last_query_micros: None,
            indexed_files,
            last_error: None,
            last_dsl_repairs: Vec::new(),
            last_dsl_mode: None,
            last_dsl_relation: None,
            last_choice_worlds: 0,
            last_budget_exhausted: None,
            asp_status_badge: String::new(),
            search_presets: Vec::new(),
            active_preset_label: None,
            preset_cursor: None,
            indexing: warm_index.is_none(),
            workspace_root,
            snapshot_path,
            edb_cache_path,
            index_config,
            index: warm_index,
            index_receiver: None,
            dsl_runner: default_search_dsl_structured_runner(),
            #[cfg(feature = "linked-ui")]
            pending_dsl_query: None,
        };

        if overlay.index.is_some() {
            overlay.run_resident_query();
        } else {
            overlay.spawn_index_build(runtime);
            overlay.rebuild_view();
        }
        overlay
    }

    pub(crate) fn attach_warm_index(&mut self, index: SharedSearchIndex) {
        if let Ok(guard) = index.read() {
            self.indexed_files = guard.file_count();
            self.generation_digest = Some(guard.generation_digest().as_str().to_owned());
        }
        self.index = Some(index);
        self.index_receiver = None;
        self.indexing = false;
        self.run_resident_query();
    }

    pub(crate) fn is_indexing(&self) -> bool {
        self.indexing
    }

    pub(crate) fn refresh_index(&mut self, runtime: &tokio::runtime::Handle) {
        if let Some(existing) = self.index.clone() {
            let snapshot_path = self.snapshot_path.clone();
            let edb_cache_path = self.edb_cache_path.clone();
            let (tx, rx) = oneshot::channel();
            self.indexing = true;
            self.index_receiver = Some(rx);
            runtime.spawn_blocking(move || {
                if let Ok(mut guard) = existing.write() {
                    let receipt = guard.sync_workspace_from_disk();
                    let _ = hydrate_workspace_open_edb_org_and_cache(&mut guard, &edb_cache_path);
                    if receipt.upserted_files > 0 || receipt.deleted_files > 0 {
                        guard.compact();
                        let _ = marlin_agent_search_engine::save_workspace_snapshot_atomic(
                            &guard,
                            &snapshot_path,
                        );
                    }
                }
                let _ = tx.send(existing);
            });
        } else {
            self.spawn_index_build(runtime);
        }
    }

    fn spawn_index_build(&mut self, runtime: &tokio::runtime::Handle) {
        let root = self.workspace_root.clone();
        let snapshot_path = self.snapshot_path.clone();
        let edb_cache_path = self.edb_cache_path.clone();
        let config = self.index_config.clone();
        let (tx, rx) = oneshot::channel();
        self.indexing = true;
        self.index_receiver = Some(rx);
        runtime.spawn_blocking(move || {
            let mut idx = marlin_agent_search_engine::open_or_build_workspace_index_safe(
                &root,
                &snapshot_path,
                config,
            );
            let _ = hydrate_workspace_open_edb_org_and_cache(&mut idx, &edb_cache_path);
            let built = Arc::new(RwLock::new(idx));
            let _ = tx.send(built);
        });
    }

    pub(crate) fn poll(&mut self) -> Option<SharedSearchIndex> {
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

    /// Attach the Scheme-introspected `:gerbil-ascent` / POO TAB completion
    /// catalog populated from `scheme/search-dsl` (`SearchLanguageRegistry.`).
    pub(crate) fn with_completion_catalog(
        mut self,
        catalog: Vec<SearchDslCompletionEntry>,
    ) -> Self {
        self.completion_catalog = catalog;
        if super::completion::supports_copilot_completion(&self.input) {
            self.refresh_completions();
        }
        self
    }

    /// Attach a custom structured `:gerbil-ascent` / Dusa DSL runner (e.g. for tests).
    #[cfg(test)]
    pub(crate) fn with_dsl_runner(mut self, runner: SearchDslStructuredRunner) -> Self {
        self.dsl_runner = Some(runner);
        if self.is_sexp_prompt() && self.index.is_some() {
            self.run_resident_query();
        }
        self
    }

    /// Take a pending S-expression query and its originating epoch for main-thread
    /// AOT `linked-ui` execution (SM8 epoch fencing).
    #[cfg(feature = "linked-ui")]
    pub(crate) fn take_pending_dsl_query(&mut self) -> Option<(SharedSearchIndex, u64, String)> {
        let query = self.pending_dsl_query.take()?;
        let index = self.index.clone()?;
        Some((index, self.epoch, query))
    }

    /// Override cache paths (e.g. unified under `.data/marlin/search/` from Scheme Artifacts).
    pub(crate) fn with_cache_paths(
        mut self,
        snapshot_path: PathBuf,
        edb_cache_path: PathBuf,
    ) -> Self {
        self.snapshot_path = snapshot_path;
        self.edb_cache_path = edb_cache_path;
        self
    }

    /// Attach the live ASP gateway status badge (e.g. `ASP:5p·153e`).
    pub(crate) fn with_asp_status_badge(mut self, badge: String) -> Self {
        self.asp_status_badge = badge;
        self
    }

    pub(crate) fn set_asp_status_badge(&mut self, badge: String) {
        self.asp_status_badge = badge;
    }

    /// Attach the Scheme-projected Helm mode preset catalog (`HelmSearchPresets.`).
    /// If the initial input matched a preset alias (e.g. `M-x helm dusa`), activates it immediately.
    pub(crate) fn with_search_presets(mut self, presets: Vec<HelmSearchPresetSpec>) -> Self {
        let initial_alias = preset_index_by_alias(&presets, &self.input);
        self.search_presets = presets;
        if let Some(idx) = initial_alias {
            self.select_search_preset(idx);
        }
        self
    }

    /// Jump directly to a numbered Scheme-declared preset tab (`0..self.search_presets.len()`),
    /// e.g. via `Alt-1`..`Alt-5`, `F1`–`F5`, or clicking a top-bar tab.
    pub(crate) fn select_search_preset(&mut self, index: usize) {
        let Some(preset) = self.search_presets.get(index).cloned() else {
            return;
        };
        self.preset_cursor = Some(index);
        self.active_preset_label = Some(preset.label);
        self.completion_mode = false;
        self.action_mode = false;
        self.set_input(preset.query);
    }

    /// Step backward through Scheme-declared presets (`BackTab` / `Alt-[`).
    pub(crate) fn cycle_search_preset_prev(&mut self) {
        if self.search_presets.is_empty() {
            return;
        }
        let prev = match self.preset_cursor {
            Some(0) | None => self.search_presets.len() - 1,
            Some(idx) => idx - 1,
        };
        self.select_search_preset(prev);
    }

    /// Cycle forward through Scheme-declared presets (`Ctrl-t`).
    pub(crate) fn cycle_search_preset(&mut self) {
        if self.search_presets.is_empty() {
            return;
        }
        let next = self
            .preset_cursor
            .map_or(0, |idx| (idx + 1) % self.search_presets.len());
        self.select_search_preset(next);
    }

    /// Apply the outcome of a live `:gerbil-ascent` / Dusa query to Helm candidates,
    /// discarding stale results whose `expected_epoch` no longer matches `self.epoch` (SM8).
    pub(crate) fn apply_dsl_result(
        &mut self,
        expected_epoch: u64,
        outcome: Result<SearchDslStructuredResult, String>,
    ) {
        if expected_epoch != self.epoch {
            return;
        }
        match outcome {
            Ok(result) => {
                let guard = self.index.as_ref().and_then(|shared| shared.read().ok());
                let index_ref = guard.as_deref();
                self.generation_digest = Some(result.generation_digest.clone());
                self.last_query_micros = Some(result.elapsed_micros);
                self.indexed_files = result.indexed_files;
                self.last_dsl_repairs = result.repairs.clone();
                self.last_dsl_mode = Some(result.mode);
                self.last_dsl_relation = Some(result.relation.clone());
                self.last_choice_worlds = result.choice_worlds.len();
                self.last_budget_exhausted = result.budget_exhausted.clone();
                self.last_error = if result.mode == SearchDslExecutionMode::Choice
                    && result.choice_worlds.is_empty()
                    && result.budget_exhausted.is_none()
                {
                    Some(
                        "  Dusa UNSAT: constraints are mutually unsatisfiable under active EDB facts"
                            .to_owned(),
                    )
                } else {
                    None
                };
                self.search_candidates = result
                    .rows
                    .iter()
                    .enumerate()
                    .map(|(idx, row)| {
                        dsl_row_to_candidate(idx, row, &result, index_ref, PREVIEW_CONTEXT_RADIUS)
                    })
                    .collect();
            }
            Err(message) => {
                self.last_dsl_repairs.clear();
                self.last_dsl_mode = None;
                self.last_dsl_relation = None;
                self.last_choice_worlds = 0;
                self.last_budget_exhausted = None;
                self.last_error = Some(message);
                self.search_candidates.clear();
            }
        }
        self.rebuild_view();
    }

    pub(crate) fn is_sexp_prompt(&self) -> bool {
        self.input.trim_start().starts_with('(')
    }

    pub(crate) fn refresh_completions(&mut self) {
        let guard = self.index.as_ref().and_then(|shared| shared.read().ok());
        self.completion_entries = super::completion::compute_copilot_completions(
            &self.input,
            &self.completion_catalog,
            guard.as_deref(),
        );
        if self.completion_entries.is_empty() {
            self.completion_selected = 0;
        } else if self.completion_selected >= self.completion_entries.len() {
            self.completion_selected = self.completion_entries.len() - 1;
        }
    }

    pub(crate) fn copilot_ghost_suffix(&self) -> Option<String> {
        super::completion::compute_ghost_suffix(
            &self.input,
            self.cursor_byte(),
            &self.completion_entries,
            self.completion_selected,
        )
    }

    /// Context-aware `TAB`: opens/toggles Copilot keyword/value/relation
    /// completion when editing an S-expression prompt (starting with `(`) or a
    /// `@lang:` / `@prefix:` facet, falling back to Action Transformer otherwise.
    pub(crate) fn toggle_tab_mode(&mut self) {
        if self.completion_mode {
            self.completion_mode = false;
            return;
        }
        if super::completion::supports_copilot_completion(&self.input)
            && (!self.completion_catalog.is_empty() || !self.is_sexp_prompt())
        {
            self.action_mode = false;
            self.refresh_completions();
            if !self.completion_entries.is_empty() {
                self.completion_mode = true;
                self.completion_selected = 0;
                return;
            }
        }
        self.toggle_action_mode();
    }

    pub(crate) fn close_completion_mode(&mut self) {
        self.completion_mode = false;
    }

    pub(super) fn on_input_changed(&mut self) {
        let next_parsed = HelmParsedInput::parse(&self.input);
        let async_changed = self.parsed.async_key() != next_parsed.async_key();
        self.parsed = next_parsed;
        self.action_mode = false;
        self.action_selected = 0;
        if super::completion::supports_copilot_completion(&self.input) {
            self.refresh_completions();
        } else {
            self.completion_entries.clear();
            self.completion_mode = false;
        }
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
        let Some(shared) = self.index.clone() else {
            self.rebuild_view();
            return;
        };
        if self.is_sexp_prompt() {
            self.run_dsl_query(shared);
            return;
        }
        #[cfg(feature = "linked-ui")]
        {
            self.pending_dsl_query = None;
        }
        self.last_dsl_repairs.clear();
        self.last_dsl_mode = None;
        self.last_dsl_relation = None;
        let has_filters = !self.parsed.languages.is_empty()
            || !self.parsed.documents.is_empty()
            || self.parsed.path_prefix.is_some()
            || self.parsed.extension.is_some()
            || self.parsed.symbol.is_some()
            || self.parsed.graph_target.is_some();

        if self.parsed.async_pattern.is_empty() && !has_filters {
            let Ok(index) = shared.read() else {
                self.rebuild_view();
                return;
            };
            let report = index.query(&WorkspaceSearchQuery {
                pattern: "",
                languages: &[],
                documents: &[],
                path_prefix: None,
                extension: None,
                symbol: None,
                graph_target: None,
                max_hits: 24,
                context_radius: PREVIEW_CONTEXT_RADIUS,
            });
            drop(index);
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

        let Ok(index) = shared.read() else {
            self.rebuild_view();
            return;
        };
        let report = index.query(&WorkspaceSearchQuery {
            pattern: &self.parsed.async_pattern,
            languages: &self.parsed.languages,
            documents: &self.parsed.documents,
            path_prefix: self.parsed.path_prefix.as_deref(),
            extension: self.parsed.extension.as_deref(),
            symbol: self.parsed.symbol.as_deref(),
            graph_target: self.parsed.graph_target.as_deref(),
            max_hits: MAX_HELM_HITS,
            context_radius: PREVIEW_CONTEXT_RADIUS,
        });
        drop(index);
        self.generation_digest = Some(report.generation_digest.as_str().to_owned());
        self.last_query_micros = Some(report.elapsed_micros);
        self.last_error = None;
        self.search_candidates = report.hits.into_iter().map(hit_to_candidate).collect();
        self.rebuild_view();
    }

    fn run_dsl_query(&mut self, shared: SharedSearchIndex) {
        let trimmed = self.input.trim();
        if trimmed.len() <= 2 || !trimmed.contains(char::is_whitespace) {
            #[cfg(feature = "linked-ui")]
            {
                self.pending_dsl_query = None;
            }
            self.last_dsl_repairs.clear();
            self.last_dsl_mode = None;
            self.last_dsl_relation = None;
            self.last_error = Some(
                "  :gerbil-ascent mode · press TAB for POO relation completions or type a query"
                    .to_owned(),
            );
            self.search_candidates.clear();
            self.rebuild_view();
            return;
        }
        if let Some(runner) = self.dsl_runner.clone() {
            #[cfg(feature = "linked-ui")]
            {
                self.pending_dsl_query = None;
            }
            let epoch = self.epoch;
            let outcome = runner(shared, trimmed, 16_384).map_err(|err| err.to_string());
            self.apply_dsl_result(epoch, outcome);
        } else {
            #[cfg(feature = "linked-ui")]
            {
                self.pending_dsl_query = Some(trimmed.to_owned());
            }
            #[cfg(not(feature = "linked-ui"))]
            {
                self.last_error = Some("search DSL runner unavailable".to_owned());
            }
            self.rebuild_view();
        }
    }

    pub(crate) fn move_selection(&mut self, delta: isize) {
        if self.completion_mode {
            let total = self.completion_entries.len();
            if total == 0 {
                self.completion_selected = 0;
            } else {
                self.completion_selected = self
                    .completion_selected
                    .saturating_add_signed(delta)
                    .min(total - 1);
            }
            return;
        }
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
            (HelmSourceKind::Dsl, Vec::new()),
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
            if !matches!(cand.source, HelmSourceKind::Search | HelmSourceKind::Dsl)
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
            let meta = match source {
                HelmSourceKind::Search => format!("{} · {} files", latency, self.indexed_files),
                HelmSourceKind::Dsl => {
                    let rel = self.last_dsl_relation.as_deref().unwrap_or("answer");
                    let mode = self
                        .last_dsl_mode
                        .map_or("ascent", SearchDslExecutionMode::as_str);
                    let world_part = if self.last_dsl_mode == Some(SearchDslExecutionMode::Choice) {
                        if self.last_budget_exhausted.is_some() {
                            " · budget-exhausted".to_owned()
                        } else if self.last_choice_worlds == 0 {
                            " · UNSAT".to_owned()
                        } else {
                            format!(" · {} worlds (SAT)", self.last_choice_worlds)
                        }
                    } else {
                        String::new()
                    };
                    if self.last_dsl_repairs.is_empty() {
                        format!("{mode}:{rel}{world_part} · {latency}")
                    } else {
                        format!(
                            "{mode}:{rel}{world_part} · {latency} · repaired[{}]",
                            self.last_dsl_repairs.join(",")
                        )
                    }
                }
                _ => String::new(),
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

        let valid_ids: Vec<String> = self
            .filtered_indices
            .iter()
            .filter_map(|&idx| self.candidate_at(idx).map(|cand| cand.id.clone()))
            .collect();
        self.marked_ids
            .retain(|mid| valid_ids.iter().any(|valid| valid == mid));

        let action_len = self.available_actions().len();
        if action_len == 0 {
            self.action_selected = 0;
            self.action_mode = false;
        } else if self.action_selected >= action_len {
            self.action_selected = action_len - 1;
        }
    }
}

pub(super) fn last_sexp_token(text: &str) -> &str {
    let bytes = text.as_bytes();
    let mut idx = bytes.len();
    while idx > 0 {
        let ch = bytes[idx - 1];
        if ch.is_ascii_whitespace() || ch == b'(' || ch == b')' || ch == b'"' {
            break;
        }
        idx -= 1;
    }
    &text[idx..]
}
