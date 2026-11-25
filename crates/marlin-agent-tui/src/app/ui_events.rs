// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Dispatch Scheme UI requests and minibuffer navigation.

use super::publish_error;
use super::{
    App, Key, MessageEvent, MessageSeverity, MessageSource, TerminalInput, UiBrowseDirection,
    UiEvent, UiHost, UiKeyAction,
};
use crate::ui_elements::{HitRegion, hit_test_command};
use crate::{command::Command, config};
use marlin_agent_ui::{
    UiCommandArguments, UiCommandCapability, UiKeyMode, UiPackageCommandSource, UiPackageEvent,
};

pub(super) fn command_menu_active(app: &App, ui: &UiHost<'_>) -> bool {
    app.command_draft.is_some()
        && app.composer.cursor_at_end()
        && ui.accepts_text(true)
        && !app.command_candidates.is_empty()
}

pub(super) fn route_terminal_key(
    app: &mut App,
    ui: &mut UiHost<'_>,
    key: Key,
) -> Result<UiKeyAction, marlin_agent_ui::UiProjectionError> {
    let command_active = app.command_draft.is_some();
    let action = ui.route_key(key, command_active, command_active, app.key_mode);
    sync_key_feedback(app, ui);
    action
}

pub(super) fn release_interrupted_key_sequence(
    app: &mut App,
    ui: &mut UiHost<'_>,
    input: Option<&TerminalInput>,
) {
    if (app.tool_review_prompt.is_some()
        || app.history_search.is_some()
        || app.helm.is_some()
        || matches!(
            input,
            Some(TerminalInput::Paste(_) | TerminalInput::MouseClick(_, _))
        ))
        && ui.cancel_key_sequence()
    {
        sync_key_feedback(app, ui);
    }
}

fn sync_key_feedback(app: &mut App, ui: &UiHost<'_>) {
    app.echo_key_sequence(ui.key_feedback());
    app.set_key_hint(ui.keypad_hint(app.key_mode), ui.transient_hint());
}

pub(super) fn apply_editor_mode(app: &mut App, ui: &mut UiHost<'_>, mode: UiKeyMode) {
    match mode {
        UiKeyMode::Normal => {
            if app.key_mode == UiKeyMode::Insert {
                app.composer.normal_cursor();
            }
            app.key_mode = mode;
            app.editor_keyboard_active = false;
        }
        UiKeyMode::Insert => match ui.focus_editor() {
            Ok(()) => app.enter_insert_mode(false),
            Err(error) => publish_error(app, MessageSource::Ui, "Input focus failed", &error),
        },
    }
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

/// Prepare a painted command for submission, or select a list row.
pub(super) fn route_frame_click(
    app: &mut App,
    ui: &mut UiHost<'_>,
    regions: &[HitRegion],
    column: u16,
    row: u16,
) -> bool {
    let Some((x, y)) = column.checked_sub(1).zip(row.checked_sub(1)) else {
        return false;
    };
    if let Some(target) = hit_test_command(regions, x, y) {
        let command = app
            .command_descriptor(target.as_str())
            .filter(|command| command.arguments == UiCommandArguments::None)
            .map(|command| command.name.clone());
        return app.tool_review_prompt.is_none()
            && app.history_search.is_none()
            && app.helm.is_none()
            && command.is_some_and(|name| app.prepare_bound_command(&name));
    }
    select_clicked_list_row(app, ui, x, y);
    false
}

/// Select a painted list row without committing a command or review decision.
fn select_clicked_list_row(app: &mut App, ui: &mut UiHost<'_>, x: u16, y: u16) {
    let clicked = app
        .list_state
        .borrow()
        .hit(x, y)
        .map(|hit| (hit.list.clone(), hit.key.clone(), hit.selects_command));
    let Some((list, key, selects_command)) = clicked else {
        return;
    };
    if !selects_command {
        if let Err(error) = ui.dispatch(UiEvent::SelectListRow { list, row: key }) {
            publish_error(app, MessageSource::Ui, "List selection failed", &error);
        }
        return;
    }
    if let Some(prompt) = app.tool_review_prompt.as_mut() {
        if let Some(index) = prompt
            .menu
            .choices
            .iter()
            .position(|choice| choice.decision.selection_id() == key.as_str())
        {
            prompt.selected = index;
        }
        return;
    }
    if command_menu_active(app, ui)
        && app.command_query_is_current()
        && let Some(target) = app
            .command_candidates
            .iter()
            .position(|name| name.as_str() == key.as_str())
        && let Some(current) = app.menu_navigation_anchor().and_then(|name| {
            app.command_candidates
                .iter()
                .position(|candidate| candidate == &name)
        })
        && target != current
        && let Ok(steps) = u16::try_from(target.abs_diff(current))
    {
        let direction = if target < current {
            UiBrowseDirection::Previous
        } else {
            UiBrowseDirection::Next
        };
        browse_menu(app, ui, direction, steps);
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
        app.set_key_hint(None, ui.transient_hint());
    }
}

pub(super) fn stage_ui_package(
    app: &mut App,
    ui: &mut UiHost<'_>,
    pending: &mut Option<config::TuiUiPackageConfig>,
) -> bool {
    let Some(package) = pending.take() else {
        return false;
    };
    let Some(directory) = package.directory.to_str() else {
        return false;
    };
    apply_package_event(
        app,
        ui,
        UiPackageEvent::Catalog {
            directory,
            module: &package.module,
            export: &package.export,
        },
    )
}

pub(super) fn activate_deferred_command(
    app: &mut App,
    ui: &mut UiHost<'_>,
    source: UiPackageCommandSource,
) -> bool {
    if app.command_draft.is_none() || app.pending.is_some() || app.workspace.is_pending() {
        return true;
    }
    let input = app.command_lookup_query();
    let Some(command) = app
        .command_descriptor(&input)
        .filter(|command| command.capability == UiCommandCapability::Deferred)
    else {
        return true;
    };
    if Command::arguments(command, &input).is_none() {
        return false;
    }
    let command = command.name.clone();
    apply_package_event(
        app,
        ui,
        UiPackageEvent::Execute {
            command: &command,
            source,
        },
    )
}

fn apply_package_event(app: &mut App, ui: &mut UiHost<'_>, event: UiPackageEvent<'_>) -> bool {
    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        ui.package_event(event)?;
        app.sync_window(ui.view())?;
        Ok(())
    })();
    if let Err(error) = result {
        publish_error(app, MessageSource::Ui, "UI package event failed", &error);
        return false;
    }
    app.set_commands(ui.view());
    app.transcript_policy = Some(ui.view().layout.transcript_policy.clone());
    true
}

/// Handle one terminal input event while the interactive Helm overlay is open.
/// Returns `Ok(true)` if the main event loop should exit (`Ctrl-q` or channel closed).
pub(super) fn handle_helm_input(
    app: &mut App,
    ui: &mut UiHost<'_>,
    runtime: &tokio::runtime::Handle,
    received: Result<std::io::Result<TerminalInput>, std::sync::mpsc::RecvTimeoutError>,
) -> std::io::Result<bool> {
    let Some(helm) = app.helm.as_mut() else {
        return Ok(false);
    };
    let mut accepted: Option<super::HelmAction> = None;
    let mut close_helm = false;
    match received {
        Ok(Ok(TerminalInput::Key(Key::Ctrl('q')))) => return Ok(true),
        Ok(Ok(TerminalInput::Key(Key::Ctrl('c' | 'g')))) => {
            close_helm = true;
            app.disarm_exit();
        }
        Ok(Ok(TerminalInput::Key(Key::Esc))) => {
            if helm.action_mode {
                helm.toggle_action_mode();
            } else {
                close_helm = true;
                app.disarm_exit();
            }
        }
        Ok(Ok(TerminalInput::Key(Key::Char('\n')))) => {
            let (should_close, action) = helm.accept(false);
            accepted = action;
            close_helm = should_close;
        }
        Ok(Ok(TerminalInput::Key(Key::Ctrl('e')))) => {
            let (should_close, action) = helm.accept(true);
            accepted = action;
            close_helm = should_close;
        }
        Ok(Ok(TerminalInput::Key(Key::Up | Key::Ctrl('p')))) => helm.move_selection(-1),
        Ok(Ok(TerminalInput::Key(Key::Down | Key::Ctrl('n')))) => helm.move_selection(1),
        Ok(Ok(TerminalInput::Key(Key::PageUp | Key::Alt('v')))) => helm.move_selection(-8),
        Ok(Ok(TerminalInput::Key(Key::PageDown | Key::Ctrl('v')))) => helm.move_selection(8),
        Ok(Ok(TerminalInput::Key(Key::Null | Key::Ctrl(' ') | Key::Alt('m')))) => {
            helm.toggle_mark_and_advance();
        }
        Ok(Ok(TerminalInput::Key(Key::Alt('a')))) => helm.toggle_mark_all(),
        Ok(Ok(TerminalInput::Key(Key::Char('\t')))) => helm.toggle_action_mode(),
        Ok(Ok(TerminalInput::Key(Key::Ctrl('z')))) => helm.toggle_follow_preview(),
        Ok(Ok(TerminalInput::Key(Key::Ctrl('o')))) => helm.jump_next_source(),
        Ok(Ok(TerminalInput::Key(Key::Ctrl('s')))) => helm.cycle_source_filter(),
        Ok(Ok(TerminalInput::Key(Key::Ctrl('l')))) => helm.refresh_index(runtime),
        Ok(Ok(TerminalInput::Key(Key::Backspace))) => helm.backspace(),
        Ok(Ok(TerminalInput::Key(Key::Char(c)))) if !c.is_control() => {
            helm.insert_text(&c.to_string());
        }
        Ok(Ok(TerminalInput::Paste(text))) => helm.insert_text(&text),
        Ok(Err(err)) => return Err(err),
        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return Ok(true),
        _ => {}
    }
    if close_helm {
        app.helm = None;
    }
    if let Some(action) = accepted {
        app.apply_helm_action(action);
        apply_ui_request(app, ui);
    }
    Ok(false)
}
