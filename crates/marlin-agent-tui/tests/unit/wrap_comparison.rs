// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Frozen pre-compaction layout for differential checks and matched diagnostics.
use std::collections::VecDeque;

use super::wrap::visit_rows;
use ratatui::{buffer::CellWidth, style::Style, text::StyledGrapheme};
use unicode_segmentation::UnicodeSegmentation;

fn previous_rows<'a>(text: &'a str, width: u16, mut visit: impl FnMut(&[&'a str]) -> bool) {
    if width == 0 {
        return;
    }
    let limit = usize::from(width);
    let mut line = Vec::new();
    let mut word = Vec::new();
    let mut whitespace: VecDeque<&str> = VecDeque::new();
    let (mut line_width, mut word_width, mut whitespace_width) = (0usize, 0usize, 0usize);
    let mut previous_non_whitespace = false;
    let mut emitted = false;
    for original in text.graphemes(true) {
        let grapheme = match original {
            "\t" => " ",
            value if value.chars().any(char::is_control) => "�",
            value => value,
        };
        let grapheme = if usize::from(grapheme.cell_width()) > limit {
            "�"
        } else {
            grapheme
        };
        let is_whitespace = StyledGrapheme::new(grapheme, Style::default()).is_whitespace();
        let symbol_width = usize::from(grapheme.cell_width());
        let word_found = previous_non_whitespace && is_whitespace;
        let untrimmed_overflow =
            line.is_empty() && word_width + whitespace_width + symbol_width > limit;
        if word_found || untrimmed_overflow {
            line.extend(whitespace.drain(..));
            line_width += whitespace_width;
            line.append(&mut word);
            line_width += word_width;
            whitespace_width = 0;
            word_width = 0;
        }
        let line_full = line_width >= limit;
        let word_overflow = symbol_width > 0 && line_width + whitespace_width + word_width >= limit;
        if line_full || word_overflow {
            let mut remaining = limit.saturating_sub(line_width);
            emitted = true;
            if !previous_bounded(&line, limit, &mut visit) {
                return;
            }
            line.clear();
            line_width = 0;
            while let Some(g) = whitespace.front() {
                let w = usize::from(g.cell_width());
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
            whitespace.push_back(grapheme);
        } else {
            word_width += symbol_width;
            word.push(grapheme);
        }
        previous_non_whitespace = !is_whitespace;
    }
    line.extend(whitespace);
    line.append(&mut word);
    if !line.is_empty() || !emitted {
        let _ = previous_bounded(&line, limit, &mut visit);
    }
}

fn previous_bounded<'a>(
    row: &[&'a str],
    limit: usize,
    visit: &mut impl FnMut(&[&'a str]) -> bool,
) -> bool {
    let mut start = 0;
    let mut width = 0;
    for (index, symbol) in row.iter().enumerate() {
        let symbol_width = usize::from(symbol.cell_width());
        if width + symbol_width > limit {
            if !visit(&row[start..index]) {
                return false;
            }
            start = index;
            width = 0;
        }
        width += symbol_width;
    }
    visit(&row[start..])
}

fn rows(text: &str, width: u16, current: bool) -> Vec<String> {
    let mut output = Vec::new();
    let mut collect = |row: &[&str]| {
        output.push(row.concat());
        true
    };
    if current {
        visit_rows(text, width, &mut collect);
    } else {
        previous_rows(text, width, &mut collect);
    }
    output
}

#[test]
fn compacted_layout_matches_frozen_previous_layout() {
    let alphabet = [
        "x", "中", " ", "\t", "\u{200b}", "\u{2060}", "🐳", "e\u{301}",
    ];
    let mut seed = 981u64;
    for _ in 0..200 {
        let mut text = String::new();
        for _ in 0..100 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            text.push_str(alphabet[(seed >> 32) as usize % alphabet.len()]);
        }
        for width in [1, 2, 5, 13, 40] {
            assert_eq!(
                rows(&text, width, true),
                rows(&text, width, false),
                "width={width}, text={text:?}"
            );
        }
    }
}

#[test]
#[ignore = "manual paired component diagnostic, not an end-to-end gate"]
fn paired_previous_and_compacted_layout() {
    use std::{hint::black_box, time::Instant};
    let workloads = [
        ("mixed", "中文abc ".repeat(25_000)),
        ("ascii", "a short paragraph with words ".repeat(5_000)),
        ("zero", "a\u{200b}\u{2060}".repeat(25_000)),
    ];
    for (name, text) in workloads {
        assert_eq!(rows(&text, 80, true), rows(&text, 80, false));
        let mut previous = Vec::new();
        let mut current = Vec::new();
        for iteration in 0..8 {
            for is_current in [iteration % 2 == 0, iteration % 2 != 0] {
                let mut amount = 0usize;
                let start = Instant::now();
                let mut consume = |row: &[&str]| {
                    amount += row.iter().map(|part| part.len()).sum::<usize>();
                    true
                };
                if is_current {
                    visit_rows(black_box(&text), 80, &mut consume);
                } else {
                    previous_rows(black_box(&text), 80, &mut consume);
                }
                black_box(amount);
                let elapsed = start.elapsed().as_micros();
                if is_current {
                    current.push(elapsed);
                } else {
                    previous.push(elapsed);
                }
            }
        }
        eprintln!("{name} previous_us={previous:?} compacted_us={current:?}");
    }
}
