// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use super::TestUiHost;
use marlin_agent_ui::{UiElement, UiOpenFacts};

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
}
