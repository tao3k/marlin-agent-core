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
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};
use std::collections::HashMap;
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

fn minibuffer_footer_rows(app: &App, base: u16) -> u16 {
    app.transient_hint.as_ref().map_or(base, |hint| {
        base.max(u16::try_from(hint.intrinsic_rows()).unwrap_or(u16::MAX))
    })
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
        .saturating_sub(minibuffer_footer_rows(app, frame.footer_rows) + 1 + transcript_rows)
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

pub(crate) fn frame_regions(
    app: &App,
    ui: &UiWorkbench,
    area: Rect,
) -> Vec<super::ui_elements::HitRegion> {
    use super::ui_elements::{Extent, layout_regions};
    let command = app.command_draft.is_some();
    let policy = if command {
        &ui.layout.command_frame
    } else {
        &ui.layout.message_frame
    };
    let transcript = policy
        .min_transcript_rows
        .min(area.height.saturating_sub(2));
    let editor = chatbox_height(app, &ui.layout, area.width, area.height);
    let footer = minibuffer_footer_rows(app, policy.footer_rows)
        .min(
            area.height
                .saturating_sub(transcript.saturating_add(editor).saturating_add(1)),
        )
        .max(policy.footer_rows.min(area.height));
    let spare = area
        .height
        .saturating_sub(footer.saturating_add(transcript).saturating_add(editor));
    let status = u16::from(
        spare > 0
            && ui.regions.iter().any(|region| match region {
                UiRegion::Status(items) => items
                    .iter()
                    .any(|item| item.id.as_str() == "modeline/left/brand"),
                _ => false,
            }),
    );
    let candidates = if command {
        ui.layout.max_command_rows.min(spare.saturating_sub(status))
    } else {
        0
    };
    let available = spare.saturating_sub(candidates).saturating_sub(status);
    let extensions = if policy.extensions {
        let count = ui
            .regions
            .iter()
            .filter(|region| {
                matches!(
                    region,
                    UiRegion::Extension {
                        focusable: true,
                        ..
                    }
                )
            })
            .count();
        (count.min(u16::MAX as usize) as u16).min(available)
    } else {
        0
    };
    let mut result = Vec::with_capacity(6);
    layout_regions(
        &ui.layout.frame_tree,
        area,
        &|id| match id.as_str() {
            "transcript" => Extent::Flex(1),
            "extensions" => Extent::Cells(extensions),
            "modeline" => Extent::Cells(status),
            "editor" => Extent::Cells(editor),
            "candidates" => Extent::Cells(candidates),
            "minibuffer" => Extent::Cells(footer),
            _ => Extent::Cells(0),
        },
        &mut result,
    );
    result
}

pub(crate) fn mouse_over_minibuffer(
    regions: &[super::ui_elements::HitRegion],
    column: u16,
    row: u16,
) -> bool {
    column > 0
        && row > 0
        && regions.iter().any(|region| {
            matches!(region.id.as_str(), "editor" | "candidates" | "minibuffer")
                && column > region.area.x
                && column - 1 < region.area.right()
                && row > region.area.y
                && row - 1 < region.area.bottom()
        })
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
        let catalog: HashMap<_, _> = ui
            .commands()
            .iter()
            .map(|command| (&command.name, command))
            .collect();
        app.command_candidates
            .iter()
            .filter_map(|name| catalog.get(name).copied())
            .collect()
    } else {
        Vec::new()
    };
    let regions = frame_regions(app, ui, area);
    let part = |name: &str| {
        regions
            .iter()
            .find(|region| region.id.as_str() == name)
            .map_or(Rect::default(), |region| region.area)
    };
    let parts = [
        part("transcript"),
        part("extensions"),
        part("modeline"),
        part("editor"),
        part("candidates"),
        part("minibuffer"),
    ];
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
        BufferView::Named(buffer) => ui.regions.iter().find_map(|region| match region {
            UiRegion::Extension {
                id,
                label,
                content,
                focusable: false,
            } if id.as_str() == buffer.as_str() => Some((
                label.as_str(),
                app.dynamic_buffer_content
                    .get(buffer)
                    .map_or(content.as_slice(), Vec::as_slice),
            )),
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
                    selected_index.map_or(0, |index| index + 1),
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
    let templates = match ui.layout.frame_tree.region_child("candidates") {
        Some(marlin_agent_ui::UiElement::Rows(templates)) => templates.as_slice(),
        _ => &[],
    };
    for (row, command) in suggestions.iter().skip(start).take(visible).enumerate() {
        let line =
            ratatui::layout::Rect::new(parts[4].x, parts[4].y + row as u16, parts[4].width, 1);
        let active_row = active == Some(&command.name);
        if let Some(template) = templates.get(usize::from(!active_row)) {
            let name = format!(
                "{:<name_width$}",
                menu_display_name(command.name.as_str(), menu_prefix)
            );
            let description = header_value(&command.description);
            super::ui_elements::render_template(
                frame,
                line,
                template,
                &ui.theme,
                &[("name", &name), ("description", &description)],
            );
        }
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
    let static_status =
        static_content.map(|(label, _)| format!("{} · Esc returns", header_value(label)));
    if app.history_search.is_none()
        && let Some(hint) = &app.transient_hint
    {
        super::ui_elements::render_element(frame, parts[5], hint, &ui.theme);
        return effective_scroll;
    }
    let prefix = if parts[3].height == 0 {
        let extras = status_items
            .into_iter()
            .flatten()
            .filter(|item| !item.id.as_str().starts_with("modeline/"))
            .take(2)
            .map(|item| display_value(&item.label, 16))
            .collect::<Vec<_>>();
        if extras.is_empty() {
            " › ".to_owned()
        } else {
            format!(" › {} · ", extras.join(" · "))
        }
    } else {
        " ".to_owned()
    };
    if let Some(view) = ui.layout.frame_tree.region_child("minibuffer") {
        let facts = |fact: &marlin_agent_ui::UiComponentId| match fact.as_str() {
            "searching" => app
                .history_search
                .as_ref()
                .is_some_and(|search| search.is_searching()),
            "search-match" => app
                .history_search
                .as_ref()
                .is_some_and(|search| search.preview().is_some()),
            "search-no-match" => app.history_search.is_some(),
            "notice" => app.minibuffer_notice.is_some(),
            "messages" => app.buffer_view == BufferView::Messages,
            "static-buffer" => static_status.is_some(),
            "quit-armed" => app.quit_armed,
            "draft-limit" => app.composer.limit_reached(),
            "runtime" => app.runtime_receipt_open,
            "workspace" => app.workspace.view().is_some(),
            "commands-focus" => ui.focus.as_str() == "commands",
            "transcript-focus" => ui.focus.as_str() == "transcript",
            "extension-focus" => ui.focus.as_str() != "composer",
            _ => false,
        };
        let bindings = [
            ("prefix", prefix.as_str()),
            (
                "notice",
                app.minibuffer_notice.as_deref().unwrap_or("").trim_start(),
            ),
            ("static-status", static_status.as_deref().unwrap_or("")),
        ];
        let inputs = super::ui_elements::UiRenderInputs {
            theme: &ui.theme,
            offset: &|_| 0,
            facts: &facts,
            bindings: &bindings,
        };
        if let Some(search) = &app.history_search {
            if let Some(cursor) = super::ui_elements::render_prompt(
                frame,
                parts[5],
                view,
                &inputs,
                "query",
                search.query(),
            ) {
                frame.set_cursor_position(cursor);
            }
            return effective_scroll;
        }
        super::ui_elements::render_with_facts(frame, parts[5], view, &mut Vec::new(), &inputs);
    }
    if parts[3].height == 0
        && ui.focus.as_str() == "composer"
        && parts[5].width > 2
        && app.transient_hint.is_none()
    {
        frame.set_cursor_position((parts[5].x + 2, parts[5].y));
    }
    effective_scroll
}
