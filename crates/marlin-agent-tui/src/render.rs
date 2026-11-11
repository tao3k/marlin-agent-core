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

pub(crate) fn draw(
    frame: &mut ratatui::Frame<'_>,
    app: &App,
    cache: &mut TranscriptViewportCache,
    anchor: &mut ScrollAnchor,
) -> usize {
    let area = frame.area();
    let parts = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(5),
            Constraint::Length(1),
        ])
        .split(area);
    frame.render_widget(
        Paragraph::new(if app.workspace.view().is_some() {
            " Marlin · workspace · Esc to return"
        } else {
            " Marlin · model gateway"
        })
        .style(
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
    let title = if app.command_draft.is_some() {
        " Command "
    } else {
        " Message "
    };
    let block = Block::default().borders(Borders::ALL).title(title);
    let inner = block.inner(parts[2]);
    frame.render_widget(block, parts[2]);
    if let Some(view) = app.composer.view(inner.width, inner.height) {
        frame.render_widget(Paragraph::new(view.rows.join("\n")), inner);
        frame.set_cursor_position((inner.x + view.cursor.0, inner.y + view.cursor.1));
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
        " Enter send  Ctrl-N newline  Ctrl-O commands  Esc/Ctrl-C cancel  Ctrl-Q quit"
    };
    frame.render_widget(Paragraph::new(status), parts[3]);
    effective_scroll
}
