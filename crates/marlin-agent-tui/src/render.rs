// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Terminal frame projection for the active conversation or workspace view.

use super::{
    app::{App, GatewayObservation},
    viewport::{ScrollAnchor, TranscriptViewportCache},
};
use marlin_agent_ui::{
    UiCommandName, UiContent, UiLayoutPolicy, UiRegion, UiRgb, UiStatusItem, UiStatusTone, UiTheme,
    UiWorkbench,
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

// Configuration and session identifiers may contain control characters. Keep
// the header bounded and prevent them from becoming terminal control input.
fn display_value(value: &str, limit: usize) -> String {
    value
        .chars()
        .take(limit)
        .map(|ch| if ch.is_control() { '�' } else { ch })
        .collect()
}

fn header_value(value: &str) -> String {
    display_value(value, 128)
}

fn trailing_label(value: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    if UnicodeWidthStr::width(value) <= width {
        return value.to_owned();
    }
    let suffix: Vec<_> = value
        .graphemes(true)
        .rev()
        .scan(1usize, |used, grapheme| {
            let next = used.saturating_add(UnicodeWidthStr::width(grapheme));
            (next <= width).then(|| {
                *used = next;
                grapheme
            })
        })
        .collect();
    format!("…{}", suffix.into_iter().rev().collect::<String>())
}

fn rgb(color: UiRgb) -> Color {
    Color::Rgb(color.red, color.green, color.blue)
}

pub(crate) fn chatbox_height(app: &App, policy: &UiLayoutPolicy, width: u16, height: u16) -> u16 {
    let expanded = app.minibuffer_hovered
        || !app.composer.text().is_empty()
        || app.command_draft.is_some()
        || app.history_search.is_some();
    if !expanded {
        return 0;
    }
    let transcript_rows = policy
        .min_transcript_rows
        .min(height.saturating_sub(2))
        .max(1);
    let limit = height
        .saturating_sub(2 + transcript_rows)
        .min(policy.max_chatbox_rows);
    let composer = app
        .history_search
        .as_ref()
        .and_then(|search| search.preview())
        .unwrap_or(&app.composer);
    if limit >= 3 {
        composer
            .preferred_rows(width.saturating_sub(2), limit.saturating_sub(2))
            .saturating_add(2)
    } else {
        limit
    }
}

pub(crate) fn mouse_over_minibuffer(
    app: &App,
    policy: &UiLayoutPolicy,
    width: u16,
    height: u16,
    row: u16,
) -> bool {
    row > 0
        && row <= height
        && row > height.saturating_sub(chatbox_height(app, policy, width, height) + 1)
}

fn draw_extension_content(
    frame: &mut ratatui::Frame<'_>,
    area: Rect,
    label: &str,
    content: &[UiContent],
    theme: &UiTheme,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ", header_value(label)))
        .border_style(Style::default().fg(rgb(theme.info)));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let lines: Vec<Line<'static>> = content
        .iter()
        .map(|item| match item {
            UiContent::Text(text) => Line::from(display_value(text, 256)),
            UiContent::Field { label, value } => Line::from(vec![
                Span::styled(
                    format!("{}: ", display_value(label, 256)),
                    Style::default().fg(rgb(theme.info)),
                ),
                Span::raw(display_value(value, 256)),
            ]),
        })
        .collect();
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

fn draw_modeline(
    frame: &mut ratatui::Frame<'_>,
    area: Rect,
    app: &App,
    ui: &UiWorkbench,
    items: &[UiStatusItem],
) {
    let slot = |name| items.iter().find(|item| item.id.as_str() == name);
    let (Some(brand), Some(model_slot), Some(activity_slot), Some(turns_slot), Some(session_slot)) = (
        slot("modeline/left/brand"),
        slot("modeline/left/model"),
        slot("modeline/left/activity"),
        slot("modeline/right/turns"),
        slot("modeline/right/session"),
    ) else {
        return;
    };
    let base = Style::default().bg(rgb(ui.theme.bar)).fg(rgb(ui.theme.ink));
    let runtime_activity = if app.workspace.view().is_some() {
        "Workspace"
    } else {
        app.activity_label()
    };
    let observed = if runtime_activity == "Idle" {
        match app.gateway_observation {
            GatewayObservation::Unchecked => Some(activity_slot),
            GatewayObservation::Online => slot("modeline/state/online"),
            GatewayObservation::Offline => slot("modeline/state/offline"),
            GatewayObservation::Unknown => slot("modeline/state/unknown"),
        }
    } else {
        None
    };
    let activity = if let Some(observed) = observed {
        if area.width >= 64 {
            observed.label.as_str()
        } else {
            observed.compact_label.as_str()
        }
    } else {
        runtime_activity
    };
    let brand_text = if area.width < 18 {
        &brand.compact_label
    } else {
        &brand.label
    };
    let right_budget = usize::from(area.width / 3);
    let session = if area.width >= 44 && right_budget > 4 {
        format!(
            " ◉ {} ",
            trailing_label(&header_value(&ui.session_label), right_budget - 4)
        )
    } else {
        String::new()
    };
    let turns_candidate = if area.width >= 72 && app.history.len() >= 2 {
        format!("⟳{}  ", app.history.len() / 2)
    } else {
        String::new()
    };
    let turns = if UnicodeWidthStr::width(turns_candidate.as_str())
        + UnicodeWidthStr::width(session.as_str())
        <= right_budget
    {
        turns_candidate
    } else {
        String::new()
    };
    let right = format!("{turns}{session}");
    let right_width = UnicodeWidthStr::width(right.as_str()) as u16;
    let model = format!("  {}  ", header_value(&ui.model_label));
    let brand_text = format!(" {} ", header_value(brand_text));
    let essential_width = UnicodeWidthStr::width(brand_text.as_str())
        + 1
        + UnicodeWidthStr::width(format!("│ {activity} ").as_str());
    let show_model = essential_width + UnicodeWidthStr::width(model.as_str())
        <= usize::from(area.width.saturating_sub(right_width));
    let mut left = vec![
        Span::styled(
            brand_text,
            base.bg(tone_color(brand.tone, &ui.theme))
                .fg(rgb(ui.theme.canvas))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("▸", base.fg(tone_color(brand.tone, &ui.theme))),
    ];
    if show_model {
        left.push(Span::styled(
            model,
            base.fg(tone_color(model_slot.tone, &ui.theme)),
        ));
    }
    left.push(Span::styled(
        format!("│ {activity} "),
        base.fg(if let Some(observed) = observed {
            tone_color(observed.tone, &ui.theme)
        } else {
            tone_color(activity_slot.active_tone, &ui.theme)
        }),
    ));
    frame.render_widget(
        Paragraph::new(Line::from(left)).style(base),
        Rect::new(area.x, area.y, area.width - right_width, area.height),
    );
    if right_width > 0 {
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(turns, base.fg(tone_color(turns_slot.tone, &ui.theme))),
                Span::styled(session, base.fg(tone_color(session_slot.tone, &ui.theme))),
            ]))
            .style(base),
            Rect::new(
                area.x + area.width - right_width,
                area.y,
                right_width,
                area.height,
            ),
        );
    }
}

fn tone_color(tone: UiStatusTone, theme: &UiTheme) -> Color {
    match tone {
        UiStatusTone::Accent => rgb(theme.accent),
        UiStatusTone::Info => rgb(theme.info),
        UiStatusTone::Success => rgb(theme.success),
        UiStatusTone::Warning => rgb(theme.warning),
        UiStatusTone::Muted => rgb(theme.muted),
    }
}

pub(crate) fn draw(
    frame: &mut ratatui::Frame<'_>,
    app: &App,
    ui: &UiWorkbench,
    cache: &mut TranscriptViewportCache,
    anchor: &mut ScrollAnchor,
) -> usize {
    let area = frame.area();
    frame.buffer_mut().set_style(
        area,
        Style::default()
            .bg(rgb(ui.theme.canvas))
            .fg(rgb(ui.theme.ink)),
    );
    // The transcript can end in a separator row, so reserve two rows when
    // possible. Bound measurement for large drafts.
    let transcript_rows = ui
        .layout
        .min_transcript_rows
        .min(area.height.saturating_sub(2))
        .max(1);
    let composer_height = chatbox_height(app, &ui.layout, area.width, area.height);
    let composer = app
        .history_search
        .as_ref()
        .and_then(|search| search.preview())
        .unwrap_or(&app.composer);
    let extensions: Vec<_> = ui
        .regions
        .iter()
        .filter_map(|region| match region {
            UiRegion::Extension { id, label, .. } => Some((id, label)),
            _ => None,
        })
        .collect();
    let status_items = ui.regions.iter().find_map(|region| match region {
        UiRegion::Status(items) if !items.is_empty() => Some(items),
        _ => None,
    });
    let suggestions: Vec<_> = if app.history_search.is_none()
        && (app.command_draft.is_some() || ui.focus.as_str() == "composer")
        && UiCommandName::is_valid_prefix(app.composer.text())
    {
        ui.commands()
            .iter()
            .filter(|command| command.name.as_str().starts_with(app.composer.text()))
            .collect()
    } else {
        Vec::new()
    };
    // Keep the core transcript and editor usable; show a moving window when
    // the available terminal height cannot display every extension.
    let spare = area
        .height
        .saturating_sub(1 + transcript_rows + composer_height);
    let status_height = u16::from(
        status_items.is_some_and(|items| {
            items
                .iter()
                .any(|item| item.id.as_str() == "modeline/left/brand")
        }) && spare > 0,
    );
    let palette_height = (suggestions.len() as u16)
        .min(ui.layout.max_command_rows)
        .min(spare.saturating_sub(status_height));
    let remaining = spare - palette_height;
    let focused_extension = extensions.iter().any(|(id, _)| *id == &ui.focus);
    let extension_height = (extensions.len() as u16)
        .min(remaining.saturating_sub(status_height))
        .max(u16::from(focused_extension && remaining > status_height));
    let parts = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(palette_height),
            Constraint::Length(extension_height),
            Constraint::Length(status_height),
            Constraint::Length(composer_height),
            Constraint::Length(1),
        ])
        .split(area);
    let focused_content = if app.workspace.view().is_none() {
        ui.regions.iter().find_map(|region| match region {
            UiRegion::Extension {
                id,
                label,
                content,
                focusable: true,
            } if id == &ui.focus => Some((label.as_str(), content.as_slice())),
            _ => None,
        })
    } else {
        None
    };
    let panel_content = focused_content;
    let transcript_area = if let Some((label, content)) = panel_content {
        if parts[0].width >= ui.layout.extension_column_min && parts[0].height >= 6 {
            let columns = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Min(40),
                    Constraint::Length(ui.layout.extension_panel_columns),
                ])
                .split(parts[0]);
            draw_extension_content(frame, columns[1], label, content, &ui.theme);
            Some(columns[0])
        } else if focused_content.is_some() {
            draw_extension_content(frame, parts[0], label, content, &ui.theme);
            None
        } else {
            Some(parts[0])
        }
    } else {
        Some(parts[0])
    };
    let effective_scroll = if let Some(transcript_area) = transcript_area {
        let mut render_view = |viewport: &super::viewport::TranscriptViewport| {
            viewport.render(transcript_area, frame.buffer_mut());
            viewport.scroll_up
        };
        match app.workspace.view() {
            Some(view) => cache.with_top_view(
                &view.transcript,
                transcript_area.width,
                transcript_area.height,
                view.scroll_from_top,
                &mut render_view,
            ),
            None => {
                let pending = app.pending.is_some();
                let scroll = anchor.adjust(
                    &app.transcript,
                    pending,
                    transcript_area.width,
                    transcript_area.height,
                    app.scroll_up,
                );
                cache.with_view(
                    &app.transcript,
                    pending,
                    transcript_area.width,
                    transcript_area.height,
                    scroll,
                    &mut render_view,
                )
            }
        }
    } else {
        app.scroll_up
    };
    let title = if app.history_search.is_some() {
        " History · Enter accept · Esc cancel "
    } else if app.command_draft.is_some() {
        " Command "
    } else {
        " Message "
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(
            if ui.focus.as_str() == "composer" || app.command_draft.is_some() {
                Style::default().fg(rgb(ui.theme.info))
            } else {
                Style::default()
            },
        );
    if parts[4].height > 0 {
        let inner = block.inner(parts[4]);
        frame.render_widget(block, parts[4]);
        if let Some(view) = composer.view(inner.width, inner.height) {
            frame.render_widget(Paragraph::new(view.rows.join("\n")), inner);
            if app.history_search.is_none()
                && (ui.focus.as_str() == "composer" || app.command_draft.is_some())
            {
                frame.set_cursor_position((inner.x + view.cursor.0, inner.y + view.cursor.1));
            }
        }
    }
    let selected = app
        .selected_command()
        .filter(|name| suggestions.iter().any(|command| &command.name == *name));
    let selected_index =
        selected.and_then(|name| suggestions.iter().position(|command| &command.name == name));
    let visible = usize::from(parts[1].height);
    let start = selected_index.map_or(0, |index| index.saturating_add(1).saturating_sub(visible));
    for (row, command) in suggestions.iter().skip(start).take(visible).enumerate() {
        let line =
            ratatui::layout::Rect::new(parts[1].x, parts[1].y + row as u16, parts[1].width, 1);
        let text = format!(
            " {} {}  {}",
            if selected == Some(&command.name) {
                "›"
            } else {
                " "
            },
            command.name.as_str(),
            header_value(&command.description)
        );
        let style = Style::default().fg(rgb(ui.theme.info));
        let style = if selected == Some(&command.name) {
            style.add_modifier(Modifier::BOLD)
        } else {
            style
        };
        frame.render_widget(Paragraph::new(text).style(style), line);
    }
    let visible = usize::from(parts[2].height);
    if visible > 0 {
        let focused = extensions.iter().position(|(id, _)| *id == &ui.focus);
        let start = focused.map_or(0, |index| index.saturating_add(1).saturating_sub(visible));
        for (row, (id, label)) in extensions.iter().skip(start).take(visible).enumerate() {
            let is_focused = *id == &ui.focus;
            let text = format!(
                " {} {}",
                if is_focused { "›" } else { "·" },
                header_value(label)
            );
            let line =
                ratatui::layout::Rect::new(parts[2].x, parts[2].y + row as u16, parts[2].width, 1);
            let style = if is_focused {
                Style::default()
                    .fg(rgb(ui.theme.info))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            frame.render_widget(Paragraph::new(text).style(style), line);
        }
    }
    if let Some(items) = status_items.filter(|_| parts[3].height > 0) {
        draw_modeline(frame, parts[3], app, ui, items);
    }
    if let Some(search) = &app.history_search {
        let prefix = if search.is_searching() {
            " Searching: "
        } else if search.preview().is_some() {
            " Search: "
        } else {
            " No match: "
        };
        let available = usize::from(parts[5].width)
            .saturating_sub(UnicodeWidthStr::width(prefix))
            .saturating_sub(1);
        let mut suffix = search.query();
        while UnicodeWidthStr::width(suffix) > available {
            let Some(first) = suffix.graphemes(true).next() else {
                break;
            };
            suffix = &suffix[first.len()..];
        }
        let status = format!("{prefix}{suffix}");
        frame.render_widget(Paragraph::new(status.clone()), parts[5]);
        if parts[5].width > 0 && parts[5].height > 0 {
            let column =
                UnicodeWidthStr::width(status.as_str()).min(usize::from(parts[5].width - 1));
            frame.set_cursor_position((parts[5].x + column as u16, parts[5].y));
        }
        return effective_scroll;
    }
    let selected_status = selected.map(|name| {
        format!(
            " Selected: {}  Enter choose  Up/Down select  Tab complete",
            name.as_str()
        )
    });
    let status = if app.quit_armed {
        " Press Ctrl-C again to quit; another key cancels. Draft will not be saved."
    } else if app.composer.limit_reached() {
        " Draft limit: 1 MiB. Insertion rejected; draft retained."
    } else if let Some(status) = selected_status.as_deref() {
        status
    } else if app.command_draft.is_some() {
        " Enter run command  Up/Down select  Tab complete  Esc cancel  Ctrl-O return"
    } else if app.workspace.view().is_some() {
        " Workspace: PageUp/PageDown scroll  Esc/Ctrl-C return  Enter submit draft"
    } else if ui.focus.as_str() == "commands" {
        " Commands: Enter open  Tab next  Shift-Tab previous  Ctrl-Q quit"
    } else if ui.focus.as_str() == "transcript" {
        " Transcript: Up/Down scroll  Tab next  Shift-Tab previous  Ctrl-Q quit"
    } else if ui.focus.as_str() != "composer" {
        " Extension: Enter activate  Tab next  Shift-Tab previous  Ctrl-Q quit"
    } else if !suggestions.is_empty() && app.composer.cursor_at_end() {
        " Command: Enter run  Up/Down select  Tab complete  Ctrl-O editor  Ctrl-Q quit"
    } else {
        " Message: Enter send  Tab/Shift-Tab focus  Ctrl-R search  Ctrl-O commands  Ctrl-Q quit"
    };
    if parts[4].height == 0 {
        let extras = status_items
            .into_iter()
            .flatten()
            .filter(|item| !item.id.as_str().starts_with("modeline/"))
            .take(2)
            .map(|item| display_value(&item.label, 16))
            .collect::<Vec<_>>();
        let prefix = if extras.is_empty() {
            " › ".to_owned()
        } else {
            format!(" › {} · ", extras.join(" · "))
        };
        frame.render_widget(
            Paragraph::new(format!("{prefix}{}", status.trim_start())),
            parts[5],
        );
        if ui.focus.as_str() == "composer" && parts[5].width > 2 {
            frame.set_cursor_position((parts[5].x + 2, parts[5].y));
        }
    } else {
        frame.render_widget(Paragraph::new(status), parts[5]);
    }
    effective_scroll
}
