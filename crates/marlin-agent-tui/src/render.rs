// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Terminal frame projection for the active conversation or workspace view.

use super::{
    app::App,
    viewport::{ScrollAnchor, TranscriptViewportCache},
};
use marlin_agent_ui::{UiCommandName, UiRegion, UiWorkbench};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph},
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

// Configuration and session identifiers may contain control characters. Keep
// the header bounded and prevent them from becoming terminal control input.
fn header_value(value: &str) -> String {
    value
        .chars()
        .take(128)
        .map(|ch| if ch.is_control() { '�' } else { ch })
        .collect()
}

pub(crate) fn draw(
    frame: &mut ratatui::Frame<'_>,
    app: &App,
    ui: &UiWorkbench,
    cache: &mut TranscriptViewportCache,
    anchor: &mut ScrollAnchor,
) -> usize {
    let area = frame.area();
    // The transcript can end in a separator row, so reserve two rows when
    // possible. Bound measurement for large drafts.
    let transcript_rows = if area.height >= 7 { 2 } else { 1 };
    let composer_limit = area.height.saturating_sub(2 + transcript_rows).min(10);
    let composer = app
        .history_search
        .as_ref()
        .and_then(|search| search.preview())
        .unwrap_or(&app.composer);
    let input_rows = composer.preferred_rows(
        area.width.saturating_sub(2),
        composer_limit.saturating_sub(2),
    );
    let composer_height = if composer_limit >= 3 {
        input_rows.saturating_add(2)
    } else {
        composer_limit
    };
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
        .saturating_sub(2 + transcript_rows + composer_height);
    let palette_height = (suggestions.len() as u16).min(4).min(spare);
    let remaining = spare - palette_height;
    let focused_extension = extensions.iter().any(|(id, _)| *id == &ui.focus);
    let status_height =
        u16::from(status_items.is_some() && remaining > u16::from(focused_extension));
    let extension_height = (extensions.len() as u16).min(remaining - status_height);
    let parts = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(composer_height),
            Constraint::Length(palette_height),
            Constraint::Length(extension_height),
            Constraint::Length(status_height),
            Constraint::Length(1),
        ])
        .split(area);
    let heading = if app.workspace.view().is_some() {
        " Marlin · workspace · Esc to return".to_owned()
    } else {
        let endpoint = app.endpoint();
        let session = app
            .session_id()
            .map_or_else(|| "temporary".to_owned(), |id| header_value(id.as_str()));
        format!(
            " Marlin · model gateway · {} · {}/{} · {}",
            app.activity_label(),
            header_value(endpoint.provider.as_str()),
            header_value(endpoint.model.as_str()),
            session,
        )
    };
    frame.render_widget(
        Paragraph::new(heading).style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        parts[0],
    );
    let mut render_view = |viewport: &super::viewport::TranscriptViewport| {
        viewport.render(parts[1], frame.buffer_mut());
        viewport.scroll_up
    };
    let effective_scroll = match app.workspace.view() {
        Some(view) => cache.with_top_view(
            &view.transcript,
            parts[1].width,
            parts[1].height,
            view.scroll_from_top,
            &mut render_view,
        ),
        None => {
            let pending = app.pending.is_some();
            let scroll = anchor.adjust(
                &app.transcript,
                pending,
                parts[1].width,
                parts[1].height,
                app.scroll_up,
            );
            cache.with_view(
                &app.transcript,
                pending,
                parts[1].width,
                parts[1].height,
                scroll,
                &mut render_view,
            )
        }
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
                Style::default().fg(Color::Cyan)
            } else {
                Style::default()
            },
        );
    let inner = block.inner(parts[2]);
    frame.render_widget(block, parts[2]);
    if let Some(view) = composer.view(inner.width, inner.height) {
        frame.render_widget(Paragraph::new(view.rows.join("\n")), inner);
        if app.history_search.is_none()
            && (ui.focus.as_str() == "composer" || app.command_draft.is_some())
        {
            frame.set_cursor_position((inner.x + view.cursor.0, inner.y + view.cursor.1));
        }
    }
    let selected = app
        .selected_command()
        .filter(|name| suggestions.iter().any(|command| &command.name == *name));
    let selected_index =
        selected.and_then(|name| suggestions.iter().position(|command| &command.name == name));
    let visible = usize::from(parts[3].height);
    let start = selected_index.map_or(0, |index| index.saturating_add(1).saturating_sub(visible));
    for (row, command) in suggestions.iter().skip(start).take(visible).enumerate() {
        let line =
            ratatui::layout::Rect::new(parts[3].x, parts[3].y + row as u16, parts[3].width, 1);
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
        let style = Style::default().fg(Color::Cyan);
        let style = if selected == Some(&command.name) {
            style.add_modifier(Modifier::BOLD)
        } else {
            style
        };
        frame.render_widget(Paragraph::new(text).style(style), line);
    }
    let visible = usize::from(parts[4].height);
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
                ratatui::layout::Rect::new(parts[4].x, parts[4].y + row as u16, parts[4].width, 1);
            let style = if is_focused {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            frame.render_widget(Paragraph::new(text).style(style), line);
        }
    }
    if let Some(items) = status_items.filter(|_| parts[5].height > 0) {
        let text = items
            .iter()
            .map(|item| header_value(&item.label))
            .collect::<Vec<_>>()
            .join(" · ");
        frame.render_widget(Paragraph::new(format!(" {text}")), parts[5]);
    }
    if let Some(search) = &app.history_search {
        let prefix = if search.is_searching() {
            " Searching: "
        } else if search.preview().is_some() {
            " Search: "
        } else {
            " No match: "
        };
        let available = usize::from(parts[6].width)
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
        frame.render_widget(Paragraph::new(status.clone()), parts[6]);
        if parts[6].width > 0 && parts[6].height > 0 {
            let column =
                UnicodeWidthStr::width(status.as_str()).min(usize::from(parts[6].width - 1));
            frame.set_cursor_position((parts[6].x + column as u16, parts[6].y));
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
    frame.render_widget(Paragraph::new(status), parts[6]);
    effective_scroll
}
