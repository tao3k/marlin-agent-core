// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Authentic Emacs Helm bottom-split Ratatui renderer.
//!
//! Renders Helm as a native bottom Emacs window (never a boxed floating modal):
//! - Sleek 1-row top window divider keeping the main conversation visible above
//! - Full-width `helm-source-header` group bars (`Workspace Search`, `Buffers`, `Commands`)
//! - Inline `helm-match` + Orderless multi-token highlighting
//! - Side-by-side `helm-follow-mode` source context preview (right pane with line numbers)
//! - Selectable `TAB` Action Transformer view
//! - Doom-style `helm-mode-line` + syntax-highlighted bottom minibuffer prompt

use marlin_agent_ui::{UiRgb, UiTheme};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Clear, Paragraph},
};
use unicode_width::UnicodeWidthStr;

use crate::helm::{HelmCandidate, HelmDisplayRow, HelmOverlay};

pub(crate) fn render_helm_window(
    helm: &HelmOverlay,
    frame: &mut Frame<'_>,
    area: Rect,
    theme: &UiTheme,
) {
    if area.width < 24 || area.height < 8 {
        return;
    }
    let helm_height = (area.height * 54 / 100)
        .max(10)
        .min(area.height.saturating_sub(2));
    let window_area = Rect {
        x: area.x,
        y: area.bottom().saturating_sub(helm_height),
        width: area.width,
        height: helm_height,
    };
    frame.render_widget(Clear, window_area);
    frame.buffer_mut().set_style(
        window_area,
        Style::default().bg(rgb(theme.canvas)).fg(rgb(theme.ink)),
    );

    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Top Emacs window divider
            Constraint::Min(3),    // Main Helm body (grouped candidates + optional right preview)
            Constraint::Length(1), // Authentic Helm modeline (`helm-mode-line`)
            Constraint::Length(1), // Bottom Emacs minibuffer prompt (`Helm> ...`)
        ])
        .split(window_area);

    render_top_divider(helm, frame, sections[0], theme);
    if helm.action_mode {
        render_action_transformer(helm, frame, sections[1], theme);
    } else {
        render_candidate_workspace(helm, frame, sections[1], theme);
    }
    render_helm_modeline(helm, frame, sections[2], theme);
    render_helm_minibuffer(helm, frame, sections[3], theme);
}

fn render_top_divider(helm: &HelmOverlay, frame: &mut Frame<'_>, area: Rect, theme: &UiTheme) {
    let width = usize::from(area.width);
    let label = if helm.action_mode {
        " Helm Actions "
    } else {
        " Helm "
    };
    let left_rule = "━━";
    let used = UnicodeWidthStr::width(left_rule) + UnicodeWidthStr::width(label);
    let right_rule = "━".repeat(width.saturating_sub(used));
    let line = Line::from(vec![
        Span::styled(left_rule, Style::default().fg(rgb(theme.accent))),
        Span::styled(
            label,
            Style::default()
                .fg(rgb(theme.accent))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(right_rule, Style::default().fg(rgb(theme.bar))),
    ]);
    frame.render_widget(Paragraph::new(line), area);
}

fn render_candidate_workspace(
    helm: &HelmOverlay,
    frame: &mut Frame<'_>,
    area: Rect,
    theme: &UiTheme,
) {
    let show_preview = helm.follow_preview
        && area.width >= 88
        && helm
            .selected_candidate()
            .is_some_and(|cand| !cand.context_lines.is_empty());

    if show_preview {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(56),
                Constraint::Length(1),
                Constraint::Percentage(44),
            ])
            .split(area);
        render_grouped_candidates(helm, frame, cols[0], theme);
        render_vertical_divider(frame, cols[1], theme);
        render_follow_preview(helm, frame, cols[2], theme);
    } else {
        render_grouped_candidates(helm, frame, area, theme);
    }
}

fn render_vertical_divider(frame: &mut Frame<'_>, area: Rect, theme: &UiTheme) {
    let lines: Vec<Line<'_>> = (0..area.height)
        .map(|_| Line::from(Span::styled("│", Style::default().fg(rgb(theme.bar)))))
        .collect();
    frame.render_widget(Paragraph::new(lines), area);
}

fn render_grouped_candidates(
    helm: &HelmOverlay,
    frame: &mut Frame<'_>,
    area: Rect,
    theme: &UiTheme,
) {
    let visible_rows = usize::from(area.height).max(1);
    let width = usize::from(area.width);
    if helm.display_rows.is_empty() {
        let msg = helm
            .last_error
            .as_deref()
            .unwrap_or("  No matches · try a pattern, @lang:rust, @prefix:crates/, or #local !neg");
        let line = Line::from(Span::styled(msg, Style::default().fg(rgb(theme.muted))));
        frame.render_widget(Paragraph::new(line), area);
        return;
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

    let mut lines = Vec::with_capacity(visible_rows);
    for row in helm.display_rows.iter().skip(offset).take(visible_rows) {
        match row {
            HelmDisplayRow::SourceHeader {
                source,
                count,
                meta,
            } => {
                lines.push(render_source_header(
                    source.section_title(),
                    *count,
                    meta,
                    width,
                    theme,
                ));
            }
            HelmDisplayRow::Candidate {
                filtered_pos,
                raw_index,
            } => {
                if let Some(cand) = helm.candidate_at(*raw_index) {
                    let is_selected = *filtered_pos == helm.selected;
                    let is_marked = helm.marked_ids.iter().any(|mid| mid == &cand.id);
                    lines.push(render_candidate_row(
                        cand,
                        is_selected,
                        is_marked,
                        &needles,
                        width,
                        theme,
                    ));
                }
            }
        }
    }
    frame.render_widget(Paragraph::new(lines), area);
}

fn render_source_header(
    title: &str,
    count: usize,
    meta: &str,
    width: usize,
    theme: &UiTheme,
) -> Line<'static> {
    let header_bg = Style::default().bg(rgb(theme.bar));
    let left_text = format!("▌ {title} ({count})");
    let right_text = if meta.is_empty() {
        String::new()
    } else {
        format!("{meta} ")
    };
    let used =
        UnicodeWidthStr::width(left_text.as_str()) + UnicodeWidthStr::width(right_text.as_str());
    let pad = " ".repeat(width.saturating_sub(used));
    Line::from(vec![
        Span::styled(
            left_text,
            header_bg.fg(rgb(theme.accent)).add_modifier(Modifier::BOLD),
        ),
        Span::styled(pad, header_bg),
        Span::styled(right_text, header_bg.fg(rgb(theme.muted))),
    ])
}

fn render_candidate_row<'a>(
    cand: &'a HelmCandidate,
    is_selected: bool,
    is_marked: bool,
    needles: &[String],
    width: usize,
    theme: &UiTheme,
) -> Line<'a> {
    let base_style = if is_selected {
        Style::default().bg(rgb(theme.bar))
    } else {
        Style::default().bg(rgb(theme.canvas))
    };

    let (gutter, gutter_color) = match (is_selected, is_marked) {
        (true, true) => ("▸*", theme.warning),
        (true, false) => ("▸ ", theme.accent),
        (false, true) => (" *", theme.warning),
        (false, false) => ("  ", theme.muted),
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

    let mut spans = Vec::new();
    spans.push(Span::styled(
        format!("{gutter} "),
        base_style
            .fg(rgb(gutter_color))
            .add_modifier(Modifier::BOLD),
    ));

    let primary_style = base_style.fg(rgb(if is_selected { theme.info } else { theme.ink }));
    let match_style = base_style
        .fg(rgb(theme.accent))
        .add_modifier(Modifier::BOLD);
    push_highlighted_spans(
        &mut spans,
        &primary_text,
        needles,
        primary_style,
        match_style,
    );

    let mut used = 3 + primary_used;
    if !snippet_text.is_empty() {
        spans.push(Span::styled("  ", base_style));
        let snippet_style = base_style.fg(rgb(theme.muted));
        let snippet_match = base_style
            .fg(rgb(theme.warning))
            .add_modifier(Modifier::BOLD);
        push_highlighted_spans(
            &mut spans,
            &snippet_text,
            needles,
            snippet_style,
            snippet_match,
        );
        used += 2 + UnicodeWidthStr::width(snippet_text.as_str());
    }

    let pad = width.saturating_sub(used + badge_width);
    if pad > 0 {
        spans.push(Span::styled(" ".repeat(pad), base_style));
    }
    if !badge.is_empty() {
        spans.push(Span::styled(badge, base_style.fg(rgb(theme.muted))));
    }
    Line::from(spans)
}

fn render_follow_preview(helm: &HelmOverlay, frame: &mut Frame<'_>, area: Rect, theme: &UiTheme) {
    let Some(cand) = helm.selected_candidate() else {
        return;
    };
    let width = usize::from(area.width);
    let height = usize::from(area.height);
    let needles = helm.parsed.highlight_needles();

    let header_bg = Style::default().bg(rgb(theme.bar));
    let title_text = clip_width(&format!(" ▎ {}", cand.display), width);
    let pad = " ".repeat(width.saturating_sub(UnicodeWidthStr::width(title_text.as_str())));
    let mut lines = Vec::with_capacity(height);
    lines.push(Line::from(vec![
        Span::styled(
            title_text,
            header_bg.fg(rgb(theme.info)).add_modifier(Modifier::BOLD),
        ),
        Span::styled(pad, header_bg),
    ]));

    let code_budget = width.saturating_sub(8);
    for ctx in cand.context_lines.iter().take(height.saturating_sub(1)) {
        let row_bg = if ctx.is_match {
            Style::default().bg(rgb(theme.bar))
        } else {
            Style::default().bg(rgb(theme.canvas))
        };
        let gutter = if ctx.is_match { "▸" } else { " " };
        let line_no = format!("{gutter}{:>4} │ ", ctx.line_number);
        let code_text = clip_width(&ctx.text, code_budget);
        let code_used = UnicodeWidthStr::width(code_text.as_str());
        let mut spans = vec![Span::styled(
            line_no,
            row_bg.fg(rgb(if ctx.is_match {
                theme.accent
            } else {
                theme.muted
            })),
        )];
        let code_style = row_bg.fg(rgb(theme.ink));
        let match_style = row_bg.fg(rgb(theme.warning)).add_modifier(Modifier::BOLD);
        push_highlighted_spans(&mut spans, &code_text, &needles, code_style, match_style);
        let tail_pad = width.saturating_sub(8 + code_used);
        if tail_pad > 0 {
            spans.push(Span::styled(" ".repeat(tail_pad), row_bg));
        }
        lines.push(Line::from(spans));
    }

    frame.render_widget(Paragraph::new(lines), area);
}

fn render_action_transformer(
    helm: &HelmOverlay,
    frame: &mut Frame<'_>,
    area: Rect,
    theme: &UiTheme,
) {
    let width = usize::from(area.width);
    let actions = helm.available_actions();
    let target_label = helm
        .selected_candidate()
        .map_or("(none)", |c| c.display.as_str());
    let mut lines = Vec::with_capacity(actions.len() + 2);
    lines.push(render_source_header(
        "Helm Actions",
        actions.len(),
        &format!("target: {}", clip_width(target_label, width / 2)),
        width,
        theme,
    ));

    for (idx, item) in actions.iter().enumerate() {
        let is_selected = idx == helm.action_selected;
        let bg = if is_selected {
            Style::default().bg(rgb(theme.bar))
        } else {
            Style::default().bg(rgb(theme.canvas))
        };
        let gutter = if is_selected { "▸ " } else { "  " };
        let key_part = format!("[{:^3}] ", item.key);
        let label_part = format!("{}. {}  ", idx + 1, item.label);
        let used = 2
            + UnicodeWidthStr::width(key_part.as_str())
            + UnicodeWidthStr::width(label_part.as_str())
            + UnicodeWidthStr::width(item.detail.as_str());
        let pad = " ".repeat(width.saturating_sub(used));
        lines.push(Line::from(vec![
            Span::styled(
                gutter,
                bg.fg(rgb(theme.accent)).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                key_part,
                bg.fg(rgb(theme.warning)).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                label_part,
                bg.fg(rgb(if is_selected { theme.info } else { theme.ink })),
            ),
            Span::styled(item.detail.clone(), bg.fg(rgb(theme.muted))),
            Span::styled(pad, bg),
        ]));
    }
    frame.render_widget(Paragraph::new(lines), area);
}

fn render_helm_modeline(helm: &HelmOverlay, frame: &mut Frame<'_>, area: Rect, theme: &UiTheme) {
    let width = usize::from(area.width);
    let bar_style = Style::default().bg(rgb(theme.bar));
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
        Span::styled(
            badge,
            Style::default()
                .bg(rgb(theme.accent))
                .fg(rgb(theme.canvas))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(left_info, bar_style.fg(rgb(theme.ink))),
        Span::styled(" ".repeat(pad), bar_style),
    ];
    if show_hints {
        spans.push(Span::styled(hints, bar_style.fg(rgb(theme.muted))));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn render_helm_minibuffer(helm: &HelmOverlay, frame: &mut Frame<'_>, area: Rect, theme: &UiTheme) {
    let width = usize::from(area.width);
    let prompt_label = "Helm> ";
    let scope_pill = format!(" [{}] ", helm.active_source.label());
    let mut spans = vec![Span::styled(
        prompt_label,
        Style::default()
            .fg(rgb(theme.accent))
            .add_modifier(Modifier::BOLD),
    )];

    // Syntax-color `@lang:`, `@doc:`, `@prefix:`, `@ext:`, `#local`, `!neg` right in the minibuffer!
    let mut in_local = false;
    for (idx, part) in helm.input.split(' ').enumerate() {
        if idx > 0 {
            spans.push(Span::styled(" ", Style::default().fg(rgb(theme.ink))));
        }
        if part.starts_with('#') {
            in_local = true;
        }
        let style = if part.starts_with("@lang:")
            || part.starts_with("@doc:")
            || part.starts_with("@prefix:")
            || part.starts_with("@ext:")
        {
            Style::default()
                .fg(rgb(theme.info))
                .add_modifier(Modifier::BOLD)
        } else if in_local && part.starts_with('!') {
            Style::default()
                .fg(rgb(theme.warning))
                .add_modifier(Modifier::BOLD)
        } else if in_local {
            Style::default().fg(rgb(theme.success))
        } else {
            Style::default().fg(rgb(theme.ink))
        };
        spans.push(Span::styled(part, style));
    }
    spans.push(Span::styled("▏", Style::default().fg(rgb(theme.accent))));

    let used =
        UnicodeWidthStr::width(prompt_label) + UnicodeWidthStr::width(helm.input.as_str()) + 1;
    let pill_w = UnicodeWidthStr::width(scope_pill.as_str());
    if width > used + pill_w {
        spans.push(Span::styled(
            " ".repeat(width - used - pill_w),
            Style::default(),
        ));
        spans.push(Span::styled(
            scope_pill,
            Style::default().fg(rgb(theme.muted)),
        ));
    }

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
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
    // Keep the immediate SourceHeader directly above the first candidate of a group visible.
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

fn push_highlighted_spans<'a>(
    out: &mut Vec<Span<'a>>,
    text: &str,
    needles: &[String],
    normal: Style,
    highlight: Style,
) {
    if needles.is_empty() || text.is_empty() {
        out.push(Span::styled(text.to_owned(), normal));
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
            out.push(Span::styled(
                text[seg_start..byte_idx].to_owned(),
                if seg_hl { highlight } else { normal },
            ));
            seg_start = byte_idx;
            seg_hl = hl;
        }
    }
    out.push(Span::styled(
        text[seg_start..].to_owned(),
        if seg_hl { highlight } else { normal },
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

fn rgb(value: UiRgb) -> Color {
    Color::Rgb(value.red, value.green, value.blue)
}
