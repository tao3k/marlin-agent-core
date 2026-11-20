// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Execute the typed editor operations selected by the Scheme keymap.

use super::App;
use marlin_agent_ui::{UiEditorAction, UiKeyMode};

impl App {
    pub(super) fn enter_insert_mode(&mut self, after: bool) {
        self.composer.enter_insert(after);
        self.key_mode = UiKeyMode::Insert;
        self.editor_keyboard_active = true;
    }

    pub(super) fn edit(&mut self, action: UiEditorAction, width: u16, accepts_text: bool) {
        if action == UiEditorAction::HistorySearch {
            self.start_history_search();
            return;
        }
        if !accepts_text {
            match action {
                UiEditorAction::PreviousLine => self.scroll(true),
                UiEditorAction::NextLine => self.scroll(false),
                _ => {}
            }
            return;
        }

        let normal = self.key_mode == UiKeyMode::Normal && self.command_draft.is_none();
        let editor = &mut self.composer;
        match action {
            UiEditorAction::PreviousPrompt => self.navigate_prompts(true),
            UiEditorAction::NextPrompt => self.navigate_prompts(false),
            UiEditorAction::AppendInsert => self.enter_insert_mode(true),
            UiEditorAction::OpenLine { below } => {
                if editor.open_line(below) {
                    self.enter_insert_mode(false);
                }
            }
            UiEditorAction::SelectWord => editor.select_word(),
            UiEditorAction::SelectPreviousWord => editor.select_adjacent_word(true),
            UiEditorAction::SelectNextWord => editor.select_adjacent_word(false),
            UiEditorAction::SelectLine => editor.select_line(),
            UiEditorAction::SelectCharacter { target, selection } => {
                editor.select_character(target, selection)
            }
            UiEditorAction::SelectDelimited {
                open,
                close,
                inclusive,
            } => editor.select_delimited(open, close, inclusive),
            UiEditorAction::PreviousSelection => editor.previous_selection(),
            UiEditorAction::ExpandSelection {
                steps,
                at_rest_start,
                ..
            } => {
                if !editor.expand_selection(steps) && at_rest_start {
                    editor.clear_selection();
                    editor.line_start();
                }
            }
            UiEditorAction::ExtendLeft => editor.extend_selection(|editor| editor.left()),
            UiEditorAction::ExtendRight => editor.extend_selection(|editor| editor.right()),
            UiEditorAction::ExtendUp | UiEditorAction::ExtendDown => {
                editor.extend_selection(|editor| {
                    editor.move_vertical(width, action == UiEditorAction::ExtendDown)
                });
            }
            UiEditorAction::ReverseSelection => editor.reverse_selection(),
            UiEditorAction::CopySelection => {
                if let Some(text) = editor.selected_text() {
                    self.yank_register = text.to_owned();
                }
            }
            UiEditorAction::CutSelection | UiEditorAction::ChangeSelection => {
                if let Some(text) = editor.selected_text() {
                    self.yank_register = text.to_owned();
                    editor.replace_selection("");
                }
                if action == UiEditorAction::ChangeSelection {
                    self.enter_insert_mode(false);
                }
            }
            UiEditorAction::PasteSelection => {
                editor.paste(&self.yank_register, normal);
            }
            UiEditorAction::DeleteChar | UiEditorAction::DeleteBackward => {
                if action == UiEditorAction::DeleteBackward {
                    editor.backspace();
                } else {
                    editor.delete();
                }
                if normal {
                    editor.clamp_normal_cursor();
                }
            }
            UiEditorAction::HistorySearch => unreachable!("handled before editing"),
            UiEditorAction::InsertNewline
            | UiEditorAction::BackwardChar
            | UiEditorAction::ForwardChar
            | UiEditorAction::PreviousLine
            | UiEditorAction::NextLine
            | UiEditorAction::NextWord
            | UiEditorAction::PreviousWord
            | UiEditorAction::DeleteLine
            | UiEditorAction::Undo
            | UiEditorAction::LineStart
            | UiEditorAction::LineEnd
            | UiEditorAction::ClearSelection => {
                editor.clear_selection();
                match action {
                    UiEditorAction::InsertNewline => editor.insert('\n'),
                    UiEditorAction::BackwardChar if normal => editor.normal_left(),
                    UiEditorAction::BackwardChar => editor.left(),
                    UiEditorAction::ForwardChar if normal => editor.normal_right(),
                    UiEditorAction::ForwardChar => editor.right(),
                    UiEditorAction::PreviousLine | UiEditorAction::NextLine => {
                        editor.move_vertical(width, action == UiEditorAction::NextLine)
                    }
                    UiEditorAction::NextWord => editor.next_word(),
                    UiEditorAction::PreviousWord => editor.previous_word(),
                    UiEditorAction::DeleteLine => editor.delete_line(),
                    UiEditorAction::Undo => editor.undo(),
                    UiEditorAction::LineStart => editor.line_start(),
                    UiEditorAction::LineEnd => editor.line_end(),
                    UiEditorAction::ClearSelection => {}
                    _ => unreachable!("handled before cursor movement"),
                }
                if normal {
                    editor.clamp_normal_cursor();
                }
            }
        }
    }
}
