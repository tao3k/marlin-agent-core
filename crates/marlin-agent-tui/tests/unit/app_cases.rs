// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use super::{
    app::{App, GatewayObservation, safe_text},
    cli::{CliAction, parse_cli},
    context_budget::ContextBudget,
    event_queue::{ReplyError, reply_channel},
    model_catalog::{ModelCatalog, ModelRoute},
    render::{chatbox_height, draw as render_draw, mouse_over_minibuffer},
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

fn selected_modeline_item(id: &str) -> marlin_agent_ui::UiStatusItem {
    use marlin_agent_ui::{UiComponentId, UiStatusItem, UiStatusTone};
    let (label, compact_label, tone, active_tone) = match id {
        "modeline/left/brand" => (
            "🐟 MARLIN",
            "🐟",
            UiStatusTone::Accent,
            UiStatusTone::Accent,
        ),
        "modeline/left/model" => ("", "", UiStatusTone::Info, UiStatusTone::Info),
        "modeline/left/activity" => ("🟡 Ready", "🟡", UiStatusTone::Info, UiStatusTone::Warning),
        "modeline/right/turns" => ("", "", UiStatusTone::Info, UiStatusTone::Info),
        "modeline/right/session" => ("", "", UiStatusTone::Muted, UiStatusTone::Muted),
        "modeline/state/online" => (
            "🟢 Online",
            "🟢",
            UiStatusTone::Success,
            UiStatusTone::Success,
        ),
        "modeline/state/offline" => (
            "🔴 Offline",
            "🔴",
            UiStatusTone::Warning,
            UiStatusTone::Warning,
        ),
        "modeline/state/unknown" => (
            "🟠 Unknown",
            "🟠",
            UiStatusTone::Warning,
            UiStatusTone::Warning,
        ),
        _ => panic!("unknown selected modeline item"),
    };
    UiStatusItem {
        id: UiComponentId::new(id).unwrap(),
        label: label.into(),
        compact_label: compact_label.into(),
        tone,
        active_tone,
    }
}

#[test]
fn minibuffer_geometry_preserves_bottom_hint_across_terminal_sizes() {
    let mut app = App::new(ModelEndpoint::new("provider", "model"));
    for height in 7..=80 {
        for width in [10, 24, 40, 80, 120, 160] {
            assert_eq!(
                chatbox_height(
                    &app,
                    &marlin_agent_ui::UiLayoutPolicy::default(),
                    width,
                    height
                ),
                0
            );
            for row in 0..=height + 1 {
                assert_eq!(
                    mouse_over_minibuffer(
                        &app,
                        &marlin_agent_ui::UiLayoutPolicy::default(),
                        width,
                        height,
                        row
                    ),
                    row == height,
                    "collapsed geometry at {width}x{height}, row {row}"
                );
            }

            app.minibuffer_hovered = true;
            let expanded = chatbox_height(
                &app,
                &marlin_agent_ui::UiLayoutPolicy::default(),
                width,
                height,
            );
            assert!((3..=10).contains(&expanded));
            assert!(expanded + 3 <= height);
            for row in 0..=height + 1 {
                assert_eq!(
                    mouse_over_minibuffer(
                        &app,
                        &marlin_agent_ui::UiLayoutPolicy::default(),
                        width,
                        height,
                        row
                    ),
                    row > height - expanded - 1 && row <= height,
                    "expanded geometry at {width}x{height}, row {row}"
                );
            }
            app.minibuffer_hovered = false;
        }
    }
    app.composer.insert_text("draft survives pointer leave");
    assert!(chatbox_height(&app, &marlin_agent_ui::UiLayoutPolicy::default(), 80, 30) >= 3);
    assert!(!mouse_over_minibuffer(
        &app,
        &marlin_agent_ui::UiLayoutPolicy::default(),
        80,
        30,
        1
    ));
}

#[test]
fn selected_layout_policy_changes_chatbox_and_hover_geometry() {
    let mut app = App::new(ModelEndpoint::new("provider", "model"));
    app.minibuffer_hovered = true;
    app.composer.insert_text(&"draft ".repeat(100));
    let compact = marlin_agent_ui::UiLayoutPolicy {
        max_chatbox_rows: 5,
        min_transcript_rows: 4,
        ..Default::default()
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
        app.minibuffer_hovered = hovered;
        if draft {
            app.composer.insert_text("retained draft");
        }
        let editor_rows = chatbox_height(&app, &marlin_agent_ui::UiLayoutPolicy::default(), width, height);
        if hovered || draft {
            prop_assert!((3..=10).contains(&editor_rows));
        } else {
            prop_assert_eq!(editor_rows, 0);
        }
        let view = UiWorkbench {
            layout: marlin_agent_ui::UiLayoutPolicy::default(),
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
        prop_assert!(row.contains("MARLIN"), "{width}x{height}: {row:?}");
        let top = (0..width)
            .map(|x| terminal.backend().buffer()[(x, 0)].symbol())
            .collect::<String>();
        prop_assert!(!top.contains("MARLIN"), "redundant top status at {width}x{height}");
    }
}

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
        layout: marlin_agent_ui::UiLayoutPolicy::default(),
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
        layout: marlin_agent_ui::UiLayoutPolicy::default(),
        theme: marlin_agent_ui::UiTheme::default(),
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
        layout: marlin_agent_ui::UiLayoutPolicy::default(),
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
        layout: marlin_agent_ui::UiLayoutPolicy::default(),
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
        layout: marlin_agent_ui::UiLayoutPolicy::default(),
        theme: marlin_agent_ui::UiTheme::default(),
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
        layout: marlin_agent_ui::UiLayoutPolicy::default(),
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
        layout: marlin_agent_ui::UiLayoutPolicy::default(),
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
fn selected_modeline_is_colored_and_sits_above_the_collapsible_minibuffer() {
    use marlin_agent_ui::{UiComponentId, UiRegion, UiStatusItem, UiWorkbench};
    let mut app = App::new(ModelEndpoint::new("deepseek", "chat"));
    assert!(app.transcript.is_empty());
    let mut view = UiWorkbench {
        layout: marlin_agent_ui::UiLayoutPolicy::default(),
        theme: marlin_agent_ui::UiTheme::default(),
        model_label: "deepseek/chat".into(),
        session_label: "session-one".into(),
        regions: vec![
            UiRegion::Transcript,
            UiRegion::Composer,
            UiRegion::Commands(vec![]),
            UiRegion::Status(vec![
                selected_modeline_item("modeline/left/brand"),
                selected_modeline_item("modeline/left/model"),
                selected_modeline_item("modeline/left/activity"),
                selected_modeline_item("modeline/right/turns"),
                selected_modeline_item("modeline/right/session"),
                selected_modeline_item("modeline/state/online"),
                selected_modeline_item("modeline/state/offline"),
                selected_modeline_item("modeline/state/unknown"),
                UiStatusItem::host(UiComponentId::new("ready").unwrap(), "Ready"),
            ]),
        ],
        focus: UiComponentId::new("composer").unwrap(),
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
    assert!(row(&terminal, 10).contains("MARLIN"));
    assert!(row(&terminal, 10).contains("🐟"));
    assert!(row(&terminal, 10).contains("deepseek/chat"));
    assert!(row(&terminal, 10).contains("🟡"), "{}", row(&terminal, 10));
    assert!(row(&terminal, 10).contains("Ready"));
    assert!(row(&terminal, 10).contains("session-one"));
    assert!(row(&terminal, 11).contains("Ready"));
    assert_eq!(
        terminal.backend().buffer()[(20, 10)].bg,
        ratatui::style::Color::Rgb(28, 31, 36)
    );
    assert_eq!(
        terminal.backend().buffer()[(4, 10)].bg,
        ratatui::style::Color::Rgb(198, 120, 221)
    );
    assert_eq!(
        terminal.backend().buffer()[(0, 0)].bg,
        ratatui::style::Color::Rgb(40, 44, 52)
    );
    view.theme.canvas = marlin_agent_ui::UiRgb::new(12, 34, 56);
    view.theme.accent = marlin_agent_ui::UiRgb::new(246, 48, 68);
    paint(&mut terminal, &app, &view);
    assert_eq!(
        terminal.backend().buffer()[(0, 0)].bg,
        ratatui::style::Color::Rgb(12, 34, 56)
    );
    assert_eq!(
        terminal.backend().buffer()[(4, 10)].bg,
        ratatui::style::Color::Rgb(246, 48, 68)
    );
    app.gateway_observation = GatewayObservation::Online;
    paint(&mut terminal, &app, &view);
    assert!(row(&terminal, 10).contains("🟢"));
    assert!(row(&terminal, 10).contains("Online"));
    app.gateway_observation = GatewayObservation::Offline;
    paint(&mut terminal, &app, &view);
    assert!(row(&terminal, 10).contains("🔴"));
    assert!(row(&terminal, 10).contains("Offline"));
    assert!(super::render::mouse_over_minibuffer(
        &app,
        &marlin_agent_ui::UiLayoutPolicy::default(),
        80,
        12,
        12
    ));
    assert!(!super::render::mouse_over_minibuffer(
        &app,
        &marlin_agent_ui::UiLayoutPolicy::default(),
        80,
        12,
        10
    ));

    app.minibuffer_hovered = true;
    paint(&mut terminal, &app, &view);
    assert!(row(&terminal, 8).contains("Message"));
    assert!(row(&terminal, 7).contains("MARLIN"));
    assert!(super::render::mouse_over_minibuffer(
        &app,
        &marlin_agent_ui::UiLayoutPolicy::default(),
        80,
        12,
        9
    ));
    assert!(!super::render::mouse_over_minibuffer(
        &app,
        &marlin_agent_ui::UiLayoutPolicy::default(),
        80,
        12,
        7
    ));
    app.minibuffer_hovered = false;
    app.composer.insert_text("draft survives hover exit");
    paint(&mut terminal, &app, &view);
    assert!(row(&terminal, 8).contains("Message"));
    assert!(row(&terminal, 9).contains("draft survives hover exit"));

    view.session_label = "archive/日本語/critical-session-42".into();
    paint(&mut terminal, &app, &view);
    let line = row(&terminal, 7);
    assert!(line.contains("critical-session-42"), "{line}");
    assert!(line.contains('…'), "{line}");
    assert!(!line.contains("archive"), "{line}");
}

#[test]
fn render_status_contributions_respects_focus_and_terminal_height() {
    use marlin_agent_ui::{UiComponentId, UiRegion, UiStatusItem, UiWorkbench};
    let app = App::new(ModelEndpoint::new("provider", "model"));
    let view = UiWorkbench {
        layout: marlin_agent_ui::UiLayoutPolicy::default(),
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
        assert!(screen.contains("Ready · bad�[2J"), "{screen}");
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
fn transcript_starts_at_the_top_without_a_duplicate_status_heading() {
    let mut app = App::new(ModelEndpoint::new("deepseek", "chat\u{1b}[2J"));
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
    assert!(row(11).contains("Runtime: Esc/Ctrl-C return"));
}

#[test]
fn minibuffer_expands_with_the_draft_and_leaves_a_transcript_row() {
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
    assert!(!row(8).contains("Message"));
    assert!(row(11).contains("Message: Enter send"));

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

#[path = "app_cases/continuation.rs"]
mod continuation;
#[path = "app_cases/gateway.rs"]
mod gateway;
