// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Navigation over completed conversation prompts while preserving the draft.

use super::composer::{Composer, MAX_DRAFT_BYTES};
use marlin_agent_protocol::{ModelGatewayMessage, ModelGatewayMessageRole};
use unicode_segmentation::UnicodeSegmentation;

const MAX_SEARCH_BYTES: usize = 256;
const SEARCH_STEP_BYTES: usize = 32 * 1024;
const SEARCH_STEP_MESSAGES: usize = 64;

struct SearchScan {
    next: usize,
    current: Option<(usize, usize)>,
    wrap: Option<usize>,
}

/// A search is a preview: it does not change the draft until accepted.
#[derive(Default)]
pub(crate) struct HistorySearch {
    query: String,
    index: Option<usize>,
    preview: Option<Composer>,
    scan: Option<SearchScan>,
}

impl HistorySearch {
    pub(crate) fn query(&self) -> &str {
        &self.query
    }
    pub(crate) fn preview(&self) -> Option<&Composer> {
        self.preview.as_ref()
    }
    pub(crate) fn is_searching(&self) -> bool {
        self.scan.is_some()
    }

    pub(crate) fn insert(&mut self, text: &str, history: &[ModelGatewayMessage]) {
        if text.chars().any(char::is_control)
            || text.len() > MAX_SEARCH_BYTES.saturating_sub(self.query.len())
        {
            return;
        }
        self.query.push_str(text);
        self.begin(history, history.len());
    }

    pub(crate) fn backspace(&mut self, history: &[ModelGatewayMessage]) {
        if let Some((start, _)) = self.query.grapheme_indices(true).next_back() {
            self.query.truncate(start);
            self.begin(history, history.len());
        }
    }

    pub(crate) fn previous(&mut self, history: &[ModelGatewayMessage]) {
        if self.is_searching() {
            return;
        }
        let end = self.index.unwrap_or(history.len());
        self.begin(history, end);
    }

    fn begin(&mut self, history: &[ModelGatewayMessage], end: usize) {
        self.index = None;
        self.preview = None;
        self.scan = Some(SearchScan {
            next: end,
            current: None,
            wrap: (end < history.len()).then_some(history.len()),
        });
        self.step(history);
    }

    /// Advance a bounded portion of the search; returns true when it finishes.
    pub(crate) fn step(&mut self, history: &[ModelGatewayMessage]) -> bool {
        let Some(mut scan) = self.scan.take() else {
            return false;
        };
        let mut bytes = SEARCH_STEP_BYTES;
        let mut messages = SEARCH_STEP_MESSAGES;
        loop {
            if scan.current.is_none() {
                if scan.next == 0 {
                    if let Some(end) = scan.wrap.take() {
                        scan.next = end;
                    } else {
                        return true;
                    }
                }
                if messages == 0 {
                    break;
                }
                messages -= 1;
                scan.next -= 1;
                if !eligible(&history[scan.next]) {
                    continue;
                }
                scan.current = Some((scan.next, 0));
            }
            let (index, offset) = scan.current.expect("search item selected");
            let text = &history[index].content;
            let mut end = offset.saturating_add(bytes).min(text.len());
            while !text.is_char_boundary(end) {
                end -= 1;
            }
            if end == offset && offset < text.len() {
                break;
            }
            // Include an overlap so matches spanning adjacent chunks survive.
            let mut start = offset.saturating_sub(self.query.len().saturating_sub(1));
            while !text.is_char_boundary(start) {
                start -= 1;
            }
            if text[start..end].contains(&self.query) {
                self.index = Some(index);
                self.preview = Some(from_prompt(text));
                return true;
            }
            bytes -= end - offset;
            if end == text.len() {
                scan.current = None;
            } else {
                scan.current = Some((index, end));
            }
            if bytes < 4 {
                break;
            }
        }
        self.scan = Some(scan);
        false
    }

    pub(crate) fn accept(self) -> Option<Composer> {
        self.preview
    }
}

#[derive(Default)]
pub(crate) struct PromptHistory {
    draft: Option<Composer>,
    index: Option<usize>,
}

impl PromptHistory {
    pub(crate) fn discard(&mut self) {
        self.draft = None;
        self.index = None;
    }
    pub(crate) fn previous(&mut self, composer: &mut Composer, history: &[ModelGatewayMessage]) {
        let end = self.index.unwrap_or(history.len());
        let Some(index) = (0..end).rev().find(|&i| eligible(&history[i])) else {
            return;
        };
        if self.draft.is_none() {
            self.draft = Some(std::mem::take(composer));
        }
        *composer = from_prompt(&history[index].content);
        self.index = Some(index);
    }

    pub(crate) fn next(&mut self, composer: &mut Composer, history: &[ModelGatewayMessage]) {
        let Some(index) = self.index else {
            return;
        };
        if let Some(next) = (index + 1..history.len()).find(|&i| eligible(&history[i])) {
            *composer = from_prompt(&history[next].content);
            self.index = Some(next);
        } else {
            self.restore(composer);
        }
    }

    pub(crate) fn restore(&mut self, composer: &mut Composer) {
        if let Some(draft) = self.draft.take() {
            *composer = draft;
        }
        self.index = None;
    }
}

fn eligible(message: &ModelGatewayMessage) -> bool {
    message.role == ModelGatewayMessageRole::User && message.content.len() <= MAX_DRAFT_BYTES
}

fn from_prompt(text: &str) -> Composer {
    let mut composer = Composer::default();
    composer.insert_text(text);
    composer
}
