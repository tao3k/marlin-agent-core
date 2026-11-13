// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use super::{
    app::{App, GatewayObservation, ModelineActivity, safe_text},
    cli::{CliAction, parse_cli},
    context_budget::ContextBudget,
    event_queue::{ReplyError, reply_channel},
    model_catalog::{ModelCatalog, ModelRoute},
    render::{chatbox_height, draw as render_draw, frame_regions},
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
use proptest::{
    prelude::{ProptestConfig, any},
    prop_assert, prop_assert_eq, proptest,
    test_runner::RngSeed,
};
use std::sync::{Arc, Mutex};

use super::ui_fixture::{test_frame, test_layout};

fn mouse_over_minibuffer(
    app: &App,
    layout: &marlin_agent_ui::UiLayoutPolicy,
    width: u16,
    height: u16,
    row: u16,
) -> bool {
    use marlin_agent_ui::{UiComponentId, UiRegion, UiStatusItem, UiWorkbench};
    let view = UiWorkbench {
        model_label: "model".into(),
        session_label: "session".into(),
        regions: vec![
            UiRegion::Transcript,
            UiRegion::Composer,
            UiRegion::Commands(vec![]),
            UiRegion::Status(vec![UiStatusItem::host(
                UiComponentId::new("modeline/left/brand").unwrap(),
                "brand",
            )]),
        ],
        layout: layout.clone(),
        theme: Default::default(),
        focus: UiComponentId::new("composer").unwrap(),
        frame: test_frame(),
        keymap: vec![],
        transients: vec![],
    };
    let regions = frame_regions(app, &view, ratatui::layout::Rect::new(0, 0, width, height));
    super::render::mouse_over_minibuffer(&regions, 1, row)
}

/// Native tests exercise only the capabilities they use. The selected Scheme
/// catalogue itself is checked by the Scheme and linked UI tests.
fn install_test_commands(app: &mut App) {
    use marlin_agent_ui::{
        UiCommand, UiCommandArguments as Args, UiCommandCapability as Cap, UiCommandName,
    };
    let entries = [
        ("help", Args::None, Cap::SelectBuffer, Some("help")),
        ("models", Args::None, Cap::Models, None),
        ("model", Args::Required, Cap::Model, None),
        ("sessions", Args::None, Cap::Sessions, None),
        ("sessions-next", Args::None, Cap::Sessions, Some("next")),
        ("session", Args::Required, Cap::Session, None),
        ("history", Args::None, Cap::History, Some("newest")),
        ("history-older", Args::None, Cap::History, Some("older")),
        ("diff", Args::Optional, Cap::Workspace, Some("diff")),
        ("context-budget", Args::None, Cap::ContextBudget, None),
        ("runtime", Args::None, Cap::Runtime, None),
        ("switch-themes", Args::None, Cap::OpenMenu, Some("theme-")),
        ("theme-pi", Args::None, Cap::SelectProfile, Some("pi")),
        ("switch-buffer", Args::None, Cap::OpenMenu, Some("buffer-")),
        (
            "buffer-messages",
            Args::None,
            Cap::SelectBuffer,
            Some("messages"),
        ),
    ];
    app.command_catalog = entries
        .into_iter()
        .map(|(name, arguments, capability, value)| UiCommand {
            name: UiCommandName::new(name).unwrap(),
            description: String::new(),
            arguments,
            capability,
            value: value.map(str::to_owned),
        })
        .collect();
}

fn selected_modeline_item(id: &str) -> marlin_agent_ui::UiStatusItem {
    use marlin_agent_ui::{UiComponentId, UiStatusItem, UiStatusTone};
    let (label, compact_label, tone, active_tone, min_width, compact_below) = match id {
        "modeline/left/brand" => (
            "🐬 BRAND",
            "🐬",
            UiStatusTone::Accent,
            UiStatusTone::Accent,
            0,
            18,
        ),
        "modeline/left/model" => (
            "  {value}  ",
            "{value}",
            UiStatusTone::Info,
            UiStatusTone::Info,
            0,
            0,
        ),
        "modeline/left/activity" => (
            "~ ◇ Standing",
            "~ ◇",
            UiStatusTone::Info,
            UiStatusTone::Warning,
            0,
            64,
        ),
        "modeline/right/turns" => (
            "T{value}  ",
            "T{value}",
            UiStatusTone::Info,
            UiStatusTone::Info,
            72,
            0,
        ),
        "modeline/right/session" => (
            " § {value} ",
            "§{value}",
            UiStatusTone::Muted,
            UiStatusTone::Muted,
            44,
            0,
        ),
        "modeline/state/online" => (
            "◆ Linked",
            "◆",
            UiStatusTone::Success,
            UiStatusTone::Success,
            0,
            64,
        ),
        "modeline/state/offline" => (
            "◇ Lost",
            "◇",
            UiStatusTone::Warning,
            UiStatusTone::Warning,
            0,
            64,
        ),
        "modeline/state/unknown" => (
            "◌ Unclear",
            "◌",
            UiStatusTone::Warning,
            UiStatusTone::Warning,
            0,
            64,
        ),
        _ => panic!("unknown selected modeline item"),
    };
    UiStatusItem {
        id: UiComponentId::new(id).unwrap(),
        label: label.into(),
        compact_label: compact_label.into(),
        tone,
        active_tone,
        min_width,
        compact_below,
        keep_priority: match id {
            "modeline/left/model" => 3,
            "modeline/right/session" => 2,
            "modeline/right/turns" => 1,
            _ => 0,
        },
    }
}

#[test]
fn minibuffer_geometry_preserves_bottom_hint_across_terminal_sizes() {
    let mut app = App::new(ModelEndpoint::new("provider", "model"));
    install_test_commands(&mut app);
    for height in 7..=80 {
        for width in [10, 24, 40, 80, 120, 160] {
            assert_eq!(chatbox_height(&app, &test_layout(), width, height), 0);
            for row in 0..=height + 1 {
                assert_eq!(
                    mouse_over_minibuffer(&app, &test_layout(), width, height, row),
                    row == height,
                    "collapsed geometry at {width}x{height}, row {row}"
                );
            }

            app.minibuffer_hovered = true;
            let expanded = chatbox_height(&app, &test_layout(), width, height);
            assert!((3..=10).contains(&expanded));
            assert!(expanded + 3 <= height);
            for row in 0..=height + 1 {
                assert_eq!(
                    mouse_over_minibuffer(&app, &test_layout(), width, height, row),
                    row > height - expanded - 1 && row <= height,
                    "expanded geometry at {width}x{height}, row {row}"
                );
            }
            app.minibuffer_hovered = false;
        }
    }
    app.composer.insert_text("draft survives pointer leave");
    assert!(chatbox_height(&app, &test_layout(), 80, 30) >= 3);
    assert!(!mouse_over_minibuffer(&app, &test_layout(), 80, 30, 1));
}

#[test]
fn selected_layout_policy_changes_chatbox_and_hover_geometry() {
    let mut app = App::new(ModelEndpoint::new("provider", "model"));
    install_test_commands(&mut app);
    app.minibuffer_hovered = true;
    app.composer.insert_text(&"draft ".repeat(100));
    let compact = marlin_agent_ui::UiLayoutPolicy {
        max_chatbox_rows: 5,
        message_frame: marlin_agent_ui::UiFrameSpec {
            min_transcript_rows: 4,
            ..test_layout().message_frame
        },
        ..test_layout()
    };
    let expanded = chatbox_height(&app, &compact, 24, 30);
    assert_eq!(expanded, 5);
    assert!(mouse_over_minibuffer(&app, &compact, 24, 30, 25));
    assert!(!mouse_over_minibuffer(&app, &compact, 24, 30, 24));
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 128,
        rng_seed: RngSeed::Fixed(42),
        failure_persistence: None,
        ..ProptestConfig::default()
    })]

    #[test]
    fn modeline_stays_above_the_minibuffer_for_generated_layouts(
        width in 24u16..=160,
        height in 7u16..=80,
        hovered in any::<bool>(),
        draft in any::<bool>(),
    ) {
        use marlin_agent_ui::{UiComponentId, UiRegion, UiWorkbench};
        let mut app = App::new(ModelEndpoint::new("provider", "model"));
    install_test_commands(&mut app);
        app.minibuffer_hovered = hovered;
        if draft {
            app.composer.insert_text("retained draft");
        }
        let editor_rows = chatbox_height(&app, &test_layout(), width, height);
        if hovered || draft {
            prop_assert!((3..=10).contains(&editor_rows));
        } else {
            prop_assert_eq!(editor_rows, 0);
        }
        let view = UiWorkbench {
            layout: test_layout(),
            theme: marlin_agent_ui::UiTheme::default(),
            model_label: "provider/model".into(),
            session_label: "temporary".into(),
            regions: vec![
                UiRegion::Transcript,
                UiRegion::Composer,
                UiRegion::Commands(vec![]),
                UiRegion::Status([
                    "modeline/left/brand",
                    "modeline/left/model",
                    "modeline/left/activity",
                    "modeline/right/turns",
                    "modeline/right/session",
                    "modeline/state/online",
                    "modeline/state/offline",
                    "modeline/state/unknown",
                ].into_iter().map(selected_modeline_item).collect()),
            ],
            focus: UiComponentId::new("composer").unwrap(),
            frame: test_frame(),
            keymap: Vec::new(),
        transients: Vec::new(),
        };
        let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(width, height))
            .unwrap();
        terminal.draw(|frame| {
            render_draw(frame, &app, &view, &mut Default::default(), &mut Default::default());
        }).unwrap();
        let modeline_y = height - editor_rows - 2;
        let row = (0..width)
            .map(|x| terminal.backend().buffer()[(x, modeline_y)].symbol())
            .collect::<String>();
        prop_assert!(row.contains("BRAND"), "{width}x{height}: {row:?}");
        let top = (0..width)
            .map(|x| terminal.backend().buffer()[(x, 0)].symbol())
            .collect::<String>();
        prop_assert!(!top.contains("BRAND"), "redundant top status at {width}x{height}");
    }
}

#[test]
fn selecting_commands_runs_defaults_and_retains_required_arguments() {
    let mut app = App::new(ModelEndpoint::new("provider", "model"));
    install_test_commands(&mut app);
    app.composer.insert_text("s");
    assert!(app.accept_command_completion("sessions"));
    assert_eq!(app.composer.text(), "sessions");
    assert!(app.accept_command_completion("sessions-next"));
    assert_eq!(app.composer.text(), "sessions-next");

    assert!(app.accept_command_completion("diff "));
    assert_eq!(app.composer.text(), "diff ");

    assert!(!app.accept_command_completion("model "));
    assert_eq!(app.composer.text(), "model ");
    assert!(app.accept_command_completion("history"));
    assert_eq!(app.composer.text(), "history");
    assert!(app.accept_command_completion("history-older"));
    assert_eq!(app.composer.text(), "history-older");
}

#[test]
fn opening_mx_selects_the_visible_first_candidate_before_arrow_navigation() {
    let mut app = App::new(ModelEndpoint::new("provider", "model"));
    install_test_commands(&mut app);
    app.toggle_command();
    app.command_candidates = vec![marlin_agent_ui::UiCommandName::new("help").unwrap()];
    app.seed_command_selection();
    assert_eq!(
        app.selected_command().map(|name| name.as_str()),
        Some("help")
    );
    app.composer.insert_text("m");
    assert!(app.selected_command().is_none());
    app.command_candidates = vec![marlin_agent_ui::UiCommandName::new("models").unwrap()];
    assert_eq!(app.menu_navigation_anchor().unwrap().as_str(), "models");
}

fn draw(
    frame: &mut ratatui::Frame<'_>,
    app: &App,
    cache: &mut TranscriptViewportCache,
    anchor: &mut super::viewport::ScrollAnchor,
) -> usize {
    use marlin_agent_ui::{UiComponentId, UiRegion, UiWorkbench};
    let view = UiWorkbench {
        layout: test_layout(),
        theme: marlin_agent_ui::UiTheme::default(),
        model_label: "test".into(),
        session_label: "test".into(),
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
    render_draw(frame, app, &view, cache, anchor)
}

#[test]
fn render_uses_the_admitted_scheme_focus() {
    use marlin_agent_ui::{UiComponentId, UiRegion, UiWorkbench};
    let app = App::new(ModelEndpoint::new("provider", "model"));
    let view = UiWorkbench {
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
        focus: UiComponentId::new("transcript").unwrap(),
        frame: test_frame(),
        keymap: Vec::new(),
        transients: Vec::new(),
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
    assert!(status.contains("transcript-focus"), "{status}");
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
        layout: test_layout(),
        theme: marlin_agent_ui::UiTheme::default(),
        model_label: "model".into(),
        session_label: "session".into(),
        regions,
        focus: UiComponentId::new("third").unwrap(),
        frame: test_frame(),
        keymap: Vec::new(),
        transients: Vec::new(),
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
    assert!(screen.contains("extension-focus"), "{screen}");
}

#[test]
fn render_focused_scheme_content_on_narrow_and_wide_terminals() {
    use marlin_agent_ui::{UiComponentId, UiContent, UiRegion, UiWorkbench};
    let app = App::new(ModelEndpoint::new("provider", "model"));
    let view = UiWorkbench {
        layout: test_layout(),
        theme: marlin_agent_ui::UiTheme::default(),
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
        frame: test_frame(),
        keymap: Vec::new(),
        transients: Vec::new(),
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
        assert!(!screen.contains("Model gateway configured"), "{screen}");
    }
}

#[test]
fn unfocused_extension_does_not_create_a_default_sidebar() {
    use marlin_agent_ui::{UiComponentId, UiContent, UiRegion, UiWorkbench};
    let app = App::new(ModelEndpoint::new("provider", "model"));
    let view = UiWorkbench {
        layout: test_layout(),
        theme: marlin_agent_ui::UiTheme::default(),
        model_label: "model".into(),
        session_label: "session".into(),
        regions: vec![
            UiRegion::Transcript,
            UiRegion::Composer,
            UiRegion::Commands(vec![]),
            UiRegion::Extension {
                id: UiComponentId::new("guide").unwrap(),
                label: "Guide".into(),
                focusable: true,
                content: vec![UiContent::Text("Get started".into())],
            },
            UiRegion::Status(vec![]),
        ],
        focus: UiComponentId::new("composer").unwrap(),
        frame: test_frame(),
        keymap: Vec::new(),
        transients: Vec::new(),
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
        assert!(!screen.contains("Get started"), "{screen}");
        assert!(!screen.contains("Model gateway configured"), "{screen}");
    }
}

#[test]
fn render_status_contributions_respects_focus_and_terminal_height() {
    use marlin_agent_ui::{UiComponentId, UiRegion, UiStatusItem, UiWorkbench};
    let app = App::new(ModelEndpoint::new("provider", "model"));
    let view = UiWorkbench {
        layout: test_layout(),
        theme: marlin_agent_ui::UiTheme::default(),
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
                UiStatusItem::host(UiComponentId::new("ready").unwrap(), "Ready"),
                UiStatusItem::host(UiComponentId::new("unsafe").unwrap(), "bad\u{1b}[2J"),
            ]),
        ],
        focus: UiComponentId::new("extension").unwrap(),
        frame: test_frame(),
        keymap: Vec::new(),
        transients: Vec::new(),
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
        assert!(screen.contains("extension-focus"), "{screen}");
        assert!(screen.contains("Ready · bad�[2J"), "{screen}");
        assert!(!screen.contains('\u{1b}'));
    }
}

#[test]
fn history_search_previews_then_accepts_without_submitting() {
    let mut app = App::new(ModelEndpoint::new("openai", "test"));
    install_test_commands(&mut app);
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
    assert!(status.contains("fixture search find"), "{status}");
    app.accept_history_search();
    assert_eq!(app.composer.text(), "find this");
    assert!(app.pending.is_none());
    assert_eq!(app.history.len(), 2);
}

#[test]
fn unfinished_history_search_cannot_accept_an_old_preview() {
    let mut app = App::new(ModelEndpoint::new("openai", "test"));
    install_test_commands(&mut app);
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
fn transcript_starts_at_the_top_without_a_duplicate_status_heading() {
    let mut app = App::new(ModelEndpoint::new("deepseek", "chat\u{1b}[2J"));
    install_test_commands(&mut app);
    app.transcript
        .push(("You".into(), "actual conversation".into()));
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
    let first_row: String = (0..100)
        .map(|x| terminal.backend().buffer()[(x, 0)].symbol())
        .collect();
    assert!(first_row.contains("You"), "{first_row}");
    assert!(!first_row.contains("Marlin"), "{first_row}");
    assert!(!first_row.contains("· model gateway ·"), "{first_row}");
    assert!(!first_row.contains('\u{1b}'));
}

#[test]
fn runtime_receipt_uses_the_full_content_width() {
    let mut app = App::new(ModelEndpoint::new("deepseek", "chat"));
    install_test_commands(&mut app);
    app.set_runtime_edge(
        RuntimeEdgePolicy::new()
            .with_timeout_ms(1234)
            .receipt()
            .unwrap(),
    );
    app.runtime_receipt_open = true;
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
    let row = |y| {
        (0..100)
            .map(|x| terminal.backend().buffer()[(x, y)].symbol())
            .collect::<String>()
    };
    assert!(row(0).contains("Runtime policy"));
    assert!(row(1).contains("Stream-open timeout: 1234 ms"));
    assert!(row(2).contains("Concurrency limit: Disabled"));
    assert!(row(3).contains("Load shedding: Disabled"));
    assert!(row(11).contains("runtime"));
}

#[test]
fn minibuffer_expands_with_the_draft_and_leaves_a_transcript_row() {
    let mut app = App::new(ModelEndpoint::new("openai", "gpt-test"));
    install_test_commands(&mut app);
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
    assert!(!row(8).contains("Message"));
    assert!(row(11).contains("fixture status"));

    app.composer
        .insert_text("one\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten");
    terminal
        .draw(|frame| {
            draw(frame, &app, &mut cache, &mut anchor);
        })
        .unwrap();
    let buffer = terminal.backend().buffer();
    let first_row: String = (0..40).map(|x| buffer[(x, 0)].symbol()).collect();
    let transcript: String = (0..40).map(|x| buffer[(x, 1)].symbol()).collect();
    let composer: String = (0..40).map(|x| buffer[(x, 3)].symbol()).collect();
    assert!(first_row.contains("You"));
    assert!(transcript.contains("visible transcript"));
    assert!(composer.contains("Message"));
    assert!(terminal.backend().cursor_position().y < 11);
}

#[test]
fn rejected_draft_insertion_is_visible_in_the_terminal_status() {
    let mut app = App::new(ModelEndpoint::new("openai", "gpt-test"));
    install_test_commands(&mut app);
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
    assert!(status.contains("draft-limit"), "{status}");
    assert_eq!(app.composer.text().len(), 1024 * 1024);
}

#[path = "app_cases/continuation.rs"]
mod continuation;
#[path = "app_cases/gateway.rs"]
mod gateway;
#[path = "app_cases/ui.rs"]
mod ui;
#[derive(Clone)]
struct FixtureGateway {
    requests: Arc<Mutex<Vec<ModelGatewayRequest>>>,
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
