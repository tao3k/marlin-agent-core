// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use super::{
    App, Arc, CliAction, FixtureGateway, GatewayObservation, ModelAlias, ModelCatalog,
    ModelEndpoint, ModelGatewayCompletionChoice, ModelGatewayCompletionResponse, ModelGatewayError,
    ModelGatewayEventStream, ModelGatewayFuture, ModelGatewayRequest, ModelGatewayResult,
    ModelGatewayTransport, ModelRoute, ModelStreamChunk, ModelStreamEvent, ModelStreamingGateway,
    Mutex, ReplyError, RuntimeEdgePolicy, TranscriptViewportCache, assistant_gateway_message, draw,
    parse_cli, reply_channel, user_gateway_message,
};

#[test]
fn workspace_view_closes_without_losing_the_message_draft_or_history() {
    use crate::harness::workspace::{WorkspaceObservation, WorkspaceView};
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
    let body: String = (0..6)
        .flat_map(|y| (0..100).map(move |x| buffer[(x, y)].symbol()))
        .collect();
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
    let first_row: String = (0..100).map(|x| buffer[(x, 0)].symbol()).collect();
    assert!(!first_row.contains("· model gateway ·"));
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
    use crate::harness::workspace::WorkspaceQuery;
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
    app.gateway_observation = GatewayObservation::Online;
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
    assert_eq!(app.gateway_observation, GatewayObservation::Unchecked);
    app.gateway_observation = GatewayObservation::Offline;
    app.toggle_command();
    app.composer.insert_text("model disabled");
    app.submit(&catalog, &runtime, &tx);
    assert_eq!(app.gateway_observation, GatewayObservation::Offline);
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
                crate::harness::edge_gateway::SharedGateway::new(
                    crate::harness::journal_gateway::JournalGateway::new(source, journal),
                )
            } else {
                crate::harness::edge_gateway::SharedGateway::new(source)
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
    assert_eq!(app.transcript[1].1.len(), 16 * 1024 * 1024);
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
    assert!(app.runtime_receipt_open);
    assert_eq!(app.runtime_edge.as_ref().unwrap().timeout_ms(), Some(1234));
    assert!(app.transcript.is_empty());
    app.cancel();
    assert!(!app.runtime_receipt_open);
    assert!(requests.lock().unwrap().is_empty());
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
