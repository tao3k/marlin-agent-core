// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! A grapheme-aware multiline composer.

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

pub(crate) const MAX_DRAFT_BYTES: usize = 1024 * 1024;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ComposerView {
    pub(crate) rows: Vec<String>,
    pub(crate) cursor: (u16, u16),
}

/// Editable multiline input with grapheme-aware cursor movement.
#[derive(Clone, Default)]
pub struct Composer {
    text: String,
    cursor: usize,
    vertical_column: Option<(u16, usize)>,
    limit_reached: bool,
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
    #[allow(dead_code, reason = "used by the external unit-test harness")]
    pub fn cursor(&self) -> usize {
        self.cursor
    }
    pub fn insert(&mut self, c: char) {
        if c.len_utf8() > MAX_DRAFT_BYTES.saturating_sub(self.text.len()) {
            self.limit_reached = true;
            return;
        }
        self.limit_reached = false;
        self.vertical_column = None;
        self.text.insert(self.cursor, c);
        self.cursor += c.len_utf8();
        self.normalize_cursor();
    }
    pub fn insert_text(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        if text.len() > MAX_DRAFT_BYTES.saturating_sub(self.text.len()) {
            self.limit_reached = true;
            return;
        }
        self.limit_reached = false;
        self.vertical_column = None;
        self.text.insert_str(self.cursor, text);
        self.cursor += text.len();
        self.normalize_cursor();
    }
    pub fn left(&mut self) {
        self.vertical_column = None;
        self.cursor = self.text[..self.cursor]
            .grapheme_indices(true)
            .next_back()
            .map_or(0, |(i, _)| i);
    }
    pub fn right(&mut self) {
        self.vertical_column = None;
        if let Some(g) = self.text[self.cursor..].graphemes(true).next() {
            self.cursor += g.len();
        }
    }

    /// Move to the start of the current logical line, independent of wrapping.
    pub fn line_start(&mut self) {
        self.vertical_column = None;
        self.cursor = self.text[..self.cursor].rfind('\n').map_or(0, |i| i + 1);
    }

    /// Move to the end of the current logical line, before its newline.
    pub fn line_end(&mut self) {
        self.vertical_column = None;
        self.cursor = self.text[self.cursor..]
            .find('\n')
            .map_or(self.text.len(), |i| self.cursor + i);
    }
    pub fn backspace(&mut self) {
        let old = self.cursor;
        self.left();
        self.text.drain(self.cursor..old);
        if self.cursor != old {
            self.limit_reached = false;
        }
        self.normalize_cursor();
    }
    pub fn delete(&mut self) {
        let old = self.cursor;
        self.right();
        self.text.drain(old..self.cursor);
        if self.cursor != old {
            self.limit_reached = false;
        }
        self.cursor = old;
        self.normalize_cursor();
    }
    pub fn take(&mut self) -> String {
        self.cursor = 0;
        self.limit_reached = false;
        self.vertical_column = None;
        std::mem::take(&mut self.text)
    }

    /// Move by one displayed row, retaining the desired cell column across
    /// short rows. No move is made when the target row does not exist.
    pub(crate) fn move_vertical(&mut self, width: u16, down: bool) {
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

    /// Layout the cursor's visible rows without retaining the entire wrapped draft.
    pub(crate) fn view(&self, width: u16, height: u16) -> Option<ComposerView> {
        if width == 0 || height == 0 {
            return None;
        }
        if self.is_plain_ascii() {
            return Some(self.ascii_view(usize::from(width), usize::from(height)));
        }
        self.view_general(width, height)
    }

    fn is_plain_ascii(&self) -> bool {
        self.text
            .bytes()
            .all(|byte| byte == b'\n' || (b' '..=b'~').contains(&byte))
    }

    fn ascii_view(&self, width: usize, height: usize) -> ComposerView {
        let (mut row, mut col) = (0usize, 0usize);
        for (index, line) in self.text[..self.cursor].split('\n').enumerate() {
            if index > 0 {
                row += 1;
                col = 0;
            }
            advance_ascii(line.len(), width, &mut row, &mut col);
        }
        let (cursor_row, cursor_col) = visible_position(row, col, width);
        let first_row = cursor_row.saturating_sub(height - 1);
        let end_row = first_row + height;
        let mut rows = vec![String::new(); height];
        (row, col) = (0, 0);
        'lines: for (index, line) in self.text.split('\n').enumerate() {
            if index > 0 {
                row += 1;
                col = 0;
            }
            if row >= end_row {
                break;
            }
            let mut offset = 0;
            if row < first_row {
                let skip = ((first_row - row).saturating_mul(width))
                    .saturating_sub(col)
                    .min(line.len());
                advance_ascii(skip, width, &mut row, &mut col);
                offset = skip;
            }
            while offset < line.len() {
                if col == width {
                    row += 1;
                    col = 0;
                }
                if row >= end_row {
                    break 'lines;
                }
                let take = (width - col).min(line.len() - offset);
                if row >= first_row {
                    rows[row - first_row].push_str(&line[offset..offset + take]);
                }
                offset += take;
                col += take;
            }
        }
        ComposerView {
            rows,
            cursor: (cursor_col as u16, (cursor_row - first_row) as u16),
        }
    }

    pub(crate) fn view_general(&self, width: u16, height: u16) -> Option<ComposerView> {
        if width == 0 || height == 0 {
            return None;
        }
        let width = usize::from(width);
        let (mut row, mut col) = (0usize, 0usize);
        for grapheme in self.text[..self.cursor].graphemes(true) {
            advance(grapheme, width, &mut row, &mut col);
        }
        let (cursor_row, cursor_col) = cursor_position(
            row,
            col,
            width,
            self.text[self.cursor..].graphemes(true).next(),
        );
        let first_row = cursor_row.saturating_sub(usize::from(height) - 1);
        let end_row = first_row + usize::from(height);
        let mut rows = vec![String::new(); usize::from(height)];
        (row, col) = (0, 0);
        for grapheme in self.text.graphemes(true) {
            if grapheme == "\n" {
                advance(grapheme, width, &mut row, &mut col);
                if row >= end_row {
                    break;
                }
                continue;
            }
            let symbol = display_symbol(grapheme, width);
            let symbol_width = UnicodeWidthStr::width(symbol);
            if col + symbol_width > width || col == width {
                row += 1;
                col = 0;
            }
            if row >= end_row {
                break;
            }
            if row >= first_row {
                rows[row - first_row].push_str(symbol);
            }
            col += symbol_width;
        }
        Some(ComposerView {
            rows,
            cursor: (cursor_col as u16, (cursor_row - first_row) as u16),
        })
    }
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
