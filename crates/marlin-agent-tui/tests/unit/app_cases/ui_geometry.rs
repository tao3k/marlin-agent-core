// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use super::{App, ModelEndpoint, test_frame, test_layout};

#[test]
fn extension_list_uses_the_projected_row_template() {
    use marlin_agent_ui::{UiComponentId, UiRegion, UiWorkbench};
    use ratatui::{Terminal, backend::TestBackend};
    let app = App::new(ModelEndpoint::new("provider", "model"));
    let view = UiWorkbench {
        model_label: "model".into(),
        session_label: "session".into(),
        regions: vec![
            UiRegion::Transcript,
            UiRegion::Composer,
            UiRegion::Extension {
                id: UiComponentId::new("extension/test").unwrap(),
                label: "Tools".into(),
                content: vec![],
                focusable: true,
            },
        ],
        layout: test_layout(),
        theme: Default::default(),
        focus: UiComponentId::new("composer").unwrap(),
        frame: test_frame(),
        keymap: Vec::new(),
        transients: Vec::new(),
    };
    let mut terminal = Terminal::new(TestBackend::new(80, 20)).unwrap();
    terminal
        .draw(|frame| {
            super::render_draw(
                frame,
                &app,
                &view,
                &mut Default::default(),
                &mut Default::default(),
            );
        })
        .unwrap();
    let buffer = terminal.backend().buffer();
    let text: String = (0..20)
        .flat_map(|y| (0..80).map(move |x| buffer[(x, y)].symbol()))
        .collect();
    assert!(text.contains("fixture: Tools"));
}

#[test]
fn command_list_tracks_the_declared_editor_size() {
    use marlin_agent_ui::{UiComponentId, UiElement, UiRegion, UiSize, UiWorkbench};
    let mut app = App::new(ModelEndpoint::new("provider", "model"));
    app.toggle_command();
    let mut view = UiWorkbench {
        model_label: "model".into(),
        session_label: "session".into(),
        regions: vec![UiRegion::Transcript, UiRegion::Composer],
        layout: test_layout(),
        theme: Default::default(),
        focus: UiComponentId::new("composer").unwrap(),
        frame: test_frame(),
        keymap: Vec::new(),
        transients: Vec::new(),
    };
    let candidate_rows = |view: &UiWorkbench| {
        super::super::render::frame_regions(&app, view, ratatui::layout::Rect::new(0, 0, 80, 12))
            .into_iter()
            .find(|region| region.id.as_str() == "candidates")
            .unwrap()
            .area
            .height
    };
    let before = candidate_rows(&view);
    let UiElement::Rows(slots) = &mut view.layout.frame_tree else {
        panic!("frame")
    };
    let UiElement::Region { child, .. } = &mut slots[3] else {
        panic!("minibuffer")
    };
    let UiElement::Rows(children) = child.as_mut() else {
        panic!("rows")
    };
    let UiElement::Sized { size, .. } = &mut children[0] else {
        panic!("editor")
    };
    *size = UiSize::Cells(4);
    assert_eq!(before.saturating_sub(candidate_rows(&view)), 3);
}

#[test]
fn selecting_buffer_previews_without_committing_and_cancel_restores() {
    use super::{install_test_commands, render_draw};
    use marlin_agent_ui::{UiCommandName, UiComponentId, UiRegion, UiWorkbench};
    use ratatui::{Terminal, backend::TestBackend};

    let mut app = App::new(ModelEndpoint::new("provider", "model"));
    install_test_commands(&mut app);
    app.transcript
        .push(("You".into(), "conversation content".into()));
    app.messages
        .push(("UI".into(), "message buffer content".into()));
    app.composer.insert_text("retained draft");
    app.toggle_command();
    let model = UiCommandName::new("models").unwrap();
    let messages = UiCommandName::new("buffer-messages").unwrap();
    app.command_candidates = vec![model.clone(), messages.clone()];
    app.command_selection = Some((String::new(), model));
    let view = UiWorkbench {
        model_label: "model".into(),
        session_label: "session".into(),
        regions: vec![
            UiRegion::Transcript,
            UiRegion::Composer,
            UiRegion::Commands(app.command_catalog.clone()),
            UiRegion::Status(vec![]),
        ],
        layout: test_layout(),
        theme: Default::default(),
        focus: UiComponentId::new("composer").unwrap(),
        frame: test_frame(),
        keymap: Vec::new(),
        transients: Vec::new(),
    };
    let mut terminal = Terminal::new(TestBackend::new(80, 40)).unwrap();
    let mut cache = Default::default();
    let mut anchor = Default::default();
    let mut draw = |terminal: &mut Terminal<TestBackend>, app: &App| {
        terminal
            .draw(|frame| {
                render_draw(frame, app, &view, &mut cache, &mut anchor);
            })
            .unwrap();
        let buffer = terminal.backend().buffer();
        (0..40)
            .flat_map(|y| (0..80).map(move |x| buffer[(x, y)].symbol()))
            .collect::<String>()
    };
    assert!(draw(&mut terminal, &app).contains("conversation content"));
    // Vertico selects the first candidate immediately, before any arrow key.
    app.command_candidates = vec![messages.clone(), UiCommandName::new("models").unwrap()];
    app.command_selection = None;
    assert!(draw(&mut terminal, &app).contains("message buffer content"));
    app.command_candidates = vec![UiCommandName::new("models").unwrap(), messages.clone()];
    app.command_selection = Some((String::new(), messages));
    let text = draw(&mut terminal, &app);
    assert!(text.contains("message buffer content"));
    assert!(!text.contains("conversation content"));
    assert_eq!(app.buffer_view, super::super::app::BufferView::Conversation);
    assert!(app.ui_request.is_none());
    app.command_selection = Some((String::new(), UiCommandName::new("models").unwrap()));
    assert!(draw(&mut terminal, &app).contains("conversation content"));
    app.command_selection = Some((
        String::new(),
        UiCommandName::new("buffer-messages").unwrap(),
    ));
    app.scroll_lines(true, 3);
    assert_eq!(app.message_scroll_up, 3);
    assert_eq!(app.scroll_up, 0);
    app.cancel();
    assert_eq!(app.composer.text(), "retained draft");
    assert!(app.ui_request.is_none());
    assert!(draw(&mut terminal, &app).contains("conversation content"));
}

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
