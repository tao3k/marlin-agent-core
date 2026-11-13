// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use super::{TestUiHost, resolve_test_choice};
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
    assert_eq!(
        host.view().layout.transcript_policy.role_tone,
        marlin_agent_ui::UiTone::Info
    );
    assert!(host.view().layout.transcript_policy.role_bold);
    assert!(
        matches!(host.view().layout.frame_tree.region_child("transcript"), Some(UiElement::Choice { fallback, .. }) if matches!(fallback.as_ref(), UiElement::Slot(id) if id.as_str() == "conversation"))
    );
    let main = host
        .view()
        .layout
        .frame_tree
        .region_child("transcript")
        .unwrap();
    let selected = resolve_test_choice(main, &|fact| {
        matches!(fact.as_str(), "messages" | "runtime")
    });
    assert!(matches!(selected, UiElement::Slot(id) if id.as_str() == "messages"));
    let selected = resolve_test_choice(main, &|fact| fact.as_str() == "static");
    assert!(
        matches!(selected, UiElement::Panel { border_tone: marlin_agent_ui::UiTone::Info, child, .. }
        if matches!(child.as_ref(), UiElement::Slot(id) if id.as_str() == "static"))
    );
    assert!(
        matches!(host.view().layout.frame_tree.region_child("minibuffer-hover"), Some(UiElement::Rows(children)) if children.len() == 3)
    );
    assert!(
        matches!(host.view().layout.frame_tree.region_child("candidates"),
        Some(UiElement::Choice { cases, fallback })
        if cases.iter().any(|(fact, _)| fact.as_str() == "selected")
            && matches!(fallback.as_ref(), UiElement::Template { .. }))
    );
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
