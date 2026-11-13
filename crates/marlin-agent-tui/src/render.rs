// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Terminal frame projection for the active conversation or workspace view.

use super::{
    app::{App, BufferView, GatewayObservation, ModelineActivity},
    viewport::{ScrollAnchor, TranscriptViewportCache},
};
use marlin_agent_ui::{
    UiContent, UiLayoutPolicy, UiRegion, UiRgb, UiStatusItem, UiStatusTone, UiTheme, UiWorkbench,
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

fn menu_display_name<'a>(name: &'a str, prefix: Option<&str>) -> &'a str {
    prefix
        .and_then(|prefix| name.strip_prefix(prefix))
        .unwrap_or(name)
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
    let frame = if app.command_draft.is_some() {
        &policy.command_frame
    } else {
        &policy.message_frame
    };
    if frame.fixed_editor_rows > 0 {
        return frame.fixed_editor_rows.min(height);
    }
    let expanded =
        app.minibuffer_hovered || !app.composer.text().is_empty() || app.history_search.is_some();
    if !expanded {
        return 0;
    }
    let transcript_rows = frame.min_transcript_rows.min(height.saturating_sub(2));
    let limit = height
        .saturating_sub(frame.footer_rows + 1 + transcript_rows)
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
    let footer_rows = if app.command_draft.is_some() {
        policy.command_frame.footer_rows
    } else {
        policy.message_frame.footer_rows
    };
    row > 0
        && row <= height
        && row > height.saturating_sub(chatbox_height(app, policy, width, height) + footer_rows)
}

fn draw_extension_content(
    frame: &mut ratatui::Frame<'_>,
    area: Rect,
    label: &str,
    content: &[UiContent],
    theme: &UiTheme,
    scroll: usize,
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
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .scroll((scroll.min(u16::MAX as usize) as u16, 0)),
        inner,
    );
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
    let observed = match app.modeline_activity() {
        ModelineActivity::Idle => match app.gateway_observation {
            GatewayObservation::Unchecked => Some(activity_slot),
            GatewayObservation::Online => slot("modeline/state/online"),
            GatewayObservation::Offline => slot("modeline/state/offline"),
            GatewayObservation::Unknown => slot("modeline/state/unknown"),
        },
        ModelineActivity::Runtime => slot("modeline/state/runtime"),
        ModelineActivity::Workspace => slot("modeline/state/workspace"),
        ModelineActivity::Recovery => slot("modeline/state/recovery"),
        ModelineActivity::ContinuationUnavailable => slot("modeline/state/continuation"),
        ModelineActivity::History => slot("modeline/state/history"),
        ModelineActivity::Working => slot("modeline/state/working"),
        ModelineActivity::ReadingWorkspace => slot("modeline/state/reading"),
    };
    let activity = if let Some(item) = observed {
        if area.width < item.compact_below {
            item.compact_label.as_str()
        } else {
            item.label.as_str()
        }
    } else {
        activity_slot.label.as_str()
    };
    let brand_text = if area.width < brand.compact_below {
        &brand.compact_label
    } else {
        &brand.label
    };
    let right_budget = usize::from(area.width / 3);
    let (session_prefix, session_suffix) = modeline_template(session_slot, area.width);
    let session_adornment =
        UnicodeWidthStr::width(session_prefix) + UnicodeWidthStr::width(session_suffix);
    let session_candidate = if right_budget > session_adornment {
        modeline_value(
            session_slot,
            &trailing_label(
                &header_value(&ui.session_label),
                right_budget - session_adornment,
            ),
            area.width,
        )
    } else {
        String::new()
    };
    let turns_candidate = if app.history.len() >= 2 {
        modeline_value(turns_slot, &(app.history.len() / 2).to_string(), area.width)
    } else {
        String::new()
    };
    let model = modeline_value(model_slot, &header_value(&ui.model_label), area.width);
    let brand_text = format!(" {} ", header_value(brand_text));
    let essential_width =
        UnicodeWidthStr::width(brand_text.as_str()) + 1 + UnicodeWidthStr::width(activity) + 1;
    let mut remaining = usize::from(area.width).saturating_sub(essential_width);
    let mut right_width = 0usize;
    let mut shown = [false; 3];
    for priority in (1_u8..=3).rev() {
        for item in items {
            let (index, text, right) = match item.id.as_str() {
                "modeline/left/model" => (0, model.as_str(), false),
                "modeline/right/turns" => (1, turns_candidate.as_str(), true),
                "modeline/right/session" => (2, session_candidate.as_str(), true),
                _ => continue,
            };
            if item.keep_priority != priority || area.width < item.min_width || text.is_empty() {
                continue;
            }
            let width = UnicodeWidthStr::width(text);
            if width <= remaining && (!right || right_width + width <= right_budget) {
                shown[index] = true;
                remaining -= width;
                if right {
                    right_width += width;
                }
            }
        }
    }
    let show_model = shown[0];
    let turns = if shown[1] {
        turns_candidate
    } else {
        String::new()
    };
    let session = if shown[2] {
        session_candidate
    } else {
        String::new()
    };
    let right_width = right_width as u16;
    let mut left = Vec::with_capacity(4);
    for item in items {
        match item.id.as_str() {
            "modeline/left/brand" => {
                left.push(Span::styled(
                    brand_text.clone(),
                    base.bg(tone_color(brand.tone, &ui.theme))
                        .fg(rgb(ui.theme.canvas))
                        .add_modifier(Modifier::BOLD),
                ));
                left.push(Span::styled(
                    "▸",
                    base.fg(tone_color(brand.tone, &ui.theme)),
                ));
            }
            "modeline/left/model" if show_model => left.push(Span::styled(
                model.clone(),
                base.fg(tone_color(model_slot.tone, &ui.theme)),
            )),
            "modeline/left/activity" => left.push(Span::styled(
                format!("{activity} "),
                base.fg(if let Some(observed) = observed {
                    tone_color(observed.tone, &ui.theme)
                } else {
                    tone_color(activity_slot.active_tone, &ui.theme)
                }),
            )),
            _ => {}
        }
    }
    frame.render_widget(
        Paragraph::new(Line::from(left)).style(base),
        Rect::new(area.x, area.y, area.width - right_width, area.height),
    );
    if right_width > 0 {
        let right = items
            .iter()
            .filter_map(|item| match item.id.as_str() {
                "modeline/right/turns" => Some(Span::styled(
                    turns.clone(),
                    base.fg(tone_color(turns_slot.tone, &ui.theme)),
                )),
                "modeline/right/session" => Some(Span::styled(
                    session.clone(),
                    base.fg(tone_color(session_slot.tone, &ui.theme)),
                )),
                _ => None,
            })
            .collect::<Vec<_>>();
        frame.render_widget(
            Paragraph::new(Line::from(right)).style(base),
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

fn modeline_template(item: &UiStatusItem, width: u16) -> (&str, &str) {
    let template = if width < item.compact_below {
        item.compact_label.as_str()
    } else {
        item.label.as_str()
    };
    template.split_once("{value}").unwrap_or(("", ""))
}

fn modeline_value(item: &UiStatusItem, value: &str, width: u16) -> String {
    let (prefix, suffix) = modeline_template(item, width);
    let mut rendered = String::with_capacity(prefix.len() + value.len() + suffix.len());
    rendered.push_str(prefix);
    rendered.push_str(value);
    rendered.push_str(suffix);
    rendered
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
    let command_active = app.command_draft.is_some();
    let frame_spec = if command_active {
        &ui.layout.command_frame
    } else {
        &ui.layout.message_frame
    };
    let transcript_rows = frame_spec
        .min_transcript_rows
        .min(area.height.saturating_sub(2));
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
            UiRegion::Extension {
                id,
                label,
                focusable: true,
                ..
            } => Some((id, label)),
            _ => None,
        })
        .collect();
    let status_items = ui.regions.iter().find_map(|region| match region {
        UiRegion::Status(items) if !items.is_empty() => Some(items),
        _ => None,
    });
    let suggestions: Vec<_> = if app.history_search.is_none() && app.command_draft.is_some() {
        app.command_candidates
            .iter()
            .filter_map(|name| ui.commands().iter().find(|command| &command.name == name))
            .collect()
    } else {
        Vec::new()
    };
    // Keep the core transcript and editor usable; show a moving window when
    // the available terminal height cannot display every extension.
    let footer_height = frame_spec.footer_rows;
    let spare = area
        .height
        .saturating_sub(footer_height + transcript_rows + composer_height);
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
    let extension_height = if frame_spec.extensions {
        (extensions.len() as u16)
            .min(remaining.saturating_sub(status_height))
            .max(u16::from(focused_extension && remaining > status_height))
    } else {
        0
    };
    let parts = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(transcript_rows),
            Constraint::Length(extension_height),
            Constraint::Length(status_height),
            Constraint::Length(composer_height),
            Constraint::Length(palette_height),
            Constraint::Length(footer_height),
        ])
        .split(area);
    let focused_content = if app.buffer_view == BufferView::Conversation
        && app.workspace.view().is_none()
        && !app.runtime_receipt_open
    {
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
    let static_content = match &app.buffer_view {
        BufferView::Processes => Some(("*Processes*", app.process_content.as_slice())),
        BufferView::Static(buffer) => ui.regions.iter().find_map(|region| match region {
            UiRegion::Extension {
                id,
                label,
                content,
                focusable: false,
            } if id.as_str() == buffer.as_str() => Some((label.as_str(), content.as_slice())),
            _ => None,
        }),
        _ => None,
    };
    let displayed_content = if app.buffer_view == BufferView::Messages {
        None
    } else if static_content.is_some() {
        static_content
    } else {
        app.runtime_receipt_open
            .then_some(("Runtime policy", app.runtime_content.as_slice()))
            .or(focused_content)
    };
    let transcript_area = if let Some((label, content)) = displayed_content {
        let scroll = if static_content.is_some() {
            app.static_buffer_scroll
        } else {
            0
        };
        draw_extension_content(frame, parts[0], label, content, &ui.theme, scroll);
        None
    } else if parts[0].height > 0 {
        Some(parts[0])
    } else {
        None
    };
    let effective_scroll = if let Some(transcript_area) = transcript_area {
        let mut render_view = |viewport: &super::viewport::TranscriptViewport| {
            viewport.render(transcript_area, frame.buffer_mut());
            viewport.scroll_up
        };
        if app.buffer_view == BufferView::Messages {
            let scroll = anchor.adjust(
                &app.messages,
                false,
                transcript_area.width,
                transcript_area.height,
                app.message_scroll_up,
            );
            cache.with_view(
                &app.messages,
                false,
                transcript_area.width,
                transcript_area.height,
                scroll,
                &mut render_view,
            )
        } else {
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
        }
    } else {
        app.scroll_up
    };
    let selected = app
        .selected_command()
        .filter(|name| suggestions.iter().any(|command| &command.name == *name));
    let active = selected.or_else(|| suggestions.first().map(|command| &command.name));
    let selected_index =
        active.and_then(|name| suggestions.iter().position(|command| &command.name == name));
    let command_prompt = app
        .command_menu
        .as_ref()
        .map_or(frame_spec.prompt.as_str(), |menu| menu.prompt.as_str());
    let title = if app.history_search.is_some() {
        " History · Enter accept · Esc cancel "
    } else {
        command_prompt
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
    if parts[3].height > 0 {
        if !frame_spec.bordered {
            let prompt = if parts[3].width >= 20 {
                format!(
                    "{}/{} {}",
                    selected_index.unwrap_or(0) + 1,
                    suggestions.len(),
                    command_prompt.trim_start()
                )
            } else {
                command_prompt.to_owned()
            };
            let prompt_width = UnicodeWidthStr::width(prompt.as_str()) as u16;
            frame.render_widget(
                Paragraph::new(prompt).style(
                    Style::default()
                        .bg(rgb(ui.theme.bar))
                        .fg(rgb(ui.theme.accent)),
                ),
                parts[3],
            );
            let inner = Rect::new(
                parts[3].x + prompt_width.min(parts[3].width),
                parts[3].y,
                parts[3].width.saturating_sub(prompt_width),
                1,
            );
            if let Some(view) = composer.view(inner.width, inner.height) {
                frame.render_widget(
                    Paragraph::new(view.rows.join("\n"))
                        .style(Style::default().bg(rgb(ui.theme.bar)).fg(rgb(ui.theme.ink))),
                    inner,
                );
                frame.set_cursor_position((inner.x + view.cursor.0, inner.y));
            }
        } else {
            let inner = block.inner(parts[3]);
            frame.render_widget(block, parts[3]);
            if let Some(view) = composer.view(inner.width, inner.height) {
                frame.render_widget(Paragraph::new(view.rows.join("\n")), inner);
                if app.history_search.is_none() && ui.focus.as_str() == "composer" {
                    frame.set_cursor_position((inner.x + view.cursor.0, inner.y + view.cursor.1));
                }
            }
        }
    }
    let visible = usize::from(parts[4].height);
    let start = selected_index.map_or(0, |index| index.saturating_add(1).saturating_sub(visible));
    let menu_prefix = app.command_menu.as_ref().map(|menu| menu.prefix.as_str());
    let name_width = suggestions
        .iter()
        .map(|command| menu_display_name(command.name.as_str(), menu_prefix).len())
        .max()
        .unwrap_or(0)
        .min(28);
    for (row, command) in suggestions.iter().skip(start).take(visible).enumerate() {
        let line =
            ratatui::layout::Rect::new(parts[4].x, parts[4].y + row as u16, parts[4].width, 1);
        let active_row = active == Some(&command.name);
        let style = if active_row {
            Style::default().bg(rgb(ui.theme.bar))
        } else {
            Style::default()
        };
        let name_style = Style::default()
            .fg(rgb(if active_row {
                ui.theme.info
            } else {
                ui.theme.ink
            }))
            .add_modifier(if active_row {
                Modifier::BOLD
            } else {
                Modifier::empty()
            });
        let row = Line::from(vec![
            Span::styled(
                if active_row { " › " } else { "   " },
                Style::default().fg(rgb(ui.theme.accent)),
            ),
            Span::styled(
                format!(
                    "{:<name_width$}",
                    menu_display_name(command.name.as_str(), menu_prefix)
                ),
                name_style,
            ),
            Span::raw("  "),
            Span::styled(
                header_value(&command.description),
                Style::default().fg(rgb(ui.theme.muted)),
            ),
        ]);
        frame.render_widget(Paragraph::new(row).style(style), line);
    }
    let visible = usize::from(parts[1].height);
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
                ratatui::layout::Rect::new(parts[1].x, parts[1].y + row as u16, parts[1].width, 1);
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
    if let Some(items) = status_items.filter(|_| parts[2].height > 0) {
        draw_modeline(frame, parts[2], app, ui, items);
    }
    if parts[5].height == 0 {
        return effective_scroll;
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
    let static_status =
        static_content.map(|(label, _)| format!(" {} · Esc returns", header_value(label)));
    let status = if let Some(hint) = app.hydra_hint.as_deref() {
        hint
    } else if let Some(notice) = app.minibuffer_notice.as_deref() {
        notice
    } else if app.buffer_view == BufferView::Messages {
        " *Messages* · Esc returns to conversation"
    } else if let Some(status) = static_status.as_deref() {
        status
    } else if app.quit_armed {
        " Press Ctrl-C again to quit; another key cancels. Draft will not be saved."
    } else if app.composer.limit_reached() {
        " Draft limit: 1 MiB. Insertion rejected; draft retained."
    } else if app.runtime_receipt_open {
        " Runtime: Esc/Ctrl-C return  Enter submit draft"
    } else if app.workspace.view().is_some() {
        " Workspace: PageUp/PageDown scroll  Esc/Ctrl-C return  Enter submit draft"
    } else if ui.focus.as_str() == "commands" {
        " Commands: Enter open  M-x search  Ctrl-Q quit"
    } else if ui.focus.as_str() == "transcript" {
        " Transcript: scroll to browse  M-x commands  Ctrl-Q quit"
    } else if ui.focus.as_str() != "composer" {
        " Extension: Enter activate  M-x commands  Ctrl-Q quit"
    } else {
        " Message: Enter send  Ctrl-R search  M-x commands  Ctrl-Q quit"
    };
    if parts[3].height == 0 {
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
