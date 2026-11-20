// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! A grapheme-aware multiline composer.

use std::collections::VecDeque;
use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::ui_elements::UiInputValue as ComposerView;

#[path = "expansion.rs"]
mod expansion;
use expansion::ExpansionUnit;
#[path = "layout.rs"]
mod layout;

pub(crate) const MAX_DRAFT_BYTES: usize = 1024 * 1024;
const MAX_UNDO_EDITS: usize = 128;
const MAX_SELECTION_HISTORY: usize = 32;

fn word_class(grapheme: &str) -> u8 {
    match grapheme.chars().next() {
        Some(character) if character.is_whitespace() => 0,
        Some(character) if character.is_alphanumeric() || character == '_' => 1,
        _ => 2,
    }
}

#[derive(Clone)]
struct UndoEdit {
    start: usize,
    inserted_bytes: usize,
    removed: String,
    cursor_before: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct SelectionState {
    anchor: Option<usize>,
    cursor: usize,
    expansion_unit: Option<ExpansionUnit>,
}

/// Editable multiline input with grapheme-aware cursor movement.
#[derive(Clone, Default)]
pub struct Composer {
    text: String,
    cursor: usize,
    anchor: Option<usize>,
    expansion_unit: Option<ExpansionUnit>,
    selection_history: VecDeque<SelectionState>,
    vertical_column: Option<(u16, usize)>,
    limit_reached: bool,
    undo: VecDeque<UndoEdit>,
    undo_bytes: usize,
    undo_insert_group: bool,
}

impl Composer {
    pub fn text(&self) -> &str {
        &self.text
    }
    pub(crate) fn cursor_at_end(&self) -> bool {
        self.cursor == self.text.len()
    }
    pub(crate) fn limit_reached(&self) -> bool {
        self.limit_reached
    }
    #[cfg(test)]
    pub fn cursor(&self) -> usize {
        self.cursor
    }
    pub fn insert(&mut self, c: char) {
        if self.anchor.is_some() && self.replace_selection(c.encode_utf8(&mut [0; 4])) {
            return;
        }
        if c.len_utf8() > MAX_DRAFT_BYTES.saturating_sub(self.text.len()) {
            self.limit_reached = true;
            return;
        }
        self.limit_reached = false;
        self.vertical_column = None;
        self.record_edit(self.cursor, c.len_utf8(), String::new(), self.cursor);
        self.undo_insert_group = true;
        self.text.insert(self.cursor, c);
        self.cursor += c.len_utf8();
        self.normalize_cursor();
    }
    pub fn insert_text(&mut self, text: &str) {
        if text.is_empty() || self.replace_selection(text) {
            return;
        }
        if text.len() > MAX_DRAFT_BYTES.saturating_sub(self.text.len()) {
            self.limit_reached = true;
            return;
        }
        self.limit_reached = false;
        self.vertical_column = None;
        self.record_edit(self.cursor, text.len(), String::new(), self.cursor);
        self.undo_insert_group = true;
        self.text.insert_str(self.cursor, text);
        self.cursor += text.len();
        self.normalize_cursor();
    }
    pub fn left(&mut self) {
        self.undo_insert_group = false;
        self.vertical_column = None;
        self.cursor = self.text[..self.cursor]
            .grapheme_indices(true)
            .next_back()
            .map_or(0, |(i, _)| i);
    }
    pub fn right(&mut self) {
        self.undo_insert_group = false;
        self.vertical_column = None;
        if let Some(g) = self.text[self.cursor..].graphemes(true).next() {
            self.cursor += g.len();
        }
    }

    /// Move to the start of the current logical line, independent of wrapping.
    pub fn line_start(&mut self) {
        self.undo_insert_group = false;
        self.vertical_column = None;
        self.cursor = self.text[..self.cursor].rfind('\n').map_or(0, |i| i + 1);
    }

    /// Move to the end of the current logical line, before its newline.
    pub fn line_end(&mut self) {
        self.undo_insert_group = false;
        self.vertical_column = None;
        self.cursor = self.text[self.cursor..]
            .find('\n')
            .map_or(self.text.len(), |i| self.cursor + i);
    }
    pub fn backspace(&mut self) {
        if self.replace_selection("") {
            return;
        }
        let old = self.cursor;
        self.left();
        if self.cursor != old {
            self.record_edit(self.cursor, 0, self.text[self.cursor..old].to_owned(), old);
        }
        self.text.drain(self.cursor..old);
        if self.cursor != old {
            self.limit_reached = false;
        }
        self.normalize_cursor();
    }
    pub fn delete(&mut self) {
        if self.replace_selection("") {
            return;
        }
        let old = self.cursor;
        self.right();
        if self.cursor != old {
            self.record_edit(old, 0, self.text[old..self.cursor].to_owned(), old);
        }
        self.text.drain(old..self.cursor);
        if self.cursor != old {
            self.limit_reached = false;
        }
        self.cursor = old;
        self.normalize_cursor();
    }
    pub fn take(&mut self) -> String {
        self.clear_selection();
        self.cursor = 0;
        self.limit_reached = false;
        self.vertical_column = None;
        self.undo.clear();
        self.undo_bytes = 0;
        self.undo_insert_group = false;
        std::mem::take(&mut self.text)
    }

    /// Enter normal mode on the preceding grapheme without crossing a line.
    pub(crate) fn normal_cursor(&mut self) {
        self.undo_insert_group = false;
        let line_start = self.text[..self.cursor]
            .rfind('\n')
            .map_or(0, |index| index + 1);
        if self.cursor > line_start {
            self.left();
        }
    }

    pub(crate) fn normal_left(&mut self) {
        let line_start = self.text[..self.cursor]
            .rfind('\n')
            .map_or(0, |index| index + 1);
        if self.cursor > line_start {
            self.left();
        }
    }

    pub(crate) fn normal_right(&mut self) {
        let line_end = self.text[self.cursor..]
            .find('\n')
            .map_or(self.text.len(), |index| self.cursor + index);
        if self.text[self.cursor..line_end]
            .graphemes(true)
            .nth(1)
            .is_some()
        {
            self.right();
        }
    }

    pub(crate) fn clamp_normal_cursor(&mut self) {
        let line_start = self.text[..self.cursor]
            .rfind('\n')
            .map_or(0, |index| index + 1);
        let at_end = self.cursor == self.text.len() || self.text[self.cursor..].starts_with('\n');
        if at_end && self.cursor > line_start {
            self.left();
        }
    }

    /// Move to the next Unicode word or punctuation group.
    pub(crate) fn next_word(&mut self) {
        self.undo_insert_group = false;
        self.vertical_column = None;
        let mut iter = self.text[self.cursor..].grapheme_indices(true).peekable();
        let Some((_, first)) = iter.peek().copied() else {
            return;
        };
        let first_class = word_class(first);
        if first_class != 0 {
            while iter
                .peek()
                .is_some_and(|(_, value)| word_class(value) == first_class)
            {
                iter.next();
            }
        }
        while iter.peek().is_some_and(|(_, value)| word_class(value) == 0) {
            iter.next();
        }
        self.cursor += iter
            .peek()
            .map_or(self.text.len() - self.cursor, |(index, _)| *index);
    }

    /// Move to the start of the previous Unicode word or punctuation group.
    pub(crate) fn previous_word(&mut self) {
        self.undo_insert_group = false;
        self.vertical_column = None;
        let mut iter = self.text[..self.cursor]
            .grapheme_indices(true)
            .rev()
            .peekable();
        while iter.peek().is_some_and(|(_, value)| word_class(value) == 0) {
            iter.next();
        }
        if let Some((_, first)) = iter.peek().copied() {
            let class = word_class(first);
            while iter
                .peek()
                .is_some_and(|(_, value)| word_class(value) == class)
            {
                self.cursor = iter.next().expect("peeked grapheme").0;
            }
        } else {
            self.cursor = 0;
        }
    }

    fn selection(&self) -> Option<Range<usize>> {
        let anchor = self.anchor?;
        (anchor != self.cursor).then(|| anchor.min(self.cursor)..anchor.max(self.cursor))
    }

    pub(crate) fn selected_text(&self) -> Option<&str> {
        self.selection().map(|range| &self.text[range])
    }

    pub(crate) fn extend_selection(&mut self, motion: impl FnOnce(&mut Self)) {
        self.change_selection(|editor| {
            editor.anchor.get_or_insert(editor.cursor);
            editor.expansion_unit = None;
            motion(editor);
        });
    }

    fn selection_state(&self) -> SelectionState {
        SelectionState {
            anchor: self.anchor,
            cursor: self.cursor,
            expansion_unit: self.expansion_unit,
        }
    }

    fn change_selection(&mut self, select: impl FnOnce(&mut Self)) {
        let previous = self.selection_state();
        select(self);
        if self.selection_state() != previous {
            if self.selection_history.len() == MAX_SELECTION_HISTORY {
                self.selection_history.pop_front();
            }
            self.selection_history.push_back(previous);
        }
    }

    pub(crate) fn previous_selection(&mut self) {
        if let Some(previous) = self.selection_history.pop_back() {
            self.anchor = previous.anchor;
            self.cursor = previous.cursor;
            self.expansion_unit = previous.expansion_unit;
            self.vertical_column = None;
            self.undo_insert_group = false;
        }
    }

    pub(crate) fn reverse_selection(&mut self) {
        self.vertical_column = None;
        if let Some(anchor) = &mut self.anchor {
            std::mem::swap(anchor, &mut self.cursor);
        }
    }

    pub(crate) fn clear_selection(&mut self) -> bool {
        let active = self.anchor.take().is_some() || !self.selection_history.is_empty();
        self.selection_history.clear();
        self.expansion_unit = None;
        active
    }

    /// Select the current word or the next non-whitespace group.
    pub(crate) fn select_word(&mut self) {
        self.change_selection(Self::mark_word);
    }

    fn mark_word(&mut self) {
        self.expansion_unit = Some(ExpansionUnit::Word);
        self.vertical_column = None;
        self.undo_insert_group = false;
        let current = self.text[self.cursor..].graphemes(true).next();
        if let Some(class) = current.map(word_class).filter(|class| *class != 0) {
            for (start, _) in self.text[..self.cursor]
                .grapheme_indices(true)
                .rev()
                .take_while(|(_, grapheme)| word_class(grapheme) == class)
            {
                self.cursor = start;
            }
        } else {
            self.next_word();
        }
        self.anchor = Some(self.cursor);
        self.move_word_end();
    }

    pub(crate) fn select_adjacent_word(&mut self, backward: bool) {
        self.change_selection(|editor| {
            editor.anchor.get_or_insert(editor.cursor);
            editor.expansion_unit = Some(ExpansionUnit::Word);
            if backward {
                editor.previous_word();
            } else {
                editor.next_word();
                editor.move_word_end();
            }
        });
    }

    fn move_word_end(&mut self) {
        let mut graphemes = self.text[self.cursor..].graphemes(true).peekable();
        if let Some(class) = graphemes.peek().map(|grapheme| word_class(grapheme)) {
            self.cursor += graphemes
                .take_while(|grapheme| word_class(grapheme) == class)
                .map(str::len)
                .sum::<usize>();
        }
    }

    pub(crate) fn select_line(&mut self) {
        self.change_selection(|editor| match editor.anchor {
            Some(anchor)
                if editor.expansion_unit == Some(ExpansionUnit::Line) && editor.cursor < anchor =>
            {
                editor.left();
                editor.line_start();
            }
            Some(_) if editor.expansion_unit == Some(ExpansionUnit::Line) => {
                editor.right();
                editor.line_end();
            }
            _ => {
                editor.line_start();
                editor.anchor = Some(editor.cursor);
                editor.line_end();
                editor.expansion_unit = Some(ExpansionUnit::Line);
            }
        });
    }

    pub(crate) fn select_delimited(&mut self, open: char, close: char, inclusive: bool) {
        let Some((left, right)) = enclosing_delimiters(&self.text, self.cursor, open, close) else {
            return;
        };
        self.change_selection(|editor| {
            let (anchor, cursor) = if inclusive {
                (right + close.len_utf8(), left)
            } else {
                (left + open.len_utf8(), right)
            };
            editor.anchor = Some(anchor);
            editor.cursor = cursor;
            editor.expansion_unit = None;
            editor.vertical_column = None;
            editor.undo_insert_group = false;
        });
    }

    pub(crate) fn delete_line(&mut self) {
        self.clear_selection();
        self.undo_insert_group = false;
        let before = self.cursor;
        let mut start = self.text[..before].rfind('\n').map_or(0, |index| index + 1);
        let end = self.text[before..]
            .find('\n')
            .map_or(self.text.len(), |index| before + index + 1);
        if end == self.text.len() && start > 0 && (start == end || !self.text.ends_with('\n')) {
            start -= 1;
        }
        if start != end {
            self.replace_range(start..end, "");
        }
    }

    /// Replace a selection as one bounded and undoable edit.
    pub(crate) fn replace_selection(&mut self, text: &str) -> bool {
        let Some(range) = self.selection() else {
            return false;
        };
        if text.len() > MAX_DRAFT_BYTES - (self.text.len() - range.len()) {
            self.limit_reached = true;
            return true;
        }
        self.replace_range(range, text);
        true
    }

    fn replace_range(&mut self, range: Range<usize>, text: &str) {
        self.undo_insert_group = false;
        self.record_edit(
            range.start,
            text.len(),
            self.text[range.clone()].to_owned(),
            self.cursor,
        );
        self.cursor = range.start + text.len();
        self.text.replace_range(range, text);
        self.vertical_column = None;
        self.limit_reached = false;
        self.normalize_cursor();
    }

    pub(crate) fn enter_insert(&mut self, after: bool) {
        if let Some(anchor) = self.anchor {
            self.cursor = if after {
                anchor.max(self.cursor)
            } else {
                anchor.min(self.cursor)
            };
        } else if after {
            self.right();
        }
        self.clear_selection();
    }

    /// Insert a logical line and leave the cursor at its start.
    pub(crate) fn open_line(&mut self, below: bool) -> bool {
        if self.text.len() == MAX_DRAFT_BYTES {
            self.limit_reached = true;
            return false;
        }
        self.clear_selection();
        if below {
            self.line_end();
            self.insert('\n');
        } else {
            self.line_start();
            self.insert('\n');
            self.left();
        }
        true
    }

    pub(crate) fn paste(&mut self, text: &str, after: bool) {
        if text.is_empty() {
            return;
        }
        if !self.replace_selection(text) {
            if text.len() > MAX_DRAFT_BYTES - self.text.len() {
                self.limit_reached = true;
                return;
            }
            if after && !self.text[self.cursor..].starts_with('\n') {
                self.right();
            }
            self.insert_text(text);
        }
        if after && !self.limit_reached {
            self.normal_cursor();
        }
    }

    pub(crate) fn undo(&mut self) {
        self.clear_selection();
        self.undo_insert_group = false;
        let Some(edit) = self.undo.pop_back() else {
            return;
        };
        self.undo_bytes -= edit.removed.len();
        let end = edit.start + edit.inserted_bytes;
        self.text.replace_range(edit.start..end, &edit.removed);
        self.cursor = edit.cursor_before.min(self.text.len());
        self.vertical_column = None;
        self.limit_reached = false;
        self.normalize_cursor();
    }

    fn record_edit(
        &mut self,
        start: usize,
        inserted_bytes: usize,
        removed: String,
        cursor_before: usize,
    ) {
        // Selection offsets refer to this draft revision only.
        self.clear_selection();
        if self.undo_insert_group
            && removed.is_empty()
            && inserted_bytes > 0
            && let Some(previous) = self.undo.back_mut()
            && previous.removed.is_empty()
            && previous.start.saturating_add(previous.inserted_bytes) == start
        {
            previous.inserted_bytes = previous.inserted_bytes.saturating_add(inserted_bytes);
            return;
        }
        self.undo_insert_group = false;
        while self.undo.len() >= MAX_UNDO_EDITS
            || self.undo_bytes.saturating_add(removed.len()) > MAX_DRAFT_BYTES
        {
            let Some(oldest) = self.undo.pop_front() else {
                break;
            };
            self.undo_bytes -= oldest.removed.len();
        }
        self.undo_bytes += removed.len();
        self.undo.push_back(UndoEdit {
            start,
            inserted_bytes,
            removed,
            cursor_before,
        });
    }

    /// Move by one displayed row, retaining the desired cell column across
    /// short rows. No move is made when the target row does not exist.
    pub(crate) fn move_vertical(&mut self, width: u16, down: bool) {
        self.undo_insert_group = false;
        if width == 0 {
            return;
        }
        if self.is_plain_ascii() {
            self.move_vertical_ascii(width, down);
            return;
        }
        self.move_vertical_general(width, down);
    }

    fn move_vertical_ascii(&mut self, width: u16, down: bool) {
        let cells = usize::from(width);
        let (mut row, mut col) = (0usize, 0usize);
        for (index, line) in self.text[..self.cursor].split('\n').enumerate() {
            if index > 0 {
                row += 1;
                col = 0;
            }
            advance_ascii(line.len(), cells, &mut row, &mut col);
        }
        let (current_row, current_col) = visible_position(row, col, cells);
        let target = if down {
            current_row.checked_add(1)
        } else {
            current_row.checked_sub(1)
        };
        let Some(target) = target else {
            return;
        };
        let desired = match self.vertical_column {
            Some((old_width, column)) if old_width == width => column,
            _ => current_col,
        };
        let (mut base_row, mut base_byte) = (0usize, 0usize);
        let mut best: Option<(usize, usize)> = None;
        for line in self.text.split('\n') {
            if base_row > target {
                break;
            }
            let relative_row = target - base_row;
            if relative_row <= line.len() / cells {
                let start = relative_row * cells;
                let offset = start.saturating_add(desired).min(line.len());
                let candidate_col = offset - start;
                if best.is_none_or(|(_, best_col)| candidate_col >= best_col) {
                    best = Some((base_byte + offset, candidate_col));
                }
            }
            let (mut last_row, mut last_col) = (base_row, 0);
            advance_ascii(line.len(), cells, &mut last_row, &mut last_col);
            base_row = last_row + 1;
            base_byte += line.len() + 1;
        }
        if let Some((offset, _)) = best {
            self.cursor = offset;
            self.vertical_column = Some((width, desired));
        }
    }

    pub(crate) fn move_vertical_general(&mut self, width: u16, down: bool) {
        if width == 0 {
            return;
        }
        let width_usize = usize::from(width);
        let (mut row, mut col) = (0, 0);
        for grapheme in self.text[..self.cursor].graphemes(true) {
            advance(grapheme, width_usize, &mut row, &mut col);
        }
        let (current_row, current_col) = cursor_position(
            row,
            col,
            width_usize,
            self.text[self.cursor..].graphemes(true).next(),
        );
        let target = if down {
            current_row.checked_add(1)
        } else {
            current_row.checked_sub(1)
        };
        let Some(target) = target else {
            return;
        };
        let desired = match self.vertical_column {
            Some((old_width, column)) if old_width == width => column,
            _ => current_col,
        };
        let mut best: Option<(usize, usize)> = None;
        (row, col) = (0, 0);
        for (offset, grapheme) in self
            .text
            .grapheme_indices(true)
            .chain(std::iter::once((self.text.len(), "")))
        {
            let (position_row, position_col) = cursor_position(
                row,
                col,
                width_usize,
                (!grapheme.is_empty()).then_some(grapheme),
            );
            if position_row == target {
                let better = best.is_none_or(|(_, best_col)| {
                    if position_col <= desired {
                        best_col > desired || position_col >= best_col
                    } else {
                        best_col > desired && position_col < best_col
                    }
                });
                if better {
                    best = Some((offset, position_col));
                }
            } else if position_row > target {
                break;
            }
            if !grapheme.is_empty() {
                advance(grapheme, width_usize, &mut row, &mut col);
            }
        }
        if let Some((offset, _)) = best {
            self.cursor = offset;
            self.vertical_column = Some((width, desired));
        }
    }

    fn normalize_cursor(&mut self) {
        if self.cursor == 0 || self.cursor == self.text.len() {
            return;
        }
        self.cursor = self
            .text
            .grapheme_indices(true)
            .map(|(start, grapheme)| start + grapheme.len())
            .find(|end| *end >= self.cursor)
            .unwrap_or(self.text.len());
    }

    /// Measure the draft only until the available space is filled.
    pub(crate) fn preferred_rows(&self, width: u16, max_rows: u16) -> u16 {
        if width == 0 || max_rows == 0 {
            return 0;
        }
        let width = usize::from(width);
        let limit = usize::from(max_rows);
        let (mut row, mut col) = (0usize, 0usize);
        for grapheme in self.text.graphemes(true) {
            advance(grapheme, width, &mut row, &mut col);
            if row >= limit {
                return max_rows;
            }
        }
        let (row, _) = visible_position(row, col, width);
        (row + 1).min(limit) as u16
    }

    fn is_plain_ascii(&self) -> bool {
        self.text
            .bytes()
            .all(|byte| byte == b'\n' || (b' '..=b'~').contains(&byte))
    }
}

/// Find the innermost matching pair around the cursor. Work on graphemes
/// so the returned offsets remain valid even in text with combining marks.
fn enclosing_delimiters(
    text: &str,
    cursor: usize,
    open: char,
    close: char,
) -> Option<(usize, usize)> {
    let mut open_bytes = [0; 4];
    let mut close_bytes = [0; 4];
    let open: &str = open.encode_utf8(&mut open_bytes);
    let close: &str = close.encode_utf8(&mut close_bytes);
    let mut starts = Vec::new();
    let mut escaped = false;
    for (offset, grapheme) in text.grapheme_indices(true) {
        if offset > cursor && starts.is_empty() {
            break;
        }
        if std::mem::replace(&mut escaped, false) {
            continue;
        }
        if grapheme == "\\" {
            escaped = true;
        } else if grapheme == close && (open != close || !starts.is_empty()) {
            if let Some(start) = starts.pop()
                && start <= cursor
                && cursor <= offset
            {
                return Some((start, offset));
            }
        } else if grapheme == open {
            starts.push(offset);
        }
    }
    None
}

fn advance_ascii(count: usize, width: usize, row: &mut usize, col: &mut usize) {
    if count != 0 {
        let cells = *col + count;
        *row += (cells - 1) / width;
        *col = 1 + (cells - 1) % width;
    }
}

fn visible_position(row: usize, col: usize, width: usize) -> (usize, usize) {
    if col == width {
        (row + 1, 0)
    } else {
        (row, col)
    }
}

fn cursor_position(row: usize, col: usize, width: usize, next: Option<&str>) -> (usize, usize) {
    if let Some(next) = next
        && next != "\n"
        && col < width
        && col + UnicodeWidthStr::width(display_symbol(next, width)) > width
    {
        return (row + 1, 0);
    }
    visible_position(row, col, width)
}

fn display_symbol(grapheme: &str, width: usize) -> &str {
    if grapheme == "\t" {
        " "
    } else if grapheme.chars().any(char::is_control) || UnicodeWidthStr::width(grapheme) > width {
        "�"
    } else {
        grapheme
    }
}

fn advance(grapheme: &str, width: usize, row: &mut usize, col: &mut usize) {
    if grapheme == "\n" {
        *row += 1;
        *col = 0;
        return;
    }
    let symbol_width = UnicodeWidthStr::width(display_symbol(grapheme, width));
    if *col + symbol_width > width || *col == width {
        *row += 1;
        *col = 0;
    }
    *col += symbol_width;
}
