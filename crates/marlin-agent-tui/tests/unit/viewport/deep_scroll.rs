// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use super::{
    Buffer, Color, Line, Modifier, Paragraph, ProptestConfig, Rect, Span, Style, TestRender,
    Transcript, TranscriptViewportCache, Widget, Wrap, full_reference, prop_assert_eq, proptest,
    safe_text, transcript_viewport, transcript_viewport_reference, vec,
};

#[test]
fn one_column_marks_a_wide_character_instead_of_dropping_it() {
    let entries = vec![("User".into(), "中".into())];
    let view = transcript_viewport(&entries, false, 1, 3, 0);
    assert!(view.lines.iter().any(|line| line.to_string() == "�"));
    assert_eq!(entries[0].1, "中");
}

#[test]
fn overscrolling_pending_message_preserves_its_wrapping() {
    let entries = Vec::new();
    let area = Rect::new(0, 0, 3, 3);
    let expected = full_reference(&entries, true, 3, 3, 100);
    let mut actual = Buffer::empty(area);
    transcript_viewport(&entries, true, 3, 3, 100).render(area, &mut actual);
    assert_eq!(actual, expected);
}

#[test]
fn scrolls_beyond_u16_without_materializing_skipped_history() {
    let entries: Vec<_> = (0..100_000)
        .map(|index| ("User".into(), format!("entry {index}")))
        .collect();
    let view = transcript_viewport(&entries, false, 80, 12, 200_000);
    assert_eq!(view.lines.len(), 12);
    assert_eq!(view.lines[0].to_string(), "entry 33329");
    let top = transcript_viewport(&entries, false, 80, 12, usize::MAX);
    assert_eq!(top.lines[0].to_string(), "User");
    assert_eq!(top.lines[1].to_string(), "entry 0");
    assert_eq!(top.lines.len(), 12);
}

#[test]
fn a_single_line_larger_than_u16_rows_reaches_its_tail_and_middle() {
    let mut content = "a".repeat(75_000);
    content.push('Z');
    content.push_str(&"a".repeat(75_000));
    let entries = vec![("User".into(), content)];
    let bottom = transcript_viewport(&entries, false, 1, 3, 0);
    assert_eq!(
        bottom
            .lines
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        vec!["a", "a", ""]
    );
    let middle = transcript_viewport(&entries, false, 1, 3, 75_000);
    assert_eq!(
        middle
            .lines
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        vec!["a", "Z", "a"]
    );
}

#[test]
fn deep_scroll_preserves_wide_unicode_graphemes() {
    let mut content = "中".repeat(70_000);
    content.push('尾');
    content.push_str(&"中".repeat(70_000));
    let entries = vec![("User".into(), content)];
    let middle = transcript_viewport(&entries, false, 2, 3, 70_000);
    assert_eq!(
        middle
            .lines
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        vec!["中", "尾", "中"]
    );
}

#[test]
fn mixed_unicode_renders_safely_in_narrow_views() {
    let alphabet = [
        "a", " ", "中", "e\u{301}", "🐳", "\u{00a0}", "\u{200b}", "ｶﾞ",
    ];
    let mut seed = 0x1234_5678u64;
    for _ in 0..100 {
        let mut text = String::new();
        for _ in 0..40 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            text.push_str(alphabet[(seed >> 32) as usize % alphabet.len()]);
        }
        let entries = vec![("User".into(), text)];
        for width in [1, 2, 3, 5, 12] {
            for scroll in [0, 1, 7, 30] {
                let area = Rect::new(0, 0, width, 5);
                let view = transcript_viewport(&entries, false, width, 5, scroll);
                let mut actual = Buffer::empty(area);
                view.render(area, &mut actual);
            }
        }
    }
}

#[test]
fn known_ratatui_narrow_unicode_failure_does_not_crash_our_view() {
    let text = "🐳🐳aa \u{a0}中\u{a0}中a🐳e\u{301}\u{a0}🐳 中 \u{a0} 🐳e\u{301}\u{200b}\u{a0}中e\u{301} e\u{301}\u{a0}中e\u{301}e\u{301} \u{200b}a中a\u{200b}\u{200b}\u{a0}\u{200b}";
    let entries = vec![("User".into(), text.into())];
    // Ratatui 0.30.2's word wrapper panics on this fixture; keep it as a
    // negative control while the dependency is pinned to this version.
    assert!(std::panic::catch_unwind(|| full_reference(&entries, false, 2, 5, 0)).is_err());
    let view = transcript_viewport(&entries, false, 2, 5, 0);
    let area = Rect::new(0, 0, 2, 5);
    let mut actual = Buffer::empty(area);
    view.render(area, &mut actual);
}

#[test]
fn bottom_view_does_not_materialize_a_long_history() {
    let entries: Vec<_> = (0..100_000)
        .map(|index| ("User".into(), format!("entry {index}")))
        .collect();
    let view = transcript_viewport(&entries, false, 80, 12, 0);
    assert!(view.lines.len() <= 12);
    assert!(
        view.lines
            .iter()
            .any(|line| line.to_string().contains("entry 99999"))
    );
}

#[test]
fn sparse_deep_scroll_matches_reference_across_mutations_and_resize() {
    let mut transcript = Transcript::new(
        (0..700)
            .map(|index| {
                (
                    match index % 3 {
                        0 => "You",
                        1 => "Assistant",
                        _ => "助手",
                    }
                    .into(),
                    match index % 5 {
                        0 => format!("回复 {index} 中"),
                        1 => format!("🐳 e\u{301} {index}"),
                        2 => format!("ｶﾞ {index} \u{200b}"),
                        _ => format!("entry {index} 中\nsecond line {}", "x".repeat(index % 11)),
                    },
                )
            })
            .collect(),
    );
    let mut cache = TranscriptViewportCache::default();
    let mut compare = |transcript: &Transcript, pending, width, scroll| {
        let expected = transcript_viewport_reference(transcript, pending, width, 9, scroll);
        cache.with_view(transcript, pending, width, 9, scroll, |actual| {
            assert_eq!(actual.lines, expected.lines);
            assert_eq!(actual.scroll_up, expected.scroll_up);
        });
    };
    for width in [7, 19, 80] {
        for pending in [false, true] {
            for scroll in [256, 257, 400, 701, 999, 1_200, 1_800, 2_200, usize::MAX] {
                compare(&transcript, pending, width, scroll);
            }
        }
    }
    transcript.append_content(0, "\nmodified old entry 🐟");
    compare(&transcript, false, 7, 400);
    transcript.append_content(699, "\nnew tail 🐟");
    compare(&transcript, true, 19, 700);
    transcript.push(("You".into(), "a new request".into()));
    compare(&transcript, false, 19, 700);
    transcript.retain_latest(500);
    compare(&transcript, false, 19, 700);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(32))]
    #[test]
    fn thinking_fold_preserves_every_stream_prefix(
        chunks in vec("[a-z\\n]{0,16}", 0..24),
        switches in vec(proptest::bool::ANY, 0..24),
    ) {
        let mut transcript = Transcript::new(Vec::new());
        transcript.push_thinking("open".into(), "closed", String::new(), true);
        let mut raw = String::new();
        for (index, chunk) in chunks.iter().enumerate() {
            transcript.append_content(0, chunk);
            raw.push_str(chunk);
            if switches.get(index).copied().unwrap_or(false) {
                prop_assert_eq!(transcript.toggle_thinking_from_end(1), true);
            }
            prop_assert_eq!(transcript.content(0), raw.as_str());
        }
        transcript.retain_latest(1);
        prop_assert_eq!(transcript.content(0), raw.as_str());
    }

    #[test]
    fn sparse_deep_scroll_preserves_randomized_unicode_rows(
        shapes in vec(0_u8..12, 256..360),
        width in 2_u16..90,
        scroll in 256_usize..2_000,
        pending in proptest::bool::ANY,
    ) {
        let entries = shapes
            .iter()
            .enumerate()
            .map(|(index, shape)| {
                let text = match shape {
                    0 => "plain text".to_owned(),
                    1 => "中 e\u{301} 🐟".to_owned(),
                    2 => "ｶﾞ\u{200b} word".to_owned(),
                    3 => "a\nb\n".to_owned(),
                    4 => "\t\x1bcontrol".to_owned(),
                    5 => "longword".repeat(10),
                    6 => "  leading spaces  ".to_owned(),
                    7 => format!("row {index} 😀"),
                    8 => "\u{0085} short".to_owned(),
                    9 => "👩‍💻👨‍👩‍👧‍👦".to_owned(),
                    10 => "中中中 e\u{301}".to_owned(),
                    _ => "👩🏽‍🚀".to_owned(),
                };
                ("Assistant".to_owned(), text)
            })
            .collect();
        let transcript = Transcript::new(entries);
        let expected = transcript_viewport_reference(&transcript, pending, width, 11, scroll);
        let mut cache = TranscriptViewportCache::default();
        cache.with_view(&transcript, pending, width, 11, scroll, |actual| {
            prop_assert_eq!(&actual.lines, &expected.lines);
            prop_assert_eq!(actual.scroll_up, expected.scroll_up);
            Ok(())
        })?;
        prop_assert_eq!(cache.row_index_bytes() > 0, true);
    }
}

#[test]
#[ignore = "manual matched deep-scroll timing; not a latency acceptance gate"]
fn compare_sparse_deep_scroll_with_full_history_scan() {
    use std::{hint::black_box, time::Instant};
    let transcript = Transcript::new(
        (0..100_000)
            .map(|index| ("User".into(), format!("entry {index}")))
            .collect(),
    );
    let scrolls: Vec<_> = (50_000..50_032).collect();
    let mut cache = TranscriptViewportCache::default();
    let first_expected = transcript_viewport(&transcript, false, 80, 12, scrolls[0]);
    let started = Instant::now();
    cache.with_view(&transcript, false, 80, 12, scrolls[0], |actual| {
        assert_eq!(actual.lines, first_expected.lines);
    });
    let first_index = started.elapsed();
    for &scroll in &scrolls {
        let expected = transcript_viewport(&transcript, false, 80, 12, scroll);
        cache.with_view(&transcript, false, 80, 12, scroll, |actual| {
            assert_eq!(actual.lines, expected.lines);
        });
    }
    let started = Instant::now();
    for &scroll in &scrolls {
        black_box(transcript_viewport(&transcript, false, 80, 12, scroll));
    }
    let full_scan = started.elapsed();
    let started = Instant::now();
    for &scroll in &scrolls {
        cache.with_view(&transcript, false, 80, 12, scroll, |view| {
            black_box(view.lines.len());
        });
    }
    let indexed = started.elapsed();
    eprintln!(
        "100k-entry deep scroll: n={} first_index={first_index:?} index_bytes={} full_scan={full_scan:?} indexed={indexed:?}",
        scrolls.len(),
        cache.row_index_bytes()
    );
}

#[test]
#[ignore = "manual first-index timing for non-ASCII history"]
fn compare_unicode_first_deep_scroll_with_full_history_scan() {
    use std::time::Instant;
    let transcript = Transcript::new(
        (0..100_000)
            .map(|index| ("助手".into(), format!("回复 {index} 中")))
            .collect(),
    );
    let started = Instant::now();
    let expected = transcript_viewport(&transcript, false, 80, 12, 50_000);
    let full_scan = started.elapsed();
    let mut cache = TranscriptViewportCache::default();
    let started = Instant::now();
    cache.with_view(&transcript, false, 80, 12, 50_000, |actual| {
        assert_eq!(actual.lines, expected.lines);
    });
    let first_index = started.elapsed();
    eprintln!(
        "100k Unicode entries first deep scroll: full_scan={full_scan:?} first_index={first_index:?} index_bytes={}",
        cache.row_index_bytes()
    );
}

#[test]
#[ignore = "manual diagnostic; wall-clock measurements are not an acceptance gate"]
fn compare_full_and_suffix_transcript_projection() {
    use std::{hint::black_box, time::Instant};
    use unicode_width::UnicodeWidthStr;

    let entries: Vec<(String, String)> = (0..10_000)
        .map(|index| ("User".into(), format!("entry {index} short")))
        .collect();
    let area = Rect::new(0, 0, 80, 12);
    let baseline = || {
        let mut lines = Vec::new();
        for (role, content) in &entries {
            lines.push(Line::from(Span::styled(
                role.clone(),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )));
            lines.extend(
                safe_text(content)
                    .lines()
                    .map(|line| Line::from(line.to_owned())),
            );
            lines.push(Line::default());
        }
        let total: usize = lines
            .iter()
            .map(|line| {
                UnicodeWidthStr::width(line.to_string().as_str())
                    .max(1)
                    .div_ceil(80)
            })
            .sum();
        let scroll = total.saturating_sub(12).min(u16::MAX as usize) as u16;
        let mut buffer = Buffer::empty(area);
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .scroll((scroll, 0))
            .render(area, &mut buffer);
        buffer
    };
    let candidate = || {
        let view = transcript_viewport(&entries, false, area.width, area.height, 0);
        let mut buffer = Buffer::empty(area);
        view.render(area, &mut buffer);
        buffer
    };
    assert_eq!(baseline(), candidate());
    let mut baseline_us = Vec::new();
    let mut candidate_us = Vec::new();
    for _ in 0..12 {
        let start = Instant::now();
        black_box(baseline());
        baseline_us.push(start.elapsed().as_micros());
        let start = Instant::now();
        black_box(candidate());
        candidate_us.push(start.elapsed().as_micros());
    }
    println!(
        "fixture=10000-short-entries terminal=80x12 scroll=0 baseline_us={baseline_us:?} candidate_us={candidate_us:?}"
    );
}

#[test]
#[ignore = "manual fold-selection timing; not an end-to-end latency gate"]
fn benchmark_sparse_thinking_fold_selection() {
    use std::{
        hint::black_box,
        time::{Duration, Instant},
    };
    let mut transcript = Transcript::new(Vec::new());
    for _ in 0..10_000 {
        transcript.push_thinking("open".into(), "closed", "x".repeat(256), true);
    }
    fn sample(transcript: &mut Transcript, ordinal: usize) -> Duration {
        let started = Instant::now();
        for _ in 0..100 {
            black_box(&mut *transcript).toggle_thinking_from_end(ordinal);
        }
        started.elapsed() / 100
    }
    let mut latest: Vec<_> = (0..100).map(|_| sample(&mut transcript, 1)).collect();
    let mut middle: Vec<_> = (0..100).map(|_| sample(&mut transcript, 5_000)).collect();
    latest.sort_unstable();
    middle.sort_unstable();
    eprintln!(
        "10k sparse folds, 100 batches × 100 toggles: newest p50={:?} p95={:?}; middle p50={:?} p95={:?}",
        latest[50], latest[95], middle[50], middle[95]
    );
}
