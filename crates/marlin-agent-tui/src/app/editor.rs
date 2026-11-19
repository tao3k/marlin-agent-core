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
        use UiEditorAction::*;

        if action == HistorySearch {
            self.start_history_search();
            return;
        }
        if !accepts_text {
            match action {
                PreviousLine => self.scroll(true),
                NextLine => self.scroll(false),
                _ => {}
            }
            return;
        }

        let normal = self.key_mode == UiKeyMode::Normal && self.command_draft.is_none();
        let editor = &mut self.composer;
        match action {
            PreviousPrompt => self.navigate_prompts(true),
            NextPrompt => self.navigate_prompts(false),
            AppendInsert => self.enter_insert_mode(true),
            OpenLine { below } => {
                if editor.open_line(below) {
                    self.enter_insert_mode(false);
                }
            }
            SelectWord => editor.select_word(),
            SelectPreviousWord => editor.select_adjacent_word(true),
            SelectNextWord => editor.select_adjacent_word(false),
            SelectLine => editor.select_line(),
            SelectCharacter { target, selection } => editor.select_character(target, selection),
            SelectDelimited {
                open,
                close,
                inclusive,
            } => editor.select_delimited(open, close, inclusive),
            PreviousSelection => editor.previous_selection(),
            ExpandSelection {
                steps,
                at_rest_start,
                ..
            } => {
                if !editor.expand_selection(steps) && at_rest_start {
                    editor.clear_selection();
                    editor.line_start();
                }
            }
            ExtendLeft => editor.extend_selection(|editor| editor.left()),
            ExtendRight => editor.extend_selection(|editor| editor.right()),
            ExtendUp | ExtendDown => {
                editor.extend_selection(|editor| editor.move_vertical(width, action == ExtendDown));
            }
            ReverseSelection => editor.reverse_selection(),
            CopySelection => {
                if let Some(text) = editor.selected_text() {
                    self.yank_register = text.to_owned();
                }
            }
            CutSelection | ChangeSelection => {
                if let Some(text) = editor.selected_text() {
                    self.yank_register = text.to_owned();
                    editor.replace_selection("");
                }
                if action == ChangeSelection {
                    self.enter_insert_mode(false);
                }
            }
            PasteSelection => {
                editor.paste(&self.yank_register, normal);
            }
            DeleteChar | DeleteBackward => {
                if action == DeleteBackward {
                    editor.backspace();
                } else {
                    editor.delete();
                }
                if normal {
                    editor.clamp_normal_cursor();
                }
            }
            HistorySearch => unreachable!("handled before editing"),
            InsertNewline | BackwardChar | ForwardChar | PreviousLine | NextLine | NextWord
            | PreviousWord | DeleteLine | Undo | LineStart | LineEnd | ClearSelection => {
                editor.clear_selection();
                match action {
                    InsertNewline => editor.insert('\n'),
                    BackwardChar if normal => editor.normal_left(),
                    BackwardChar => editor.left(),
                    ForwardChar if normal => editor.normal_right(),
                    ForwardChar => editor.right(),
                    PreviousLine | NextLine => editor.move_vertical(width, action == NextLine),
                    NextWord => editor.next_word(),
                    PreviousWord => editor.previous_word(),
                    DeleteLine => editor.delete_line(),
                    Undo => editor.undo(),
                    LineStart => editor.line_start(),
                    LineEnd => editor.line_end(),
                    ClearSelection => {}
                    _ => unreachable!("handled before cursor movement"),
                }
                if normal {
                    editor.clamp_normal_cursor();
                }
            }
        }
    }
}
