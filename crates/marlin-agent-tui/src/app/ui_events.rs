// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Dispatch Scheme UI requests and minibuffer navigation.

use super::runner::publish_error;
use super::{
    App, MessageEvent, MessageSeverity, MessageSource, UiBrowseDirection, UiEvent, UiHost,
};

pub(super) fn command_menu_active(app: &App, ui: &UiHost<'_>) -> bool {
    app.command_draft.is_some()
        && app.composer.cursor_at_end()
        && ui.accepts_text(true)
        && !app.command_candidates.is_empty()
}

pub(super) fn browse_menu(
    app: &mut App,
    ui: &UiHost<'_>,
    direction: UiBrowseDirection,
    steps: u16,
) {
    let anchor = app.menu_navigation_anchor();
    match ui.browse_command_steps(
        &app.command_lookup_query(),
        &app.command_candidates,
        anchor.as_ref(),
        direction,
        steps,
        app.pending.is_some(),
    ) {
        Ok(choice) => {
            app.command_selection = choice.map(|name| (app.composer.text().to_owned(), name));
            app.command_selection_locked = app.command_selection.is_some();
        }
        Err(error) => publish_error(app, MessageSource::Ui, "Command menu failed", &error),
    }
}

pub(super) fn apply_ui_request(app: &mut App, ui: &mut UiHost<'_>) {
    if let Some(name) = app.command_history_request.take()
        && let Err(error) = ui.dispatch(UiEvent::RecordCommand(name))
    {
        publish_error(app, MessageSource::Ui, "Command history failed", &error);
    }
    if let Some(event) = app.ui_request.take() {
        let summary = match event {
            UiEvent::SelectBuffer(_) => "Buffer selection failed",
            _ => "Profile selection failed",
        };
        match ui.dispatch(event) {
            Ok(_) => {
                app.transcript_policy = Some(ui.view().layout.transcript_policy.clone());
                if let Err(error) = app.sync_window(ui.view()) {
                    app.publish_message(MessageEvent {
                        source: MessageSource::Ui,
                        severity: MessageSeverity::Error,
                        summary,
                        detail: error,
                    });
                }
            }
            Err(error) => publish_error(app, MessageSource::Ui, summary, &error),
        }
    }
    if let Some(id) = app.transient_request.take() {
        if let Err(error) = ui.enter_transient(&id) {
            publish_error(app, MessageSource::Ui, "Command group failed", &error);
        }
        app.transient_hint = ui.transient_hint().cloned();
    }
}
