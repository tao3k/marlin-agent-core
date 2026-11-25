// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Pure declarative `UiElement` projection for the Emacs Helm bottom-split window.
//!
//! Produces only host-neutral `marlin_agent_ui::UiElement` trees rendered by
//! `crate::ui_elements::render_overlay_element` (zero direct Ratatui widget calls).

use marlin_agent_ui::{UiBackground, UiElement, UiSize, UiTemplateSpan, UiTemplateValue, UiTone};
use ratatui::layout::Rect;
use unicode_width::UnicodeWidthStr;

use super::{HelmCandidate, HelmDisplayRow, HelmOverlay};

/// Computes the bottom-split Emacs window rectangle for Helm, leaving the top
/// conversation view unobstructed.
pub(crate) fn helm_window_area(area: Rect) -> Option<Rect> {
    if area.width < 24 || area.height < 8 {
        return None;
    }
    let helm_height = (area.height * 54 / 100)
        .max(10)
        .min(area.height.saturating_sub(2));
    Some(Rect {
        x: area.x,
        y: area.bottom().saturating_sub(helm_height),
        width: area.width,
        height: helm_height,
    })
}

/// Projects the live `HelmOverlay` state into a declarative `UiElement` tree.
pub(crate) fn project_helm_ui_element(helm: &HelmOverlay, window_area: Rect) -> UiElement {
    let body_height = window_area.height.saturating_sub(3).max(1);
    let body = if helm.action_mode {
        project_action_transformer(helm, window_area.width, body_height)
    } else {
        project_candidate_workspace(helm, window_area.width, body_height)
    };
    UiElement::Rows(vec![
        UiElement::Sized {
            size: UiSize::Cells(1),
            child: Box::new(project_top_divider(helm, window_area.width)),
        },
        UiElement::Sized {
            size: UiSize::Flex(1),
            child: Box::new(body),
        },
        UiElement::Sized {
            size: UiSize::Cells(1),
            child: Box::new(project_helm_modeline(helm, window_area.width)),
        },
        UiElement::Sized {
            size: UiSize::Cells(1),
            child: Box::new(project_helm_minibuffer(helm, window_area.width)),
        },
    ])
}

fn span(text: impl Into<String>, tone: UiTone, bold: bool) -> UiTemplateSpan {
    UiTemplateSpan {
        value: UiTemplateValue::Literal(text.into()),
        tone,
        bold,
    }
}

fn line_row(background: UiBackground, spans: Vec<UiTemplateSpan>) -> UiElement {
    UiElement::Sized {
        size: UiSize::Cells(1),
        child: Box::new(UiElement::Template { background, spans }),
    }
}

fn project_top_divider(helm: &HelmOverlay, width: u16) -> UiElement {
    let width = usize::from(width);
    let label = if helm.action_mode {
        " Helm Actions "
    } else {
        " Helm "
    };
    let left_rule = "━━";
    let used = UnicodeWidthStr::width(left_rule) + UnicodeWidthStr::width(label);
    let right_rule = "━".repeat(width.saturating_sub(used));
    UiElement::Template {
        background: UiBackground::Canvas,
        spans: vec![
            span(left_rule, UiTone::Accent, false),
            span(label, UiTone::Accent, true),
            span(right_rule, UiTone::Muted, false),
        ],
    }
}

fn project_candidate_workspace(helm: &HelmOverlay, width: u16, height: u16) -> UiElement {
    let show_preview = helm.follow_preview
        && width >= 88
        && helm
            .selected_candidate()
            .is_some_and(|cand| !cand.context_lines.is_empty());

    if show_preview {
        let left_width = ((u32::from(width.saturating_sub(1)) * 56) / 100).max(1) as u16;
        let right_width = width.saturating_sub(left_width).saturating_sub(1).max(1);
        let divider_rows: Vec<UiElement> = (0..height)
            .map(|_| line_row(UiBackground::Canvas, vec![span("│", UiTone::Muted, false)]))
            .collect();
        UiElement::Columns(vec![
            UiElement::Sized {
                size: UiSize::Flex(56),
                child: Box::new(project_grouped_candidates(helm, left_width, height)),
            },
            UiElement::Sized {
                size: UiSize::Cells(1),
                child: Box::new(UiElement::Rows(divider_rows)),
            },
            UiElement::Sized {
                size: UiSize::Flex(44),
                child: Box::new(project_follow_preview(helm, right_width, height)),
            },
        ])
    } else {
        project_grouped_candidates(helm, width, height)
    }
}

fn project_grouped_candidates(helm: &HelmOverlay, width: u16, height: u16) -> UiElement {
    let visible_rows = usize::from(height).max(1);
    let width = usize::from(width);
    if helm.display_rows.is_empty() {
        let msg = helm.last_error.as_deref().unwrap_or(
            "  No matches · try pattern, @sym:name, @graph:target, @lang:rust, @prefix:crates/, or #local !neg",
        );
        return UiElement::Rows(vec![line_row(
            UiBackground::Canvas,
            vec![span(clip_width(msg, width), UiTone::Muted, false)],
        )]);
    }

    let selected_row_idx = helm
        .display_rows
        .iter()
        .position(|row| {
            matches!(row, HelmDisplayRow::Candidate { filtered_pos, .. } if *filtered_pos == helm.selected)
        })
        .unwrap_or(0);
    let offset = compute_display_offset(
        selected_row_idx,
        visible_rows,
        helm.display_rows.len(),
        &helm.display_rows,
    );
    let needles = helm.parsed.highlight_needles();

    let mut rows = Vec::with_capacity(visible_rows);
    for row in helm.display_rows.iter().skip(offset).take(visible_rows) {
        match row {
            HelmDisplayRow::SourceHeader {
                source,
                count,
                meta,
            } => {
                rows.push(project_source_header(
                    source.section_title(),
                    *count,
                    meta,
                    width,
                ));
            }
            HelmDisplayRow::Candidate {
                filtered_pos,
                raw_index,
            } => {
                if let Some(cand) = helm.candidate_at(*raw_index) {
                    let is_selected = *filtered_pos == helm.selected;
                    let is_marked = helm.marked_ids.iter().any(|mid| mid == &cand.id);
                    rows.push(project_candidate_row(
                        cand,
                        is_selected,
                        is_marked,
                        &needles,
                        width,
                    ));
                }
            }
        }
    }
    UiElement::Rows(rows)
}

fn project_source_header(title: &str, count: usize, meta: &str, width: usize) -> UiElement {
    let left_text = format!("▌ {title} ({count})");
    let right_text = if meta.is_empty() {
        String::new()
    } else {
        format!("{meta} ")
    };
    let used =
        UnicodeWidthStr::width(left_text.as_str()) + UnicodeWidthStr::width(right_text.as_str());
    let pad = " ".repeat(width.saturating_sub(used));
    line_row(
        UiBackground::Bar,
        vec![
            span(left_text, UiTone::Accent, true),
            span(pad, UiTone::Muted, false),
            span(right_text, UiTone::Muted, false),
        ],
    )
}

fn project_candidate_row(
    cand: &HelmCandidate,
    is_selected: bool,
    is_marked: bool,
    needles: &[String],
    width: usize,
) -> UiElement {
    let bg = if is_selected {
        UiBackground::Bar
    } else {
        UiBackground::Canvas
    };

    let (gutter, gutter_tone) = match (is_selected, is_marked) {
        (true, true) => ("▸*", UiTone::Warning),
        (true, false) => ("▸ ", UiTone::Accent),
        (false, true) => (" *", UiTone::Warning),
        (false, false) => ("  ", UiTone::Muted),
    };

    let badge = if cand.badge.is_empty() {
        String::new()
    } else {
        format!(" [{}]", cand.badge)
    };
    let badge_width = UnicodeWidthStr::width(badge.as_str());
    let content_budget = width.saturating_sub(3 + badge_width);
    let primary_max = (content_budget * 56 / 100).max(16).min(content_budget);
    let primary_text = clip_width(&cand.display, primary_max);
    let primary_used = UnicodeWidthStr::width(primary_text.as_str());
    let snippet_budget = content_budget.saturating_sub(primary_used + 2);
    let snippet_text = if snippet_budget >= 6 && !cand.snippet.is_empty() {
        clip_width(&cand.snippet, snippet_budget)
    } else {
        String::new()
    };

    let mut spans = vec![span(format!("{gutter} "), gutter_tone, true)];
    let primary_tone = if is_selected {
        UiTone::Info
    } else {
        UiTone::Normal
    };
    push_highlighted_spans(
        &mut spans,
        &primary_text,
        needles,
        primary_tone,
        UiTone::Accent,
    );

    let mut used = 3 + primary_used;
    if !snippet_text.is_empty() {
        spans.push(span("  ", UiTone::Muted, false));
        push_highlighted_spans(
            &mut spans,
            &snippet_text,
            needles,
            UiTone::Muted,
            UiTone::Warning,
        );
        used += 2 + UnicodeWidthStr::width(snippet_text.as_str());
    }

    let pad = width.saturating_sub(used + badge_width);
    if pad > 0 {
        spans.push(span(" ".repeat(pad), UiTone::Normal, false));
    }
    if !badge.is_empty() {
        spans.push(span(badge, UiTone::Muted, false));
    }
    line_row(bg, spans)
}

fn project_follow_preview(helm: &HelmOverlay, width: u16, height: u16) -> UiElement {
    let Some(cand) = helm.selected_candidate() else {
        return UiElement::Rows(Vec::new());
    };
    let width = usize::from(width);
    let height = usize::from(height);
    let needles = helm.parsed.highlight_needles();

    let raw_title = if let Some(rel) = cand.graph_relations.first() {
        format!(" ▎ {} · {rel}", cand.display)
    } else {
        format!(" ▎ {}", cand.display)
    };
    let title_text = clip_width(&raw_title, width);
    let pad = " ".repeat(width.saturating_sub(UnicodeWidthStr::width(title_text.as_str())));
    let mut rows = Vec::with_capacity(height);
    rows.push(line_row(
        UiBackground::Bar,
        vec![
            span(title_text, UiTone::Info, true),
            span(pad, UiTone::Normal, false),
        ],
    ));

    let code_budget = width.saturating_sub(8);
    for ctx in cand.context_lines.iter().take(height.saturating_sub(1)) {
        let row_bg = if ctx.is_match {
            UiBackground::Bar
        } else {
            UiBackground::Canvas
        };
        let gutter = if ctx.is_match { "▸" } else { " " };
        let line_no = format!("{gutter}{:>4} │ ", ctx.line_number);
        let code_text = clip_width(&ctx.text, code_budget);
        let code_used = UnicodeWidthStr::width(code_text.as_str());
        let mut spans = vec![span(
            line_no,
            if ctx.is_match {
                UiTone::Accent
            } else {
                UiTone::Muted
            },
            false,
        )];
        push_highlighted_spans(
            &mut spans,
            &code_text,
            &needles,
            UiTone::Normal,
            UiTone::Warning,
        );
        let tail_pad = width.saturating_sub(8 + code_used);
        if tail_pad > 0 {
            spans.push(span(" ".repeat(tail_pad), UiTone::Normal, false));
        }
        rows.push(line_row(row_bg, spans));
    }

    UiElement::Rows(rows)
}

fn project_action_transformer(helm: &HelmOverlay, width: u16, height: u16) -> UiElement {
    let width = usize::from(width);
    let actions = helm.available_actions();
    let target_label = helm
        .selected_candidate()
        .map_or("(none)", |c| c.display.as_str());
    let mut rows = Vec::with_capacity(actions.len() + 1);
    rows.push(project_source_header(
        "Helm Actions",
        actions.len(),
        &format!("target: {}", clip_width(target_label, width / 2)),
        width,
    ));

    for (idx, item) in actions
        .iter()
        .enumerate()
        .take(usize::from(height).saturating_sub(1))
    {
        let is_selected = idx == helm.action_selected;
        let bg = if is_selected {
            UiBackground::Bar
        } else {
            UiBackground::Canvas
        };
        let gutter = if is_selected { "▸ " } else { "  " };
        let key_part = format!("[{:^3}] ", item.key);
        let label_part = format!("{}. {}  ", idx + 1, item.label);
        let used = 2
            + UnicodeWidthStr::width(key_part.as_str())
            + UnicodeWidthStr::width(label_part.as_str())
            + UnicodeWidthStr::width(item.detail.as_str());
        let pad = " ".repeat(width.saturating_sub(used));
        rows.push(line_row(
            bg,
            vec![
                span(gutter, UiTone::Accent, true),
                span(key_part, UiTone::Warning, true),
                span(
                    label_part,
                    if is_selected {
                        UiTone::Info
                    } else {
                        UiTone::Normal
                    },
                    false,
                ),
                span(item.detail.clone(), UiTone::Muted, false),
                span(pad, UiTone::Normal, false),
            ],
        ));
    }
    UiElement::Rows(rows)
}

fn project_helm_modeline(helm: &HelmOverlay, width: u16) -> UiElement {
    let width = usize::from(width);
    let badge = if helm.action_mode {
        " HELM-ACT "
    } else {
        " HELM "
    };
    let pos_text = if helm.filtered_indices.is_empty() {
        "0/0".to_owned()
    } else {
        format!("{}/{}", helm.selected + 1, helm.filtered_indices.len())
    };
    let mark_part = if helm.marked_ids.is_empty() {
        String::new()
    } else {
        format!(" · *{}", helm.marked_ids.len())
    };
    let gen_part = helm
        .generation_digest
        .as_deref()
        .and_then(|d| d.strip_prefix("blake3-256:"))
        .map(|hex| format!(" · {}", &hex[..hex.len().min(7)]))
        .unwrap_or_default();
    let follow_part = if helm.follow_preview {
        " · follow"
    } else {
        ""
    };
    let left_info = format!(
        " {} · {}{} · {}{}{} ",
        helm.active_source.label(),
        pos_text,
        mark_part,
        helm.format_latency(),
        follow_part,
        gen_part
    );
    let hints = if helm.action_mode {
        "RET:Run  TAB/ESC:Candidates  C-g:Quit "
    } else {
        "RET:Open  TAB:Actions  C-SPC:Mark  C-o:Source  C-z:Follow  C-l:Refresh  C-g:Quit "
    };

    let left_w = UnicodeWidthStr::width(badge) + UnicodeWidthStr::width(left_info.as_str());
    let hints_w = UnicodeWidthStr::width(hints);
    let show_hints = width >= left_w + hints_w;
    let pad = if show_hints {
        width.saturating_sub(left_w + hints_w)
    } else {
        width.saturating_sub(left_w)
    };

    let mut spans = vec![
        span(badge, UiTone::Accent, true),
        span(left_info, UiTone::Normal, false),
        span(" ".repeat(pad), UiTone::Normal, false),
    ];
    if show_hints {
        spans.push(span(hints, UiTone::Muted, false));
    }
    UiElement::Template {
        background: UiBackground::Bar,
        spans,
    }
}

fn project_helm_minibuffer(helm: &HelmOverlay, width: u16) -> UiElement {
    let width = usize::from(width);
    let prompt_label = "Helm> ";
    let scope_pill = format!(" [{}] ", helm.active_source.label());
    let mut spans = vec![span(prompt_label, UiTone::Accent, true)];

    let mut in_local = false;
    for (idx, part) in helm.input.split(' ').enumerate() {
        if idx > 0 {
            spans.push(span(" ", UiTone::Normal, false));
        }
        if part.starts_with('#') {
            in_local = true;
        }
        let (tone, bold) = if part.starts_with("@lang:")
            || part.starts_with("@doc:")
            || part.starts_with("@prefix:")
            || part.starts_with("@ext:")
            || part.starts_with("@sym:")
            || part.starts_with("@graph:")
        {
            (UiTone::Info, true)
        } else if in_local && part.starts_with('!') {
            (UiTone::Warning, true)
        } else if in_local {
            (UiTone::Success, false)
        } else {
            (UiTone::Normal, false)
        };
        spans.push(span(part, tone, bold));
    }
    spans.push(span("▏", UiTone::Accent, false));

    let used =
        UnicodeWidthStr::width(prompt_label) + UnicodeWidthStr::width(helm.input.as_str()) + 1;
    let pill_w = UnicodeWidthStr::width(scope_pill.as_str());
    if width > used + pill_w {
        spans.push(span(
            " ".repeat(width - used - pill_w),
            UiTone::Normal,
            false,
        ));
        spans.push(span(scope_pill, UiTone::Muted, false));
    }

    UiElement::Template {
        background: UiBackground::Canvas,
        spans,
    }
}

fn compute_display_offset(
    selected_row: usize,
    visible_rows: usize,
    total_rows: usize,
    rows: &[HelmDisplayRow],
) -> usize {
    if total_rows <= visible_rows {
        return 0;
    }
    let mut start = selected_row.saturating_sub(visible_rows / 3);
    if selected_row > 0
        && start == selected_row
        && matches!(
            rows.get(selected_row - 1),
            Some(HelmDisplayRow::SourceHeader { .. })
        )
    {
        start = selected_row - 1;
    }
    start.min(total_rows.saturating_sub(visible_rows))
}

fn push_highlighted_spans(
    out: &mut Vec<UiTemplateSpan>,
    text: &str,
    needles: &[String],
    normal: UiTone,
    highlight: UiTone,
) {
    if needles.is_empty() || text.is_empty() {
        out.push(span(text, normal, false));
        return;
    }
    let lower = text.to_ascii_lowercase();
    let mut mask = vec![false; text.len()];
    for needle in needles {
        let needle_lower = needle.to_ascii_lowercase();
        if needle_lower.is_empty() {
            continue;
        }
        let mut cursor = 0usize;
        while let Some(pos) = lower[cursor..].find(&needle_lower) {
            let start = cursor + pos;
            let end = (start + needle_lower.len()).min(text.len());
            for slot in &mut mask[start..end] {
                *slot = true;
            }
            cursor = end.max(start + 1);
            if cursor >= lower.len() {
                break;
            }
        }
    }

    let mut seg_start = 0usize;
    let mut seg_hl = mask[0];
    for (byte_idx, _) in text.char_indices().skip(1) {
        let hl = mask[byte_idx];
        if hl != seg_hl {
            out.push(span(
                &text[seg_start..byte_idx],
                if seg_hl { highlight } else { normal },
                seg_hl,
            ));
            seg_start = byte_idx;
            seg_hl = hl;
        }
    }
    out.push(span(
        &text[seg_start..],
        if seg_hl { highlight } else { normal },
        seg_hl,
    ));
}

fn clip_width(text: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    if UnicodeWidthStr::width(text) <= max_width {
        return text.to_owned();
    }
    let mut out = String::new();
    let mut w = 0usize;
    let limit = max_width.saturating_sub(1);
    for ch in text.chars() {
        let cw = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        if w + cw > limit {
            break;
        }
        out.push(ch);
        w += cw;
    }
    out.push('…');
    out
}
