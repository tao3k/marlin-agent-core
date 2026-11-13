// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use super::composer::Composer;

#[test]
fn word_and_line_edits_undo_without_breaking_unicode_boundaries() {
    let mut composer = Composer::default();
    composer.insert_text("abc βeta\nlast");
    composer.normal_cursor();
    composer.previous_word();
    assert_eq!(&composer.text()[composer.cursor()..], "last");
    composer.previous_word();
    assert_eq!(&composer.text()[composer.cursor()..], "βeta\nlast");
    composer.delete_line();
    assert_eq!(composer.text(), "last");
    composer.undo();
    assert_eq!(composer.text(), "abc βeta\nlast");

    let mut combined = Composer::default();
    combined.insert_text("e");
    combined.insert_text("\u{301}");
    combined.undo();
    assert_eq!(combined.text(), "");
}

#[test]
fn line_boundaries_handle_unicode_empty_lines_and_wrapped_text() {
    let mut composer = Composer::default();
    composer.insert_text("first\n中e\u{301}🐳wrapped\n\nlast");
    composer.line_start();
    composer.insert_text("^");
    composer.line_end();
    composer.insert_text("$");
    composer.line_start();
    composer.left();
    composer.line_start();
    composer.line_end();
    composer.insert_text("empty");
    composer.left();
    for _ in 0..4 {
        composer.left();
    }
    composer.left();
    composer.line_start();
    composer.insert_text("[");
    composer.line_end();
    composer.insert_text("]");
    assert_eq!(
        composer.text(),
        "first\n[中e\u{301}🐳wrapped]\nempty\n^last$"
    );
}
#[test]
fn editing_preserves_grapheme_boundaries() {
    let mut c = Composer::default();
    for ch in "中e\u{301}🐳".chars() {
        c.insert(ch);
    }
    c.left();
    c.backspace();
    assert_eq!(c.text(), "中🐳");
    c.delete();
    assert_eq!(c.take(), "中");
    assert_eq!(c.cursor(), 0);
}

#[test]
fn multiline_view_tracks_cursor_and_preserves_long_draft() {
    let mut c = Composer::default();
    c.insert_text("one\ntwo\nthree\nfour");
    let view = c.view(8, 3).unwrap();
    assert_eq!(view.rows, ["two", "three", "four"]);
    assert_eq!(view.cursor, (4, 2));
    for _ in 0..4 {
        c.left();
    }
    c.left();
    let view = c.view(8, 3).unwrap();
    assert_eq!(view.rows, ["one", "two", "three"]);
    assert_eq!(view.cursor, (5, 2));
    assert_eq!(c.text(), "one\ntwo\nthree\nfour");
    assert!(c.view(0, 3).is_none());
    assert!(c.view(8, 0).is_none());
}

#[test]
fn wrapping_and_grapheme_join_keep_cursor_on_displayed_cells() {
    let mut c = Composer::default();
    c.insert_text("ab中Z");
    let view = c.view(4, 2).unwrap();
    assert_eq!(view.rows, ["ab中", "Z"]);
    assert_eq!(view.cursor, (1, 1));
    c.take();
    c.insert_text("👩👩");
    c.left();
    c.insert('\u{200d}');
    assert_eq!(c.cursor(), c.text().len());
    c.backspace();
    assert!(c.text().is_empty());
    c.insert_text("👩‍x👩");
    c.left();
    c.left();
    c.delete();
    assert_eq!(c.text(), "👩‍👩");
    assert_eq!(c.cursor(), c.text().len());
    c.take();
    c.insert_text("ab");
    c.left();
    c.left();
    c.delete();
    assert_eq!(c.cursor(), 0);
    c.take();
    c.insert_text("a\x1b中");
    let view = c.view(1, 3).unwrap();
    assert_eq!(view.rows, ["�", "�", ""]);
    assert_eq!(view.cursor, (0, 2));
}

#[test]
fn vertical_movement_retains_column_across_short_and_empty_lines() {
    let mut c = Composer::default();
    c.insert_text("abcdef\nx\n\n123456");
    c.move_vertical(20, false);
    assert_eq!(c.view(20, 5).unwrap().cursor, (0, 2));
    c.move_vertical(20, false);
    assert_eq!(c.view(20, 5).unwrap().cursor, (1, 1));
    c.move_vertical(20, false);
    assert_eq!(c.view(20, 5).unwrap().cursor, (6, 0));
    c.move_vertical(20, true);
    c.move_vertical(20, true);
    c.move_vertical(20, true);
    assert_eq!(c.view(20, 5).unwrap().cursor, (6, 3));
    c.move_vertical(20, true);
    assert_eq!(c.cursor(), c.text().len());
    c.move_vertical(20, false);
    c.left();
    c.move_vertical(20, false);
    assert_eq!(c.view(20, 5).unwrap().cursor, (1, 0));
}

#[test]
fn vertical_movement_uses_display_width_and_grapheme_boundaries() {
    let mut c = Composer::default();
    c.insert_text("abc中Z1x");
    assert_eq!(c.view(4, 4).unwrap().cursor, (1, 2));
    c.move_vertical(4, false);
    assert_eq!(c.view(4, 4).unwrap().cursor, (0, 1));
    c.move_vertical(4, false);
    assert_eq!(c.view(4, 4).unwrap().cursor, (1, 0));
    c.move_vertical(4, true);
    assert_eq!(c.view(4, 4).unwrap().cursor, (0, 1));
    c.move_vertical(0, true);
    assert_eq!(c.view(4, 4).unwrap().cursor, (0, 1));
}

#[test]
fn draft_byte_budget_rejects_whole_insertions_and_preserves_edits() {
    let mut c = Composer::default();
    let initial = "a".repeat(1024 * 1024 - 1);
    c.insert_text(&initial);
    c.insert('🐳');
    assert!(c.limit_reached());
    assert_eq!(c.text(), initial);
    assert_eq!(c.cursor(), initial.len());
    c.insert_text("bc");
    assert!(c.limit_reached());
    assert_eq!(c.text(), initial);
    c.insert('b');
    assert!(!c.limit_reached());
    assert_eq!(c.text().len(), 1024 * 1024);
    c.insert('\n');
    assert!(c.limit_reached());
    c.backspace();
    assert!(!c.limit_reached());
    assert_eq!(c.take(), initial);
    assert!(!c.limit_reached());
    c.insert_text("🐳");
    assert_eq!(c.text(), "🐳");
}

#[test]
fn ascii_layout_matches_unicode_path_for_cursors_and_narrow_views() {
    let alphabet = ['a', ' ', '\n'];
    for len in 0..=6u32 {
        for mut sequence in 0..alphabet.len().pow(len) {
            let mut input = String::new();
            for _ in 0..len {
                input.push(alphabet[sequence % alphabet.len()]);
                sequence /= alphabet.len();
            }
            let mut c = Composer::default();
            c.insert_text(&input);
            for _ in 0..=len {
                for width in 1..=5 {
                    for height in 1..=4 {
                        assert_eq!(
                            c.view(width, height),
                            c.view_general(width, height),
                            "input={input:?} cursor={} width={width} height={height}",
                            c.cursor()
                        );
                    }
                    let (mut fast, mut reference) = (c.clone(), c.clone());
                    for (direction, movement_width) in [
                        (false, width),
                        (false, width),
                        (true, width),
                        (true, width),
                        (false, width + 1),
                        (true, width),
                    ] {
                        fast.move_vertical(movement_width, direction);
                        reference.move_vertical_general(movement_width, direction);
                        assert_eq!(
                            fast.cursor(),
                            reference.cursor(),
                            "input={input:?} initial={} width={movement_width} down={direction}",
                            c.cursor()
                        );
                    }
                }
                c.left();
            }
        }
    }
}

#[test]
#[ignore = "manual matched component timing; not a latency acceptance test"]
fn composer_layout_diagnostic() {
    use std::{hint::black_box, time::Instant};
    for (name, source) in [
        ("ascii-1m", "a".repeat(1024 * 1024)),
        ("lines-256k", "some text here\n".repeat(17_000)),
        ("unicode-256k", "中🐳é".repeat(25_000)),
    ] {
        let mut c = Composer::default();
        c.insert_text(&source);
        let mut samples = Vec::new();
        for _ in 0..7 {
            let start = Instant::now();
            black_box(c.view(98, 3).unwrap());
            samples.push(start.elapsed().as_micros());
        }
        samples.sort_unstable();
        eprintln!(
            "{name} bytes={} view_median_us={} samples_us={samples:?}",
            source.len(),
            samples[3]
        );
        let mut moves = Vec::new();
        for _ in 0..7 {
            let start = Instant::now();
            c.move_vertical(98, false);
            black_box(c.cursor());
            moves.push(start.elapsed().as_micros());
        }
        moves.sort_unstable();
        eprintln!("{name} up_median_us={} samples_us={moves:?}", moves[3]);
    }
}
#[test]
fn preferred_rows_wraps_unicode_and_respects_available_space() {
    let mut composer = super::composer::Composer::default();
    assert_eq!(composer.preferred_rows(4, 8), 1);
    composer.insert_text("中文\nx\n\ny");
    assert_eq!(composer.preferred_rows(4, 8), 4);
    assert_eq!(composer.preferred_rows(4, 2), 2);
    assert_eq!(composer.preferred_rows(0, 8), 0);
}
