// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use super::{
    App, Arc, ContextBudget, FixtureGateway, GatewayObservation, ModelAlias, ModelCatalog,
    ModelContinuation, ModelContinuationKind, ModelEndpoint, ModelGatewayCompletionChoice,
    ModelGatewayCompletionResponse, ModelGatewayError, ModelGatewayEventStream, ModelGatewayFuture,
    ModelGatewayRequest, ModelGatewayResult, ModelRoute, ModelStreamChunk, ModelStreamEvent,
    ModelStreamingGateway, ModelThinkingChunk, Mutex, ReplyError, assistant_gateway_message,
    reply_channel, safe_text, user_gateway_message,
};

#[test]
fn gateway_indicator_distinguishes_transport_failure_from_unchecked_route() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let gateway = FixtureGateway {
        requests: Arc::new(Mutex::new(Vec::new())),
    };
    let (tx, _rx) = reply_channel(8, 4096);
    let mut app = App::new(ModelEndpoint::new("openai", "gpt-test"));
    assert_eq!(app.gateway_observation, GatewayObservation::Unchecked);
    app.composer.insert_text("check transport");
    app.submit(&gateway, &runtime, &tx);
    assert!(app.pending.is_some());
    app.receive(
        1,
        Err(ReplyError::Gateway(ModelGatewayError::StreamOpenTransport)),
    );
    assert_eq!(app.gateway_observation, GatewayObservation::Offline);
    assert!(
        app.transcript
            .last()
            .unwrap()
            .1
            .contains("remote outcome is unknown")
    );
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
    assert_eq!(app.gateway_observation, GatewayObservation::Online);
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
            crate::harness::edge_gateway::SharedGateway::new(
                crate::harness::journal_gateway::JournalGateway::new(ThinkingGateway, journal),
            )
        } else {
            crate::harness::edge_gateway::SharedGateway::new(ThinkingGateway)
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
                .block_on(crate::harness::replay::load_session(
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
    use crate::harness::replay::{ReplayOutcome, ReplayTurn};
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
    use crate::harness::replay::{ReplayOutcome, ReplayTurn};
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
    use crate::harness::replay::{ReplayOutcome, ReplayTurn};
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
    assert_eq!(app.transcript[1].1, "hel");
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
    use crate::harness::replay::{ReplayOutcome, ReplayTurn};
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
    assert_eq!(app.transcript[1].1, "hel");
    assert!(app.transcript.last().unwrap().1.contains("cancelled"));
}

#[test]
fn output_strips_terminal_controls() {
    assert_eq!(safe_text("a\x1b[2J\tb\n"), "a�[2J b\n");
}
