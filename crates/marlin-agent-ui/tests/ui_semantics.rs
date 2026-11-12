// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use marlin_agent_ui::{UiComponentId, UiEvent, UiIntent, UiLayoutPolicy, UiRegion, UiWorkbench};

fn id(value: &str) -> UiComponentId {
    UiComponentId::new(value).unwrap()
}

fn workbench(focus: &str) -> UiWorkbench {
    UiWorkbench {
        layout: UiLayoutPolicy::default(),
        model_label: "provider/model".into(),
        session_label: "session".into(),
        regions: vec![
            UiRegion::Transcript,
            UiRegion::Composer,
            UiRegion::Extension {
                id: id("guide"),
                label: "Guide".into(),
                focusable: true,
                content: vec![],
            },
        ],
        focus: id(focus),
    }
}

#[test]
fn shared_ui_admission_rejects_effect_and_snapshot_substitution() {
    let composer = workbench("composer");
    let guide = workbench("guide");
    let focus = UiIntent::Focus(id("guide"));
    assert!(
        composer
            .admit_successor(&guide, UiEvent::NextFocus, &focus)
            .is_ok()
    );
    assert!(
        composer
            .admit_successor(&guide, UiEvent::Submit, &focus)
            .is_err()
    );

    let mut altered = guide.clone();
    altered.regions[2] = UiRegion::Transcript;
    assert!(
        composer
            .admit_successor(&altered, UiEvent::NextFocus, &focus)
            .is_err()
    );
    let mut changed_layout = guide.clone();
    changed_layout.layout.max_chatbox_rows = 5;
    assert!(
        composer
            .admit_successor(&changed_layout, UiEvent::NextFocus, &focus)
            .is_err()
    );
    assert!(
        guide
            .admit_successor(&composer, UiEvent::Submit, &UiIntent::Focus(id("composer")))
            .is_ok()
    );
}
