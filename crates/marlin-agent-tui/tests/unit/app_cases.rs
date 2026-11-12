// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use super::{
    app::{App, safe_text},
    cli::{CliAction, parse_cli},
    context_budget::ContextBudget,
    event_queue::{ReplyError, reply_channel},
    model_catalog::{ModelCatalog, ModelRoute},
    render::draw as render_draw,
    viewport::{Transcript, TranscriptViewportCache},
};
use marlin_agent_protocol::{
    ModelAlias, ModelContinuation, ModelContinuationKind, ModelEndpoint,
    ModelGatewayCompletionChoice, ModelGatewayCompletionResponse, ModelGatewayError,
    ModelGatewayFuture, ModelGatewayRequest, ModelGatewayResult, ModelGatewayTransport,
    assistant_gateway_message, user_gateway_message,
};
use marlin_agent_runtime::RuntimeEdgePolicy;
use marlin_agent_stream::{
    ModelGatewayEventStream, ModelStreamChunk, ModelStreamEvent, ModelStreamingGateway,
    ModelThinkingChunk,
};
use std::sync::{Arc, Mutex};

#[test]
fn selecting_commands_runs_defaults_and_retains_required_arguments() {
    let mut app = App::new(ModelEndpoint::new("provider", "model"));
    app.composer.insert_text("/s");
    assert!(app.accept_command_completion("/sessions "));
    assert_eq!(app.composer.text(), "/sessions ");

    assert!(app.accept_command_completion("/diff "));
    assert_eq!(app.composer.text(), "/diff ");

    assert!(!app.accept_command_completion("/model "));
    assert_eq!(app.composer.text(), "/model ");
    assert!(!app.accept_command_completion("/history "));
    assert_eq!(app.composer.text(), "/history ");
}

fn draw(
    frame: &mut ratatui::Frame<'_>,
    app: &App,
    cache: &mut TranscriptViewportCache,
    anchor: &mut super::viewport::ScrollAnchor,
) -> usize {
    use marlin_agent_ui::{UiComponentId, UiRegion, UiWorkbench};
    let view = UiWorkbench {
        model_label: "test".into(),
        session_label: "test".into(),
        regions: vec![
            UiRegion::Transcript,
            UiRegion::Composer,
            UiRegion::Commands(vec![]),
            UiRegion::Status(vec![]),
        ],
        focus: UiComponentId::new("composer").unwrap(),
    };
    render_draw(frame, app, &view, cache, anchor)
}

#[test]
fn command_menu_filters_scheme_descriptors_and_preserves_draft() {
    use marlin_agent_ui::{UiCommand, UiCommandName, UiComponentId, UiRegion, UiWorkbench};
    let mut app = App::new(ModelEndpoint::new("provider", "model"));
    app.composer.insert_text("preserved draft");
    app.toggle_command();
    app.composer.insert_text("mo");
    let command = |name, description: &str| UiCommand {
        name: UiCommandName::new(name).unwrap(),
        description: description.into(),
        accepts_arguments: false,
    };
    let view = UiWorkbench {
        model_label: "model".into(),
        session_label: "session".into(),
        regions: vec![
            UiRegion::Transcript,
            UiRegion::Composer,
            UiRegion::Commands(vec![
                command("/help", "Keyboard shortcuts"),
                command("/models", "List models"),
            ]),
            UiRegion::Status(vec![]),
        ],
        focus: UiComponentId::new("composer").unwrap(),
    };
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
    assert!(!text.contains("Keyboard shortcuts"), "{text}");
    app.toggle_command();
    assert_eq!(app.composer.text(), "preserved draft");
    app.composer.take();
    app.composer.insert_text("/mo");
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
    assert!(text.contains("Tab complete"), "{text}");
}

#[test]
fn command_menu_scrolls_to_the_selected_entry() {
    use marlin_agent_ui::{UiCommand, UiCommandName, UiComponentId, UiRegion, UiWorkbench};
    let mut app = App::new(ModelEndpoint::new("provider", "model"));
    app.toggle_command();
    let names = [
        "/help",
        "/models",
        "/model",
        "/sessions",
        "/session",
        "/clear",
    ];
    let commands = names
        .into_iter()
        .map(|name| UiCommand {
            name: UiCommandName::new(name).unwrap(),
            description: name.into(),
            accepts_arguments: false,
        })
        .collect();
    let view = UiWorkbench {
        model_label: "model".into(),
        session_label: "session".into(),
        regions: vec![
            UiRegion::Transcript,
            UiRegion::Composer,
            UiRegion::Commands(commands),
            UiRegion::Status(vec![]),
        ],
        focus: UiComponentId::new("composer").unwrap(),
    };
    app.command_selection = Some(("/".into(), UiCommandName::new("/clear").unwrap()));
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(60, 12)).unwrap();
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
        .flat_map(|y| (0..60).map(move |x| buffer[(x, y)].symbol()))
        .collect();
    assert!(text.contains("› /clear"), "{text}");
    assert!(!text.contains("/help  /help"), "{text}");
    app.composer.insert_text("h");
    assert!(app.selected_command().is_none());
}

#[test]
fn render_uses_the_admitted_scheme_focus() {
    use marlin_agent_ui::{UiComponentId, UiRegion, UiWorkbench};
    let app = App::new(ModelEndpoint::new("provider", "model"));
    let view = UiWorkbench {
        model_label: "model".into(),
        session_label: "session".into(),
        regions: vec![
            UiRegion::Transcript,
            UiRegion::Composer,
            UiRegion::Commands(vec![]),
            UiRegion::Status(vec![]),
        ],
        focus: UiComponentId::new("transcript").unwrap(),
    };
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 10)).unwrap();
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
    let status: String = (0..80)
        .map(|x| terminal.backend().buffer()[(x, 9)].symbol())
        .collect();
    assert!(status.contains("Transcript: Up/Down scroll"), "{status}");
}

#[test]
fn render_keeps_focused_extension_visible_and_sanitizes_its_label() {
    use marlin_agent_ui::{UiComponentId, UiRegion, UiWorkbench};
    let app = App::new(ModelEndpoint::new("provider", "model"));
    let mut regions = vec![
        UiRegion::Transcript,
        UiRegion::Composer,
        UiRegion::Commands(vec![]),
    ];
    for (id, label) in [
        ("first", "first"),
        ("second", "second"),
        ("third", "third\u{1b}[31m"),
    ] {
        regions.push(UiRegion::Extension {
            id: UiComponentId::new(id).unwrap(),
            label: label.into(),
            focusable: true,
            content: vec![],
        });
    }
    regions.push(UiRegion::Status(vec![]));
    let view = UiWorkbench {
        model_label: "model".into(),
        session_label: "session".into(),
        regions,
        focus: UiComponentId::new("third").unwrap(),
    };
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(40, 8)).unwrap();
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
    let screen: String = (0..8)
        .flat_map(|y| (0..40).map(move |x| buffer[(x, y)].symbol()))
        .collect();
    assert!(screen.contains("third�[31m"), "{screen}");
    assert!(!screen.contains('\u{1b}'));
    assert!(screen.contains("Extension: Enter activate"), "{screen}");
}

#[test]
fn render_focused_scheme_content_on_narrow_and_wide_terminals() {
    use marlin_agent_ui::{UiComponentId, UiContent, UiRegion, UiWorkbench};
    let app = App::new(ModelEndpoint::new("provider", "model"));
    let view = UiWorkbench {
        model_label: "model".into(),
        session_label: "session".into(),
        regions: vec![
            UiRegion::Transcript,
            UiRegion::Composer,
            UiRegion::Commands(vec![]),
            UiRegion::Extension {
                id: UiComponentId::new("details").unwrap(),
                label: "Details".into(),
                focusable: true,
                content: vec![
                    UiContent::Text("Scheme component body".into()),
                    UiContent::Field {
                        label: "Source".into(),
                        value: "safe\u{1b}[2J".into(),
                    },
                ],
            },
            UiRegion::Status(vec![]),
        ],
        focus: UiComponentId::new("details").unwrap(),
    };
    for width in [80, 120] {
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(width, 20)).unwrap();
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
        let screen: String = (0..20)
            .flat_map(|y| (0..width).map(move |x| buffer[(x, y)].symbol()))
            .collect();
        assert!(screen.contains("Scheme component body"), "{screen}");
        assert!(screen.contains("Source: safe�[2J"), "{screen}");
        assert!(!screen.contains('\u{1b}'), "{screen}");
        assert_eq!(screen.contains("Model gateway configured"), width == 120);
    }
}

#[test]
fn render_status_contributions_respects_focus_and_terminal_height() {
    use marlin_agent_ui::{UiComponentId, UiRegion, UiStatusItem, UiWorkbench};
    let app = App::new(ModelEndpoint::new("provider", "model"));
    let view = UiWorkbench {
        model_label: "model".into(),
        session_label: "session".into(),
        regions: vec![
            UiRegion::Transcript,
            UiRegion::Composer,
            UiRegion::Commands(vec![]),
            UiRegion::Extension {
                id: UiComponentId::new("extension").unwrap(),
                label: "Focused panel".into(),
                focusable: true,
                content: vec![],
            },
            UiRegion::Status(vec![
                UiStatusItem {
                    id: UiComponentId::new("ready").unwrap(),
                    label: "Ready".into(),
                },
                UiStatusItem {
                    id: UiComponentId::new("unsafe").unwrap(),
                    label: "bad\u{1b}[2J".into(),
                },
            ]),
        ],
        focus: UiComponentId::new("extension").unwrap(),
    };
    for height in [8, 9] {
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(60, height)).unwrap();
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
        let screen: String = (0..height)
            .flat_map(|y| (0..60).map(move |x| buffer[(x, y)].symbol()))
            .collect();
        assert!(screen.contains("Focused panel"), "{screen}");
        assert!(screen.contains("Extension: Enter activate"), "{screen}");
        assert_eq!(screen.contains("Ready · bad�[2J"), height == 9, "{screen}");
        assert!(!screen.contains('\u{1b}'));
    }
}

#[test]
fn history_search_previews_then_accepts_without_submitting() {
    let mut app = App::new(ModelEndpoint::new("openai", "test"));
    app.history = Arc::new(vec![
        user_gateway_message("find this"),
        assistant_gateway_message("reply"),
    ]);
    app.composer.insert_text("my draft");
    app.start_history_search();
    app.history_search
        .as_mut()
        .unwrap()
        .insert("find", &app.history);
    assert_eq!(app.composer.text(), "my draft");
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(40, 10)).unwrap();
    terminal
        .draw(|frame| {
            draw(
                frame,
                &app,
                &mut Default::default(),
                &mut Default::default(),
            );
        })
        .unwrap();
    let buffer = terminal.backend().buffer();
    let status: String = (0..40).map(|x| buffer[(x, 9)].symbol()).collect();
    assert!(status.contains("Search: find"), "{status}");
    app.accept_history_search();
    assert_eq!(app.composer.text(), "find this");
    assert!(app.pending.is_none());
    assert_eq!(app.history.len(), 2);
}

#[test]
fn unfinished_history_search_cannot_accept_an_old_preview() {
    let mut app = App::new(ModelEndpoint::new("openai", "test"));
    app.history = Arc::new(vec![user_gateway_message("x".repeat(100_000))]);
    app.composer.insert_text("preserve draft");
    app.start_history_search();
    app.history_search
        .as_mut()
        .unwrap()
        .insert("missing", &app.history);
    assert!(app.history_search.as_ref().unwrap().is_searching());
    app.accept_history_search();
    assert!(app.history_search.is_some());
    assert_eq!(app.composer.text(), "preserve draft");
}

#[test]
fn heading_shows_the_selected_model_and_sanitizes_control_characters() {
    let app = App::new(ModelEndpoint::new("deepseek", "chat\u{1b}[2J"));
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(100, 12)).unwrap();
    terminal
        .draw(|frame| {
            draw(
                frame,
                &app,
                &mut Default::default(),
                &mut Default::default(),
            );
        })
        .unwrap();
    let heading: String = (0..100)
        .map(|x| terminal.backend().buffer()[(x, 0)].symbol())
        .collect();
    assert!(heading.contains("deepseek/chat�[2J"), "{heading}");
    assert!(heading.contains("Idle · deepseek"), "{heading}");
    assert!(heading.contains("temporary"), "{heading}");
    assert!(!heading.contains('\u{1b}'));
}

#[test]
fn composer_expands_with_the_draft_and_leaves_a_transcript_row() {
    let mut app = App::new(ModelEndpoint::new("openai", "gpt-test"));
    app.transcript = Transcript::new(vec![("You".into(), "visible transcript".into())]);
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(40, 12)).unwrap();
    let mut cache = TranscriptViewportCache::default();
    let mut anchor = Default::default();
    terminal
        .draw(|frame| {
            draw(frame, &app, &mut cache, &mut anchor);
        })
        .unwrap();
    let row = |y| {
        (0..40)
            .map(|x| terminal.backend().buffer()[(x, y)].symbol())
            .collect::<String>()
    };
    assert!(row(8).contains("Message"));

    app.composer
        .insert_text("one\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten");
    terminal
        .draw(|frame| {
            draw(frame, &app, &mut cache, &mut anchor);
        })
        .unwrap();
    let buffer = terminal.backend().buffer();
    let header: String = (0..40).map(|x| buffer[(x, 0)].symbol()).collect();
    let transcript: String = (0..40).map(|x| buffer[(x, 1)].symbol()).collect();
    let composer: String = (0..40).map(|x| buffer[(x, 3)].symbol()).collect();
    assert!(header.contains("Marlin"));
    assert!(transcript.contains("visible transcript"));
    assert!(composer.contains("Message"));
    assert!(terminal.backend().cursor_position().y < 11);
}

#[test]
fn rejected_draft_insertion_is_visible_in_the_terminal_status() {
    let mut app = App::new(ModelEndpoint::new("openai", "gpt-test"));
    app.composer.insert_text(&"a".repeat(1024 * 1024));
    app.composer.insert('b');
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(100, 12)).unwrap();
    let mut cache = TranscriptViewportCache::default();
    terminal
        .draw(|frame| {
            draw(frame, &app, &mut cache, &mut Default::default());
        })
        .unwrap();
    let buffer = terminal.backend().buffer();
    let status: String = (0..100).map(|x| buffer[(x, 11)].symbol()).collect();
    assert!(
        status.contains("Insertion rejected; draft retained."),
        "{status}"
    );
    assert_eq!(app.composer.text().len(), 1024 * 1024);
}

#[test]
fn workspace_view_closes_without_losing_the_message_draft_or_history() {
    use super::workspace::{WorkspaceObservation, WorkspaceView};
    let mut app = App::new(ModelEndpoint::new("openai", "gpt-test"));
    app.composer.insert_text("keep this draft");
    app.scroll_up = 7;
    let original_transcript_len = app.transcript.len();
    app.workspace.show(WorkspaceView::from_observation(
        WorkspaceObservation::Status(marlin_git_utils::GitStatus {
            entries: Vec::new(),
        }),
    ));
    app.scroll(false);
    assert_eq!(app.workspace.view().unwrap().scroll_from_top, 10);
    app.scroll(true);
    assert_eq!(app.workspace.view().unwrap().scroll_from_top, 0);
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(100, 12)).unwrap();
    let mut cache = TranscriptViewportCache::default();
    let mut anchor = Default::default();
    terminal
        .draw(|frame| {
            draw(frame, &app, &mut cache, &mut anchor);
        })
        .unwrap();
    let buffer = terminal.backend().buffer();
    let heading: String = (0..100).map(|x| buffer[(x, 0)].symbol()).collect();
    assert!(heading.contains("workspace"));
    let body: String = (0..100).map(|x| buffer[(x, 2)].symbol()).collect();
    assert!(body.contains("Worktree clean"));
    assert!(!app.interrupt());
    assert!(app.workspace.view().is_none());
    assert_eq!(app.composer.text(), "keep this draft");
    assert_eq!(app.scroll_up, 7);
    assert_eq!(app.transcript.len(), original_transcript_len);
    assert!(app.history.is_empty());
    terminal
        .draw(|frame| {
            draw(frame, &app, &mut cache, &mut anchor);
        })
        .unwrap();
    let buffer = terminal.backend().buffer();
    let heading: String = (0..100).map(|x| buffer[(x, 0)].symbol()).collect();
    assert!(heading.contains("model gateway"));
}

#[test]
fn repeated_resize_during_stream_matches_a_fresh_terminal_frame() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let gateway = FixtureGateway {
        requests: Arc::new(Mutex::new(Vec::new())),
    };
    let (tx, _rx) = reply_channel(8, 4096);
    let mut app = App::new(ModelEndpoint::new("openai", "gpt-test"));
    app.composer.insert_text("start");
    app.submit(&gateway, &runtime, &tx);
    assert_eq!(app.activity_label(), "Working");
    app.composer.insert_text("draft 中🐳");
    app.transcript.push(("Assistant".into(), "start".into()));
    let index = app.transcript.len() - 1;
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 20)).unwrap();
    let mut cache = TranscriptViewportCache::default();
    let mut anchor = Default::default();
    for (width, height, chunk) in [
        (80, 20, "alpha"),
        (19, 10, "中"),
        (5, 8, "e\u{301}"),
        (100, 24, "omega"),
        (24, 11, "🐳"),
        (80, 20, "end"),
    ] {
        app.transcript.append_content(index, chunk);
        terminal.backend_mut().resize(width, height);
        terminal
            .draw(|frame| {
                draw(frame, &app, &mut cache, &mut anchor);
            })
            .unwrap();
        let mut fresh =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(width, height)).unwrap();
        fresh
            .draw(|frame| {
                draw(
                    frame,
                    &app,
                    &mut Default::default(),
                    &mut Default::default(),
                );
            })
            .unwrap();
        assert_eq!(
            terminal.backend().buffer(),
            fresh.backend().buffer(),
            "frame mismatch after resize to {width}x{height}"
        );
        assert_eq!(
            terminal.backend().cursor_position(),
            fresh.backend().cursor_position()
        );
    }
    app.cancel();
    assert_eq!(app.activity_label(), "Idle");
}

#[test]
fn pending_workspace_read_retains_input_until_cancelled() {
    use super::workspace::WorkspaceQuery;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let gateway = FixtureGateway {
        requests: Arc::new(Mutex::new(Vec::new())),
    };
    let (tx, _rx) = reply_channel(8, 4096);
    let mut app = App::new(ModelEndpoint::new("openai", "gpt-test"));
    assert!(app.workspace.start(
        &runtime,
        std::env::current_dir().unwrap(),
        WorkspaceQuery::Status
    ));
    app.composer.insert_text("retain input");
    app.submit(&gateway, &runtime, &tx);
    assert_eq!(app.composer.text(), "retain input");
    assert!(app.pending.is_none());
    assert!(!app.interrupt());
    assert!(!app.workspace.is_pending());
    assert_eq!(app.composer.text(), "retain input");
    assert!(gateway.requests.lock().unwrap().is_empty());
}

#[test]
fn cli_parses_explicit_typed_session_and_rejects_bad_arguments() {
    let args = parse_cli(
        ["config.toml", "--session", "session:two"]
            .into_iter()
            .map(Into::into),
    )
    .unwrap();
    assert_eq!(args.config.to_str(), Some("config.toml"));
    assert!(
        matches!(args.action, CliAction::Open { session: Some(id) } if id.as_str() == "session:two")
    );
    assert!(matches!(
        parse_cli(["config.toml"].into_iter().map(Into::into))
            .unwrap()
            .action,
        CliAction::Open { session: None }
    ));
    assert!(
        matches!(parse_cli(["config.toml", "--list-sessions", "--after", "session:a"].into_iter().map(Into::into)).unwrap().action, CliAction::List { after: Some(id) } if id.as_str() == "session:a")
    );
    for invalid in [
        vec![],
        vec!["config.toml", "--session"],
        vec!["config.toml", "--session", "  "],
        vec!["config.toml", "--other", "id"],
        vec!["config.toml", "--session", "id", "extra"],
        vec!["config.toml", "--list-sessions", "--after"],
    ] {
        assert!(parse_cli(invalid.into_iter().map(Into::into)).is_err());
    }
}

#[cfg(unix)]
#[test]
fn cli_rejects_a_non_utf8_session_identity() {
    use std::{ffi::OsString, os::unix::ffi::OsStringExt};
    let args = [
        OsString::from("config.toml"),
        OsString::from("--session"),
        OsString::from_vec(vec![0xff]),
    ];
    assert!(parse_cli(args.into_iter()).is_err());
}

#[derive(Clone)]
struct FixtureGateway {
    requests: Arc<Mutex<Vec<ModelGatewayRequest>>>,
}

#[test]
fn model_selection_retains_draft_and_failed_selection_keeps_previous_route() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let requests = Arc::new(Mutex::new(Vec::new()));
    let gateway = Arc::new(FixtureGateway {
        requests: requests.clone(),
    });
    let default = ModelEndpoint::new("openai", "gpt-first");
    let alternate = ModelEndpoint::new("openai", "gpt-second").with_alias("alt");
    let catalog = ModelCatalog::new(vec![
        ModelRoute {
            alias: ModelAlias::new("default"),
            endpoint: default.clone(),
            gateway: Some(gateway.clone()),
        },
        ModelRoute {
            alias: ModelAlias::new("alt"),
            endpoint: alternate.clone(),
            gateway: Some(gateway),
        },
        ModelRoute {
            alias: ModelAlias::new("disabled"),
            endpoint: ModelEndpoint::new("openai", "gpt-third").with_alias("disabled"),
            gateway: None,
        },
    ])
    .unwrap();
    let mut app = App::new(default);
    app.set_models(catalog.clone());
    let (tx, mut rx) = reply_channel(8, 4096);
    app.composer.insert_text("draft");
    app.toggle_command();
    app.composer.insert_text("models");
    app.submit(&catalog, &runtime, &tx);
    assert_eq!(app.composer.text(), "draft");
    assert!(
        app.transcript
            .last()
            .unwrap()
            .1
            .contains("credential unavailable")
    );
    app.toggle_command();
    app.composer.insert_text("model alt");
    app.submit(&catalog, &runtime, &tx);
    assert_eq!(app.composer.text(), "draft");
    app.toggle_command();
    app.composer.insert_text("model disabled");
    app.submit(&catalog, &runtime, &tx);
    assert!(
        app.transcript
            .last()
            .unwrap()
            .1
            .contains("current model retained")
    );
    app.submit(&catalog, &runtime, &tx);
    while app.pending.is_some() {
        let (id, event) = rx.blocking_recv().unwrap().into_parts();
        app.receive(id, event);
    }
    let sent = requests.lock().unwrap();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].endpoint(), &alternate);
    assert_eq!(sent[0].messages()[0].content, "draft");
}

#[derive(Clone)]
struct OversizedGateway;

#[test]
fn predispatch_rejection_preserves_draft_and_allows_a_later_turn() {
    use marlin_agent_sessions::{ConversationJournal, ConversationTurnOutcome};
    use marlin_agent_storage::{AgentId, InMemoryAgentStorage, ProjectId, SessionId};
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Clone)]
    struct RejectOnce(Arc<AtomicUsize>);
    impl ModelStreamingGateway for RejectOnce {
        fn stream(
            &self,
            _: ModelGatewayRequest,
        ) -> ModelGatewayFuture<ModelGatewayResult<ModelGatewayEventStream>> {
            let attempt = self.0.fetch_add(1, Ordering::SeqCst);
            Box::pin(async move {
                if attempt == 0 {
                    return Err(ModelGatewayError::RequestNotDispatched(
                        "fixture rejection".into(),
                    ));
                }
                Ok(
                    Box::pin(tokio_stream::iter(vec![Ok(ModelStreamEvent::Completed {
                        response: ModelGatewayCompletionResponse::new(
                            "id",
                            "model",
                            vec![ModelGatewayCompletionChoice::new(
                                0,
                                assistant_gateway_message("answer"),
                                None,
                            )],
                        ),
                    })])) as ModelGatewayEventStream,
                )
            })
        }
    }

    for persistent in [false, true] {
        for draft_during_request in [false, true] {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            let storage = InMemoryAgentStorage::new();
            let project = ProjectId::new("project:reject").unwrap();
            let session = SessionId::new("session:reject").unwrap();
            let agent = AgentId::new("agent:reject").unwrap();
            let source = RejectOnce(Arc::new(AtomicUsize::new(0)));
            let gateway = if persistent {
                let journal = runtime
                    .block_on(ConversationJournal::open(
                        storage.clone(),
                        project.clone(),
                        session.clone(),
                        agent.clone(),
                        |_| {},
                    ))
                    .unwrap();
                super::edge_gateway::SharedGateway::new(
                    super::journal_gateway::JournalGateway::new(source, journal),
                )
            } else {
                super::edge_gateway::SharedGateway::new(source)
            };
            let (tx, mut rx) = reply_channel(8, 4096);
            let mut app = App::new(ModelEndpoint::new("openai", "gpt-test"));
            if persistent {
                app.restore(Vec::new());
            }
            app.composer.insert_text("first prompt");
            app.submit(&gateway, &runtime, &tx);
            if draft_during_request {
                app.composer.insert_text("new draft");
            }
            let (id, reply) = rx.blocking_recv().unwrap().into_parts();
            app.receive(id, reply);
            assert_eq!(
                app.composer.text(),
                if draft_during_request {
                    "new draft"
                } else {
                    "first prompt"
                }
            );
            assert!(
                app.transcript
                    .iter()
                    .any(|(role, text)| role == "You" && text == "first prompt")
            );
            app.submit(&gateway, &runtime, &tx);
            assert!(
                app.pending.is_some(),
                "pre-dispatch failure must not require replay"
            );
            while app.pending.is_some() {
                let (id, reply) = rx.blocking_recv().unwrap().into_parts();
                app.receive(id, reply);
            }
            assert_eq!(app.history.last().unwrap().content, "answer");
            if persistent {
                let mut turns = Vec::new();
                runtime
                    .block_on(ConversationJournal::open(
                        storage,
                        project,
                        session,
                        agent,
                        |turn| turns.push(turn.clone()),
                    ))
                    .unwrap();
                assert!(matches!(
                    turns[0].outcome,
                    Some(ConversationTurnOutcome::RejectedBeforeDispatch { .. })
                ));
                assert!(matches!(
                    turns[1].outcome,
                    Some(ConversationTurnOutcome::Completed { .. })
                ));
            }
        }
    }
}

#[test]
fn predispatch_claim_after_stream_start_does_not_release_uncertain_session() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let gateway = FixtureGateway {
        requests: Arc::new(Mutex::new(Vec::new())),
    };
    let (tx, _rx) = reply_channel(8, 4096);
    let mut app = App::new(ModelEndpoint::new("openai", "gpt-test"));
    app.restore(Vec::new());
    app.composer.insert_text("first");
    app.submit(&gateway, &runtime, &tx);
    app.receive(
        1,
        Ok(ModelStreamEvent::Started {
            transport: ModelGatewayTransport::Sse,
        }),
    );
    app.receive(
        1,
        Err(ReplyError::Gateway(
            ModelGatewayError::RequestNotDispatched("invalid claim".into()),
        )),
    );
    app.composer.insert_text("new draft");
    app.submit(&gateway, &runtime, &tx);
    assert!(app.pending.is_none());
    assert_eq!(app.composer.text(), "new draft");
    assert!(app.transcript.last().unwrap().1.contains("uncertain"));
}

#[test]
fn predispatch_claim_from_opened_stream_is_treated_as_uncertain() {
    #[derive(Clone)]
    struct MislabelledStream;
    impl ModelStreamingGateway for MislabelledStream {
        fn stream(
            &self,
            _: ModelGatewayRequest,
        ) -> ModelGatewayFuture<ModelGatewayResult<ModelGatewayEventStream>> {
            Box::pin(async {
                Ok(Box::pin(tokio_stream::iter(vec![Err(
                    ModelGatewayError::RequestNotDispatched("invalid claim".into()),
                )])) as ModelGatewayEventStream)
            })
        }
    }
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let (tx, mut rx) = reply_channel(8, 4096);
    let mut app = App::new(ModelEndpoint::new("openai", "gpt-test"));
    app.restore(Vec::new());
    app.composer.insert_text("first");
    app.submit(&MislabelledStream, &runtime, &tx);
    let (id, reply) = rx.blocking_recv().unwrap().into_parts();
    app.receive(id, reply);
    app.composer.insert_text("new draft");
    app.submit(&MislabelledStream, &runtime, &tx);
    assert!(app.pending.is_none());
    assert_eq!(app.composer.text(), "new draft");
    assert!(app.transcript.last().unwrap().1.contains("uncertain"));
}

#[test]
fn ctrl_c_prioritizes_cancellation_then_requires_consecutive_exit_confirmation() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let gateway = FixtureGateway {
        requests: Arc::new(Mutex::new(Vec::new())),
    };
    let (tx, _) = reply_channel(8, 4096);
    let mut app = App::new(ModelEndpoint::new("openai", "gpt-test"));
    app.composer.insert_text("draft");
    app.toggle_command();
    app.composer.insert_text("help");
    assert!(!app.interrupt());
    assert_eq!(app.composer.text(), "draft");
    assert!(!app.interrupt());
    assert_eq!(app.composer.text(), "draft");
    app.disarm_exit();
    assert!(!app.interrupt());
    assert!(app.interrupt());

    app.disarm_exit();
    app.restore(Vec::new());
    app.submit(&gateway, &runtime, &tx);
    assert!(app.pending.is_some());
    assert!(!app.interrupt());
    assert!(app.pending.is_none());
    assert!(
        app.transcript
            .last()
            .unwrap()
            .1
            .contains("cancelled locally")
    );
    app.composer.insert_text("next");
    app.submit(&gateway, &runtime, &tx);
    assert!(app.pending.is_none());
    assert_eq!(app.composer.text(), "next");
}

impl ModelStreamingGateway for OversizedGateway {
    fn stream(
        &self,
        _: ModelGatewayRequest,
    ) -> ModelGatewayFuture<ModelGatewayResult<ModelGatewayEventStream>> {
        Box::pin(async {
            let events = (1..=257).map(|sequence| {
                Ok(ModelStreamEvent::Chunk(ModelStreamChunk::new(
                    sequence,
                    "x".repeat(64 * 1024),
                )))
            });
            Ok(Box::pin(tokio_stream::iter(events)) as ModelGatewayEventStream)
        })
    }
}

#[test]
fn nonpersistent_turn_stops_at_output_budget_without_committing_history() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let (tx, mut rx) = reply_channel(8, 1024 * 1024);
    let mut app = App::new(ModelEndpoint::new("openai", "gpt-test"));
    app.composer.insert_text("prompt");
    app.submit(&OversizedGateway, &runtime, &tx);
    while app.pending.is_some() {
        let item = runtime
            .block_on(async {
                tokio::time::timeout(std::time::Duration::from_secs(10), rx.recv()).await
            })
            .expect("stream event timed out")
            .expect("stream closed");
        let (id, event) = item.into_parts();
        app.receive(id, event);
    }
    assert!(app.history.is_empty());
    assert_eq!(app.transcript[2].1.len(), 16 * 1024 * 1024);
    assert!(
        app.transcript
            .last()
            .unwrap()
            .1
            .contains("model output exceeded")
    );
}

#[test]
fn runtime_command_projects_typed_open_policy_receipt() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let requests = Arc::new(Mutex::new(Vec::new()));
    let gateway = FixtureGateway {
        requests: requests.clone(),
    };
    let (tx, _) = reply_channel(8, 4096);
    let mut app = App::new(ModelEndpoint::new("openai", "gpt-test"));
    app.set_runtime_edge(
        RuntimeEdgePolicy::new()
            .with_timeout_ms(1234)
            .receipt()
            .unwrap(),
    );
    app.composer.insert_text("/runtime");
    app.submit(&gateway, &runtime, &tx);
    let (role, detail) = app.transcript.last().unwrap();
    assert_eq!(role, "Runtime");
    assert!(
        detail.contains("1234")
            && detail.contains("timeout")
            && detail.contains("does not cover stream duration")
    );
    assert!(requests.lock().unwrap().is_empty());
}

impl ModelStreamingGateway for FixtureGateway {
    fn stream(
        &self,
        request: ModelGatewayRequest,
    ) -> ModelGatewayFuture<ModelGatewayResult<ModelGatewayEventStream>> {
        self.requests.lock().unwrap().push(request);
        Box::pin(async {
            let response = ModelGatewayCompletionResponse::new(
                "fixture",
                "gpt-test",
                vec![ModelGatewayCompletionChoice::new(
                    0,
                    assistant_gateway_message("hello"),
                    Some("stop".into()),
                )],
            );
            Ok(Box::pin(tokio_stream::iter(vec![
                Ok(ModelStreamEvent::Started {
                    transport: ModelGatewayTransport::Sse,
                }),
                Ok(ModelStreamEvent::Chunk(ModelStreamChunk::new(1, "hel"))),
                Ok(ModelStreamEvent::Chunk(ModelStreamChunk::new(2, "lo"))),
                Ok(ModelStreamEvent::Completed { response }),
            ])) as ModelGatewayEventStream)
        })
    }
}

#[derive(Clone)]
struct ThreadCheckingGateway {
    inner: FixtureGateway,
    thread: Arc<Mutex<Option<std::thread::ThreadId>>>,
}

impl ModelStreamingGateway for ThreadCheckingGateway {
    fn stream(
        &self,
        request: ModelGatewayRequest,
    ) -> ModelGatewayFuture<ModelGatewayResult<ModelGatewayEventStream>> {
        *self.thread.lock().unwrap() = Some(std::thread::current().id());
        self.inner.stream(request)
    }
}

#[test]
fn request_preparation_runs_off_terminal_thread_and_keeps_full_context() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let requests = Arc::new(Mutex::new(Vec::new()));
    let thread = Arc::new(Mutex::new(None));
    let gateway = ThreadCheckingGateway {
        inner: FixtureGateway {
            requests: requests.clone(),
        },
        thread: thread.clone(),
    };
    let (tx, mut rx) = reply_channel(8, 4096);
    let mut app = App::new(ModelEndpoint::new("openai", "gpt-test"));
    app.history = Arc::new(vec![
        user_gateway_message("earlier"),
        assistant_gateway_message("reply"),
    ]);
    app.composer.insert_text("next");
    let terminal_thread = std::thread::current().id();
    app.submit(&gateway, &runtime, &tx);
    while app.pending.is_some() {
        let (id, event) = rx.blocking_recv().unwrap().into_parts();
        app.receive(id, event);
    }
    assert_ne!(thread.lock().unwrap().unwrap(), terminal_thread);
    let requests = requests.lock().unwrap();
    assert_eq!(
        requests[0]
            .messages()
            .iter()
            .map(|message| message.content.as_str())
            .collect::<Vec<_>>(),
        ["earlier", "reply", "next"]
    );
}

#[test]
fn cancelling_before_background_task_runs_does_not_open_gateway() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let requests = Arc::new(Mutex::new(Vec::new()));
    let gateway = FixtureGateway {
        requests: requests.clone(),
    };
    let (tx, _rx) = reply_channel(8, 4096);
    let mut app = App::new(ModelEndpoint::new("openai", "gpt-test"));
    app.composer.insert_text("prompt");
    app.submit(&gateway, &runtime, &tx);
    app.cancel();
    runtime.block_on(async { tokio::task::yield_now().await });
    assert!(requests.lock().unwrap().is_empty());
}

#[test]
fn gateway_turns_show_chunks_and_carry_only_completed_history() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let requests = Arc::new(Mutex::new(Vec::new()));
    let gateway = FixtureGateway {
        requests: requests.clone(),
    };
    let (tx, mut rx) = reply_channel(8, 4096);
    let mut app = App::new(ModelEndpoint::new("openai", "gpt-test"));
    for c in "first".chars() {
        app.composer.insert(c);
    }
    app.submit(&gateway, &runtime, &tx);
    let (id, started) = rx.blocking_recv().unwrap().into_parts();
    app.receive(id, started);
    let (_, chunk) = rx.blocking_recv().unwrap().into_parts();
    app.receive(id, chunk);
    assert_eq!(app.transcript.last().unwrap().1, "hel");
    assert!(app.history.is_empty());
    let (_, chunk) = rx.blocking_recv().unwrap().into_parts();
    app.receive(id, chunk);
    let (_, completed) = rx.blocking_recv().unwrap().into_parts();
    app.receive(id, completed);
    assert_eq!(app.history.len(), 2);
    for c in "second".chars() {
        app.composer.insert(c);
    }
    app.submit(&gateway, &runtime, &tx);
    app.receive(
        id,
        Err(ReplyError::Gateway(ModelGatewayError::Completion(
            "stale".into(),
        ))),
    );
    assert!(app.pending.is_some());
    let (next_id, started) = rx.blocking_recv().unwrap().into_parts();
    app.receive(next_id, started);
    let requests = requests.lock().unwrap();
    assert_eq!(requests[1].messages().len(), 3);
    assert_eq!(requests[1].messages()[0].content, "first");
    assert_eq!(requests[1].messages()[1].content, "hello");
    assert_eq!(requests[1].messages()[2].content, "second");
}

#[test]
fn alternating_thinking_and_answer_segments_are_bounded() {
    #[derive(Clone)]
    struct AlternatingGateway;
    impl ModelStreamingGateway for AlternatingGateway {
        fn stream(
            &self,
            _: ModelGatewayRequest,
        ) -> ModelGatewayFuture<ModelGatewayResult<ModelGatewayEventStream>> {
            Box::pin(async {
                let events = (0..=4096u64).map(|index| {
                    if index % 2 == 0 {
                        Ok(ModelStreamEvent::Chunk(ModelStreamChunk::new(
                            index / 2 + 1,
                            "a",
                        )))
                    } else {
                        Ok(ModelStreamEvent::Thinking(ModelThinkingChunk::new(
                            index / 2 + 1,
                            "t",
                        )))
                    }
                });
                Ok(Box::pin(tokio_stream::iter(events)) as ModelGatewayEventStream)
            })
        }
    }
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let (tx, mut rx) = reply_channel(8, 4096);
    let mut app = App::new(ModelEndpoint::new("openai", "gpt-test"));
    app.composer.insert_text("prompt");
    app.submit(&AlternatingGateway, &runtime, &tx);
    while app.pending.is_some() {
        let (id, event) = rx.blocking_recv().unwrap().into_parts();
        app.receive(id, event);
    }
    assert!(
        app.transcript
            .last()
            .unwrap()
            .1
            .contains("too many alternating segments")
    );
    assert!(app.history.is_empty());
    app.composer.insert_text("next");
    app.submit(&AlternatingGateway, &runtime, &tx);
    assert!(app.pending.is_none());
    assert_eq!(app.composer.text(), "next");
}

#[test]
fn interleaved_thinking_stays_ordered_and_blocks_unverified_followup() {
    #[derive(Clone)]
    struct ThinkingGateway;
    impl ModelStreamingGateway for ThinkingGateway {
        fn stream(
            &self,
            _: ModelGatewayRequest,
        ) -> ModelGatewayFuture<ModelGatewayResult<ModelGatewayEventStream>> {
            Box::pin(async {
                let response = ModelGatewayCompletionResponse::new(
                    "id",
                    "model",
                    vec![ModelGatewayCompletionChoice::new(
                        0,
                        assistant_gateway_message("ab"),
                        None,
                    )],
                );
                Ok(Box::pin(tokio_stream::iter(vec![
                    Ok(ModelStreamEvent::Chunk(ModelStreamChunk::new(1, "a"))),
                    Ok(ModelStreamEvent::Thinking(ModelThinkingChunk::new(
                        1,
                        "secret-thought",
                    ))),
                    Ok(ModelStreamEvent::Chunk(ModelStreamChunk::new(2, "b"))),
                    Ok(ModelStreamEvent::Completed { response }),
                ])) as ModelGatewayEventStream)
            })
        }
    }
    let runtime = tokio::runtime::Runtime::new().unwrap();
    for persistent in [false, true] {
        use marlin_agent_storage::{AgentId, InMemoryAgentStorage, ProjectId, SessionId};
        let storage = InMemoryAgentStorage::new();
        let project = ProjectId::new("project:thinking").unwrap();
        let session = SessionId::new("session:thinking").unwrap();
        let agent = AgentId::new("agent:thinking").unwrap();
        let gateway = if persistent {
            let journal = runtime
                .block_on(marlin_agent_sessions::ConversationJournal::open(
                    storage.clone(),
                    project.clone(),
                    session.clone(),
                    agent.clone(),
                    |_| {},
                ))
                .unwrap();
            super::edge_gateway::SharedGateway::new(super::journal_gateway::JournalGateway::new(
                ThinkingGateway,
                journal,
            ))
        } else {
            super::edge_gateway::SharedGateway::new(ThinkingGateway)
        };
        let (tx, mut rx) = reply_channel(8, 4096);
        let mut app = App::new(ModelEndpoint::new("openai", "gpt-test"));
        if persistent {
            app.restore(Vec::new());
        }
        app.composer.insert_text("first");
        app.submit(&gateway, &runtime, &tx);
        while app.pending.is_some() {
            let (id, event) = rx.blocking_recv().unwrap().into_parts();
            app.receive(id, event);
        }
        let relevant: Vec<_> = app
            .transcript
            .iter()
            .filter(|(role, _)| role == "Assistant" || role == "Thinking")
            .map(|(role, text)| (role.as_str(), text.as_str()))
            .collect();
        assert_eq!(
            relevant,
            [
                ("Assistant", "a"),
                ("Thinking", "secret-thought"),
                ("Assistant", "b")
            ]
        );
        assert_eq!(app.history[1].content, "ab");
        app.composer.insert_text("next");
        app.submit(&gateway, &runtime, &tx);
        assert!(app.pending.is_none());
        assert_eq!(app.composer.text(), "next");
        if persistent {
            let mut turns = Vec::new();
            runtime
                .block_on(marlin_agent_sessions::ConversationJournal::open(
                    storage.clone(),
                    project.clone(),
                    session.clone(),
                    agent.clone(),
                    |turn| turns.push(turn.clone()),
                ))
                .unwrap();
            assert_eq!(turns[0].partial_thinking, "secret-thought");
            assert!(turns[0].outcome.is_some());
            let (_, replay) = runtime
                .block_on(super::replay::load_session(
                    storage, project, session, agent,
                ))
                .unwrap();
            let mut restored = App::new(ModelEndpoint::new("openai", "gpt-test"));
            restored.restore(replay);
            let rendered: Vec<_> = restored
                .transcript
                .iter()
                .filter(|(role, _)| role == "Assistant" || role == "Thinking (recorded)")
                .map(|(role, text)| (role.as_str(), text.as_str()))
                .collect();
            assert_eq!(
                rendered,
                [
                    ("Assistant", "a"),
                    ("Thinking (recorded)", "secret-thought"),
                    ("Assistant", "b")
                ]
            );
        }
    }
    assert!(
        !format!("{:?}", ModelThinkingChunk::new(1, "secret-thought")).contains("secret-thought")
    );
}

#[test]
fn restored_answer_suffix_merges_with_last_text_checkpoint() {
    use super::replay::{ReplayOutcome, ReplayTurn};
    use marlin_agent_sessions::{ConversationCheckpointChannel, ConversationCheckpointSpan};
    let mut app = App::new(ModelEndpoint::new("openai", "gpt-test"));
    app.restore(vec![ReplayTurn {
        id: serde_json::from_str("1").unwrap(),
        prompt: "prompt".into(),
        thinking: String::new(),
        checkpoint_spans: vec![ConversationCheckpointSpan {
            channel: ConversationCheckpointChannel::Text,
            start: 0,
            end: 2,
        }],
        outcome: ReplayOutcome::Completed(assistant_gateway_message("answer")),
    }]);
    let answers: Vec<_> = app
        .transcript
        .iter()
        .filter(|(role, _)| role == "Assistant")
        .map(|(_, text)| text.as_str())
        .collect();
    assert_eq!(answers, ["answer"]);
}

#[test]
fn recorded_thinking_is_visible_and_blocks_unverified_continuation() {
    use super::replay::{ReplayOutcome, ReplayTurn};
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let requests = Arc::new(Mutex::new(Vec::new()));
    let gateway = FixtureGateway {
        requests: requests.clone(),
    };
    let (tx, _) = reply_channel(8, 4096);
    let mut app = App::new(ModelEndpoint::new("openai", "gpt-test"));
    app.restore(vec![ReplayTurn {
        id: serde_json::from_str("1").unwrap(),
        prompt: "question".into(),
        thinking: "reasoning prefix".into(),
        checkpoint_spans: vec![marlin_agent_sessions::ConversationCheckpointSpan {
            channel: marlin_agent_sessions::ConversationCheckpointChannel::Thinking,
            start: 0,
            end: "reasoning prefix".len(),
        }],
        outcome: ReplayOutcome::Completed(assistant_gateway_message("answer")),
    }]);
    assert!(
        app.transcript
            .iter()
            .any(|(role, text)| role == "Thinking (recorded)" && text == "reasoning prefix")
    );
    app.composer.insert_text("new question");
    app.submit(&gateway, &runtime, &tx);
    assert!(requests.lock().unwrap().is_empty());
    assert_eq!(app.composer.text(), "new question");
    assert!(
        app.transcript
            .iter()
            .any(|(_, text)| text.contains("not yet verified"))
    );
}

#[test]
fn restored_session_reports_incompatible_model_route() {
    use super::replay::{ReplayOutcome, ReplayTurn};
    let original = ModelEndpoint::new("openai", "gpt-original");
    let mut app = App::new(ModelEndpoint::new("openai", "gpt-alternate"));
    let mut answer = assistant_gateway_message("answer");
    answer.continuation = Some(ModelContinuation {
        endpoint: original,
        kind: ModelContinuationKind::OpenAiResponses,
        destination: None,
        payload: "opaque".into(),
    });
    app.restore(vec![ReplayTurn {
        id: serde_json::from_str("1").unwrap(),
        prompt: "prompt".into(),
        thinking: String::new(),
        checkpoint_spans: Vec::new(),
        outcome: ReplayOutcome::Completed(answer),
    }]);
    assert!(
        app.transcript
            .last()
            .unwrap()
            .1
            .contains("Restored session")
    );
    assert!(app.transcript.last().unwrap().1.contains("original route"));
    assert_eq!(app.history.len(), 2);
}

#[test]
fn incompatible_continuation_retains_draft_without_dispatch() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let requests = Arc::new(Mutex::new(Vec::new()));
    let gateway = FixtureGateway {
        requests: requests.clone(),
    };
    let (tx, _rx) = reply_channel(8, 4096);
    let original = ModelEndpoint::new("openai", "gpt-original");
    let alternate = ModelEndpoint::new("openai", "gpt-alternate").with_alias("alt");
    let mut app = App::new(original.clone());
    let catalog = ModelCatalog::new(vec![
        ModelRoute {
            alias: ModelAlias::new("default"),
            endpoint: original.clone(),
            gateway: Some(Arc::new(gateway.clone())),
        },
        ModelRoute {
            alias: ModelAlias::new("alt"),
            endpoint: alternate,
            gateway: Some(Arc::new(gateway.clone())),
        },
    ])
    .unwrap();
    app.set_models(catalog);
    let mut answer = assistant_gateway_message("answer");
    answer.continuation = Some(ModelContinuation {
        endpoint: original,
        kind: ModelContinuationKind::OpenAiResponses,
        destination: None,
        payload: "opaque".into(),
    });
    Arc::make_mut(&mut app.history).extend([user_gateway_message("prompt"), answer]);
    app.composer.insert_text("keep draft");
    app.toggle_command();
    app.composer.insert_text("model alt");
    app.submit(&gateway, &runtime, &tx);
    assert!(
        app.transcript
            .last()
            .unwrap()
            .1
            .contains("sending is blocked")
    );
    assert_eq!(app.composer.text(), "keep draft");
    app.submit(&gateway, &runtime, &tx);
    assert!(requests.lock().unwrap().is_empty());
    assert!(app.pending.is_none());
    assert_eq!(app.composer.text(), "keep draft");
    assert!(
        app.transcript
            .last()
            .unwrap()
            .1
            .contains("continuation belongs")
    );
}

#[test]
fn historical_unbound_continuation_retains_draft_without_dispatch() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let requests = Arc::new(Mutex::new(Vec::new()));
    let gateway = FixtureGateway {
        requests: requests.clone(),
    };
    let (tx, _rx) = reply_channel(8, 4096);
    let endpoint = ModelEndpoint::new("openai", "gpt-original");
    let mut app = App::new(endpoint.clone());
    let mut answer = assistant_gateway_message("answer");
    answer.continuation = Some(ModelContinuation {
        endpoint,
        kind: ModelContinuationKind::OpenAiResponses,
        destination: None,
        payload: "opaque".into(),
    });
    Arc::make_mut(&mut app.history).extend([user_gateway_message("prompt"), answer]);
    app.composer.insert_text("keep draft");
    app.submit(&gateway, &runtime, &tx);
    assert_eq!(app.composer.text(), "keep draft");
    assert!(
        app.transcript
            .last()
            .unwrap()
            .1
            .contains("no destination binding")
    );
    assert!(requests.lock().unwrap().is_empty());
}

#[test]
fn context_limit_retains_draft_and_does_not_dispatch_or_drop_history() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let requests = Arc::new(Mutex::new(Vec::new()));
    let gateway = FixtureGateway {
        requests: requests.clone(),
    };
    let (tx, mut rx) = reply_channel(8, 4096);
    let mut app = App::new(ModelEndpoint::new("openai", "gpt-test"));
    app.composer.insert_text("first");
    app.submit(&gateway, &runtime, &tx);
    while app.pending.is_some() {
        let (id, event) = rx.blocking_recv().unwrap().into_parts();
        app.receive(id, event);
    }
    app.context_budget = ContextBudget {
        bytes: usize::MAX,
        turns: 1,
    };
    app.composer.insert_text("second");
    app.submit(&gateway, &runtime, &tx);
    assert_eq!(requests.lock().unwrap().len(), 1);
    assert_eq!(app.composer.text(), "second");
    assert_eq!(app.history.len(), 2);
    assert!(app.transcript.last().unwrap().1.contains("Draft retained"));
}

#[test]
fn command_entry_preserves_draft_cursor_and_never_sends_plain_text() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let requests = Arc::new(Mutex::new(Vec::new()));
    let gateway = FixtureGateway {
        requests: requests.clone(),
    };
    let (tx, _) = reply_channel(8, 4096);
    let mut app = App::new(ModelEndpoint::new("openai", "gpt-test"));
    app.composer.insert_text("a中b");
    app.composer.left();
    app.toggle_command();
    assert_eq!(app.composer.text(), "/");
    app.composer.insert_text("help");
    app.submit(&gateway, &runtime, &tx);
    assert_eq!(app.composer.text(), "a中b");
    app.composer.insert('X');
    assert_eq!(app.composer.text(), "a中Xb");
    app.toggle_command();
    app.composer.insert_text("temporary");
    app.cancel();
    assert_eq!(app.composer.text(), "a中Xb");
    app.toggle_command();
    app.composer.backspace();
    app.composer.insert_text("not a command");
    app.submit(&gateway, &runtime, &tx);
    assert_eq!(app.composer.text(), "a中Xb");
    assert!(requests.lock().unwrap().is_empty());
    assert!(app.transcript.last().unwrap().1.contains("Draft retained"));
    app.toggle_command();
    app.composer.insert_text("session \"session:missing\"");
    app.submit(&gateway, &runtime, &tx);
    assert_eq!(app.composer.text(), "a中Xb");
    assert!(requests.lock().unwrap().is_empty());
}

#[test]
fn context_command_reports_original_draft_without_dispatching_it() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let requests = Arc::new(Mutex::new(Vec::new()));
    let gateway = FixtureGateway {
        requests: requests.clone(),
    };
    let (tx, _) = reply_channel(8, 4096);
    let mut app = App::new(ModelEndpoint::new("openai", "gpt-test"));
    app.history = Arc::new(vec![
        user_gateway_message("old"),
        assistant_gateway_message("reply"),
    ]);
    app.composer.insert_text("draft 中");
    app.toggle_command();
    app.composer.insert_text("context");
    app.submit(&gateway, &runtime, &tx);
    assert_eq!(app.composer.text(), "draft 中");
    let message = &app.transcript.last().unwrap().1;
    assert!(message.contains("Completed turns: 1 /"), "{message}");
    assert!(message.contains("9-byte draft"), "{message}");
    assert!(message.contains("not token counts"), "{message}");
    assert!(requests.lock().unwrap().is_empty());
}

#[test]
fn interrupted_stream_keeps_partial_output_out_of_history() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let gateway = FixtureGateway {
        requests: Arc::new(Mutex::new(Vec::new())),
    };
    let (tx, mut rx) = reply_channel(8, 4096);
    let mut app = App::new(ModelEndpoint::new("openai", "gpt-test"));
    for c in "first".chars() {
        app.composer.insert(c);
    }
    app.submit(&gateway, &runtime, &tx);
    let (id, event) = rx.blocking_recv().unwrap().into_parts();
    app.receive(id, event);
    let (_, event) = rx.blocking_recv().unwrap().into_parts();
    app.receive(id, event);
    app.receive(
        id,
        Err(ReplyError::Gateway(ModelGatewayError::Completion(
            "connection lost".into(),
        ))),
    );
    assert!(app.pending.is_none());
    assert!(app.history.is_empty());
    assert_eq!(app.transcript[2].1, "hel");
    assert!(app.transcript.last().unwrap().1.contains("connection lost"));
}

#[test]
fn sending_a_recalled_prompt_restores_the_original_draft() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let requests = Arc::new(Mutex::new(Vec::new()));
    let gateway = FixtureGateway {
        requests: requests.clone(),
    };
    let (tx, mut rx) = reply_channel(8, 4096);
    let mut app = App::new(ModelEndpoint::new("openai", "gpt-test"));
    app.history = Arc::new(vec![
        user_gateway_message("previous"),
        assistant_gateway_message("answer"),
    ]);
    app.composer.insert_text("new draft");
    app.composer.left();
    app.navigate_prompts(true);
    assert_eq!(app.composer.text(), "previous");
    app.composer.insert_text(" revised");
    app.submit(&gateway, &runtime, &tx);
    assert_eq!(app.composer.text(), "new draft");
    app.composer.insert_text("!");
    assert_eq!(app.composer.text(), "new draf!t");
    let _ = rx.blocking_recv().unwrap();
    assert_eq!(
        requests.lock().unwrap()[0]
            .messages()
            .last()
            .unwrap()
            .content,
        "previous revised"
    );
}

#[test]
fn legacy_failure_blocks_submission_without_losing_draft() {
    use super::replay::{ReplayOutcome, ReplayTurn};
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let id = runtime.block_on(async {
        let mut journal = marlin_agent_sessions::ConversationJournal::open(
            marlin_agent_storage::InMemoryAgentStorage::new(),
            marlin_agent_storage::ProjectId::new("project:legacy").unwrap(),
            marlin_agent_storage::SessionId::new("session:legacy").unwrap(),
            marlin_agent_storage::AgentId::new("agent:legacy").unwrap(),
            |_| {},
        )
        .await
        .unwrap();
        journal
            .start(
                marlin_agent_sessions::ConversationTurnStart {
                    endpoint: ModelEndpoint::new("openai", "gpt-test"),
                    user_text: "previous".into(),
                },
                1,
            )
            .await
            .unwrap()
    });
    let requests = Arc::new(Mutex::new(Vec::new()));
    let gateway = FixtureGateway {
        requests: requests.clone(),
    };
    let (tx, _) = reply_channel(8, 4096);
    let mut app = App::new(ModelEndpoint::new("openai", "gpt-test"));
    app.restore(vec![ReplayTurn {
        id,
        prompt: "previous".into(),
        thinking: String::new(),
        checkpoint_spans: Vec::new(),
        outcome: ReplayOutcome::LegacyFailed {
            diagnostic: "old error".into(),
            partial: "partial".into(),
        },
    }]);
    app.composer.insert_text("keep draft");
    app.submit(&gateway, &runtime, &tx);
    assert_eq!(app.composer.text(), "keep draft");
    assert!(requests.lock().unwrap().is_empty());
    assert!(
        app.transcript
            .iter()
            .any(|(_, text)| text.contains("Remote retirement is unverified"))
    );
}

#[test]
fn persistent_session_stops_after_a_mismatched_completion() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let requests = Arc::new(Mutex::new(Vec::new()));
    let gateway = FixtureGateway {
        requests: requests.clone(),
    };
    let (tx, mut rx) = reply_channel(8, 4096);
    let mut app = App::new(ModelEndpoint::new("openai", "gpt-test"));
    app.restore(Vec::new());
    app.composer.insert_text("first");
    app.submit(&gateway, &runtime, &tx);
    let (id, started) = rx.blocking_recv().unwrap().into_parts();
    app.receive(id, started);
    let (_, chunk) = rx.blocking_recv().unwrap().into_parts();
    app.receive(id, chunk);
    let response = ModelGatewayCompletionResponse::new(
        "fixture",
        "gpt-test",
        vec![ModelGatewayCompletionChoice::new(
            0,
            assistant_gateway_message("mismatch"),
            None,
        )],
    );
    app.receive(id, Ok(ModelStreamEvent::Completed { response }));
    assert!(app.pending.is_none());
    app.composer.insert_text("second");
    app.submit(&gateway, &runtime, &tx);
    assert_eq!(requests.lock().unwrap().len(), 1);
    assert!(app.transcript.last().unwrap().1.contains("uncertain"));
    assert_eq!(app.composer.take(), "second");
    app.composer.insert_text("/help");
    app.submit(&gateway, &runtime, &tx);
    assert_eq!(app.transcript.last().unwrap().0, "Help");
    assert_eq!(requests.lock().unwrap().len(), 1);
}

#[test]
fn cancelled_turn_ignores_late_events() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let gateway = FixtureGateway {
        requests: Arc::new(Mutex::new(Vec::new())),
    };
    let (tx, mut rx) = reply_channel(8, 4096);
    let mut app = App::new(ModelEndpoint::new("openai", "gpt-test"));
    for c in "first".chars() {
        app.composer.insert(c);
    }
    app.submit(&gateway, &runtime, &tx);
    let (id, event) = rx.blocking_recv().unwrap().into_parts();
    app.receive(id, event);
    let (_, event) = rx.blocking_recv().unwrap().into_parts();
    app.receive(id, event);
    app.cancel();
    app.receive(
        id,
        Ok(ModelStreamEvent::Chunk(ModelStreamChunk::new(2, "ignored"))),
    );
    assert!(app.pending.is_none());
    assert!(app.history.is_empty());
    assert_eq!(app.transcript[2].1, "hel");
    assert!(app.transcript.last().unwrap().1.contains("cancelled"));
}

#[test]
fn output_strips_terminal_controls() {
    assert_eq!(safe_text("a\x1b[2J\tb\n"), "a�[2J b\n");
}
