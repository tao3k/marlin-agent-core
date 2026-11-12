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
    UiProjectionError, UiRegion, UiScrollDirection, UiWorkbench,
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
            Key::Char('\n') if focused_extension(&self.view) => {
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

fn focused_extension(view: &UiWorkbench) -> bool {
    view.regions.iter().any(|region| {
        matches!(region, UiRegion::Extension { id, focusable: true, .. } if id == &view.focus)
    })
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
        || focused_extension(view);
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
    let supported = matches!(
        (event, intent),
        (
            UiEvent::NextFocus | UiEvent::PreviousFocus | UiEvent::Dismiss,
            UiIntent::Focus(_)
        ) | (UiEvent::ScrollUp, UiIntent::Scroll(UiScrollDirection::Up))
            | (
                UiEvent::ScrollDown,
                UiIntent::Scroll(UiScrollDirection::Down)
            )
    ) || (event == UiEvent::Submit
        && match intent {
            UiIntent::SubmitDraft => previous.focus.as_str() == "composer",
            UiIntent::Focus(_) => focused_extension(previous),
            UiIntent::Scroll(_) => false,
        });
    if !supported {
        return Err(UiProjectionError::Invalid(
            "UI intent does not match the event",
        ));
    }
    if previous.model_label != next.model_label
        || previous.session_label != next.session_label
        || previous.regions != next.regions
    {
        return Err(UiProjectionError::Invalid(
            "UI event changed the admitted workbench",
        ));
    }
    let expected = match intent {
        UiIntent::Focus(id) => id,
        UiIntent::SubmitDraft | UiIntent::Scroll(_) => &previous.focus,
    };
    if &next.focus != expected {
        return Err(UiProjectionError::Invalid(
            "UI intent and successor focus disagree",
        ));
    }
    Ok(())
}

#[cfg(all(test, not(feature = "linked-ui")))]
mod tests {
    use super::*;
    use marlin_agent_ui::UiComponentId;

    #[test]
    #[ignore = "requires a compiled Marlin Scheme dependency closure and Gerbil toolchain"]
    fn real_vm_routes_a_contributed_method_without_submitting_the_draft() {
        use gerbil_scheme::IdentifierKind;
        use marlin_agent_protocol::ModelEndpoint;
        use marlin_agent_ui::UiStatusItem;
        use std::path::Path;

        let package = Path::new(env!("CARGO_MANIFEST_DIR"));
        let fixture = package.join("tests/fixtures/ui-extension");
        let root = package.join("../..").canonicalize().unwrap();
        let root = root.to_str().unwrap();
        let runtime = GerbilRuntime::initialize().unwrap();
        assert_eq!(
            runtime
                .build_package_spec(fixture.to_str().unwrap(), "ui-test-spec")
                .unwrap(),
            0
        );
        let module = runtime
            .intern_identifier(IdentifierKind::Symbol, ":marlin-tui-ui-test/fixture")
            .unwrap();
        let load = |name| {
            let name = runtime
                .intern_identifier(IdentifierKind::Symbol, name)
                .unwrap();
            runtime
                .resolve_package_export(root, &module, &name)
                .unwrap()
        };
        let extension = load("test-extension");
        let methods = load("test-methods");
        let extensions = [extension];
        let bundles = [methods];
        let status = [UiStatusItem {
            id: UiComponentId::new("test/status").unwrap(),
            label: "VM ready".into(),
        }];
        let mut host = UiHost::open(
            &runtime,
            root,
            &UiOpenFacts {
                model_label: "model",
                session_label: "session",
                status: &status,
                extensions: &extensions,
                method_bundles: &bundles,
            },
        )
        .unwrap();
        assert_eq!(host.view().focus.as_str(), "composer");
        assert_eq!(
            host.route_key(Key::Char('\t'), false, false).unwrap(),
            UiKeyAction::Handled
        );
        assert_eq!(host.view().focus.as_str(), "commands");
        assert_eq!(
            host.route_key(Key::Char('\t'), false, false).unwrap(),
            UiKeyAction::Handled
        );
        assert_eq!(host.view().focus.as_str(), "test/extension");
        let app = super::super::app::App::new(ModelEndpoint::new("provider", "model"));
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(60, 10)).unwrap();
        terminal
            .draw(|frame| {
                super::super::render::draw(
                    frame,
                    &app,
                    host.view(),
                    &mut Default::default(),
                    &mut Default::default(),
                );
            })
            .unwrap();
        let buffer = terminal.backend().buffer();
        let screen: String = (0..10)
            .flat_map(|y| (0..60).map(move |x| buffer[(x, y)].symbol()))
            .collect();
        assert!(
            screen.contains("VM ready") && screen.contains("Test extension"),
            "{screen}"
        );
        host.refresh_labels("other/model", "other-session").unwrap();
        assert_eq!(host.view().focus.as_str(), "test/extension");
        assert_eq!(host.view().model_label, "other/model");
        assert_eq!(
            host.route_key(Key::Char('\n'), false, false).unwrap(),
            UiKeyAction::Handled
        );
        assert_eq!(host.view().focus.as_str(), "composer");

        let forged_bundles = [load("test-forged-methods")];
        let mut forged = UiHost::open(
            &runtime,
            root,
            &UiOpenFacts {
                model_label: "model",
                session_label: "session",
                status: &[],
                extensions: &extensions,
                method_bundles: &forged_bundles,
            },
        )
        .unwrap();
        forged.route_key(Key::Char('\t'), false, false).unwrap();
        forged.route_key(Key::Char('\t'), false, false).unwrap();
        let previous = forged.view().clone();
        assert!(forged.route_key(Key::Char('\n'), false, false).is_err());
        assert_eq!(forged.view(), &previous);
    }

    fn view(focus: &str) -> UiWorkbench {
        UiWorkbench {
            model_label: "model".into(),
            session_label: "session".into(),
            regions: vec![
                UiRegion::Transcript,
                UiRegion::Composer,
                UiRegion::Commands(vec![]),
                UiRegion::Status(vec![]),
            ],
            focus: UiComponentId::new(focus).unwrap(),
        }
    }

    #[test]
    fn completion_respects_ambiguity_exact_names_and_arguments() {
        use marlin_agent_ui::{UiCommand, UiCommandName};
        let mut workbench = view("commands");
        workbench.regions[2] = UiRegion::Commands(
            [("/help", false), ("/model", true), ("/models", false)]
                .into_iter()
                .map(|(name, accepts_arguments)| UiCommand {
                    name: UiCommandName::new(name).unwrap(),
                    description: String::new(),
                    accepts_arguments,
                })
                .collect(),
        );
        assert_eq!(complete_command(&workbench, "/"), None);
        assert_eq!(complete_command(&workbench, "/mo"), None);
        assert_eq!(complete_command(&workbench, "/hel"), Some("/help".into()));
        assert_eq!(
            complete_command(&workbench, "/model"),
            Some("/model ".into())
        );
        assert_eq!(complete_command(&workbench, "/model foo"), None);
        assert_eq!(complete_command(&workbench, "/模型"), None);
        assert_eq!(
            complete_command(&workbench, &format!("/{}", "m".repeat(1_000_000))),
            None
        );
        let help = UiCommandName::new("/help").unwrap();
        let forged = UiCommandName::new("/forged").unwrap();
        assert!(admit_command_choice(&workbench, "/he", Some(&help)).is_ok());
        assert!(admit_command_choice(&workbench, "/mo", Some(&help)).is_err());
        assert!(admit_command_choice(&workbench, "/", Some(&forged)).is_err());
    }

    #[test]
    fn command_descriptions_cannot_invent_handlers_or_argument_contracts() {
        use marlin_agent_ui::UiCommand;
        let mut workbench = view("composer");
        let command = |name, accepts_arguments| UiCommand {
            name: UiCommandName::new(name).unwrap(),
            description: "description".into(),
            accepts_arguments,
        };
        workbench.regions[2] = UiRegion::Commands(vec![command("/help", false)]);
        assert!(admit_terminal_view(&workbench).is_ok());
        workbench.regions[2] = UiRegion::Commands(vec![command("/made-up", false)]);
        assert!(admit_terminal_view(&workbench).is_err());
        workbench.regions[2] = UiRegion::Commands(vec![command("/model", false)]);
        assert!(admit_terminal_view(&workbench).is_err());
    }

    #[test]
    fn focus_transition_requires_matching_intent_and_preserves_tree() {
        let previous = view("composer");
        let mut next = view("commands");
        let intent = UiIntent::Focus(UiComponentId::new("commands").unwrap());
        assert!(admit_transition(&previous, &next, UiEvent::NextFocus, &intent).is_ok());
        assert!(admit_transition(&previous, &next, UiEvent::Submit, &intent).is_err());
        next.focus = UiComponentId::new("transcript").unwrap();
        assert!(admit_transition(&previous, &next, UiEvent::NextFocus, &intent).is_err());
        next.focus = UiComponentId::new("commands").unwrap();
        next.model_label = "changed".into();
        assert!(admit_transition(&previous, &next, UiEvent::NextFocus, &intent).is_err());
    }

    #[test]
    fn scroll_direction_and_extension_focus_are_checked() {
        let previous = view("transcript");
        assert!(
            admit_transition(
                &previous,
                &previous,
                UiEvent::ScrollUp,
                &UiIntent::Scroll(UiScrollDirection::Up)
            )
            .is_ok()
        );
        assert!(
            admit_transition(
                &previous,
                &previous,
                UiEvent::ScrollUp,
                &UiIntent::Scroll(UiScrollDirection::Down)
            )
            .is_err()
        );
        let mut extended = previous.clone();
        extended.regions.push(UiRegion::Extension {
            id: UiComponentId::new("sample").unwrap(),
            label: "sample".into(),
            focusable: true,
        });
        extended.focus = UiComponentId::new("sample").unwrap();
        assert!(admit_terminal_view(&extended).is_ok());
        let mut returned = extended.clone();
        returned.focus = UiComponentId::new("composer").unwrap();
        assert!(
            admit_transition(
                &extended,
                &returned,
                UiEvent::Submit,
                &UiIntent::Focus(returned.focus.clone())
            )
            .is_ok()
        );
        assert!(
            admit_transition(
                &extended,
                &extended,
                UiEvent::Submit,
                &UiIntent::SubmitDraft
            )
            .is_err()
        );
        let mut before = extended.clone();
        before.focus = UiComponentId::new("commands").unwrap();
        assert!(
            admit_transition(
                &before,
                &extended,
                UiEvent::NextFocus,
                &UiIntent::Focus(UiComponentId::new("sample").unwrap())
            )
            .is_ok()
        );
        if let Some(UiRegion::Extension { focusable, .. }) = extended.regions.last_mut() {
            *focusable = false;
        }
        assert!(admit_terminal_view(&extended).is_err());
        assert!(admit_terminal_view(&view("missing")).is_err());
    }
}
