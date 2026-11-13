// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Admission of functional Scheme UI states into the terminal owner.

use super::command::Command;
use gerbil_scheme::GerbilRuntime;
#[cfg(feature = "linked-ui")]
use marlin_agent_ui::UiLinkedExports;
use marlin_agent_ui::{
    UiBrowseDirection, UiCommandName, UiEditorAction, UiEvent, UiIntent, UiKeyCommand, UiModule,
    UiNativeSession, UiOpenFacts, UiProjectionError, UiTransient, UiTransientForeign,
    UiTransientId, UiWorkbench,
};
use std::collections::HashSet;
use termion::event::Key;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum UiKeyAction {
    Handled,
    Submit,
    OpenCommands,
    Cancel,
    Edit(UiEditorAction),
    InvokeCommand(UiCommandName),
    Scroll(bool),
    Unhandled,
}

pub(crate) struct UiHost<'runtime> {
    module: UiModule<'runtime>,
    session: UiNativeSession<'runtime>,
    view: UiWorkbench,
    admitted_commands: HashSet<UiCommandName>,
    key_prefix: Vec<String>,
    transient: Option<UiTransient>,
    transient_warning: bool,
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
        // Successor admission keeps the command catalog unchanged.
        let admitted_commands = view
            .commands()
            .iter()
            .map(|command| command.name.clone())
            .collect();
        Ok(Self {
            module,
            session,
            view,
            admitted_commands,
            key_prefix: Vec::new(),
            transient: None,
            transient_warning: false,
        })
    }

    pub(crate) fn view(&self) -> &UiWorkbench {
        &self.view
    }

    pub(crate) fn tool_review_menu(
        &self,
    ) -> Result<marlin_agent_ui::UiToolReviewMenu, UiProjectionError> {
        self.module.tool_review_menu(&self.session)
    }

    pub(crate) fn transient_hint(&self) -> Option<&marlin_agent_ui::UiElement> {
        self.transient.as_ref().map(|transient| {
            if self.transient_warning {
                &transient.warning_hint
            } else {
                &transient.hint
            }
        })
    }

    pub(crate) fn enter_transient(&mut self, id: &UiTransientId) -> Result<(), UiProjectionError> {
        let transient = self
            .view
            .transients
            .iter()
            .find(|transient| &transient.id == id)
            .ok_or(UiProjectionError::Invalid(
                "transient is not in the selected catalog",
            ))?;
        self.transient = Some(transient.clone());
        self.transient_warning = false;
        self.key_prefix.clear();
        Ok(())
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
            || candidate.frame != self.view.frame
            || candidate.keymap != self.view.keymap
            || candidate.transients != self.view.transients
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

    pub(crate) fn command_candidates(
        &self,
        query: &str,
        locked: Option<&UiCommandName>,
    ) -> Result<marlin_agent_ui::UiCommandCandidates, UiProjectionError> {
        let candidates = self
            .module
            .command_candidates(&self.session, query, locked)?;
        if candidates
            .names
            .iter()
            .any(|name| !self.admitted_commands.contains(name))
        {
            return Err(UiProjectionError::Invalid(
                "command candidate is not in the admitted catalog",
            ));
        }
        Ok(candidates)
    }

    pub(crate) fn complete_command(
        &self,
        draft: &str,
        candidates: &[UiCommandName],
    ) -> Option<String> {
        complete_command(&self.view, draft, candidates)
    }

    pub(crate) fn browse_command_steps(
        &self,
        query: &str,
        candidates: &[UiCommandName],
        previous: Option<&UiCommandName>,
        direction: UiBrowseDirection,
        steps: u8,
        restricted: bool,
    ) -> Result<Option<UiCommandName>, UiProjectionError> {
        let choice = self.module.browse_command_steps(
            &self.session,
            marlin_agent_ui::UiBrowseRequest {
                query,
                previous,
                direction,
                steps,
                allowed: restricted.then_some(candidates),
            },
        )?;
        admit_command_choice(candidates, choice.as_ref())?;
        Ok(choice)
    }

    pub(crate) fn complete_selected_command(
        &self,
        candidates: &[UiCommandName],
        selected: &UiCommandName,
    ) -> Option<String> {
        if !candidates.contains(selected) {
            return None;
        }
        let command = self
            .view
            .commands()
            .iter()
            .find(|command| &command.name == selected)?;
        let mut result = command.name.as_str().to_owned();
        if command.accepts_arguments() {
            result.push(' ');
        }
        Some(result)
    }

    /// Resolve the Mini Buffer's default choice through Scheme's ordered menu.
    pub(crate) fn accept_command(
        &self,
        query: &str,
        candidates: &[UiCommandName],
        selected: Option<&UiCommandName>,
    ) -> Result<Option<String>, UiProjectionError> {
        if let Some(name) = selected {
            return Ok(self.complete_selected_command(candidates, name));
        }
        if let Some(completed) = self.complete_command(query, candidates) {
            return Ok(Some(completed));
        }
        Ok(candidates
            .first()
            .and_then(|name| self.complete_selected_command(candidates, name)))
    }

    pub(crate) fn route_key(
        &mut self,
        key: Key,
        command_active: bool,
        draft_is_command: bool,
    ) -> Result<UiKeyAction, UiProjectionError> {
        if let Some(transient) = &self.transient {
            self.transient_warning = false;
            if matches!(key, Key::Esc | Key::Ctrl('g')) {
                self.transient = None;
                return Ok(UiKeyAction::Handled);
            }
            if let Some(head) = key_token(key).and_then(|token| {
                transient
                    .heads
                    .iter()
                    .find(|head| head.key.as_str() == token)
            }) {
                let command = head.command.clone();
                if head.exit {
                    self.transient = None;
                }
                return Ok(command.map_or(UiKeyAction::Handled, UiKeyAction::InvokeCommand));
            }
            match transient.foreign {
                UiTransientForeign::Exit => self.transient = None,
                UiTransientForeign::Warn => {
                    self.transient_warning = true;
                    return Ok(UiKeyAction::Handled);
                }
                UiTransientForeign::Run => {}
            }
        }
        if !self.key_prefix.is_empty() {
            let mut sequence = std::mem::take(&mut self.key_prefix);
            if let Some(token) = key_token(key) {
                sequence.push(token);
                let keys: Vec<_> = sequence.iter().map(String::as_str).collect();
                if let Some(command) = self.view.key_command(&keys) {
                    return Ok(self.key_command_action(command));
                }
                if self.view.key_prefix(&keys) {
                    self.key_prefix = sequence;
                    return Ok(UiKeyAction::Handled);
                }
            }
        }
        if matches!(key, Key::Ctrl(_) | Key::Alt(_))
            && let Some(token) = key_token(key)
        {
            if let Some(command) = self.view.key_command(&[&token]) {
                return Ok(self.key_command_action(command));
            }
            if self.view.key_prefix(&[&token]) {
                self.key_prefix = vec![token];
                return Ok(UiKeyAction::Handled);
            }
        }
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

    fn key_command_action(&mut self, command: UiKeyCommand) -> UiKeyAction {
        match command {
            UiKeyCommand::OpenCommands => UiKeyAction::OpenCommands,
            UiKeyCommand::Cancel => UiKeyAction::Cancel,
            UiKeyCommand::Edit(action) => UiKeyAction::Edit(action),
            UiKeyCommand::Invoke(name) => UiKeyAction::InvokeCommand(name),
        }
    }

    /// Publish a successor only after its projection and intent agree.
    pub(crate) fn dispatch(&mut self, event: UiEvent) -> Result<UiIntent, UiProjectionError> {
        let (successor, intent) = self.module.dispatch(&self.session, event.clone())?;
        let candidate = self.module.project(&successor)?;
        admit_transition(&self.view, &candidate, event, &intent)?;
        self.session = successor;
        self.view = candidate;
        Ok(intent)
    }
}

fn key_token(key: Key) -> Option<String> {
    match key {
        Key::Ctrl(value) if value.is_ascii_alphanumeric() => Some(format!("C-{value}")),
        Key::Alt(value) if value.is_ascii_alphanumeric() => Some(format!("M-{value}")),
        Key::Char(value) if value.is_ascii_alphanumeric() => Some(value.to_string()),
        _ => None,
    }
}

fn admit_command_choice(
    candidates: &[UiCommandName],
    choice: Option<&UiCommandName>,
) -> Result<(), UiProjectionError> {
    if let Some(name) = choice
        && !candidates.contains(name)
    {
        return Err(UiProjectionError::Invalid(
            "command choice is not in the admitted menu",
        ));
    }
    Ok(())
}

fn complete_command(
    view: &UiWorkbench,
    draft: &str,
    candidates: &[UiCommandName],
) -> Option<String> {
    if !UiCommandName::is_valid_prefix(draft) {
        return None;
    }
    let first = candidates.first()?;
    let exact = candidates.iter().find(|name| name.as_str() == draft);
    let selected = match exact {
        Some(name) => name,
        None if candidates.len() == 1 => first,
        None => return None,
    };
    let selected = view
        .commands()
        .iter()
        .find(|command| &command.name == selected)?;
    let mut completed = selected.name.as_str().to_owned();
    if selected.accepts_arguments() {
        completed.push(' ');
    }
    (completed != draft).then_some(completed)
}

fn admit_terminal_view(view: &UiWorkbench) -> Result<(), UiProjectionError> {
    fn collect_slots<'a>(
        element: &'a marlin_agent_ui::UiElement,
        slots: &mut HashSet<&'a str>,
    ) -> bool {
        match element {
            marlin_agent_ui::UiElement::Region { id, .. } => slots.insert(id.as_str()),
            marlin_agent_ui::UiElement::Rows(children)
            | marlin_agent_ui::UiElement::Columns(children) => {
                children.iter().all(|child| collect_slots(child, slots))
            }
            marlin_agent_ui::UiElement::Sized { child, .. } => collect_slots(child, slots),
            // The terminal's required slots must exist independent of facts.
            marlin_agent_ui::UiElement::Choice { .. } => false,
            marlin_agent_ui::UiElement::Text(_) | marlin_agent_ui::UiElement::Template { .. } => {
                true
            }
        }
    }
    let mut slots = HashSet::new();
    if !collect_slots(&view.layout.frame_tree, &mut slots)
        || [
            "transcript",
            "extensions",
            "modeline",
            "editor",
            "candidates",
            "minibuffer",
        ]
        .iter()
        .any(|name| !slots.contains(name))
    {
        return Err(UiProjectionError::Invalid("invalid terminal frame regions"));
    }
    if !matches!(view.layout.frame_tree.region_child("candidates"),
        Some(marlin_agent_ui::UiElement::Rows(templates)) if templates.len() == 2
            && templates.iter().all(|item| matches!(item, marlin_agent_ui::UiElement::Template { .. })))
    {
        return Err(UiProjectionError::Invalid("invalid candidate presentation"));
    }
    let admitted_command = |name: &UiCommandName| {
        view.commands().iter().any(|command| {
            &command.name == name && command.arguments == marlin_agent_ui::UiCommandArguments::None
        })
    };
    for binding in &view.keymap {
        let valid = match &binding.command {
            UiKeyCommand::Invoke(name) => admitted_command(name),
            _ => true,
        };
        if !valid {
            return Err(UiProjectionError::Invalid(
                "UI key binding names an unavailable command",
            ));
        }
    }
    if view.transients.iter().any(|transient| {
        transient.heads.iter().any(|head| {
            head.command
                .as_ref()
                .is_some_and(|name| !admitted_command(name))
        }) || !view.commands().iter().any(|command| {
            command.capability == marlin_agent_ui::UiCommandCapability::EnterTransient
                && command.value.as_deref() == Some(transient.id.as_str())
        })
    }) || view.commands().iter().any(|command| {
        command.capability == marlin_agent_ui::UiCommandCapability::EnterTransient
            && !view
                .transients
                .iter()
                .any(|transient| Some(transient.id.as_str()) == command.value.as_deref())
    }) {
        return Err(UiProjectionError::Invalid(
            "transient catalog and command descriptors diverge",
        ));
    }
    if view.frame.windows.len() != 1
        || view.frame.id.as_str() != "main"
        || view.frame.selected_window.as_str() != "main"
        || view.frame.minibuffer.as_str() != "minibuffer"
        || view.frame.buffers.iter().any(|buffer| {
            !matches!(buffer.as_str(), "conversation" | "messages" | "processes")
                && !view.regions.iter().any(|region| {
                    matches!(region,
                    marlin_agent_ui::UiRegion::Extension { id, focusable: false, .. }
                        if id.as_str() == buffer.as_str())
                })
        })
        || view.frame.selected_buffer().is_none()
    {
        return Err(UiProjectionError::Invalid(
            "unsupported terminal frame binding",
        ));
    }
    for command in view.commands() {
        if !Command::admit(command) {
            return Err(UiProjectionError::Invalid(
                "command capability has no registered terminal handler",
            ));
        }
        if command.capability == marlin_agent_ui::UiCommandCapability::SelectBuffer
            && !view
                .frame
                .buffers
                .iter()
                .any(|buffer| Some(buffer.as_str()) == command.value.as_deref())
        {
            return Err(UiProjectionError::Invalid(
                "command selects an unavailable buffer",
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
