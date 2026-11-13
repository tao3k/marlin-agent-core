// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use super::TestUiHost;
use marlin_agent_ui::{UiElement, UiOpenFacts, UiSize};

#[test]
fn linked_scheme_projects_the_editor_presentation() {
    let runtime = super::initialize().unwrap();
    let facts = UiOpenFacts {
        model_label: "model",
        session_label: "session",
        status: &[],
        extensions: &[],
        method_bundles: &[],
    };
    // SAFETY: the runtime and exports originate in the same linked programme.
    let host = unsafe { TestUiHost::open_linked(&runtime, super::exports(), &facts) }.unwrap();
    fn find_region<'a>(element: &'a UiElement, name: &str) -> Option<&'a UiElement> {
        match element {
            UiElement::Region { id, .. } if id.as_str() == name => Some(element),
            UiElement::Region { child, .. }
            | UiElement::Sized { child, .. }
            | UiElement::Panel { child, .. } => find_region(child, name),
            UiElement::Rows(children) | UiElement::Columns(children) => {
                children.iter().find_map(|child| find_region(child, name))
            }
            _ => None,
        }
    }
    assert!(
        matches!(find_region(&host.view().layout.frame_tree, "transcript"),
        Some(UiElement::Region { scroll_target: Some(target), .. }) if target.as_str() == "transcript")
    );
    fn find_size<'a>(element: &'a UiElement, name: &str) -> Option<&'a UiSize> {
        match element {
            UiElement::Sized { size, child } => match child.as_ref() {
                UiElement::Region { id, .. } if id.as_str() == name => Some(size),
                _ => find_size(child, name),
            },
            UiElement::Region { child, .. } | UiElement::Panel { child, .. } => {
                find_size(child, name)
            }
            UiElement::Rows(children) | UiElement::Columns(children) => {
                children.iter().find_map(|child| find_size(child, name))
            }
            _ => None,
        }
    }
    for (region, fact) in [
        ("candidates", "command"),
        ("extensions", "extensions-enabled"),
    ] {
        assert!(matches!(find_size(&host.view().layout.frame_tree, region),
            Some(UiSize::Choice { cases, .. }) if cases.iter().any(|(id, size)| id.as_str() == fact && matches!(size, UiSize::Min(..)))));
    }
    assert!(
        matches!(find_size(&host.view().layout.frame_tree, "editor"),
        Some(UiSize::Choice { cases, .. }) if cases.iter().any(|(id, size)| id.as_str() == "editor-expanded" && matches!(size, UiSize::Min(..))))
    );
    assert!(
        matches!(host.view().layout.frame_tree.region_child("extensions"),
        Some(UiElement::Choice { cases, .. }) if cases.iter().any(|(id, row)|
            id.as_str() == "selected" && matches!(row, UiElement::Template { .. })))
    );
    // Scheme owns the workbench shape. This checks that its linked editor
    // projection reaches the native typed element boundary.
    let Some(UiElement::Choice { cases, fallback }) =
        host.view().layout.frame_tree.region_child("editor")
    else {
        panic!("expected Scheme editor choice")
    };
    assert!(matches!(
        fallback.as_ref(),
        UiElement::Input { bordered: true, .. }
    ));
    assert!(cases.iter().any(|(fact, input)| fact.as_str() == "command"
        && matches!(
            input,
            UiElement::Input {
                bordered: false,
                ..
            }
        )));
    // Exercise the projected dependency graph, not only its decoded shape.
    let mut app = super::super::app::App::new(marlin_agent_protocol::ModelEndpoint::new(
        "provider", "model",
    ));
    app.toggle_command();
    let regions = super::super::render::frame_regions(
        &app,
        host.view(),
        ratatui::layout::Rect::new(0, 0, 80, 12),
    );
    let region = |name| {
        regions
            .iter()
            .find(|region| region.id.as_str() == name)
            .unwrap()
            .area
    };
    let editor = region("editor");
    let candidates = region("candidates");
    assert_eq!(editor.height, 1);
    assert!(candidates.height > 0);
    assert_eq!(editor.bottom(), candidates.top());
}
