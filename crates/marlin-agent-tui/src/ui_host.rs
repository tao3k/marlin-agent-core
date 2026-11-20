// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Admission of functional Scheme UI states into the terminal owner.

use super::command::Command;
use marlin_agent_ui::{
    UiBrowseDirection, UiCharacterSelection, UiCommandName, UiEditorAction, UiEvent, UiIntent,
    UiKeyCommand, UiKeyMode, UiModule, UiNativeSession, UiOpenFacts, UiProjectionError,
    UiTransient, UiTransientForeign, UiTransientId, UiWorkbench,
};
use std::collections::HashSet;
use termion::event::Key;

#[cfg(all(test, not(feature = "linked-ui")))]
use super::ui_fixture::{test_frame, test_layout};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum UiKeyAction {
    Handled,
    Submit,
    OpenCommands,
    Cancel,
    Edit(UiEditorAction),
    Mode(UiKeyMode),
    InvokeCommand(UiCommandName),
    Scroll(bool),
    Unhandled,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum KeySequenceState {
    Pending,
    Complete,
    Undefined,
}

pub(crate) struct UiHost<'runtime> {
    module: UiModule<'runtime>,
    session: UiNativeSession<'runtime>,
    view: UiWorkbench,
    admitted_commands: HashSet<UiCommandName>,
    key_prefix: Vec<String>,
    completed_keys: Vec<String>,
    completed_key_state: KeySequenceState,
    pending_character: Option<UiCharacterSelection>,
    transient: Option<UiTransient>,
    transient_warning: bool,
}

impl<'runtime> UiHost<'runtime> {
    pub(crate) fn from_module(
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
            completed_keys: Vec::new(),
            completed_key_state: KeySequenceState::Complete,
            pending_character: None,
            transient: None,
            transient_warning: false,
        })
    }

    pub(crate) fn view(&self) -> &UiWorkbench {
        &self.view
    }

    pub(crate) fn package_event(
        &mut self,
        event: marlin_agent_ui::UiPackageEvent<'_>,
    ) -> Result<(), UiProjectionError> {
        let session = self.module.package_event(&self.session, event)?;
        let view = self.module.project(&session)?;
        admit_terminal_view(&view)?;
        self.admitted_commands = view
            .commands()
            .iter()
            .map(|command| command.name.clone())
            .collect();
        self.session = session;
        self.view = view;
        Ok(())
    }

    pub(crate) fn key_feedback(&self) -> (&[String], KeySequenceState) {
        if self.key_prefix.is_empty() {
            (&self.completed_keys, self.completed_key_state)
        } else {
            (&self.key_prefix, KeySequenceState::Pending)
        }
    }

    pub(crate) fn has_undefined_key_sequence(&self) -> bool {
        self.completed_key_state == KeySequenceState::Undefined
    }

    pub(crate) fn retry_undefined_key_sequence(&mut self) {
        if self.has_undefined_key_sequence() {
            self.completed_keys.pop();
            self.key_prefix = std::mem::take(&mut self.completed_keys);
        }
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

    pub(crate) fn keypad_hint(
        &self,
        mode: UiKeyMode,
    ) -> Option<(marlin_agent_ui::UiElement, std::time::Duration)> {
        if self.key_prefix.is_empty() || self.pending_character.is_some() {
            return None;
        }
        let keys: Vec<_> = self.key_prefix.iter().map(String::as_str).collect();
        match self.view.key_command(&keys, Some(mode)) {
            Some(UiKeyCommand::PrefixHint { view, delay }) => Some((*view, delay)),
            _ => None,
        }
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
        self.pending_character = None;
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
        if !labels_only_successor(&self.view, &candidate, model, session) {
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

    pub(crate) fn focus_editor(&mut self) -> Result<(), UiProjectionError> {
        if !self.accepts_text(false) {
            let target = marlin_agent_ui::UiComponentId::new("composer")?;
            self.dispatch(UiEvent::SelectFocus(target))?;
        }
        Ok(())
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
        steps: u16,
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
        mode: UiKeyMode,
    ) -> Result<UiKeyAction, UiProjectionError> {
        self.completed_keys.clear();
        self.completed_key_state = KeySequenceState::Complete;
        let scope = (!command_active).then_some(mode);
        if let Some(selection) = self.pending_character.take() {
            let mut sequence = std::mem::take(&mut self.key_prefix);
            if !command_active && self.accepts_text(false) {
                return Ok(match key {
                    Key::Char(target) if target == '\n' || !target.is_control() => {
                        sequence.push(key_token(key).unwrap_or_else(|| target.to_string()));
                        self.completed_keys = sequence;
                        UiKeyAction::Edit(UiEditorAction::SelectCharacter { target, selection })
                    }
                    Key::Ctrl('q' | 'c') => UiKeyAction::Unhandled,
                    _ => UiKeyAction::Handled,
                });
            }
        }
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
            match key {
                Key::Esc | Key::Ctrl('g') => return Ok(UiKeyAction::Handled),
                Key::Backspace => {
                    sequence.pop();
                    self.key_prefix = sequence;
                    return Ok(UiKeyAction::Handled);
                }
                _ => {}
            }
            if let Some(token) = key_token(key) {
                sequence.push(token);
                let keys: Vec<_> = sequence.iter().map(String::as_str).collect();
                if let Some(command) = self.view.key_command(&keys, scope) {
                    return Ok(self.key_command_action(command, || sequence, true));
                }
                if self.view.key_prefix(&keys, scope) {
                    self.key_prefix = sequence;
                } else {
                    self.completed_keys = sequence;
                    self.completed_key_state = KeySequenceState::Undefined;
                }
            }
            return Ok(UiKeyAction::Handled);
        }
        if (matches!(key, Key::Ctrl(_) | Key::Alt(_))
            || (!command_active && (mode == UiKeyMode::Normal || key == Key::Esc)))
            && let Some(token) = key_token(key)
        {
            if let Some(command) = self.view.key_command(&[&token], scope) {
                return Ok(self.key_command_action(command, || vec![token], false));
            }
            if command_active
                && matches!(
                    self.view.key_command(&[&token], Some(mode)),
                    Some(UiKeyCommand::OpenCommands)
                )
            {
                return Ok(UiKeyAction::OpenCommands);
            }
            if self.view.key_prefix(&[&token], scope) {
                self.key_prefix = vec![token];
                return Ok(UiKeyAction::Handled);
            }
        }
        if !command_active
            && mode == UiKeyMode::Normal
            && matches!(key, Key::Char(c) if !c.is_control())
        {
            return Ok(UiKeyAction::Handled);
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
            Key::Char('\n')
                if mode == UiKeyMode::Normal && self.view.focus.as_str() == "commands" =>
            {
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

    fn key_command_action(
        &mut self,
        command: UiKeyCommand,
        keys: impl FnOnce() -> Vec<String>,
        from_prefix: bool,
    ) -> UiKeyAction {
        let action = match command {
            UiKeyCommand::OpenCommands => UiKeyAction::OpenCommands,
            UiKeyCommand::Cancel => UiKeyAction::Cancel,
            UiKeyCommand::Edit(action) => UiKeyAction::Edit(action),
            UiKeyCommand::ReadCharacter(selection) => {
                if self.accepts_text(false) {
                    self.pending_character = Some(selection);
                    self.key_prefix = keys();
                }
                return UiKeyAction::Handled;
            }
            UiKeyCommand::PrefixHint { .. } => {
                self.key_prefix = keys();
                return UiKeyAction::Handled;
            }
            UiKeyCommand::Mode(mode) => UiKeyAction::Mode(mode),
            UiKeyCommand::Invoke(name) => UiKeyAction::InvokeCommand(name),
        };
        if from_prefix {
            self.completed_keys = keys();
        }
        action
    }

    pub(crate) fn cancel_key_sequence(&mut self) -> bool {
        if self.pending_character.take().is_none() && self.key_prefix.is_empty() {
            return false;
        }
        self.key_prefix.clear();
        self.completed_keys.clear();
        true
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

fn labels_only_successor(
    previous: &UiWorkbench,
    candidate: &UiWorkbench,
    model: &str,
    session: &str,
) -> bool {
    candidate.model_label == model
        && candidate.session_label == session
        && candidate.layout == previous.layout
        && candidate.theme == previous.theme
        && candidate.regions == previous.regions
        && candidate.focus == previous.focus
        && candidate.frame == previous.frame
        && candidate.keymap == previous.keymap
        && candidate.transients == previous.transients
}

fn key_token(key: Key) -> Option<String> {
    match key {
        Key::Esc => Some("ESC".into()),
        Key::Backspace => Some("BACKSPACE".into()),
        Key::Delete => Some("DELETE".into()),
        Key::Char('\n') => Some("RET".into()),
        Key::Char(' ') => Some("SPC".into()),
        Key::Ctrl(value) if value.is_ascii_alphanumeric() => Some(format!("C-{value}")),
        Key::Alt(value) if value.is_ascii_graphic() => Some(format!("M-{value}")),
        Key::Char(value) if value.is_ascii_graphic() => Some(value.to_string()),
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
    fn scroll_targets_admitted(element: &marlin_agent_ui::UiElement) -> bool {
        match element {
            marlin_agent_ui::UiElement::Region {
                scroll_target,
                child,
                ..
            } => {
                scroll_target
                    .as_ref()
                    .is_none_or(|target| matches!(target.as_str(), "transcript" | "candidates"))
                    && scroll_targets_admitted(child)
            }
            marlin_agent_ui::UiElement::Choice { cases, fallback } => {
                cases
                    .iter()
                    .all(|(_, child)| scroll_targets_admitted(child))
                    && scroll_targets_admitted(fallback)
            }
            marlin_agent_ui::UiElement::Rows(children)
            | marlin_agent_ui::UiElement::Columns(children) => {
                children.iter().all(scroll_targets_admitted)
            }
            marlin_agent_ui::UiElement::Bar { segments, .. } => segments
                .iter()
                .all(|segment| scroll_targets_admitted(&segment.child)),
            marlin_agent_ui::UiElement::Sized { child, .. }
            | marlin_agent_ui::UiElement::Panel { child, .. }
            | marlin_agent_ui::UiElement::List { child, .. } => scroll_targets_admitted(child),
            _ => true,
        }
    }
    fn main_presentation(element: &marlin_agent_ui::UiElement, within_panel: bool) -> bool {
        match element {
            marlin_agent_ui::UiElement::Slot(source) => {
                *source != marlin_agent_ui::UiSlotSource::TransientHint && within_panel
            }
            marlin_agent_ui::UiElement::Panel { child, .. } => main_presentation(child, true),
            marlin_agent_ui::UiElement::List { source, child, .. } => {
                use marlin_agent_ui::UiListSource;
                matches!(
                    source,
                    UiListSource::Review
                        | UiListSource::Messages
                        | UiListSource::Workspace
                        | UiListSource::Conversation
                        | UiListSource::Rows(_)
                ) && main_presentation(child, within_panel)
            }
            marlin_agent_ui::UiElement::Sized { child, .. }
            | marlin_agent_ui::UiElement::Region { child, .. } => {
                main_presentation(child, within_panel)
            }
            marlin_agent_ui::UiElement::Rows(children)
            | marlin_agent_ui::UiElement::Columns(children) => children
                .iter()
                .all(|child| main_presentation(child, within_panel)),
            marlin_agent_ui::UiElement::Bar { segments, .. } => segments
                .iter()
                .all(|segment| main_presentation(&segment.child, within_panel)),
            marlin_agent_ui::UiElement::Choice { cases, fallback } => {
                cases
                    .iter()
                    .all(|(_, item)| main_presentation(item, within_panel))
                    && main_presentation(fallback, within_panel)
            }
            marlin_agent_ui::UiElement::Text(_) | marlin_agent_ui::UiElement::Template { .. } => {
                true
            }
            marlin_agent_ui::UiElement::Input { .. }
            | marlin_agent_ui::UiElement::Prompt { .. } => false,
        }
    }
    fn input_count(element: &marlin_agent_ui::UiElement) -> Option<usize> {
        match element {
            marlin_agent_ui::UiElement::Input { .. } => Some(1),
            marlin_agent_ui::UiElement::Choice { cases, fallback } => {
                let count = input_count(fallback)?;
                cases
                    .iter()
                    .all(|(_, item)| input_count(item) == Some(count))
                    .then_some(count)
            }
            marlin_agent_ui::UiElement::Rows(children)
            | marlin_agent_ui::UiElement::Columns(children) => children
                .iter()
                .try_fold(0usize, |sum, child| sum.checked_add(input_count(child)?)),
            marlin_agent_ui::UiElement::Bar { segments, .. } => {
                segments.iter().try_fold(0usize, |sum, segment| {
                    sum.checked_add(input_count(&segment.child)?)
                })
            }
            marlin_agent_ui::UiElement::Panel { child, .. }
            | marlin_agent_ui::UiElement::Sized { child, .. }
            | marlin_agent_ui::UiElement::Region { child, .. } => input_count(child),
            marlin_agent_ui::UiElement::Text(_) | marlin_agent_ui::UiElement::Template { .. } => {
                Some(0)
            }
            marlin_agent_ui::UiElement::Slot(_)
            | marlin_agent_ui::UiElement::List { .. }
            | marlin_agent_ui::UiElement::Prompt { .. } => None,
        }
    }
    fn collect_slots<'a>(
        element: &'a marlin_agent_ui::UiElement,
        slots: &mut HashSet<&'a str>,
        within_region: bool,
    ) -> bool {
        match element {
            marlin_agent_ui::UiElement::Region { id, child, .. } => {
                slots.insert(id.as_str()) && collect_slots(child, slots, true)
            }
            marlin_agent_ui::UiElement::Rows(children)
            | marlin_agent_ui::UiElement::Columns(children) => children
                .iter()
                .all(|child| collect_slots(child, slots, within_region)),
            marlin_agent_ui::UiElement::Bar { segments, .. } => segments
                .iter()
                .all(|segment| collect_slots(&segment.child, slots, within_region)),
            marlin_agent_ui::UiElement::Sized { child, .. }
            | marlin_agent_ui::UiElement::Panel { child, .. } => {
                collect_slots(child, slots, within_region)
            }
            marlin_agent_ui::UiElement::List { .. } => within_region,
            // The terminal's required slots must exist independent of facts.
            marlin_agent_ui::UiElement::Choice { .. } => within_region,
            marlin_agent_ui::UiElement::Text(_)
            | marlin_agent_ui::UiElement::Slot(_)
            | marlin_agent_ui::UiElement::Template { .. }
            | marlin_agent_ui::UiElement::Prompt { .. }
            | marlin_agent_ui::UiElement::Input { .. } => true,
        }
    }
    use marlin_agent_ui::{UiFrameRole, UiListSource};
    let mut slots = HashSet::new();
    if !scroll_targets_admitted(&view.layout.frame_tree)
        || !collect_slots(&view.layout.frame_tree, &mut slots, false)
        || UiFrameRole::ALL
            .iter()
            .any(|role| !slots.contains(view.layout.regions.id(*role).as_str()))
    {
        return Err(UiProjectionError::Invalid("invalid terminal frame regions"));
    }
    fn candidate_presentation(element: &marlin_agent_ui::UiElement) -> bool {
        match element {
            marlin_agent_ui::UiElement::Template { .. } | marlin_agent_ui::UiElement::Text(_) => {
                true
            }
            marlin_agent_ui::UiElement::Choice { cases, fallback } => {
                cases.iter().all(|(_, item)| candidate_presentation(item))
                    && candidate_presentation(fallback)
            }
            marlin_agent_ui::UiElement::Rows(children)
            | marlin_agent_ui::UiElement::Columns(children) => {
                children.iter().all(candidate_presentation)
            }
            marlin_agent_ui::UiElement::Bar { segments, .. } => segments
                .iter()
                .all(|segment| candidate_presentation(&segment.child)),
            marlin_agent_ui::UiElement::Sized { child, .. }
            | marlin_agent_ui::UiElement::Panel { child, .. }
            | marlin_agent_ui::UiElement::List { child, .. }
            | marlin_agent_ui::UiElement::Region { child, .. } => candidate_presentation(child),
            marlin_agent_ui::UiElement::Slot(_)
            | marlin_agent_ui::UiElement::Input { .. }
            | marlin_agent_ui::UiElement::Prompt { .. } => false,
        }
    }
    let child_for = |role| {
        view.layout
            .frame_tree
            .region_child(view.layout.regions.id(role).as_str())
    };
    let list = |role, capability| match child_for(role) {
        Some(marlin_agent_ui::UiElement::List {
            id, child, source, ..
        }) if *source == capability && candidate_presentation(child) => Some(id),
        _ => None,
    };
    let (Some(candidates), Some(extensions)) = (
        list(UiFrameRole::Candidates, UiListSource::Candidates),
        list(UiFrameRole::Extensions, UiListSource::Extensions),
    ) else {
        return Err(UiProjectionError::Invalid("invalid candidate presentation"));
    };
    if candidates == extensions {
        return Err(UiProjectionError::Invalid("duplicate UI list identity"));
    }
    if child_for(UiFrameRole::Editor).and_then(input_count) != Some(1) {
        return Err(UiProjectionError::Invalid("invalid input presentation"));
    }
    if !child_for(UiFrameRole::Transcript).is_some_and(|element| main_presentation(element, false))
    {
        return Err(UiProjectionError::Invalid("invalid main presentation"));
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
