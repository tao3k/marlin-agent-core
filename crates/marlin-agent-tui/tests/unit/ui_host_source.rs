// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use super::{
    UiHost, UiKeyAction, admit_command_choice, admit_terminal_view, admit_transition,
    complete_command,
};
use gerbil_scheme::GerbilRuntime;
use marlin_agent_ui::{
    UiCommandName, UiComponentId, UiEvent, UiIntent, UiOpenFacts, UiRegion, UiScrollDirection,
    UiWorkbench,
};
use termion::event::Key;

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
    let status = [UiStatusItem::host(
        UiComponentId::new("test/status").unwrap(),
        "VM ready",
    )];
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
    let modeline = host
        .view()
        .regions
        .iter()
        .find_map(|region| match region {
            UiRegion::Status(items) => Some(items.as_slice()),
            _ => None,
        })
        .unwrap();
    assert_eq!(modeline[0].label, "🐟 MARLIN");
    assert_eq!(modeline[0].compact_label, "🐟");
    assert_eq!(modeline[2].label, "🟡 Ready");
    assert_eq!(modeline[2].compact_label, "🟡");
    assert_eq!(
        modeline[2].active_tone,
        marlin_agent_ui::UiStatusTone::Warning
    );
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
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(60, 20)).unwrap();
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
    let screen: String = (0..20)
        .flat_map(|y| (0..60).map(move |x| buffer[(x, y)].symbol()))
        .collect();
    assert!(
        screen.contains("VM ready")
            && screen.contains("Test extension")
            && screen.contains("Scheme component body")
            && screen.contains("Source: selected module"),
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
        layout: marlin_agent_ui::UiLayoutPolicy::default(),
        theme: marlin_agent_ui::UiTheme::default(),
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
        content: vec![],
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
