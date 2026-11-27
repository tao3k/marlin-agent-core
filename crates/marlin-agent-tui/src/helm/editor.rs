// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Shared `Composer`-backed Emacs editing operations for Helm's minibuffer.

use marlin_agent_ui::UiEditorAction;

use super::overlay::HelmOverlay;

const MAX_HELM_INPUT_BYTES: usize = 2_048;

fn sanitize_bounded_helm_input(text: &str, max_bytes: usize) -> String {
    let mut clean = String::new();
    for ch in text.chars().filter(|ch| !ch.is_control()) {
        if clean.len() + ch.len_utf8() > max_bytes {
            break;
        }
        clean.push(ch);
    }
    clean
}

impl HelmOverlay {
    pub(crate) fn cursor_byte(&self) -> usize {
        self.editor.cursor()
    }

    pub(crate) fn set_input(&mut self, text: impl Into<String>) {
        let raw = text.into();
        let clean = sanitize_bounded_helm_input(&raw, MAX_HELM_INPUT_BYTES);
        self.editor.set_text_at_end(clean);
        self.sync_editor_input();
        self.on_input_changed();
    }

    pub(super) fn sync_editor_input(&mut self) -> bool {
        let next = self.editor.text();
        if self.input == next {
            return false;
        }
        self.input.clear();
        if self.input.capacity() > MAX_HELM_INPUT_BYTES * 2 && next.len() <= MAX_HELM_INPUT_BYTES {
            self.input.shrink_to(MAX_HELM_INPUT_BYTES);
        }
        self.input.push_str(next);
        true
    }

    /// Clear the Helm minibuffer prompt immediately without closing Helm.
    #[cfg(test)]
    pub(crate) fn clear_input(&mut self) {
        if self.input.is_empty() && self.preset_cursor.is_none() {
            return;
        }
        self.editor.take();
        self.input.clear();
        if self.input.capacity() > MAX_HELM_INPUT_BYTES * 2 {
            self.input.shrink_to(MAX_HELM_INPUT_BYTES);
        }
        self.preset_cursor = None;
        self.active_preset_label = None;
        self.completion_mode = false;
        self.action_mode = false;
        self.on_input_changed();
    }

    pub(crate) fn insert_text(&mut self, text: &str) {
        if self.action_mode {
            return;
        }
        let remaining = MAX_HELM_INPUT_BYTES.saturating_sub(self.input.len());
        if remaining == 0 {
            return;
        }
        let clean = sanitize_bounded_helm_input(text, remaining);
        if clean.is_empty() {
            return;
        }
        self.editor.insert_text(&clean);
        if self.sync_editor_input() {
            self.on_input_changed();
        }
    }

    pub(crate) fn backspace(&mut self) {
        if self.action_mode {
            self.action_mode = false;
            return;
        }
        self.editor.backspace();
        if self.sync_editor_input() {
            self.on_input_changed();
        }
    }

    /// Execute any global Scheme Emacs/Meow editing action (`C-a`, `C-e`, `C-b`,
    /// `C-f`, `C-d`, `M-b`, `M-f`, `Undo`, `PasteSelection`, etc.) directly on
    /// Helm's shared `Composer`.
    pub(crate) fn edit(&mut self, action: UiEditorAction, yank_register: &mut String) {
        if self.action_mode {
            return;
        }
        match action {
            UiEditorAction::LineStart => self.editor.line_start(),
            UiEditorAction::LineEnd => self.editor.line_end(),
            UiEditorAction::BackwardChar => self.editor.left(),
            UiEditorAction::ForwardChar => self.editor.right(),
            UiEditorAction::PreviousWord => self.editor.previous_word(),
            UiEditorAction::NextWord => self.editor.next_word(),
            UiEditorAction::DeleteChar => self.editor.delete(),
            UiEditorAction::DeleteBackward => self.editor.backspace(),
            UiEditorAction::DeleteLine => {
                let killed = self.editor.kill_to_line_start();
                if !killed.is_empty() {
                    *yank_register = killed;
                }
            }
            UiEditorAction::Undo => self.editor.undo(),
            UiEditorAction::ClearSelection => {
                self.editor.clear_selection();
            }
            UiEditorAction::CopySelection => {
                if let Some(selected) = self.editor.selected_text() {
                    *yank_register = selected.to_owned();
                }
            }
            UiEditorAction::CutSelection | UiEditorAction::ChangeSelection => {
                if let Some(selected) = self.editor.selected_text() {
                    *yank_register = selected.to_owned();
                    self.editor.replace_selection("");
                }
            }
            UiEditorAction::PasteSelection => {
                self.insert_text(yank_register);
                return;
            }
            _ => {}
        }
        self.finish_editor_mutation();
    }

    pub(crate) fn kill_to_line_end(&mut self, yank_register: &mut String) {
        if self.action_mode {
            return;
        }
        let killed = self.editor.kill_to_line_end();
        if !killed.is_empty() {
            *yank_register = killed;
        }
        self.finish_editor_mutation();
    }

    pub(crate) fn kill_to_line_start(&mut self, yank_register: &mut String) {
        if self.action_mode {
            return;
        }
        let killed = self.editor.kill_to_line_start();
        if !killed.is_empty() {
            *yank_register = killed;
        }
        self.finish_editor_mutation();
    }

    pub(crate) fn delete_backward_word(&mut self, yank_register: &mut String) {
        if self.action_mode {
            return;
        }
        let killed = self.editor.delete_backward_word();
        if !killed.is_empty() {
            *yank_register = killed;
        }
        self.finish_editor_mutation();
    }

    fn finish_editor_mutation(&mut self) {
        if self.sync_editor_input() {
            if self.input.is_empty() {
                self.preset_cursor = None;
                self.active_preset_label = None;
            }
            self.on_input_changed();
        }
    }
}
