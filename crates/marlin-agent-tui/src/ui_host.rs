// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Admission of functional Scheme UI states into the terminal owner.

use super::command::CommandKind;
use gerbil_scheme::GerbilRuntime;
#[cfg(feature = "linked-ui")]
use marlin_agent_ui::UiLinkedExports;
use marlin_agent_ui::{
    UiBrowseDirection, UiCommandName, UiEvent, UiIntent, UiModule, UiNativeSession, UiOpenFacts,
    UiProjectionError, UiWorkbench,
};
use termion::event::Key;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum UiKeyAction {
    Handled,
    Submit,
    OpenCommands,
    Scroll(bool),
    Unhandled,
}

pub(crate) struct UiHost<'runtime> {
    module: UiModule<'runtime>,
    session: UiNativeSession<'runtime>,
    view: UiWorkbench,
}

impl<'runtime> UiHost<'runtime> {
    #[cfg(not(feature = "linked-ui"))]
    pub(crate) fn open(
        runtime: &'runtime GerbilRuntime,
        directory: &str,
        facts: &UiOpenFacts<'_, '_>,
    ) -> Result<Self, UiProjectionError> {
        let module = UiModule::resolve(runtime, directory)?;
        Self::from_module(module, facts)
    }

    /// Open the same UI protocol from the selected linked programme.
    ///
    /// # Safety
    /// The caller must supply exports from `runtime`'s live AOT graph.
    #[cfg(feature = "linked-ui")]
    pub(crate) unsafe fn open_linked(
        runtime: &'runtime GerbilRuntime,
        exports: UiLinkedExports,
        facts: &UiOpenFacts<'_, '_>,
    ) -> Result<Self, UiProjectionError> {
        // SAFETY: the linked runtime and exports are selected by the binary.
        let module = unsafe { UiModule::resolve_linked(runtime, exports)? };
        Self::from_module(module, facts)
    }

    fn from_module(
        module: UiModule<'runtime>,
        facts: &UiOpenFacts<'_, '_>,
    ) -> Result<Self, UiProjectionError> {
        let started = std::time::Instant::now();
        let trace = |stage: &str| {
            if std::env::var_os("MARLIN_TUI_STARTUP_TRACE").is_some() {
                eprintln!("marlin-tui UI open {stage}: {:?}", started.elapsed());
            }
        };
        trace("module resolved");
        let session = module.open(facts)?;
        trace("session constructed");
        let view = module.project(&session)?;
        trace("session projected");
        admit_terminal_view(&view)?;
        trace("view admitted");
        Ok(Self {
            module,
            session,
            view,
        })
    }

    pub(crate) fn view(&self) -> &UiWorkbench {
        &self.view
    }

    pub(crate) fn refresh_labels(
        &mut self,
        model: &str,
        session: &str,
    ) -> Result<(), UiProjectionError> {
        if self.view.model_label == model && self.view.session_label == session {
            return Ok(());
        }
        let successor = self.module.refresh_labels(&self.session, model, session)?;
        let candidate = self.module.project(&successor)?;
        admit_terminal_view(&candidate)?;
        if candidate.model_label != model
            || candidate.session_label != session
            || candidate.regions != self.view.regions
            || candidate.focus != self.view.focus
        {
            return Err(UiProjectionError::Invalid(
                "UI label refresh changed the workbench",
            ));
        }
        self.session = successor;
        self.view = candidate;
        Ok(())
    }

    pub(crate) fn accepts_text(&self, command_active: bool) -> bool {
        command_active || self.view.focus.as_str() == "composer"
    }

    pub(crate) fn complete_command(&self, draft: &str) -> Option<String> {
        complete_command(&self.view, draft)
    }

    pub(crate) fn browse_command(
        &self,
        query: &str,
        previous: Option<&UiCommandName>,
        direction: UiBrowseDirection,
    ) -> Result<Option<UiCommandName>, UiProjectionError> {
        let choice = self
            .module
            .browse_command(&self.session, query, previous, direction)?;
        admit_command_choice(&self.view, query, choice.as_ref())?;
        Ok(choice)
    }

    pub(crate) fn complete_selected_command(
        &self,
        query: &str,
        selected: &UiCommandName,
    ) -> Option<String> {
        let command =
            self.view.commands().iter().find(|command| {
                &command.name == selected && selected.as_str().starts_with(query)
            })?;
        let mut result = command.name.as_str().to_owned();
        if command.accepts_arguments {
            result.push(' ');
        }
        Some(result)
    }

    pub(crate) fn has_command_prefix(&self, draft: &str) -> bool {
        UiCommandName::is_valid_prefix(draft)
            && self
                .view
                .commands()
                .iter()
                .any(|command| command.name.as_str().starts_with(draft))
    }

    pub(crate) fn route_key(
        &mut self,
        key: Key,
        command_active: bool,
        draft_is_command: bool,
    ) -> Result<UiKeyAction, UiProjectionError> {
        match key {
            Key::Char('\t') if !command_active => {
                self.dispatch(UiEvent::NextFocus)?;
                Ok(UiKeyAction::Handled)
            }
            Key::BackTab if !command_active => {
                self.dispatch(UiEvent::PreviousFocus)?;
                Ok(UiKeyAction::Handled)
            }
            Key::Char('\n') if command_active => Ok(UiKeyAction::Submit),
            Key::Char('\n') if self.view.focus.as_str() == "commands" => {
                Ok(UiKeyAction::OpenCommands)
            }
            Key::Char('\n') if self.view.focus.as_str() == "composer" => {
                if !draft_is_command {
                    self.dispatch(UiEvent::Submit)?;
                }
                Ok(UiKeyAction::Submit)
            }
            Key::Char('\n') if self.view.focused_extension() => {
                self.dispatch(UiEvent::Submit)?;
                Ok(UiKeyAction::Handled)
            }
            Key::Char('\n') => Ok(UiKeyAction::Handled),
            Key::Up | Key::Down if !command_active && self.view.focus.as_str() == "transcript" => {
                let up = key == Key::Up;
                self.dispatch(if up {
                    UiEvent::ScrollUp
                } else {
                    UiEvent::ScrollDown
                })?;
                Ok(UiKeyAction::Scroll(up))
            }
            _ => Ok(UiKeyAction::Unhandled),
        }
    }

    /// Publish a successor only after its projection and intent agree.
    pub(crate) fn dispatch(&mut self, event: UiEvent) -> Result<UiIntent, UiProjectionError> {
        let (successor, intent) = self.module.dispatch(&self.session, event)?;
        let candidate = self.module.project(&successor)?;
        admit_transition(&self.view, &candidate, event, &intent)?;
        self.session = successor;
        self.view = candidate;
        Ok(intent)
    }
}

fn admit_command_choice(
    view: &UiWorkbench,
    query: &str,
    choice: Option<&UiCommandName>,
) -> Result<(), UiProjectionError> {
    if let Some(name) = choice
        && !view
            .commands()
            .iter()
            .any(|command| &command.name == name && name.as_str().starts_with(query))
    {
        return Err(UiProjectionError::Invalid(
            "command choice is not in the admitted menu",
        ));
    }
    Ok(())
}

fn complete_command(view: &UiWorkbench, draft: &str) -> Option<String> {
    if !UiCommandName::is_valid_prefix(draft) {
        return None;
    }
    let mut candidates = view
        .commands()
        .iter()
        .filter(|command| command.name.as_str().starts_with(draft));
    let first = candidates.next()?;
    let exact = view
        .commands()
        .iter()
        .find(|command| command.name.as_str() == draft);
    let selected = match exact {
        Some(command) => command,
        None if candidates.next().is_none() => first,
        None => return None,
    };
    let mut completed = selected.name.as_str().to_owned();
    if selected.accepts_arguments {
        completed.push(' ');
    }
    (completed != draft).then_some(completed)
}

fn admit_terminal_view(view: &UiWorkbench) -> Result<(), UiProjectionError> {
    for command in view.commands() {
        let Some(kind) = CommandKind::from_name(command.name.as_str()) else {
            return Err(UiProjectionError::Invalid(
                "command has no registered terminal handler",
            ));
        };
        if command.accepts_arguments != kind.accepts_arguments() {
            return Err(UiProjectionError::Invalid(
                "command argument description disagrees with handler",
            ));
        }
    }
    let focus_supported = matches!(view.focus.as_str(), "transcript" | "composer" | "commands")
        || view.focused_extension();
    if !focus_supported {
        return Err(UiProjectionError::Invalid(
            "terminal focus is not supported",
        ));
    }
    Ok(())
}

fn admit_transition(
    previous: &UiWorkbench,
    next: &UiWorkbench,
    event: UiEvent,
    intent: &UiIntent,
) -> Result<(), UiProjectionError> {
    admit_terminal_view(next)?;
    previous.admit_successor(next, event, intent)
}

#[cfg(all(test, not(feature = "linked-ui")))]
#[path = "../tests/unit/ui_host_source.rs"]
mod tests;
