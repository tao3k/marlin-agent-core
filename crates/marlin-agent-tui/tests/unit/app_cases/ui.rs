// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use super::{
    App, FixtureGateway, ModelEndpoint, install_test_commands, modeline_fixture_item,
    mouse_over_minibuffer, render_draw, reply_channel, test_frame, test_layout,
};
use std::sync::{Arc, Mutex};

#[test]
fn process_buffer_refresh_reuses_unchanged_runtime_snapshot() {
    let mut app = App::new(ModelEndpoint::new("provider", "model"));
    assert!(app.refresh_process_content());
    let snapshot = app.dynamic_buffer_content.clone();
    assert!(!app.refresh_process_content());
    assert_eq!(app.dynamic_buffer_content, snapshot);
    assert_eq!(app.dynamic_buffer_content.len(), 1);
}

#[test]
fn dynamic_buffer_rejects_oversized_replacement_without_changing_its_view() {
    use marlin_agent_ui::{UiBufferId, UiContent};

    let mut app = App::new(ModelEndpoint::new("provider", "model"));
    let id = UiBufferId::new("processes").unwrap();
    assert_eq!(
        app.replace_dynamic_buffer_content(id.clone(), vec![UiContent::Text("ready".into())]),
        Ok(true)
    );
    assert_eq!(
        app.replace_dynamic_buffer_content(id.clone(), vec![UiContent::Text("ready".into())]),
        Ok(false)
    );
    assert!(
        app.replace_dynamic_buffer_content(
            id.clone(),
            vec![UiContent::Text("x".repeat(32 * 1024 + 1))]
        )
        .is_err()
    );
    assert_eq!(
        app.dynamic_buffer_content.get(&id),
        Some(&vec![UiContent::Text("ready".into())])
    );
    assert!(app.transcript.is_empty());
}

#[test]
fn runtime_events_enter_messages_in_bounded_batches_without_polluting_conversation() {
    let (owner, events) = marlin_agent_runtime::TokioAgentRuntime::new(64);
    let sink = owner.event_sink();
    let mut app = App::with_runtime(ModelEndpoint::new("provider", "model"), owner, events);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    runtime.block_on(async {
        for index in 0..3 {
            let topic = if index == 2 {
                format!("\u{1b}{}", "x".repeat(1024))
            } else {
                "tool.progress".to_owned()
            };
            sink.emit(marlin_agent_protocol::AgentEvent::new(
                topic,
                format!("step {index}"),
            ))
            .await
            .unwrap();
        }
    });

    assert_eq!(app.poll_runtime_events(2), 2);
    assert_eq!(app.messages.len(), 2);
    assert!(app.transcript.is_empty());
    assert_eq!(app.poll_runtime_events(2), 1);
    assert!(app.messages.last().unwrap().1.contains("step 2"));
    assert!(!app.messages.last().unwrap().1.contains('\u{1b}'));
    assert!(app.messages.last().unwrap().1.chars().count() < 300);
    assert_eq!(app.poll_runtime_events(2), 0);
}

#[test]
fn typed_thinking_capability_reaches_the_selected_segment() {
    use marlin_agent_ui::{UiCommand, UiCommandArguments, UiCommandCapability, UiCommandName};
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let gateway = FixtureGateway {
        requests: Arc::new(Mutex::new(Vec::new())),
    };
    let (tx, _) = reply_channel(8, 4096);
    let mut app = App::new(ModelEndpoint::new("provider", "model"));
    app.command_catalog = vec![UiCommand {
        name: UiCommandName::new("thought").unwrap(),
        description: String::new(),
        arguments: UiCommandArguments::Optional,
        capability: UiCommandCapability::ToggleThinking,
        value: None,
    }];
    app.transcript
        .push_thinking("open".into(), "closed", "older".into(), true);
    app.transcript
        .push_thinking("open".into(), "closed", "newer".into(), true);

    app.toggle_command();
    app.composer.insert_text("thought 2");
    app.submit(&gateway, &runtime, &tx);
    assert_eq!(app.transcript[0].1, "older");
    assert!(app.transcript[1].1.is_empty());
}

#[test]
fn command_menu_filters_scheme_descriptors_and_preserves_draft() {
    use marlin_agent_ui::{UiCommand, UiCommandName, UiComponentId, UiRegion, UiWorkbench};
    let mut app = App::new(ModelEndpoint::new("provider", "model"));
    install_test_commands(&mut app);
    app.composer.insert_text("preserved draft");
    app.toggle_command();
    app.composer.insert_text("mo");
    let command = |name, description: &str| UiCommand {
        name: UiCommandName::new(name).unwrap(),
        description: description.into(),
        arguments: marlin_agent_ui::UiCommandArguments::None,
        capability: marlin_agent_ui::UiCommandCapability::Clear,
        value: None,
    };
    let view = UiWorkbench {
        layout: test_layout(),
        theme: marlin_agent_ui::UiTheme::default(),
        model_label: "model".into(),
        session_label: "session".into(),
        regions: vec![
            UiRegion::Transcript,
            UiRegion::Composer,
            UiRegion::Commands(vec![
                command("help", "Keyboard shortcuts"),
                command("models", "List models"),
            ]),
            UiRegion::Status(vec![]),
        ],
        focus: UiComponentId::new("composer").unwrap(),
        frame: test_frame(),
        keymap: Vec::new(),
        transients: Vec::new(),
    };
    app.command_candidates = vec![UiCommandName::new("models").unwrap()];
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(60, 10)).unwrap();
    terminal
        .draw(|frame| {
            render_draw(
                frame,
                &app,
                &view,
                &mut Default::default(),
                &mut Default::default(),
            );
        })
        .unwrap();
    let buffer = terminal.backend().buffer();
    let text: String = (0..10)
        .flat_map(|y| (0..60).map(move |x| buffer[(x, y)].symbol()))
        .collect();
    assert!(text.contains("List models"), "{text}");
    assert!(text.contains("M-x mo"), "{text}");
    assert!(text.contains("› models"), "{text}");
    assert!(!text.contains("Keyboard shortcuts"), "{text}");
    app.toggle_command();
    assert_eq!(app.composer.text(), "preserved draft");
    app.composer.take();
    app.composer.insert_text("mo");
    terminal
        .draw(|frame| {
            render_draw(
                frame,
                &app,
                &view,
                &mut Default::default(),
                &mut Default::default(),
            );
        })
        .unwrap();
    let buffer = terminal.backend().buffer();
    let text: String = (0..10)
        .flat_map(|y| (0..60).map(move |x| buffer[(x, y)].symbol()))
        .collect();
    assert!(!text.contains("List models"), "{text}");
    assert!(!text.contains("Tab complete"), "{text}");
}

#[test]
fn command_menu_scrolls_to_the_selected_entry() {
    use marlin_agent_ui::{UiCommand, UiCommandName, UiComponentId, UiRegion, UiWorkbench};
    let mut app = App::new(ModelEndpoint::new("provider", "model"));
    install_test_commands(&mut app);
    app.toggle_command();
    let names = ["help", "models", "model", "sessions", "session", "clear"];
    let commands = names
        .into_iter()
        .map(|name| UiCommand {
            name: UiCommandName::new(name).unwrap(),
            description: name.into(),
            arguments: marlin_agent_ui::UiCommandArguments::None,
            capability: marlin_agent_ui::UiCommandCapability::Clear,
            value: None,
        })
        .collect();
    app.command_candidates = names
        .into_iter()
        .map(|name| UiCommandName::new(name).unwrap())
        .collect();
    let view = UiWorkbench {
        layout: test_layout(),
        theme: marlin_agent_ui::UiTheme::default(),
        model_label: "model".into(),
        session_label: "session".into(),
        regions: vec![
            UiRegion::Transcript,
            UiRegion::Composer,
            UiRegion::Commands(commands),
            UiRegion::Status(vec![]),
        ],
        focus: UiComponentId::new("composer").unwrap(),
        frame: test_frame(),
        keymap: Vec::new(),
        transients: Vec::new(),
    };
    app.command_selection = Some(("".into(), UiCommandName::new("clear").unwrap()));
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(60, 10)).unwrap();
    terminal
        .draw(|frame| {
            render_draw(
                frame,
                &app,
                &view,
                &mut Default::default(),
                &mut Default::default(),
            );
        })
        .unwrap();
    let buffer = terminal.backend().buffer();
    let text: String = (0..10)
        .flat_map(|y| (0..60).map(move |x| buffer[(x, y)].symbol()))
        .collect();
    assert!(text.contains("› clear"), "{text}");
    assert!(!text.contains("help  /help"), "{text}");
    app.composer.insert_text("h");
    assert!(app.selected_command().is_none());
}

#[test]
fn mx_minibuffer_and_candidates_are_below_modeline() {
    use marlin_agent_ui::{UiCommand, UiCommandName, UiComponentId, UiRegion, UiWorkbench};
    let mut app = App::new(ModelEndpoint::new("provider", "model"));
    install_test_commands(&mut app);
    app.toggle_command();
    let mut commands = vec![UiCommand {
        name: UiCommandName::new("help").unwrap(),
        description: "Keyboard shortcuts".into(),
        arguments: marlin_agent_ui::UiCommandArguments::None,
        capability: marlin_agent_ui::UiCommandCapability::Clear,
        value: None,
    }];
    for index in 0..14 {
        commands.push(UiCommand {
            name: UiCommandName::new(&format!("command-{index:02}")).unwrap(),
            description: "Additional command".into(),
            arguments: marlin_agent_ui::UiCommandArguments::None,
            capability: marlin_agent_ui::UiCommandCapability::Clear,
            value: None,
        });
    }
    app.command_candidates = commands
        .iter()
        .map(|command| command.name.clone())
        .collect();
    let view = UiWorkbench {
        layout: marlin_agent_ui::UiLayoutPolicy {
            max_command_rows: 10,
            command_frame: marlin_agent_ui::UiFrameSpec {
                min_transcript_rows: 4,
                ..test_layout().command_frame
            },
            ..test_layout()
        },
        theme: Default::default(),
        model_label: "model".into(),
        session_label: "session".into(),
        regions: vec![
            UiRegion::Transcript,
            UiRegion::Composer,
            UiRegion::Commands(commands),
            UiRegion::Status(
                [
                    "modeline/left/brand",
                    "modeline/left/model",
                    "modeline/left/activity",
                    "modeline/right/turns",
                    "modeline/right/session",
                    "modeline/state/online",
                    "modeline/state/offline",
                    "modeline/state/unknown",
                ]
                .into_iter()
                .map(modeline_fixture_item)
                .collect(),
            ),
        ],
        focus: UiComponentId::new("composer").unwrap(),
        frame: test_frame(),
        keymap: Vec::new(),
        transients: Vec::new(),
    };
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 18)).unwrap();
    terminal
        .draw(|frame| {
            render_draw(
                frame,
                &app,
                &view,
                &mut Default::default(),
                &mut Default::default(),
            );
        })
        .unwrap();
    let rows: Vec<String> = (0..18)
        .map(|y| {
            (0..80)
                .map(|x| terminal.backend().buffer()[(x, y)].symbol())
                .collect()
        })
        .collect();
    let modeline = rows.iter().position(|row| row.contains("ANCHOR")).unwrap();
    let prompt = rows.iter().position(|row| row.contains("M-x ")).unwrap();
    let candidate = rows
        .iter()
        .position(|row| row.contains("Keyboard shortcuts"))
        .unwrap();
    assert!(modeline < prompt && prompt + 1 == candidate, "{rows:?}");
    assert!(!rows[prompt].contains('┌') && !rows[prompt].contains('│'));
    assert!(
        !rows.iter().any(|row| row.contains("command-13")),
        "{rows:?}"
    );
    assert!(!rows.iter().any(|row| row.contains("Tab complete")));

    app.command_candidates.truncate(1);
    terminal
        .draw(|frame| {
            render_draw(
                frame,
                &app,
                &view,
                &mut Default::default(),
                &mut Default::default(),
            );
        })
        .unwrap();
    let buffer = terminal.backend().buffer();
    let stable_modeline = (0..18)
        .find(|y| {
            (0..80)
                .map(|x| buffer[(x, *y)].symbol())
                .collect::<String>()
                .contains("ANCHOR")
        })
        .unwrap();
    assert_eq!(
        usize::from(stable_modeline),
        modeline,
        "filtering must keep the minibuffer height fixed"
    );
    app.toggle_command();
    terminal
        .draw(|frame| {
            render_draw(
                frame,
                &app,
                &view,
                &mut Default::default(),
                &mut Default::default(),
            );
        })
        .unwrap();
    let buffer = terminal.backend().buffer();
    let restored_modeline = (0..18)
        .find(|y| {
            (0..80)
                .map(|x| buffer[(x, *y)].symbol())
                .collect::<String>()
                .contains("ANCHOR")
        })
        .unwrap();
    assert!(
        usize::from(restored_modeline) > modeline,
        "closing must return the candidate area"
    );
    app.toggle_command();
    let mut reordered = view.clone();
    let marlin_agent_ui::UiElement::Rows(slots) = &mut reordered.layout.frame_tree else {
        panic!("test frame must be rows");
    };
    let marlin_agent_ui::UiElement::Region { child, .. } = &mut slots[3] else {
        panic!("minibuffer")
    };
    let marlin_agent_ui::UiElement::Rows(minibuffer) = child.as_mut() else {
        panic!("minibuffer rows")
    };
    let marlin_agent_ui::UiElement::Sized { child, .. } = &mut minibuffer[0] else {
        panic!("editor size")
    };
    let marlin_agent_ui::UiElement::Region { child, .. } = child.as_mut() else {
        panic!("editor region")
    };
    let original = std::mem::replace(child, Box::new(marlin_agent_ui::UiElement::Text(vec![])));
    **child = marlin_agent_ui::UiElement::Rows(vec![marlin_agent_ui::UiElement::Sized {
        size: marlin_agent_ui::UiSize::Flex(1),
        child: original,
    }]);
    slots.swap(2, 3);
    let geometry = super::super::render::frame_regions(
        &app,
        &reordered,
        ratatui::layout::Rect::new(0, 0, 80, 18),
    );
    let editor = geometry
        .iter()
        .find(|region| region.id.as_str() == "editor")
        .unwrap();
    assert_eq!(
        super::super::ui_elements::hit_test(&geometry, editor.area.x, editor.area.y)
            .unwrap()
            .id
            .as_str(),
        "editor"
    );
    terminal
        .draw(|frame| {
            render_draw(
                frame,
                &app,
                &reordered,
                &mut Default::default(),
                &mut Default::default(),
            );
        })
        .unwrap();
    let prompt: String = (0..80)
        .map(|x| terminal.backend().buffer()[(x, editor.area.y)].symbol())
        .collect();
    assert!(
        prompt.contains("M-x "),
        "render and hit test must share the Scheme geometry: {prompt:?}"
    );
}

#[test]
fn mx_switch_themes_keeps_message_draft_and_emits_one_profile_request() {
    use marlin_agent_ui::{UiEvent, UiProfileName};
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let requests = Arc::new(Mutex::new(Vec::new()));
    let gateway = FixtureGateway {
        requests: requests.clone(),
    };
    let (tx, _) = reply_channel(8, 4096);
    let mut app = App::new(ModelEndpoint::new("provider", "model"));
    install_test_commands(&mut app);
    app.composer.insert_text("retained message");
    app.toggle_command();
    app.composer.insert_text("switch-themes");
    app.submit(&gateway, &runtime, &tx);
    assert_eq!(
        app.command_history_request.take().unwrap().as_str(),
        "switch-themes"
    );
    assert_eq!(app.composer.text(), "");
    assert_eq!(app.command_lookup_query(), "theme-");
    assert!(app.command_draft.is_some());
    app.composer.insert_text("pi");
    app.submit(&gateway, &runtime, &tx);
    assert_eq!(
        app.command_history_request.take().unwrap().as_str(),
        "theme-pi"
    );
    assert_eq!(app.composer.text(), "retained message");
    assert!(app.command_draft.is_none());
    assert_eq!(
        app.ui_request,
        Some(UiEvent::SelectProfile(UiProfileName::new("pi").unwrap()))
    );
    assert!(requests.lock().unwrap().is_empty());
}

#[test]
fn cancelled_and_incomplete_commands_do_not_enter_history() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let gateway = FixtureGateway {
        requests: Arc::new(Mutex::new(Vec::new())),
    };
    let (tx, _) = reply_channel(8, 4096);
    let mut app = App::new(ModelEndpoint::new("provider", "model"));
    install_test_commands(&mut app);
    app.toggle_command();
    app.composer.insert_text("model");
    app.cancel();
    assert!(app.command_history_request.is_none());
    app.toggle_command();
    app.composer.insert_text("model");
    app.submit(&gateway, &runtime, &tx);
    assert!(app.command_history_request.is_none());
    app.toggle_command();
    app.composer.insert_text("model private-argument");
    app.submit(&gateway, &runtime, &tx);
    assert_eq!(
        app.command_history_request.take().unwrap().as_str(),
        "model"
    );
}

#[test]
fn scoped_buffer_completion_and_cancel_keep_the_message_draft() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let gateway = FixtureGateway {
        requests: Arc::new(Mutex::new(Vec::new())),
    };
    let (tx, _) = reply_channel(8, 4096);
    let mut app = App::new(ModelEndpoint::new("provider", "model"));
    install_test_commands(&mut app);
    app.composer.insert_text("unsent message");
    app.toggle_command();
    app.composer.insert_text("switch-buffer");
    app.submit(&gateway, &runtime, &tx);
    app.command_query = Some("buffer-".into());
    assert!(app.command_query_is_current());
    assert!(app.accept_command_completion("buffer-messages"));
    assert_eq!(app.composer.text(), "messages");
    assert_eq!(app.command_lookup_query(), "buffer-messages");
    assert!(!app.command_query_is_current());
    app.cancel();
    assert!(app.command_menu.is_none());
    assert_eq!(app.composer.text(), "unsent message");
    assert!(app.ui_request.is_none());
}

#[test]
fn event_messages_use_a_switchable_buffer_without_polluting_conversation() {
    use crate::harness::app::{BufferView, MessageEvent, MessageSeverity, MessageSource};
    use marlin_agent_ui::{UiComponentId, UiRegion, UiWorkbench};
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let requests = Arc::new(Mutex::new(Vec::new()));
    let gateway = FixtureGateway {
        requests: requests.clone(),
    };
    let (tx, _) = reply_channel(8, 4096);
    let mut app = App::new(ModelEndpoint::new("provider", "model"));
    install_test_commands(&mut app);
    app.transcript
        .push(("You".into(), "conversation remains here".into()));
    let event = MessageEvent {
        source: MessageSource::Ui,
        severity: MessageSeverity::Error,
        summary: "Profile selection failed",
        detail: "native status 3",
    };
    app.publish_message(event);
    app.publish_message(event);
    assert_eq!(app.transcript.len(), 1);
    assert_eq!(app.messages.len(), 2);
    assert_eq!(
        app.minibuffer_notice.as_deref(),
        Some("Profile selection failed")
    );
    assert_eq!(
        app.messages[0].1,
        "Profile selection failed: native status 3"
    );
    assert_eq!(app.messages[1], app.messages[0]);
    app.composer.insert_text("retained draft");
    app.toggle_command();
    app.composer.insert_text("switch-buffer");
    app.submit(&gateway, &runtime, &tx);
    assert_eq!(app.composer.text(), "");
    assert_eq!(app.command_lookup_query(), "buffer-");
    app.composer.insert_text("messages");
    app.submit(&gateway, &runtime, &tx);
    assert_eq!(app.buffer_view, BufferView::Conversation);
    assert_eq!(
        app.ui_request,
        Some(marlin_agent_ui::UiEvent::SelectBuffer(
            marlin_agent_ui::UiBufferId::new("messages").unwrap()
        ))
    );
    assert_eq!(app.composer.text(), "retained draft");
    let mut view = UiWorkbench {
        layout: test_layout(),
        theme: Default::default(),
        model_label: "model".into(),
        session_label: "session".into(),
        regions: vec![
            UiRegion::Transcript,
            UiRegion::Composer,
            UiRegion::Commands(vec![]),
            UiRegion::Status(vec![]),
        ],
        focus: UiComponentId::new("composer").unwrap(),
        frame: test_frame(),
        keymap: Vec::new(),
        transients: Vec::new(),
    };
    view.frame.windows[0].buffer = marlin_agent_ui::UiBufferId::new("messages").unwrap();
    app.sync_window(&view).unwrap();
    assert_eq!(app.buffer_view, BufferView::Messages);
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 12)).unwrap();
    terminal
        .draw(|frame| {
            render_draw(
                frame,
                &app,
                &view,
                &mut Default::default(),
                &mut Default::default(),
            );
        })
        .unwrap();
    let buffer = terminal.backend().buffer();
    let text: String = (0..12)
        .flat_map(|y| (0..80).map(move |x| buffer[(x, y)].symbol()))
        .collect();
    assert!(text.contains("native status 3"));
    assert!(!text.contains("conversation remains here"));
    let marlin_agent_ui::UiElement::Rows(slots) = &mut view.layout.frame_tree else {
        panic!("frame")
    };
    let marlin_agent_ui::UiElement::Sized { child, .. } = &mut slots[0] else {
        panic!("slot")
    };
    let marlin_agent_ui::UiElement::Region { child, .. } = child.as_mut() else {
        panic!("region")
    };
    let marlin_agent_ui::UiElement::Choice { cases, .. } = child.as_mut() else {
        panic!("main choice")
    };
    let (_, selected) = cases
        .iter_mut()
        .find(|(fact, _)| fact.as_str() == "messages")
        .unwrap();
    *selected = marlin_agent_ui::UiElement::Rows(vec![marlin_agent_ui::UiElement::Sized {
        size: marlin_agent_ui::UiSize::Flex(1),
        child: Box::new(marlin_agent_ui::UiElement::Slot(
            UiComponentId::new("conversation").unwrap(),
        )),
    }]);
    terminal
        .draw(|frame| {
            render_draw(
                frame,
                &app,
                &view,
                &mut Default::default(),
                &mut Default::default(),
            );
        })
        .unwrap();
    let buffer = terminal.backend().buffer();
    let text: String = (0..12)
        .flat_map(|y| (0..80).map(move |x| buffer[(x, y)].symbol()))
        .collect();
    assert!(text.contains("conversation remains here"));
    assert!(!text.contains("native status 3"));
    app.cancel();
    assert_eq!(
        app.ui_request,
        Some(marlin_agent_ui::UiEvent::SelectBuffer(
            marlin_agent_ui::UiBufferId::new("conversation").unwrap()
        ))
    );
    assert!(requests.lock().unwrap().is_empty());
}

#[test]
fn scheme_static_buffer_replaces_window_content_without_changing_conversation() {
    use crate::harness::app::BufferView;
    use marlin_agent_ui::{UiBufferId, UiComponentId, UiContent, UiRegion, UiWorkbench};
    let mut app = App::new(ModelEndpoint::new("provider", "model"));
    app.transcript
        .push(("You".into(), "private conversation".into()));
    let mut frame = test_frame();
    frame.buffers.push(UiBufferId::new("help").unwrap());
    frame.windows[0].buffer = UiBufferId::new("help").unwrap();
    let mut view = UiWorkbench {
        layout: test_layout(),
        theme: Default::default(),
        model_label: "model".into(),
        session_label: "session".into(),
        regions: vec![
            UiRegion::Transcript,
            UiRegion::Composer,
            UiRegion::Commands(vec![]),
            UiRegion::Extension {
                id: UiComponentId::new("help").unwrap(),
                label: "*Help*".into(),
                content: vec![UiContent::Text("Ctrl-R  Search prompts".into())],
                focusable: false,
            },
            UiRegion::Status(vec![]),
        ],
        focus: UiComponentId::new("composer").unwrap(),
        frame,
        keymap: Vec::new(),
        transients: Vec::new(),
    };
    app.sync_window(&view).unwrap();
    assert_eq!(
        app.buffer_view,
        BufferView::Named(UiBufferId::new("help").unwrap())
    );
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 16)).unwrap();
    terminal
        .draw(|frame| {
            render_draw(
                frame,
                &app,
                &view,
                &mut Default::default(),
                &mut Default::default(),
            );
        })
        .unwrap();
    let buffer = terminal.backend().buffer();
    let text: String = (0..16)
        .flat_map(|y| (0..80).map(move |x| buffer[(x, y)].symbol()))
        .collect();
    assert!(text.contains("Ctrl-R  Search prompts"), "{text}");
    assert!(!text.contains("private conversation"), "{text}");
    assert_eq!(app.transcript.len(), 1);

    let processes = UiBufferId::new("processes").unwrap();
    view.frame.buffers.push(processes.clone());
    view.frame.windows[0].buffer = processes.clone();
    view.regions.push(UiRegion::Extension {
        id: UiComponentId::new("processes").unwrap(),
        label: "*Processes*".into(),
        content: vec![],
        focusable: false,
    });
    app.replace_dynamic_buffer_content(
        processes.clone(),
        vec![UiContent::Text("Generation 7 Running".into())],
    )
    .unwrap();
    app.sync_window(&view).unwrap();
    assert_eq!(app.buffer_view, BufferView::Named(processes));
    terminal
        .draw(|frame| {
            render_draw(
                frame,
                &app,
                &view,
                &mut Default::default(),
                &mut Default::default(),
            );
        })
        .unwrap();
    let buffer = terminal.backend().buffer();
    let text: String = (0..16)
        .flat_map(|y| (0..80).map(move |x| buffer[(x, y)].symbol()))
        .collect();
    assert!(text.contains("*Processes*"), "{text}");
    assert!(text.contains("Generation 7 Running"), "{text}");
    assert!(!text.contains("private conversation"), "{text}");
}

#[test]
fn modeline_sits_above_the_collapsible_minibuffer() {
    use marlin_agent_ui::{UiComponentId, UiRegion, UiWorkbench};
    let mut app = App::new(ModelEndpoint::new("deepseek", "chat"));
    install_test_commands(&mut app);
    assert!(app.transcript.is_empty());
    let view = UiWorkbench {
        layout: test_layout(),
        theme: marlin_agent_ui::UiTheme::default(),
        model_label: "deepseek/chat".into(),
        session_label: "session-one".into(),
        regions: vec![
            UiRegion::Transcript,
            UiRegion::Composer,
            UiRegion::Commands(vec![]),
            UiRegion::Status(vec![
                modeline_fixture_item("modeline/left/brand"),
                modeline_fixture_item("modeline/left/model"),
                modeline_fixture_item("modeline/left/activity"),
                modeline_fixture_item("modeline/right/turns"),
                modeline_fixture_item("modeline/right/session"),
                modeline_fixture_item("modeline/state/online"),
                modeline_fixture_item("modeline/state/offline"),
                modeline_fixture_item("modeline/state/unknown"),
            ]),
        ],
        focus: UiComponentId::new("composer").unwrap(),
        frame: test_frame(),
        keymap: Vec::new(),
        transients: Vec::new(),
    };
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 12)).unwrap();
    fn paint(
        terminal: &mut ratatui::Terminal<ratatui::backend::TestBackend>,
        app: &App,
        view: &UiWorkbench,
    ) {
        terminal
            .draw(|frame| {
                render_draw(
                    frame,
                    app,
                    view,
                    &mut Default::default(),
                    &mut Default::default(),
                );
            })
            .unwrap();
    }
    paint(&mut terminal, &app, &view);
    let row = |terminal: &ratatui::Terminal<ratatui::backend::TestBackend>, y| {
        (0..80)
            .map(|x| terminal.backend().buffer()[(x, y)].symbol())
            .collect::<String>()
    };
    assert!(row(&terminal, 10).contains("ANCHOR"));
    assert!(mouse_over_minibuffer(&app, &test_layout(), 80, 12, 12));
    assert!(!mouse_over_minibuffer(&app, &test_layout(), 80, 12, 10));

    app.minibuffer_hovered = true;
    paint(&mut terminal, &app, &view);
    assert!(row(&terminal, 8).contains("Message"));
    assert!(row(&terminal, 7).contains("ANCHOR"));
    assert!(mouse_over_minibuffer(&app, &test_layout(), 80, 12, 9));
    assert!(!mouse_over_minibuffer(&app, &test_layout(), 80, 12, 7));
    app.minibuffer_hovered = false;
    app.composer.insert_text("draft survives hover exit");
    paint(&mut terminal, &app, &view);
    assert!(row(&terminal, 8).contains("Message"));
    assert!(row(&terminal, 9).contains("draft survives hover exit"));
}
