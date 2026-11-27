// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Helm Action Transformer menu, live scoping (`@prefix:`, `@lang:`, `@sym:`,
//! `@graph:`), and primary / citation-export resolution.

use super::overlay::HelmOverlay;
use super::types::{HelmAction, HelmMenuActionItem, HelmMenuActionKind, HelmSourceKind};

impl HelmOverlay {
    pub(crate) fn available_actions(&self) -> Vec<HelmMenuActionItem> {
        let mut items = Vec::new();
        let Some(selected) = self.selected_candidate() else {
            return items;
        };
        let primary_label = match selected.source {
            HelmSourceKind::Buffers => format!("Switch to buffer {}", selected.display),
            HelmSourceKind::Commands => format!("Invoke command M-x {}", selected.display),
            HelmSourceKind::Search | HelmSourceKind::Dsl | HelmSourceKind::All => {
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
                key: "M-e",
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
        if let Some(sym) = selected.primary_symbol.as_deref() {
            items.push(HelmMenuActionItem {
                kind: HelmMenuActionKind::ScopeSymbol,
                key: "s",
                label: format!("Jump to symbol definition @sym:{sym}"),
                detail: "Locate exact symbol definitions across workspace".to_owned(),
            });
            items.push(HelmMenuActionItem {
                kind: HelmMenuActionKind::ScopeGraph,
                key: "g",
                label: format!("Explore call/import graph @graph:{sym}"),
                detail: "Show definers, importers, callers, and dependencies".to_owned(),
            });
        } else if let Some(stem) = selected.path.as_deref().and_then(file_stem_for_graph) {
            items.push(HelmMenuActionItem {
                kind: HelmMenuActionKind::ScopeGraph,
                key: "g",
                label: format!("Explore module graph @graph:{stem}"),
                detail: "Show importers and dependencies of this module".to_owned(),
            });
        }
        items
    }

    /// Execute the active action. Returns `(true, Some(action))` when Helm should close
    /// and hand off to the TUI host, or `(false, None)` when an in-Helm transformer
    /// action (like scoping `@prefix:`, `@lang:`, `@sym:`, or `@graph:`) updated Helm live.
    pub(crate) fn accept(&mut self, export_citations_only: bool) -> (bool, Option<HelmAction>) {
        if self.completion_mode {
            if let Some(entry) = self
                .completion_entries
                .get(self.completion_selected)
                .cloned()
            {
                let chain_into_value = entry.kind
                    == marlin_agent_search_engine::SearchDslCompletionKind::Keyword
                    && entry.insert_text.trim_end().ends_with(':');
                let trimmed = self.input.trim();
                let next = if trimmed == "(" || trimmed.is_empty() {
                    entry.insert_text
                } else {
                    let token = super::completion::trailing_completion_fragment(&self.input);
                    let mut updated = self.input.clone();
                    let closed_quoted =
                        token.len() >= 2 && token.starts_with('"') && token.ends_with('"');
                    if closed_quoted
                        && entry.kind
                            == marlin_agent_search_engine::SearchDslCompletionKind::Keyword
                    {
                        updated.push(' ');
                    } else if !token.is_empty() && updated.ends_with(token) {
                        let keep = updated.len().saturating_sub(token.len());
                        updated.truncate(keep);
                    }
                    updated.push_str(&entry.insert_text);
                    updated
                };
                self.completion_mode = false;
                self.set_input(next);
                if chain_into_value {
                    self.refresh_completions();
                    if !self.completion_entries.is_empty() {
                        self.completion_mode = true;
                        self.completion_selected = 0;
                    }
                }
            } else {
                self.completion_mode = false;
            }
            return (false, None);
        }
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
                HelmMenuActionKind::ScopeSymbol => {
                    self.apply_symbol_scope();
                    (false, None)
                }
                HelmMenuActionKind::ScopeGraph => {
                    self.apply_graph_scope();
                    (false, None)
                }
            };
        }
        (true, self.resolve_primary_action(export_citations_only))
    }

    fn apply_symbol_scope(&mut self) {
        let Some(sym) = self
            .selected_candidate()
            .and_then(|c| c.primary_symbol.clone())
        else {
            return;
        };
        self.set_input(format!("@sym:{sym}"));
    }

    fn apply_graph_scope(&mut self) {
        let target = self.selected_candidate().and_then(|c| {
            c.primary_symbol
                .clone()
                .or_else(|| c.path.as_deref().and_then(file_stem_for_graph))
        });
        let Some(target) = target else {
            return;
        };
        self.set_input(format!("@graph:{target}"));
    }

    fn apply_directory_scope(&mut self) {
        if self.is_sexp_prompt() {
            return;
        }
        let Some(path) = self.selected_candidate().and_then(|c| c.path.clone()) else {
            return;
        };
        let Some((dir, _)) = path.rsplit_once('/') else {
            return;
        };
        let token = format!("@prefix:{dir}/");
        let (async_raw, local_suffix) = match self.input.find('#') {
            Some(idx) => (self.input[..idx].trim(), Some(self.input[idx..].trim())),
            None => (self.input.trim(), None),
        };
        let mut parts: Vec<&str> = async_raw
            .split_whitespace()
            .filter(|t| !t.starts_with("@prefix:"))
            .collect();
        parts.insert(0, token.as_str());
        let rebuilt = match local_suffix {
            Some(local) if !local.is_empty() => format!("{} {local}", parts.join(" ")),
            _ => parts.join(" "),
        };
        if rebuilt != self.input {
            self.set_input(rebuilt);
        }
    }

    fn apply_language_scope(&mut self) {
        if self.is_sexp_prompt() {
            return;
        }
        let Some(lang) = self.selected_candidate().and_then(|c| c.language.clone()) else {
            return;
        };
        let token = format!("@lang:{lang}");
        let (async_raw, local_suffix) = match self.input.find('#') {
            Some(idx) => (self.input[..idx].trim(), Some(self.input[idx..].trim())),
            None => (self.input.trim(), None),
        };
        if async_raw.split_whitespace().any(|t| t == token) {
            return;
        }
        let rebuilt_async = if async_raw.is_empty() {
            token
        } else {
            format!("{token} {async_raw}")
        };
        let rebuilt = match local_suffix {
            Some(local) if !local.is_empty() => format!("{rebuilt_async} {local}"),
            _ => rebuilt_async,
        };
        self.set_input(rebuilt);
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
}

fn file_stem_for_graph(path: &str) -> Option<String> {
    let base = path.rsplit('/').next().unwrap_or(path);
    let stem = base.rsplit_once('.').map_or(base, |(s, _)| s);
    (!stem.is_empty() && stem != "mod" && stem != "lib" && stem != "index").then(|| stem.to_owned())
}
