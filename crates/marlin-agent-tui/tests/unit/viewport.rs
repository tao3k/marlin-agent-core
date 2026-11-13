// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use super::wrap::visit_rows_counted;
use super::{
    app::safe_text,
    viewport::{
        ScrollAnchor, Transcript, TranscriptViewport, TranscriptViewportCache, transcript_from_top,
        transcript_viewport, transcript_viewport_reference,
    },
};
use proptest::{collection::vec, prelude::ProptestConfig, prop_assert_eq, proptest};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget, Wrap},
};

trait TestRender {
    fn render(&self, area: Rect, buffer: &mut Buffer);
}

impl TestRender for TranscriptViewport {
    fn render(&self, area: Rect, buffer: &mut Buffer) {
        self.render_with_role_style(
            area,
            buffer,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
    }
}

fn full_reference(
    entries: &[(String, String)],
    pending: bool,
    width: u16,
    height: u16,
    scroll_up: u16,
) -> Buffer {
    let mut lines = Vec::new();
    for (role, content) in entries {
        if !role.is_empty() {
            lines.push(Line::from(Span::styled(
                role.clone(),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )));
        }
        lines.extend(
            safe_text(content)
                .lines()
                .map(|line| Line::from(line.to_owned())),
        );
        lines.push(Line::default());
    }
    if pending {
        lines.push(Line::from("Waiting for gateway…"));
    }
    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    let scroll = paragraph
        .line_count(width)
        .saturating_sub(usize::from(height) + usize::from(scroll_up))
        .min(u16::MAX as usize) as u16;
    let area = Rect::new(0, 0, width, height);
    let mut buffer = Buffer::empty(area);
    paragraph.scroll((scroll, 0)).render(area, &mut buffer);
    buffer
}

#[test]
fn compact_transcript_omits_repeated_roles_without_losing_scroll_geometry() {
    let entries = vec![
        (String::new(), "u: hello".to_owned()),
        ("folded".to_owned(), String::new()),
        (String::new(), "answer with 中 and emoji 🐟".to_owned()),
    ];
    for (width, height, scroll) in [(12, 7, 0), (12, 4, 2), (30, 5, 0)] {
        let area = Rect::new(0, 0, width, height);
        let expected = full_reference(&entries, false, width, height, scroll);
        let view = transcript_viewport(&entries, false, width, height, usize::from(scroll));
        let mut actual = Buffer::empty(area);
        view.render(area, &mut actual);
        assert_eq!(actual, expected);
    }
}

#[test]
fn viewport_cache_invalidates_for_transcript_and_view_changes() {
    let mut transcript = Transcript::new(vec![("You".into(), "hello 中".into())]);
    let mut cache = TranscriptViewportCache::default();
    let mut check = |transcript: &Transcript, pending, width, height, scroll| {
        let expected = transcript_viewport(transcript, pending, width, height, scroll);
        cache.with_view(transcript, pending, width, height, scroll, |actual| {
            assert_eq!(actual.lines, expected.lines);
        });
    };
    check(&transcript, false, 5, 4, 0);
    check(&transcript, false, 5, 4, 0);
    transcript.append_content(0, "e\u{301}\nnew line");
    check(&transcript, false, 5, 4, 0);
    check(&transcript, true, 5, 4, 0);
    check(&transcript, true, 3, 4, 0);
    check(&transcript, true, 3, 2, 0);
    check(&transcript, true, 3, 2, 2);
    transcript.push(("Assistant".into(), "reply".into()));
    check(&transcript, true, 3, 2, 2);
    transcript.clear();
    check(&transcript, true, 3, 2, 2);
    let another = Transcript::new(vec![("Other".into(), "different".into())]);
    check(&another, true, 3, 2, 2);
    assert_eq!(cache.build_count(), 9);
}

#[test]
fn theme_changes_restyle_cached_roles_without_rebuilding_rows() {
    let transcript = Transcript::new(vec![("Role".into(), "content".into())]);
    let mut cache = TranscriptViewportCache::default();
    let area = Rect::new(0, 0, 20, 3);
    let mut first = Buffer::empty(area);
    let mut second = Buffer::empty(area);
    cache.with_view(&transcript, false, 20, 3, 0, |view| {
        view.render_with_role_style(area, &mut first, Style::default().fg(Color::Red));
    });
    cache.with_view(&transcript, false, 20, 3, 0, |view| {
        view.render_with_role_style(area, &mut second, Style::default().fg(Color::Blue));
    });
    assert_eq!(cache.build_count(), 1);
    assert_eq!(first[(0, 0)].symbol(), second[(0, 0)].symbol());
    assert_eq!(first[(0, 0)].fg, Color::Red);
    assert_eq!(second[(0, 0)].fg, Color::Blue);
}

#[test]
fn cache_does_not_confuse_distinct_transcripts_with_equal_revisions() {
    let first = Transcript::new(vec![("A".into(), "first".into())]);
    let second = Transcript::new(vec![("B".into(), "second".into())]);
    let mut cache = TranscriptViewportCache::default();
    cache.with_view(&first, false, 20, 5, 0, |_| {});
    cache.with_view(&second, false, 20, 5, 0, |view| {
        assert!(view.lines.iter().any(|line| line.to_string() == "second"));
    });
    assert_eq!(cache.build_count(), 2);
}

#[test]
fn folded_thinking_keeps_stream_content_without_rebuilding_hidden_rows() {
    let mut transcript = Transcript::new(vec![("You".into(), "question".into())]);
    transcript.push_thinking("open".into(), "closed", String::new(), true);
    let mut cache = TranscriptViewportCache::default();
    cache.with_view(&transcript, false, 30, 5, 0, |_| {});
    assert_eq!(cache.build_count(), 1);
    transcript.append_content(1, "secret\nreasoning");
    cache.with_view(&transcript, false, 30, 5, 0, |view| {
        let visible = format!("{:?}", view.lines);
        assert!(visible.contains("closed"));
        assert!(!visible.contains("secret"));
    });
    assert_eq!(cache.build_count(), 1);
    assert_eq!(transcript.content(1), "secret\nreasoning");
    assert!(transcript.toggle_thinking_from_end(1));
    assert_eq!(transcript[1].0, "open");
    cache.with_view(&transcript, false, 30, 5, 0, |view| {
        assert!(format!("{:?}", view.lines).contains("secret"));
    });
    assert_eq!(cache.build_count(), 2);
    assert!(transcript.toggle_thinking_from_end(1));
    assert_eq!(transcript[1].0, "closed");
    assert_eq!(transcript.content(1), "secret\nreasoning");
    transcript.retain_latest(1);
    assert_eq!(transcript.content(0), "secret\nreasoning");
    assert!(transcript.toggle_thinking_from_end(1));
    assert_eq!(transcript[0].1, "secret\nreasoning");
}

#[test]
fn thinking_folds_can_be_selected_independently_from_newest() {
    let mut transcript = Transcript::new(Vec::new());
    transcript.push_thinking("open".into(), "closed", "older raw".into(), true);
    transcript.push((String::new(), "answer".into()));
    transcript.push_thinking("open".into(), "closed", "newer raw".into(), true);

    assert!(!transcript.toggle_thinking_from_end(0));
    assert!(!transcript.toggle_thinking_from_end(3));
    assert!(transcript.toggle_thinking_from_end(2));
    assert_eq!(transcript[0].1, "older raw");
    assert!(transcript[2].1.is_empty());
    assert_eq!(transcript.content(2), "newer raw");
    assert!(transcript.toggle_thinking_from_end(1));
    assert_eq!(transcript[2].1, "newer raw");
    assert!(transcript.toggle_thinking_from_end(2));
    assert_eq!(transcript.content(0), "older raw");
    assert!(transcript[0].1.is_empty());
}

#[test]
fn oversized_viewport_is_rendered_but_not_retained() {
    let transcript = Transcript::new(vec![("User".into(), "hello".into())]);
    let mut cache = TranscriptViewportCache::with_budget(1);
    for _ in 0..2 {
        cache.with_view(&transcript, false, 20, 5, 0, |view| {
            assert!(view.lines.iter().any(|line| line.to_string() == "hello"));
        });
    }
    assert_eq!(cache.build_count(), 2);
}

#[test]
fn oversized_combining_grapheme_is_not_kept_in_the_cache() {
    let content = format!("a{}", "\u{301}".repeat(10_000));
    let transcript = Transcript::new(vec![("User".into(), content.clone())]);
    let mut cache = TranscriptViewportCache::with_budget(4096);
    for _ in 0..2 {
        cache.with_view(&transcript, false, 20, 4, 0, |view| {
            assert!(view.lines.iter().any(|line| line.to_string() == content));
        });
    }
    assert_eq!(cache.build_count(), 2);
    assert_eq!(transcript[0].1, content);
}

#[test]
fn streaming_control_sanitization_matches_whole_line_sanitization() {
    let source = "a\t\0\x1b\u{85}中\r\nsecond\tline\x7f";
    let original = vec![("User".into(), source.into())];
    let sanitized = vec![("User".into(), safe_text(source))];
    for width in [1, 2, 5, 20] {
        for scroll in [0, 1, 8, 100] {
            let actual = transcript_viewport(&original, false, width, 5, scroll);
            let expected = transcript_viewport(&sanitized, false, width, 5, scroll);
            assert_eq!(
                actual.lines, expected.lines,
                "width={width} scroll={scroll}"
            );
        }
    }
    assert_eq!(original[0].1, source);
}

#[test]
fn long_zero_width_runs_remain_exact_and_use_few_segments() {
    let text = "\u{200b}".repeat(200_000);
    let mut rows = Vec::new();
    let peak = visit_rows_counted(&text, 80, |row| {
        rows.push(row.concat());
        true
    });
    assert_eq!(rows, vec![text]);
    assert!(peak <= 3, "peak temporary segments: {peak}");
}

#[test]
fn zero_width_runs_preserve_visible_rows_and_wrapping() {
    let gap = "\u{200b}".repeat(10_000);
    let text = format!("alpha{gap} beta{gap}中");
    for width in [2, 5, 9, 40] {
        let mut rows = Vec::new();
        let peak = visit_rows_counted(&text, width, |row| {
            rows.push(row.concat());
            true
        });
        let mut plain = Vec::new();
        let short = "alpha\u{200b} beta\u{200b}中";
        visit_rows_counted(short, width, |row| {
            plain.push(row.concat().replace('\u{200b}', ""));
            true
        });
        assert_eq!(
            rows.iter()
                .map(|row| row.replace('\u{200b}', ""))
                .collect::<Vec<_>>(),
            plain
        );
        assert!(
            peak <= usize::from(width) * 4 + 10,
            "width={width}, peak={peak}"
        );
    }
}

#[test]
fn compacted_zero_width_rows_match_terminal_reference() {
    let samples = [
        "abc\u{200b}\u{2060} def",
        "  \u{200b}ab 中\u{2060}cd  ",
        "\u{200b}\u{2060} one two three",
        "start\u{200b}\u{2060}\u{200b} end",
    ];
    for text in samples {
        let entries = vec![("User".into(), text.into())];
        for width in [3, 5, 9, 20] {
            for scroll in [0, 1, 4, 20] {
                let expected = full_reference(&entries, false, width, 5, scroll);
                let mut actual = Buffer::empty(Rect::new(0, 0, width, 5));
                transcript_viewport(&entries, false, width, 5, usize::from(scroll))
                    .render(actual.area, &mut actual);
                assert_eq!(
                    actual, expected,
                    "text={text:?}, width={width}, scroll={scroll}"
                );
            }
        }
    }
}

#[test]
fn long_line_tail_matches_previous_layout() {
    for text in [
        "abc 中文 def  ".repeat(700),
        "é e\u{301} 👩‍💻 \u{200b}x  ".repeat(600),
        "verylongunbrokenword".repeat(400),
    ] {
        let entries = vec![("Assistant".into(), text)];
        for width in [7, 19, 80] {
            for scroll in [0_usize, 1, 5] {
                let expected = transcript_viewport_reference(&entries, false, width, 7, scroll);
                let area = Rect::new(0, 0, width, 7);
                let mut actual = Buffer::empty(area);
                transcript_viewport(&entries, false, width, 7, scroll).render(area, &mut actual);
                let mut reference = Buffer::empty(area);
                expected.render(area, &mut reference);
                assert_eq!(actual, reference, "width={width}, scroll={scroll}");
            }
        }
    }
}

#[test]
fn long_line_tail_falls_back_when_row_slice_budget_is_exceeded() {
    let entries = vec![("Assistant".into(), "abcdef ".repeat(5_000))];
    let area = Rect::new(0, 0, 100, 200);
    let expected = transcript_viewport_reference(&entries, false, area.width, area.height, 0);
    let mut actual = Buffer::empty(area);
    transcript_viewport(&entries, false, area.width, area.height, 0).render(area, &mut actual);
    let mut reference = Buffer::empty(area);
    expected.render(area, &mut reference);
    assert_eq!(actual, reference);
}

#[test]
#[ignore = "manual matched timing; not an end-to-end performance gate"]
fn compare_cached_and_uncached_unchanged_long_line() {
    use std::{hint::black_box, time::Instant};
    let transcript = Transcript::new(vec![("Assistant".into(), "中文abc ".repeat(25_000))]);
    let mut cache = TranscriptViewportCache::default();
    let expected = transcript_viewport(&transcript, false, 80, 12, 0);
    cache.with_view(&transcript, false, 80, 12, 0, |view| {
        assert_eq!(view.lines, expected.lines);
    });
    let start = Instant::now();
    for _ in 0..20 {
        black_box(transcript_viewport(&transcript, false, 80, 12, 0));
    }
    let uncached = start.elapsed();
    let start = Instant::now();
    for _ in 0..20 {
        cache.with_view(&transcript, false, 80, 12, 0, |view| {
            black_box(view);
        });
    }
    let cached = start.elapsed();
    eprintln!("20 unchanged long-line viewport layouts: uncached={uncached:?}, cached={cached:?}");
}

#[test]
#[ignore = "manual matched component timing; not an end-to-end gate"]
fn compare_long_line_tail_single_and_double_pass() {
    use std::{hint::black_box, time::Instant};
    let entries = vec![("Assistant".into(), "中文abc ".repeat(25_000))];
    let reference = transcript_viewport_reference(&entries, false, 80, 12, 0);
    let candidate = transcript_viewport(&entries, false, 80, 12, 0);
    assert_eq!(reference.lines, candidate.lines);
    let mut old_us = Vec::new();
    let mut new_us = Vec::new();
    for _ in 0..8 {
        let start = Instant::now();
        black_box(transcript_viewport_reference(
            black_box(&entries),
            false,
            80,
            12,
            0,
        ));
        old_us.push(start.elapsed().as_micros());
        let start = Instant::now();
        black_box(transcript_viewport(black_box(&entries), false, 80, 12, 0));
        new_us.push(start.elapsed().as_micros());
    }
    eprintln!(
        "long-line tail bytes={} viewport=80x12 old_us={old_us:?} new_us={new_us:?}",
        entries[0].1.len()
    );
}

#[test]
fn suffix_projection_matches_full_renderer_for_scroll_and_resize() {
    let entries = vec![
        (
            "You".into(),
            "hello long wrapped words that need spaces".into(),
        ),
        (
            "Assistant".into(),
            "中文🐳 e\u{301}\nline two\t\x1b[2J\n".into(),
        ),
        ("You".into(), String::new()),
        ("Assistant".into(), "abc\n\nxyz".into()),
    ];
    for width in [4, 9, 23, 80] {
        for height in [1, 3, 8] {
            for scroll in [0, 1, 3, 12, 80] {
                for pending in [false, true] {
                    let expected = full_reference(&entries, pending, width, height, scroll);
                    let view =
                        transcript_viewport(&entries, pending, width, height, usize::from(scroll));
                    let area = Rect::new(0, 0, width, height);
                    let mut actual = Buffer::empty(area);
                    view.render(area, &mut actual);
                    assert_eq!(
                        actual, expected,
                        "width={width} height={height} scroll={scroll} pending={pending}"
                    );
                }
            }
        }
    }
}

#[test]
fn top_origin_projection_matches_bottom_origin_and_clamps() {
    let entries: Vec<(String, String)> = vec![
        ("You".into(), "one two three four\n中文 e\u{301}\n".into()),
        ("Assistant".into(), "last line with words\nnext".into()),
    ];
    for width in [1, 4, 11, 80] {
        for height in [1, 4, 20] {
            for pending in [false, true] {
                let mut total = 0usize;
                let mut count = |text: &str| {
                    super::wrap::visit_rows(text, width, |_| {
                        total += 1;
                        true
                    })
                };
                for (role, content) in &entries {
                    count(role);
                    for line in content.split_terminator('\n') {
                        count(line);
                    }
                    count("");
                }
                if pending {
                    count("Waiting for gateway…");
                }
                let max = total.saturating_sub(usize::from(height));
                for offset in [0, 1, 5, 1000, usize::MAX] {
                    let top = transcript_from_top(&entries, pending, width, height, offset);
                    let actual_offset = offset.min(max);
                    assert_eq!(
                        top.scroll_up, actual_offset,
                        "width={width} height={height} offset={offset} pending={pending}"
                    );
                    let bottom =
                        transcript_viewport(&entries, pending, width, height, max - actual_offset);
                    assert_eq!(
                        top.lines, bottom.lines,
                        "width={width} height={height} offset={offset} pending={pending}"
                    );
                }
            }
        }
    }
}

#[test]
fn viewport_cache_distinguishes_top_and_bottom_origins() {
    let transcript = Transcript::new(vec![(
        "You".into(),
        (0..30).map(|i| format!("row {i}\n")).collect(),
    )]);
    let mut cache = TranscriptViewportCache::default();
    let first = cache.with_top_view(&transcript, 20, 5, 0, |view| view.lines.clone());
    let last = cache.with_view(&transcript, false, 20, 5, 0, |view| view.lines.clone());
    assert_ne!(first, last);
    cache.with_top_view(&transcript, 20, 5, 0, |view| assert_eq!(view.lines, first));
    assert_eq!(cache.build_count(), 3);
}

#[test]
#[ignore = "manual component diagnostic; not an end-to-end performance gate"]
fn compare_workspace_top_projection_with_reverse_scan() {
    use std::{hint::black_box, time::Instant};
    let entries = vec![("Workspace".to_string(), "abc中 def ".repeat(80_000))];
    let old = transcript_viewport(&entries, false, 80, 12, usize::MAX);
    let new = transcript_from_top(&entries, false, 80, 12, 0);
    assert_eq!(old.lines, new.lines);
    let mut reverse_us = Vec::new();
    let mut forward_us = Vec::new();
    for _ in 0..5 {
        let start = Instant::now();
        black_box(transcript_viewport(
            black_box(&entries),
            false,
            80,
            12,
            usize::MAX,
        ));
        reverse_us.push(start.elapsed().as_micros());
        let start = Instant::now();
        black_box(transcript_from_top(black_box(&entries), false, 80, 12, 0));
        forward_us.push(start.elapsed().as_micros());
    }
    eprintln!(
        "workspace top component, bytes={}, reverse_us={reverse_us:?}, forward_us={forward_us:?}",
        entries[0].1.len()
    );
}

#[test]
fn scroll_offset_is_clamped_to_the_actual_top_after_layout_and_resize() {
    let entries: Vec<(String, String)> = vec![
        ("You".into(), "one two three four\nsecond".into()),
        ("Assistant".into(), "中文 and more words".into()),
    ];
    for width in [1, 4, 12, 80] {
        for height in [1, 4, 20] {
            for pending in [false, true] {
                let mut total = 0usize;
                let mut count = |text: &str| {
                    super::wrap::visit_rows(text, width, |_| {
                        total += 1;
                        true
                    });
                };
                for (role, content) in &entries {
                    count(role);
                    for line in content.split_terminator('\n') {
                        count(line);
                    }
                    count("");
                }
                if pending {
                    count("Waiting for gateway…");
                }
                let maximum = total.saturating_sub(usize::from(height));
                for requested in [0, 1, 5, 10, 1000, usize::MAX] {
                    let view = transcript_viewport(&entries, pending, width, height, requested);
                    assert_eq!(
                        view.scroll_up,
                        requested.min(maximum),
                        "width={width} height={height} pending={pending} requested={requested}"
                    );
                }
            }
        }
    }
}

#[test]
fn tail_output_and_terminal_transition_preserve_scrolled_rows() {
    let history = (0..40)
        .map(|i| format!("old row {i}\n"))
        .collect::<String>();
    let mut transcript = Transcript::new(vec![
        ("You".into(), history),
        ("Assistant".into(), "initial".into()),
    ]);
    let mut anchor = ScrollAnchor::default();
    let width = 30;
    let height = 4;
    let mut scroll = anchor.adjust(&transcript, true, width, height, 10);
    let before = transcript_viewport(&transcript, true, width, height, scroll).lines;
    transcript.append_content(1, "\none\ntwo\nthree\n");
    scroll = anchor.adjust(&transcript, true, width, height, scroll);
    assert_eq!(
        transcript_viewport(&transcript, true, width, height, scroll).lines,
        before
    );
    scroll = anchor.adjust(&transcript, false, width, height, scroll);
    assert_eq!(
        transcript_viewport(&transcript, false, width, height, scroll).lines,
        before
    );
    transcript.push(("Status".into(), "done".into()));
    scroll = anchor.adjust(&transcript, false, width, height, scroll);
    assert_eq!(
        transcript_viewport(&transcript, false, width, height, scroll).lines,
        before
    );
    assert_eq!(anchor.adjust(&transcript, false, width, height, 0), 0);
}

#[test]
fn anchor_resets_for_resize_clear_and_non_tail_mutation() {
    let mut transcript = Transcript::new(vec![
        ("A".into(), "first".into()),
        ("B".into(), "last".into()),
    ]);
    let mut anchor = ScrollAnchor::default();
    assert_eq!(anchor.adjust(&transcript, true, 20, 5, 5), 5);
    transcript.append_content(0, "\nchanged");
    assert_eq!(anchor.adjust(&transcript, true, 20, 5, 5), 5);
    transcript.append_content(1, "\nchanged");
    assert_eq!(anchor.adjust(&transcript, true, 10, 5, 5), 5);
    transcript.clear();
    transcript.push(("C".into(), "new".into()));
    assert_eq!(anchor.adjust(&transcript, true, 10, 5, 5), 5);
}

#[test]
fn incremental_tail_counts_match_full_layout_across_chunk_boundaries() {
    for width in [1, 3, 8, 30] {
        let mut transcript = Transcript::new(vec![("Assistant".into(), String::new())]);
        let mut anchor = ScrollAnchor::default();
        let mut scroll = anchor.adjust(&transcript, true, width, 5, 10_000);
        let count = |text: &str| {
            let mut total = 0;
            for line in text.split_terminator('\n') {
                super::wrap::visit_rows(line, width, |_| {
                    total += 1;
                    true
                });
            }
            total
        };
        for chunk in [
            "hello ", "世", "界\n", "\n", "ab\r", "\n中", "e", "\u{301}", " z\n", "last",
        ] {
            let before = count(&transcript[0].1);
            transcript.append_content(0, chunk);
            let after = count(&transcript[0].1);
            let expected = scroll + after - before;
            scroll = anchor.adjust(&transcript, true, width, 5, scroll);
            assert_eq!(scroll, expected, "width={width}, chunk={chunk:?}");
        }
    }
}

#[path = "viewport/deep_scroll.rs"]
mod deep_scroll;
