// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use super::{App, ModelEndpoint, test_frame, test_layout};

#[test]
fn frame_hit_testing_uses_the_selected_scheme_branch() {
    use marlin_agent_ui::{UiComponentId, UiElement, UiRegion, UiSize, UiWorkbench};

    let id = |name| UiComponentId::new(name).unwrap();
    let region = |name| UiElement::Region {
        id: id(name),
        scroll_target: Some(marlin_agent_ui::UiScrollTarget::new("transcript").unwrap()),
        child: Box::new(UiElement::Text(Vec::new())),
    };
    let mut layout = test_layout();
    layout.frame_tree = UiElement::Rows(vec![UiElement::Sized {
        size: UiSize::Flex(1),
        child: Box::new(UiElement::Choice {
            cases: vec![(id("messages"), region("message-scroll"))],
            fallback: Box::new(region("conversation-scroll")),
        }),
    }]);
    let view = UiWorkbench {
        model_label: "model".into(),
        session_label: "session".into(),
        regions: vec![UiRegion::Transcript],
        layout,
        theme: Default::default(),
        focus: id("composer"),
        frame: test_frame(),
        keymap: Vec::new(),
        transients: Vec::new(),
    };
    let mut app = App::new(ModelEndpoint::new("provider", "model"));
    let hit = |app: &App| {
        let regions = super::super::render::frame_regions(
            app,
            &view,
            ratatui::layout::Rect::new(0, 0, 80, 20),
        );
        let hit = super::super::ui_elements::hit_test_scroll(&regions, 0, 0).unwrap();
        (
            hit.id.as_str().to_owned(),
            hit.scroll_target().unwrap().to_owned(),
        )
    };
    assert_eq!(
        hit(&app),
        ("conversation-scroll".into(), "transcript".into())
    );
    app.buffer_view = super::super::app::BufferView::Messages;
    assert_eq!(hit(&app), ("message-scroll".into(), "transcript".into()));
}
