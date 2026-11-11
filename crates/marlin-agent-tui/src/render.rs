// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Terminal frame projection for the active conversation or workspace view.

use super::{
    app::App,
    viewport::{ScrollAnchor, TranscriptViewportCache},
};
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
    let parts = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(composer_height),
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
    let block = Block::default().borders(Borders::ALL).title(title);
    let inner = block.inner(parts[2]);
    frame.render_widget(block, parts[2]);
    if let Some(view) = composer.view(inner.width, inner.height) {
        frame.render_widget(Paragraph::new(view.rows.join("\n")), inner);
        if app.history_search.is_none() {
            frame.set_cursor_position((inner.x + view.cursor.0, inner.y + view.cursor.1));
        }
    }
    if let Some(search) = &app.history_search {
        let prefix = if search.is_searching() {
            " Searching: "
        } else if search.preview().is_some() {
            " Search: "
        } else {
            " No match: "
        };
        let available = usize::from(parts[3].width)
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
        frame.render_widget(Paragraph::new(status.clone()), parts[3]);
        if parts[3].width > 0 && parts[3].height > 0 {
            let column =
                UnicodeWidthStr::width(status.as_str()).min(usize::from(parts[3].width - 1));
            frame.set_cursor_position((parts[3].x + column as u16, parts[3].y));
        }
        return effective_scroll;
    }
    let status = if app.quit_armed {
        " Press Ctrl-C again to quit; another key cancels. Draft will not be saved."
    } else if app.composer.limit_reached() {
        " Draft limit: 1 MiB. Insertion rejected; draft retained."
    } else if app.command_draft.is_some() {
        " Enter run command  Esc cancel  Ctrl-O return to draft"
    } else if app.workspace.view().is_some() {
        " Workspace: PageUp/PageDown scroll  Esc/Ctrl-C return  Enter submit draft"
    } else {
        " Enter send  Ctrl-N newline  Ctrl-R search  Ctrl-O commands  Ctrl-Q quit"
    };
    frame.render_widget(Paragraph::new(status), parts[3]);
    effective_scroll
}
