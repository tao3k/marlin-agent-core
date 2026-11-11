// SPDX-FileCopyrightText: 2016-2022 Florian Dehau
// SPDX-FileCopyrightText: 2023-2025 The Ratatui Developers
// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Streaming terminal row layout, independent of the terminal renderer.
//!
//! The word-boundary and whitespace rules are adapted from Ratatui Widgets
//! 0.3.2's `reflow::WordWrapper`; see `THIRD_PARTY_NOTICES.org`.

use std::collections::VecDeque;

use ratatui::{buffer::CellWidth, style::Style, text::StyledGrapheme};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone, Copy)]
struct Segment<'a> {
    text: &'a str,
    width: usize,
}

fn join_zero_width<'a>(last: &mut Segment<'a>, item: Segment<'a>, source: &'a str) -> bool {
    // Only adjacent borrowed source slices are eligible. `get` validates both
    // the bounds and UTF-8 boundaries before a merged slice is constructed.
    if last.width != 0
        || item.width != 0
        || last.text.as_ptr().wrapping_add(last.text.len()) != item.text.as_ptr()
    {
        return false;
    }
    let Some(start) = last
        .text
        .as_ptr()
        .addr()
        .checked_sub(source.as_ptr().addr())
    else {
        return false;
    };
    let Some(end) = start
        .checked_add(last.text.len())
        .and_then(|end| end.checked_add(item.text.len()))
    else {
        return false;
    };
    let Some(joined) = source.get(start..end) else {
        return false;
    };
    last.text = joined;
    true
}

fn push_compact<'a>(target: &mut Vec<Segment<'a>>, item: Segment<'a>, source: &'a str) {
    if item.width == 0
        && let Some(last) = target.last_mut()
        && join_zero_width(last, item, source)
    {
        return;
    }
    target.push(item);
}

fn append_compact<'a>(
    target: &mut Vec<Segment<'a>>,
    items: impl IntoIterator<Item = Segment<'a>>,
    source: &'a str,
) {
    let mut items = items.into_iter();
    if let Some(first) = items.next() {
        push_compact(target, first, source);
    }
    // Each source buffer is already compact; only its boundary can merge.
    target.extend(items);
}

fn push_whitespace<'a>(target: &mut VecDeque<Segment<'a>>, item: Segment<'a>, source: &'a str) {
    if item.width == 0
        && let Some(last) = target.back_mut()
        && join_zero_width(last, item, source)
    {
        return;
    }
    target.push_back(item);
}

/// Visit each rendered row without retaining the full wrapped paragraph.
/// Returning false from `visit` stops further iteration.
pub(crate) fn visit_rows<'a>(text: &'a str, width: u16, mut visit: impl FnMut(&[&'a str]) -> bool) {
    visit_rows_inner(text, width, &mut visit, |_| {});
}

#[cfg(test)]
#[allow(dead_code, reason = "used by the external unit-test harness")]
pub(crate) fn visit_rows_counted<'a>(
    text: &'a str,
    width: u16,
    mut visit: impl FnMut(&[&'a str]) -> bool,
) -> usize {
    let mut peak = 0;
    visit_rows_inner(text, width, &mut visit, |count| peak = peak.max(count));
    peak
}

fn visit_rows_inner<'a>(
    text: &'a str,
    width: u16,
    visit: &mut impl FnMut(&[&'a str]) -> bool,
    mut observe: impl FnMut(usize),
) {
    if width == 0 {
        return;
    }
    let limit = usize::from(width);
    let mut line: Vec<Segment<'a>> = Vec::new();
    let mut word: Vec<Segment<'a>> = Vec::new();
    let mut whitespace: VecDeque<Segment<'a>> = VecDeque::new();
    let mut rendered_row: Vec<&str> = Vec::new();
    let (mut line_width, mut word_width, mut whitespace_width) = (0usize, 0usize, 0usize);
    let mut previous_non_whitespace = false;
    let mut emitted = false;
    for original in text.graphemes(true) {
        // Unicode grapheme segmentation isolates controls (apart from CRLF,
        // which the caller separates at the newline). Sanitize each cluster
        // before layout without copying a whole provider-controlled line.
        let grapheme = match original {
            "\t" => " ",
            value if value.chars().any(char::is_control) => "�",
            value => value,
        };
        // The terminal cannot fit a wide grapheme in a one-cell viewport.
        // Render an explicit placeholder instead of silently dropping it.
        let width = usize::from(grapheme.cell_width());
        let (grapheme, symbol_width) = if width > limit {
            ("�", 1)
        } else {
            (grapheme, width)
        };
        let is_whitespace = StyledGrapheme::new(grapheme, Style::default()).is_whitespace();
        let item = Segment {
            text: grapheme,
            width: symbol_width,
        };
        let word_found = previous_non_whitespace && is_whitespace;
        let untrimmed_overflow =
            line.is_empty() && word_width + whitespace_width + symbol_width > limit;
        if word_found || untrimmed_overflow {
            append_compact(&mut line, whitespace.drain(..), text);
            line_width += whitespace_width;
            append_compact(&mut line, word.drain(..), text);
            line_width += word_width;
            whitespace_width = 0;
            word_width = 0;
        }
        let line_full = line_width >= limit;
        let word_overflow = symbol_width > 0 && line_width + whitespace_width + word_width >= limit;
        if line_full || word_overflow {
            let mut remaining = limit.saturating_sub(line_width);
            emitted = true;
            if !visit_bounded(&line, limit, &mut rendered_row, visit) {
                return;
            }
            line.clear();
            line_width = 0;
            while let Some(g) = whitespace.front() {
                let w = g.width;
                if w > remaining {
                    break;
                }
                whitespace_width -= w;
                remaining -= w;
                whitespace.pop_front();
            }
            if is_whitespace && whitespace.is_empty() {
                continue;
            }
        }
        if is_whitespace {
            whitespace_width += symbol_width;
            push_whitespace(&mut whitespace, item, text);
        } else {
            word_width += symbol_width;
            push_compact(&mut word, item, text);
        }
        previous_non_whitespace = !is_whitespace;
        observe(line.len() + word.len() + whitespace.len());
    }
    append_compact(&mut line, whitespace, text);
    append_compact(&mut line, word, text);
    observe(line.len());
    if !line.is_empty() || !emitted {
        let _ = visit_bounded(&line, limit, &mut rendered_row, visit);
    }
}

// Guard against oversized rows from the upstream word-boundary state machine.
fn visit_bounded<'a>(
    row: &[Segment<'a>],
    limit: usize,
    parts: &mut Vec<&'a str>,
    visit: &mut impl FnMut(&[&'a str]) -> bool,
) -> bool {
    let mut start = 0;
    let mut width = 0usize;
    for (index, symbol) in row.iter().enumerate() {
        let symbol_width = symbol.width;
        if width + symbol_width > limit {
            parts.clear();
            parts.extend(row[start..index].iter().map(|item| item.text));
            if !visit(parts) {
                return false;
            }
            start = index;
            width = 0;
        }
        width += symbol_width;
    }
    parts.clear();
    parts.extend(row[start..].iter().map(|item| item.text));
    visit(parts)
}
