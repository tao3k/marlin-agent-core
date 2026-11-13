// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use super::{
    UiHost, UiKeyAction, admit_command_choice, admit_terminal_view, admit_transition,
    complete_command, test_frame, test_layout,
};
use gerbil_scheme::GerbilRuntime;
use marlin_agent_ui::{
    UiCommandName, UiComponentId, UiEditorAction, UiEvent, UiIntent, UiOpenFacts, UiRegion,
    UiScrollDirection, UiWorkbench,
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
    assert_eq!(
        host.command_candidates("mode", None)
            .unwrap()
            .names
            .iter()
            .map(UiCommandName::as_str)
            .collect::<Vec<_>>(),
        vec!["model", "models"]
    );
    let restricted = [UiCommandName::new("models").unwrap()];
    assert_eq!(
        host.browse_command_steps(
            "mode",
            &restricted,
            None,
            marlin_agent_ui::UiBrowseDirection::Next,
            1,
            true,
        )
        .unwrap(),
        Some(restricted[0].clone())
    );
    assert_eq!(
        host.route_key(
            Key::Ctrl('r'),
            false,
            false,
            marlin_agent_ui::UiKeyMode::Insert
        )
        .unwrap(),
        UiKeyAction::Edit(UiEditorAction::HistorySearch)
    );
    assert_eq!(
        host.route_key(
            Key::Ctrl('n'),
            false,
            false,
            marlin_agent_ui::UiKeyMode::Insert
        )
        .unwrap(),
        UiKeyAction::Edit(UiEditorAction::InsertNewline)
    );
    assert_eq!(
        host.route_key(
            Key::Alt('p'),
            false,
            false,
            marlin_agent_ui::UiKeyMode::Insert
        )
        .unwrap(),
        UiKeyAction::Edit(UiEditorAction::PreviousPrompt)
    );
    assert_eq!(
        host.route_key(
            Key::Char('\t'),
            false,
            false,
            marlin_agent_ui::UiKeyMode::Insert
        )
        .unwrap(),
        UiKeyAction::Handled
    );
    assert_eq!(host.view().focus.as_str(), "commands");
    assert_eq!(
        host.route_key(
            Key::Char('\t'),
            false,
            false,
            marlin_agent_ui::UiKeyMode::Insert
        )
        .unwrap(),
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
        host.route_key(
            Key::Char('\n'),
            false,
            false,
            marlin_agent_ui::UiKeyMode::Insert
        )
        .unwrap(),
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
    forged
        .route_key(
            Key::Char('\t'),
            false,
            false,
            marlin_agent_ui::UiKeyMode::Insert,
        )
        .unwrap();
    forged
        .route_key(
            Key::Char('\t'),
            false,
            false,
            marlin_agent_ui::UiKeyMode::Insert,
        )
        .unwrap();
    let previous = forged.view().clone();
    assert!(
        forged
            .route_key(
                Key::Char('\n'),
                false,
                false,
                marlin_agent_ui::UiKeyMode::Insert
            )
            .is_err()
    );
    assert_eq!(forged.view(), &previous);
}

fn view(focus: &str) -> UiWorkbench {
    UiWorkbench {
        layout: test_layout(),
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
        frame: test_frame(),
        keymap: Vec::new(),
        transients: Vec::new(),
    }
}

#[test]
fn terminal_admits_composed_frame_groups_and_rejects_missing_slots() {
    use marlin_agent_ui::UiElement;
    let mut workbench = view("composer");
    let UiElement::Rows(slots) = &workbench.layout.frame_tree else {
        panic!("fixture frame")
    };
    workbench.layout.frame_tree = UiElement::Rows(vec![
        UiElement::Rows(slots[..2].to_vec()),
        UiElement::Rows(slots[2..].to_vec()),
    ]);
    assert!(admit_terminal_view(&workbench).is_ok());
    let UiElement::Rows(groups) = &mut workbench.layout.frame_tree else {
        unreachable!()
    };
    let UiElement::Rows(first) = &mut groups[0] else {
        unreachable!()
    };
    first.remove(0);
    assert!(admit_terminal_view(&workbench).is_err());
}

#[test]
fn terminal_rejects_unbound_main_source() {
    use marlin_agent_ui::UiElement;
    let mut workbench = view("composer");
    let UiElement::Rows(slots) = &mut workbench.layout.frame_tree else {
        panic!("frame")
    };
    let UiElement::Sized { child, .. } = &mut slots[0] else {
        panic!("slot")
    };
    let UiElement::Region { child, .. } = child.as_mut() else {
        panic!("region")
    };
    **child = UiElement::Rows(vec![UiElement::Slot(
        UiComponentId::new("conversation").unwrap(),
    )]);
    assert!(admit_terminal_view(&workbench).is_ok());
    let UiElement::Rows(slots) = &mut workbench.layout.frame_tree else {
        unreachable!()
    };
    let UiElement::Sized { child, .. } = &mut slots[0] else {
        unreachable!()
    };
    let UiElement::Region { child, .. } = child.as_mut() else {
        unreachable!()
    };
    **child = UiElement::Rows(vec![UiElement::Slot(
        UiComponentId::new("unbound").unwrap(),
    )]);
    assert!(admit_terminal_view(&workbench).is_err());
}

#[test]
fn terminal_rejects_unknown_scroll_target_inside_choice() {
    use marlin_agent_ui::{UiElement, UiScrollTarget};
    let mut workbench = view("composer");
    let UiElement::Rows(slots) = &mut workbench.layout.frame_tree else {
        panic!("frame")
    };
    let UiElement::Sized { child, .. } = &mut slots[0] else {
        panic!("transcript size")
    };
    let UiElement::Region { child, .. } = child.as_mut() else {
        panic!("transcript region")
    };
    **child = UiElement::Choice {
        cases: vec![(
            UiComponentId::new("messages").unwrap(),
            UiElement::Region {
                id: UiComponentId::new("nested").unwrap(),
                scroll_target: Some(UiScrollTarget::new("unregistered", 3).unwrap()),
                child: Box::new(UiElement::Text(vec![])),
            },
        )],
        fallback: Box::new(UiElement::Slot(UiComponentId::new("conversation").unwrap())),
    };
    assert!(admit_terminal_view(&workbench).is_err());
}

#[test]
fn terminal_checks_bar_segment_children_before_admission() {
    use marlin_agent_ui::{UiBackground, UiBarSegment, UiBarSide, UiElement, UiScrollTarget};
    let mut workbench = view("composer");
    let UiElement::Rows(slots) = &mut workbench.layout.frame_tree else {
        panic!("frame")
    };
    let UiElement::Sized { child, .. } = &mut slots[0] else {
        panic!("transcript size")
    };
    let UiElement::Region { child, .. } = child.as_mut() else {
        panic!("transcript region")
    };
    **child = UiElement::Bar {
        background: UiBackground::Bar,
        right_divisor: 3,
        segments: vec![UiBarSegment {
            side: UiBarSide::Left,
            priority: 0,
            min_width: 0,
            condition: None,
            child: Box::new(UiElement::Slot(UiComponentId::new("unbound").unwrap())),
        }],
    };
    assert!(admit_terminal_view(&workbench).is_err());

    let UiElement::Rows(slots) = &mut workbench.layout.frame_tree else {
        unreachable!()
    };
    let UiElement::Sized { child, .. } = &mut slots[0] else {
        unreachable!()
    };
    let UiElement::Region { child, .. } = child.as_mut() else {
        unreachable!()
    };
    let UiElement::Bar { segments, .. } = child.as_mut() else {
        unreachable!()
    };
    *segments[0].child = UiElement::Region {
        id: UiComponentId::new("nested").unwrap(),
        scroll_target: Some(UiScrollTarget::new("unregistered", 3).unwrap()),
        child: Box::new(UiElement::Text(vec![])),
    };
    assert!(admit_terminal_view(&workbench).is_err());
}

#[test]
fn terminal_admits_nested_input_and_rejects_duplicate_editors() {
    use marlin_agent_ui::{UiElement, UiSize};
    let mut workbench = view("composer");
    let UiElement::Rows(slots) = &mut workbench.layout.frame_tree else {
        panic!("frame")
    };
    let UiElement::Region { child, .. } = &mut slots[3] else {
        panic!("minibuffer")
    };
    let UiElement::Rows(children) = child.as_mut() else {
        panic!("minibuffer rows")
    };
    let UiElement::Sized { child, .. } = &mut children[0] else {
        panic!("editor size")
    };
    let UiElement::Region { child, .. } = child.as_mut() else {
        panic!("editor region")
    };
    let original = child.clone();
    **child = UiElement::Rows(vec![UiElement::Sized {
        size: UiSize::Flex(1),
        child: original.clone(),
    }]);
    assert!(admit_terminal_view(&workbench).is_ok());
    let UiElement::Rows(slots) = &mut workbench.layout.frame_tree else {
        unreachable!()
    };
    let UiElement::Region { child, .. } = &mut slots[3] else {
        unreachable!()
    };
    let UiElement::Rows(children) = child.as_mut() else {
        unreachable!()
    };
    let UiElement::Sized { child, .. } = &mut children[0] else {
        unreachable!()
    };
    let UiElement::Region { child, .. } = child.as_mut() else {
        unreachable!()
    };
    **child = UiElement::Rows(vec![*original.clone(), *original]);
    assert!(admit_terminal_view(&workbench).is_err());
}

#[test]
fn completion_respects_ambiguity_exact_names_and_arguments() {
    use marlin_agent_ui::{UiCommand, UiCommandArguments, UiCommandCapability, UiCommandName};
    let mut workbench = view("commands");
    workbench.regions[2] = UiRegion::Commands(
        [("help", false), ("model", true), ("models", false)]
            .into_iter()
            .map(|(name, accepts_arguments)| UiCommand {
                name: UiCommandName::new(name).unwrap(),
                description: String::new(),
                arguments: if accepts_arguments {
                    UiCommandArguments::Required
                } else {
                    UiCommandArguments::None
                },
                capability: UiCommandCapability::Clear,
                value: None,
            })
            .collect(),
    );
    let help = UiCommandName::new("help").unwrap();
    let model = UiCommandName::new("model").unwrap();
    let models = UiCommandName::new("models").unwrap();
    assert_eq!(
        complete_command(
            &workbench,
            "",
            &[help.clone(), model.clone(), models.clone()]
        ),
        None
    );
    assert_eq!(
        complete_command(&workbench, "mo", &[model.clone(), models]),
        None
    );
    assert_eq!(
        complete_command(&workbench, "hel", std::slice::from_ref(&help)),
        Some("help".into())
    );
    assert_eq!(
        complete_command(&workbench, "model", &[model]),
        Some("model ".into())
    );
    assert_eq!(complete_command(&workbench, "model foo", &[]), None);
    assert_eq!(complete_command(&workbench, "/模型", &[]), None);
    assert_eq!(
        complete_command(&workbench, &format!("/{}", "m".repeat(1_000_000)), &[]),
        None
    );
    let forged = UiCommandName::new("forged").unwrap();
    assert!(admit_command_choice(std::slice::from_ref(&help), Some(&help)).is_ok());
    assert!(admit_command_choice(&[], Some(&help)).is_err());
    assert!(admit_command_choice(&[help], Some(&forged)).is_err());
}

#[test]
fn command_descriptions_cannot_invent_handlers_or_argument_contracts() {
    use marlin_agent_ui::{UiCommand, UiCommandArguments, UiCommandCapability};
    let mut workbench = view("composer");
    let command = |name, arguments, capability| UiCommand {
        name: UiCommandName::new(name).unwrap(),
        description: "description".into(),
        arguments,
        capability,
        value: None,
    };
    workbench.regions[2] = UiRegion::Commands(vec![command(
        "any-name",
        UiCommandArguments::None,
        UiCommandCapability::Clear,
    )]);
    assert!(admit_terminal_view(&workbench).is_ok());
    workbench.regions[2] = UiRegion::Commands(vec![command(
        "made-up",
        UiCommandArguments::Required,
        UiCommandCapability::Clear,
    )]);
    assert!(admit_terminal_view(&workbench).is_err());
    workbench.regions[2] = UiRegion::Commands(vec![command(
        "choose-model",
        UiCommandArguments::None,
        UiCommandCapability::Model,
    )]);
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
