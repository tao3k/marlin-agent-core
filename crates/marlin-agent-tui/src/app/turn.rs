// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Turn submission, completion and cancellation.

use super::{
    App, Arc, Command, GatewayObservation, MAX_RESPONSE_BYTES, MAX_TURN_SEGMENTS, MessageEvent,
    MessageSeverity, MessageSource, ModelGatewayError, ModelGatewayMessageRole,
    ModelGatewayRequest, ModelStreamEvent, ModelStreamInterruption, ModelStreamingGateway,
    OutputLimit, PendingTurn, Reply, ReplyError, ReplySender, SessionBrowser, SessionId, UiContent,
    UiEvent, matches_observed_anthropic_thinking, parse_query, user_gateway_message,
};
use std::env;
use tokio_stream::StreamExt;

impl App {
    fn runtime_session_id(&self) -> marlin_agent_runtime::SessionId {
        self.current_session.as_ref().map_or_else(
            || self.runtime_owner.session().session_id().clone(),
            |session| marlin_agent_runtime::SessionId::new(session.as_str()),
        )
    }

    pub(crate) async fn shutdown(&mut self) -> std::io::Result<()> {
        self.runtime_owner.task_tracker().close();
        if let Some(pending) = self.pending.take() {
            pending.task.abort();
            let _ = pending.task.join().await;
        }
        if let Some(browser) = self.session_browser.as_mut() {
            browser.shutdown().await;
        }
        self.workspace.shutdown().await;
        let receipt = self
            .runtime_owner
            .shutdown_tasks(
                &marlin_agent_runtime::RuntimeTaskTrackerPolicy::cancel_on_shutdown(None),
            )
            .await;
        if receipt.status() != &marlin_agent_runtime::RuntimeTaskShutdownStatus::Completed {
            return Err(std::io::Error::other(format!(
                "runtime cleanup incomplete: {receipt:?}"
            )));
        }
        Ok(())
    }

    pub(crate) fn submit<G: ModelStreamingGateway + Clone>(
        &mut self,
        gateway: &G,
        runtime: &tokio::runtime::Runtime,
        tx: &ReplySender,
    ) {
        if self.command_draft.is_none() && self.composer.text().trim().is_empty() {
            return;
        }
        if self.submit_process_command() {
            return;
        }
        if self.pending.is_some() {
            let Some(_) = self.command_draft.as_ref() else {
                return;
            };
            let input = match &self.command_menu {
                Some(menu) => format!("{}{}", menu.prefix, self.composer.text()),
                None => self.composer.text().to_owned(),
            };
            if !matches!(
                self.command_descriptor(&input)
                    .and_then(|descriptor| Command::parse(descriptor, &input)),
                Some(Command::ToggleThinking(_))
            ) {
                self.publish_message(MessageEvent {
                    source: MessageSource::Ui,
                    severity: MessageSeverity::Warning,
                    summary: "Command unavailable while streaming",
                    detail: "The current model turn must finish before this command can run.",
                });
                return;
            }
        }
        if self
            .session_browser
            .as_ref()
            .is_some_and(SessionBrowser::is_opening)
        {
            self.publish_message(MessageEvent {
                source: MessageSource::Ui,
                severity: MessageSeverity::Warning,
                summary: "Session switch in progress",
                detail: "Draft retained.",
            });
            return;
        }
        if self.workspace.is_pending() {
            self.publish_message(MessageEvent {
                source: MessageSource::Workspace,
                severity: MessageSeverity::Warning,
                summary: "Workspace read in progress",
                detail: "Input retained.",
            });
            return;
        }
        self.workspace.close_view();
        self.runtime_receipt_open = false;
        let command = if let Some(draft) = self.command_draft.take() {
            let input = self.composer.take();
            let input = match self.command_menu.take() {
                Some(menu) => format!("{}{}", menu.prefix, input),
                None => input,
            };
            self.composer = draft;
            Some(input)
        } else {
            None
        };
        if let Some(input) = command {
            let descriptor = self.command_descriptor(&input).cloned();
            match descriptor
                .as_ref()
                .and_then(|descriptor| Command::parse(descriptor, &input))
            {
                Some(Command::Sessions(next)) => self.show_sessions(runtime, next),
                Some(Command::Models) => self.show_models(),
                Some(Command::Workspace { mode, arguments }) => {
                    let query_text = match arguments {
                        Some(arguments) => format!("{mode} {arguments}"),
                        None => mode.to_owned(),
                    };
                    let (severity, message) = match parse_query(&query_text) {
                        Err(error) => (MessageSeverity::Warning, error.to_owned()),
                        Ok(query) => match env::current_dir() {
                            Ok(path) => {
                                let session = self.runtime_session_id();
                                if self.workspace.start(runtime, &session, path, query) {
                                    (MessageSeverity::Info, "Reading workspace…".to_owned())
                                } else {
                                    (
                                        MessageSeverity::Warning,
                                        "A workspace read is already running.".to_owned(),
                                    )
                                }
                            }
                            Err(error) => (
                                MessageSeverity::Error,
                                format!("Cannot determine current directory: {error}"),
                            ),
                        },
                    };
                    self.publish_message(MessageEvent {
                        source: MessageSource::Workspace,
                        severity,
                        summary: "Workspace command",
                        detail: &message,
                    });
                }
                Some(Command::Model(model)) => self.select_model(model),
                Some(Command::History(older)) => self.request_history_page(runtime, older),
                Some(Command::ContextBudget) => {
                    if self.history_position.is_some() {
                        self.publish_message(MessageEvent {
                            source: MessageSource::Ui,
                            severity: MessageSeverity::Warning,
                            summary: "Context budget unavailable",
                            detail: "Context budget unavailable in read-only history; load a full session first.",
                        });
                    } else if let Some(usage) = self
                        .context_budget
                        .measure(&self.history, self.composer.text())
                    {
                        let message = format!(
                            "Completed turns: {} / {}. Next request accounting: {} / {} bytes, including a {}-byte draft. Budget: {}. Counts include message structures and text; they are not token counts or total process memory.",
                            usage.completed_turns,
                            self.context_budget.turns,
                            usage.request_bytes,
                            self.context_budget.bytes,
                            self.composer.text().len(),
                            if usage.allowed {
                                "within limit"
                            } else {
                                "exceeded"
                            }
                        );
                        self.transcript.push(("Context".into(), message));
                    } else {
                        self.publish_message(MessageEvent {
                            source: MessageSource::Ui,
                            severity: MessageSeverity::Error,
                            summary: "Context accounting overflow",
                            detail: "Context accounting overflowed; the request cannot be admitted.",
                        });
                    }
                }
                Some(Command::Runtime) => {
                    if self.runtime_edge.is_some() {
                        self.runtime_receipt_open = true;
                    } else {
                        self.publish_message(MessageEvent {
                            source: MessageSource::Ui,
                            severity: MessageSeverity::Warning,
                            summary: "Runtime policy unavailable",
                            detail: "Runtime stream-open policy is unavailable.",
                        });
                    }
                }
                Some(Command::SelectUi(event)) => self.ui_request = Some(event),
                Some(Command::EnterTransient(id)) => self.transient_request = Some(id),
                Some(Command::KillProcess(_)) => {
                    unreachable!("process command handled at admission")
                }
                Some(Command::OpenMenu(prefix)) => {
                    self.command_draft = Some(std::mem::take(&mut self.composer));
                    self.command_menu = Some(super::state::CommandMenu {
                        prefix: prefix.to_owned(),
                        prompt: descriptor
                            .as_ref()
                            .expect("admitted menu descriptor")
                            .description
                            .clone(),
                    });
                    self.command_selection = None;
                }
                Some(Command::Clear) => self.transcript.clear(),
                Some(Command::ToggleThinking(index)) => {
                    if !self.transcript.toggle_thinking_from_end(index) {
                        self.publish_message(MessageEvent {
                            source: MessageSource::Ui,
                            severity: MessageSeverity::Info,
                            summary: "No thinking segment",
                            detail: "There is no thinking segment in the displayed transcript.",
                        });
                    }
                }
                Some(Command::Session(id)) => self.request_session_switch(runtime, id),
                Some(Command::Inspect(id)) => {
                    match serde_json::from_str::<String>(id).ok().and_then(|value| SessionId::new(value).ok()) {
                        Some(id) => self.request_history(runtime, id, None),
                        None => self.publish_message(MessageEvent {
                            source: MessageSource::Input,
                            severity: MessageSeverity::Warning,
                            summary: "Invalid session ID",
                            detail: "Use M-x inspect followed by a nonempty JSON-quoted session ID.",
                        }),
                    }
                }
                None => self.publish_message(MessageEvent {
                    source: MessageSource::Input,
                    severity: MessageSeverity::Warning,
                    summary: "Unknown command or arguments",
                    detail: "Use M-x to see available commands.",
                }),
            }
            return;
        }
        if self.history_position.is_some() {
            self.publish_message(MessageEvent {
                source: MessageSource::Input,
                severity: MessageSeverity::Warning,
                summary: "Read-only history",
                detail: "Load a full session before sending. Draft retained.",
            });
            return;
        }
        if self.buffer_view != super::BufferView::Conversation {
            self.ui_request = Some(UiEvent::SelectBuffer(
                marlin_agent_ui::UiBufferId::new("conversation").expect("static buffer ID"),
            ));
        }
        if self.needs_replay {
            self.publish_message(MessageEvent {
                source: MessageSource::Gateway,
                severity: MessageSeverity::Error,
                summary: "Session state is uncertain",
                detail: "Reopen it to inspect the journal; unfinished turns require recovery.",
            });
            return;
        }
        if self.pending_tool_request {
            self.publish_message(MessageEvent {
                source: MessageSource::Gateway,
                severity: MessageSeverity::Warning,
                summary: "Tool review pending",
                detail: "The pending model tool request must be reviewed before another turn. Draft retained.",
            });
            return;
        }
        if self.unverified_thinking {
            let message = if self.persistent {
                "Prior thinking cannot be verified for continuation. Select another session; draft retained."
            } else {
                "Prior thinking cannot be verified for continuation. Restart to begin a new session; draft retained."
            };
            self.publish_message(MessageEvent {
                source: MessageSource::Gateway,
                severity: MessageSeverity::Error,
                summary: "Continuation unavailable",
                detail: message,
            });
            return;
        }
        if self.has_incompatible_continuation() {
            self.publish_message(MessageEvent {
                source: MessageSource::Gateway,
                severity: MessageSeverity::Error,
                summary: "Incompatible continuation",
                detail: "A provider continuation belongs to a different model route. Select its original route or open another session; draft retained.",
            });
            return;
        }
        if self.history.iter().any(|message| {
            message
                .continuation
                .as_ref()
                .is_some_and(|state| state.destination.is_none())
        }) {
            self.publish_message(MessageEvent {
                source: MessageSource::Gateway,
                severity: MessageSeverity::Error,
                summary: "Unbound continuation",
                detail: "Historical provider continuation has no destination binding. Open another session or reconcile its origin; draft retained.",
            });
            return;
        }
        if !self
            .context_budget
            .allows(&self.history, self.composer.text())
        {
            self.publish_message(MessageEvent {
                source: MessageSource::Input,
                severity: MessageSeverity::Warning,
                summary: "Context budget exceeded",
                detail: "Full model context exceeds the TUI budget. Draft retained; M-x opens commands without losing it.",
            });
            return;
        }
        let input = self.composer.take();
        self.prompt_history.restore(&mut self.composer);
        self.push_prompt(input.clone());
        let history = Arc::clone(&self.history);
        let endpoint = self.endpoint.clone();
        let prompt = input.clone();
        let gateway = gateway.clone();
        self.next_id += 1;
        let id = self.next_id;
        let tx = tx.clone();
        let handle = self.runtime_owner.spawn_in_session_on(runtime, &self.runtime_session_id(), marlin_agent_runtime::RuntimeOperationId::new("turn.stream"), async move {
            let mut messages = history.as_ref().clone();
            drop(history);
            messages.push(user_gateway_message(prompt));
            let request = ModelGatewayRequest::new(endpoint, messages);
            tokio::task::yield_now().await;
            let future = gateway.stream(request);
            let mut output_limit = OutputLimit::new(MAX_RESPONSE_BYTES);
            let mut stream = match future.await {
                Ok(stream) => stream,
                Err(error) => {
                    let _ = tx.send(id, Err(error)).await;
                    return;
                }
            };
            while let Some(event) = stream.next().await {
                // Once a stream has opened, an item cannot establish that no
                // request was dispatched, even if a gateway mislabels it.
                let event = event.map_err(|error| match error {
                    ModelGatewayError::RequestNotDispatched(_) => ModelGatewayError::StreamInterrupted(
                        marlin_agent_protocol::ModelStreamInterruption::UnclassifiedGatewayFailure,
                    ),
                    other => other,
                }).and_then(|event| {
                    output_limit.check(&event)?;
                    Ok(event)
                });
                let terminal = matches!(event, Ok(ModelStreamEvent::Completed { .. } | ModelStreamEvent::ToolRequested { .. }) | Err(_));
                if !matches!(tx.send(id, event).await, Ok(true)) {
                    return;
                }
                if terminal {
                    return;
                }
            }
            let _ = tx
                .send(
                    id,
                    Err(ModelGatewayError::StreamInterrupted(
                        marlin_agent_protocol::ModelStreamInterruption::EndedWithoutTerminal,
                    )),
                )
                .await;
        });
        self.pending = Some(PendingTurn {
            id,
            task: handle,
            prompt: input,
            output: None,
            output_segments: Vec::new(),
            thinking: None,
            thinking_segments: Vec::new(),
            had_thinking: false,
            observed_stream: false,
            segments: 0,
            next_sequence: 1,
            next_thinking_sequence: 1,
        });
        self.scroll_up = 0;
    }

    pub(crate) fn receive(&mut self, id: u64, result: Reply) {
        if self.pending.as_ref().map(|pending| pending.id) != Some(id) {
            return;
        }
        match result {
            Ok(ModelStreamEvent::Started { .. }) => {
                self.pending
                    .as_mut()
                    .expect("matching pending turn")
                    .observed_stream = true;
            }
            Ok(ModelStreamEvent::Chunk(chunk)) => {
                let pending = self.pending.as_mut().expect("matching pending turn");
                pending.observed_stream = true;
                if chunk.sequence != pending.next_sequence {
                    self.fail_turn("model stream chunk sequence is invalid".into());
                    return;
                }
                let Some(next) = pending.next_sequence.checked_add(1) else {
                    self.fail_turn("model stream chunk sequence exhausted".into());
                    return;
                };
                pending.next_sequence = next;
                pending.thinking = None;
                let index = if let Some(index) = pending.output {
                    index
                } else {
                    if pending.segments >= MAX_TURN_SEGMENTS {
                        self.fail_turn("model output has too many alternating segments".into());
                        return;
                    }
                    let role = self.transcript_policy.as_ref().map_or_else(
                        || "Assistant".to_owned(),
                        |policy| policy.assistant_role.clone(),
                    );
                    self.transcript.push((role, String::new()));
                    let index = self.transcript.len() - 1;
                    pending.segments += 1;
                    pending.output = Some(index);
                    pending.output_segments.push(index);
                    index
                };
                self.transcript.append_content(index, &chunk.content);
            }
            Ok(ModelStreamEvent::Thinking(chunk)) => {
                let pending = self.pending.as_mut().expect("matching pending turn");
                pending.observed_stream = true;
                if chunk.sequence != pending.next_thinking_sequence || chunk.content.is_empty() {
                    self.fail_turn("model thinking sequence is invalid".into());
                    return;
                }
                let Some(next) = pending.next_thinking_sequence.checked_add(1) else {
                    self.fail_turn("model thinking sequence exhausted".into());
                    return;
                };
                pending.next_thinking_sequence = next;
                pending.had_thinking = true;
                pending.output = None;
                if pending.thinking.is_none() && pending.segments >= MAX_TURN_SEGMENTS {
                    self.fail_turn("model output has too many alternating segments".into());
                    return;
                }
                let index = if let Some(index) = pending.thinking {
                    index
                } else {
                    if let Some(policy) = &self.transcript_policy {
                        self.transcript.push_thinking(
                            policy.thinking_expanded_role.clone(),
                            &policy.thinking_collapsed_role,
                            String::new(),
                            policy.default_collapsed,
                        );
                    } else {
                        self.transcript.push(("Thinking".into(), String::new()));
                    }
                    pending.segments += 1;
                    let index = self.transcript.len() - 1;
                    pending.thinking = Some(index);
                    pending.thinking_segments.push(index);
                    index
                };
                self.transcript.append_content(index, &chunk.content);
            }
            Ok(
                event @ (ModelStreamEvent::Completed { .. }
                | ModelStreamEvent::ToolRequested { .. }),
            ) => {
                let tool_requested = matches!(&event, ModelStreamEvent::ToolRequested { .. });
                let response = match event {
                    ModelStreamEvent::Completed { response }
                    | ModelStreamEvent::ToolRequested { response } => response,
                    _ => unreachable!("terminal event pattern"),
                };
                match response
                    .choices
                    .into_iter()
                    .find(|choice| choice.message.role == ModelGatewayMessageRole::Assistant)
                {
                    Some(choice) => {
                        let pending = self.pending.as_ref().expect("matching pending turn");
                        let mut remainder = Some(choice.message.content.as_str());
                        for index in &pending.output_segments {
                            remainder = remainder.and_then(|text| {
                                text.strip_prefix(self.transcript[*index].1.as_str())
                            });
                        }
                        if !pending.output_segments.is_empty() && remainder != Some("") {
                            self.fail_turn(
                                "model stream completion does not match its chunks".into(),
                            );
                            return;
                        }
                        let thinking_verified = !pending.had_thinking || {
                            let thinking: String = pending
                                .thinking_segments
                                .iter()
                                .map(|index| self.transcript.content(*index))
                                .collect();
                            matches_observed_anthropic_thinking(&choice.message, &thinking)
                        };
                        let pending = self.pending.take().expect("matching pending turn");
                        if pending.output_segments.is_empty()
                            && (!tool_requested || !choice.message.content.is_empty())
                        {
                            self.push_answer("Assistant", choice.message.content.clone());
                        }
                        if tool_requested {
                            self.pending_tool_request = true;
                            self.publish_message(MessageEvent {
                            source: MessageSource::Gateway,
                            severity: MessageSeverity::Warning,
                            summary: "Tool review pending",
                            detail: &format!(
                                "The model requested {} tool(s). Execution and further turns await authorization.",
                                choice.message.tool_calls.len(),
                            ),
                        });
                        } else {
                            let history = Arc::make_mut(&mut self.history);
                            history.push(user_gateway_message(pending.prompt));
                            history.push(choice.message);
                        }
                        self.gateway_observation = GatewayObservation::Online;
                        if !thinking_verified {
                            self.unverified_thinking = true;
                            self.publish_message(MessageEvent {
                            source: MessageSource::Gateway,
                            severity: MessageSeverity::Error,
                            summary: "Thinking unverified",
                            detail: "Thinking was observed but has not been verified against a provider continuation; further requests are blocked.",
                        });
                        }
                    }
                    None => self.fail_turn("Gateway returned no assistant choice.".into()),
                }
            }
            Err(ReplyError::Gateway(ModelGatewayError::RequestNotDispatched(message)))
                if self
                    .pending
                    .as_ref()
                    .is_some_and(|pending| !pending.observed_stream) =>
            {
                let pending = self.pending.take().expect("matching pending turn");
                pending.task.abort();
                let explanation = if self.composer.text().is_empty() {
                    self.composer.insert_text(&pending.prompt);
                    "Prompt restored to the composer."
                } else {
                    "Current draft retained; the rejected prompt remains visible above."
                };
                self.publish_message(MessageEvent {
                    source: MessageSource::Gateway,
                    severity: MessageSeverity::Error,
                    summary: "Request not dispatched",
                    detail: &format!("Request not dispatched: {message}. {explanation}"),
                });
            }
            Err(error) => {
                if let ReplyError::Gateway(source) = &error {
                    self.gateway_observation = match source {
                        ModelGatewayError::StreamOpenTransport
                        | ModelGatewayError::StreamInterrupted(
                            ModelStreamInterruption::Transport,
                        ) => GatewayObservation::Offline,
                        ModelGatewayError::RequestNotDispatched(_) => self.gateway_observation,
                        _ => GatewayObservation::Unknown,
                    };
                }
                self.fail_turn(error.to_string());
            }
        }
    }

    fn fail_turn(&mut self, message: String) {
        if let Some(pending) = self.pending.take() {
            self.unverified_thinking |= pending.had_thinking;
            pending.task.abort();
        }
        self.needs_replay |= self.persistent;
        self.publish_message(MessageEvent {
            source: MessageSource::Gateway,
            severity: MessageSeverity::Error,
            summary: "Model request failed",
            detail: &message,
        });
    }

    pub(crate) fn cancel(&mut self) {
        self.command_selection = None;
        if let Some(draft) = self.command_draft.take() {
            self.command_menu = None;
            self.composer = draft;
            return;
        }
        if self.buffer_view != super::BufferView::Conversation {
            self.ui_request = Some(UiEvent::SelectBuffer(
                marlin_agent_ui::UiBufferId::new("conversation").expect("static buffer ID"),
            ));
            return;
        }
        if let Some(pending) = self.pending.take() {
            self.unverified_thinking |= pending.had_thinking;
            pending.task.abort();
            self.needs_replay |= self.persistent;
            let message = if self.persistent {
                "Request cancelled locally. Reopen the session to inspect its journal; an unfinished turn cannot be retried automatically."
            } else if pending.had_thinking {
                "Request cancelled locally. Observed thinking is not verified for continuation; further requests are blocked."
            } else {
                "Request cancelled locally."
            };
            self.publish_message(MessageEvent {
                source: MessageSource::Gateway,
                severity: MessageSeverity::Warning,
                summary: "Request cancelled locally",
                detail: message,
            });
        } else if self
            .session_browser
            .as_mut()
            .is_some_and(SessionBrowser::cancel_open)
        {
            self.publish_message(MessageEvent {
                source: MessageSource::Ui,
                severity: MessageSeverity::Info,
                summary: "Session switch cancelled",
                detail: "Current session retained.",
            });
        } else if self.workspace.cancel() {
            self.publish_message(MessageEvent {
                source: MessageSource::Workspace,
                severity: MessageSeverity::Info,
                summary: "Workspace read cancelled",
                detail: "The pending read was cancelled.",
            });
        } else {
            self.workspace.close_view();
            self.runtime_receipt_open = false;
        }
    }

    /// Ctrl-C cancels active work or command entry; when idle it requires a
    /// second consecutive press to exit, preserving the draft on the first.
    pub(crate) fn interrupt(&mut self) -> bool {
        if self.command_draft.is_some()
            || self.pending.is_some()
            || self.workspace.is_pending()
            || self.workspace.view().is_some()
            || self.runtime_receipt_open
            || self.buffer_view != super::BufferView::Conversation
            || self
                .session_browser
                .as_ref()
                .is_some_and(SessionBrowser::is_opening)
        {
            self.quit_armed = false;
            self.cancel();
            return false;
        }
        if self.quit_armed {
            return true;
        }
        self.quit_armed = true;
        false
    }

    pub(crate) fn disarm_exit(&mut self) {
        self.quit_armed = false;
    }

    pub(crate) fn scroll(&mut self, up: bool) {
        self.scroll_lines(up, 10);
    }

    pub(crate) fn scroll_lines(&mut self, up: bool, lines: usize) {
        if matches!(self.buffer_view, super::BufferView::Named(_)) {
            self.static_buffer_scroll = if up {
                self.static_buffer_scroll.saturating_sub(lines)
            } else {
                self.static_buffer_scroll.saturating_add(lines)
            };
            return;
        }
        if self.buffer_view == super::BufferView::Messages {
            self.message_scroll_up = if up {
                self.message_scroll_up.saturating_add(lines)
            } else {
                self.message_scroll_up.saturating_sub(lines)
            };
        } else if let Some(view) = self.workspace.view_mut() {
            view.scroll_from_top = if up {
                view.scroll_from_top.saturating_sub(lines)
            } else {
                view.scroll_from_top.saturating_add(lines)
            };
        } else {
            self.scroll_up = if up {
                self.scroll_up.saturating_add(lines)
            } else {
                self.scroll_up.saturating_sub(lines)
            };
        }
    }

    pub(super) fn set_effective_scroll(&mut self, value: usize) {
        if matches!(self.buffer_view, super::BufferView::Named(_)) {
            return;
        }
        if self.buffer_view == super::BufferView::Messages {
            self.message_scroll_up = value;
            return;
        }
        match self.workspace.view_mut() {
            Some(view) => view.scroll_from_top = value,
            None => self.scroll_up = value,
        }
    }
}

impl App {
    fn submit_process_command(&mut self) -> bool {
        let Some(_) = self.command_draft else {
            return false;
        };
        let input = match &self.command_menu {
            Some(menu) => format!("{}{}", menu.prefix, self.composer.text()),
            None => self.composer.text().to_owned(),
        };
        let command = self
            .command_descriptor(&input)
            .and_then(|descriptor| Command::parse(descriptor, &input));
        match command {
            Some(Command::SelectUi(UiEvent::SelectBuffer(buffer)))
                if buffer.as_str() == "processes" =>
            {
                self.ui_request = Some(UiEvent::SelectBuffer(buffer));
                self.refresh_process_content();
            }
            Some(Command::KillProcess(generation)) => self.stop_process(generation),
            _ => return false,
        }
        self.composer = self
            .command_draft
            .take()
            .expect("process command has a saved draft");
        self.command_menu = None;
        true
    }

    fn stop_process(&mut self, generation: std::num::NonZeroU64) {
        let session = self.runtime_session_id();
        let stopped = self
            .runtime_owner
            .process_registry()
            .stop_session_generation(&session, generation);
        let (severity, summary, detail) = if stopped {
            (
                MessageSeverity::Info,
                "Process stop requested",
                "Runtime is reaping the selected process.",
            )
        } else {
            (
                MessageSeverity::Warning,
                "Process unavailable",
                "Use a live generation from this session's Processes buffer.",
            )
        };
        self.publish_message(MessageEvent {
            source: MessageSource::Runtime,
            severity,
            summary,
            detail,
        });
    }

    pub(crate) fn refresh_process_content(&mut self) -> bool {
        let session = self.runtime_session_id();
        let registry = self.runtime_owner.process_registry();
        let revision = registry.managed_revision();
        if self.process_snapshot.as_ref() == Some(&(session.clone(), revision)) {
            return false;
        }
        let rows = registry.managed_processes(&session);
        let mut content = Vec::with_capacity(rows.len() + 1);
        content.push(UiContent::Text(if rows.is_empty() {
            "No managed processes in this session.".into()
        } else {
            "Generation   State   PID   Operation".into()
        }));
        for row in rows {
            let id = row.resource_id().expect("managed process row");
            let operation: String = id
                .operation()
                .as_str()
                .chars()
                .take(128)
                .map(|ch| if ch.is_control() { '�' } else { ch })
                .collect();
            use marlin_agent_runtime::process::RuntimeProcessStatus;
            let state = match row.status {
                RuntimeProcessStatus::Running => "Running",
                RuntimeProcessStatus::Finished => "Exited",
                RuntimeProcessStatus::Failed => "Exited (failed)",
                RuntimeProcessStatus::Orphaned => "Orphaned",
                RuntimeProcessStatus::CleanupRequested => "Cleaning up",
            };
            content.push(UiContent::Text(format!(
                "{}   {}   {}   {}",
                id.generation().get(),
                state,
                row.pid,
                operation
            )));
        }
        let id = marlin_agent_ui::UiBufferId::new("processes").expect("static Buffer identity");
        match self.replace_dynamic_buffer_content(id, content) {
            Ok(changed) => {
                self.process_snapshot = Some((session, revision));
                changed
            }
            Err(detail) => {
                self.process_snapshot = Some((session, revision));
                self.publish_message(MessageEvent {
                    source: MessageSource::Runtime,
                    severity: MessageSeverity::Error,
                    summary: "Process Buffer update rejected",
                    detail,
                });
                false
            }
        }
    }
}
