// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Viewport layout for the native editable text value.

use marlin_agent_ui::UiKeyBinding;
use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use super::{
    Composer, ComposerView, advance, advance_ascii, cursor_position, display_symbol,
    expansion::{MarkLayout, OffsetMark},
    visible_position,
};

impl Composer {
    /// Layout the cursor's visible rows without retaining the entire wrapped draft.
    pub(crate) fn view(&self, width: u16, height: u16) -> Option<ComposerView> {
        self.view_with_marks(width, height, &[])
    }

    pub(crate) fn view_with_expansion(
        &self,
        width: u16,
        height: u16,
        bindings: &[UiKeyBinding],
    ) -> Option<ComposerView> {
        self.view_with_marks(width, height, &self.expansion_marks(bindings))
    }

    fn view_with_marks(
        &self,
        width: u16,
        height: u16,
        marks: &[OffsetMark],
    ) -> Option<ComposerView> {
        if width == 0 || height == 0 {
            return None;
        }
        if self.is_plain_ascii() {
            return Some(self.ascii_view(usize::from(width), usize::from(height), marks));
        }
        self.layout_general(width, height, marks)
    }

    fn ascii_view(&self, width: usize, height: usize, marks: &[OffsetMark]) -> ComposerView {
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
        let selected = self.selection();
        let mut selection = Vec::new();
        let mut marks = MarkLayout::new(marks);
        let mut line_offset = 0;
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
                    let visible_row = row - first_row;
                    rows[visible_row].push_str(&line[offset..offset + take]);
                    let byte_start = line_offset + offset;
                    marks.span(byte_start..byte_start + take, visible_row, col, 1);
                    if let Some(range) = &selected {
                        let start = range.start.max(byte_start);
                        let end = range.end.min(byte_start + take);
                        if start < end {
                            selection.push((
                                visible_row as u16,
                                (col + start - byte_start) as u16..(col + end - byte_start) as u16,
                            ));
                        }
                    }
                }
                offset += take;
                col += take;
            }
            if row >= first_row {
                let end = line_offset + line.len();
                marks.span(end..end + 1, row - first_row, col.min(width - 1), 1);
            }
            line_offset += line.len() + 1;
        }
        ComposerView {
            rows,
            cursor: (cursor_col as u16, (cursor_row - first_row) as u16),
            selection,
            marks: marks.visible,
        }
    }

    #[cfg(test)]
    pub(crate) fn view_general(&self, width: u16, height: u16) -> Option<ComposerView> {
        self.layout_general(width, height, &[])
    }

    fn layout_general(
        &self,
        width: u16,
        height: u16,
        marks: &[OffsetMark],
    ) -> Option<ComposerView> {
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
        let selected = self.selection();
        let mut selection: Vec<(u16, Range<u16>)> = Vec::new();
        let mut marks = MarkLayout::new(marks);
        let mut last_width = 1;
        for (offset, grapheme) in self.text.grapheme_indices(true) {
            if grapheme == "\n" {
                if row >= first_row {
                    let cells = if col == width { last_width } else { 1 };
                    marks.span(
                        offset..offset + 1,
                        row - first_row,
                        col.min(width - cells),
                        cells,
                    );
                }
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
                let visible_row = row - first_row;
                rows[visible_row].push_str(symbol);
                marks.span(offset..offset + 1, visible_row, col, symbol_width);
                if symbol_width > 0
                    && selected
                        .as_ref()
                        .is_some_and(|range| range.contains(&offset))
                {
                    let line = visible_row as u16;
                    let end = (col + symbol_width) as u16;
                    if let Some((_, range)) = selection.last_mut().filter(|(y, _)| *y == line) {
                        range.end = end;
                    } else {
                        selection.push((line, col as u16..end));
                    }
                }
            }
            if symbol_width > 0 {
                last_width = symbol_width;
            }
            col += symbol_width;
        }
        if (first_row..end_row).contains(&row) {
            let end = self.text.len();
            let cells = if col == width { last_width } else { 1 };
            marks.span(end..end + 1, row - first_row, col.min(width - cells), cells);
        }
        Some(ComposerView {
            rows,
            cursor: (cursor_col as u16, (cursor_row - first_row) as u16),
            selection,
            marks: marks.visible,
        })
    }
}
