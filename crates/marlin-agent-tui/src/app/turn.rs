// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Turn submission, completion and cancellation.

use super::{
    App, Arc, Command, GatewayObservation, MAX_RESPONSE_BYTES, MAX_TURN_SEGMENTS,
    ModelGatewayError, ModelGatewayMessageRole, ModelGatewayRequest, ModelStreamEvent,
    ModelStreamInterruption, ModelStreamingGateway, OutputLimit, PendingTurn, Reply, ReplyError,
    ReplySender, SessionBrowser, SessionId, matches_observed_anthropic_thinking, parse_query,
    user_gateway_message,
};
use std::env;
use tokio_stream::StreamExt;

impl App {
    pub(crate) fn submit<G: ModelStreamingGateway + Clone>(
        &mut self,
        gateway: &G,
        runtime: &tokio::runtime::Runtime,
        tx: &ReplySender,
    ) {
        if self.pending.is_some()
            || (self.command_draft.is_none() && self.composer.text().trim().is_empty())
        {
            return;
        }
        if self
            .session_browser
            .as_ref()
            .is_some_and(SessionBrowser::is_opening)
        {
            self.transcript.push((
                "Sessions".into(),
                "Session switch in progress; draft retained.".into(),
            ));
            return;
        }
        if self.workspace.is_pending() {
            self.transcript.push((
                "Workspace".into(),
                "Workspace read in progress; input retained.".into(),
            ));
            return;
        }
        self.workspace.close_view();
        self.runtime_receipt_open = false;
        let command = if let Some(draft) = self.command_draft.take() {
            let input = self.composer.take();
            self.composer = draft;
            if !input.trim_start().starts_with('/') {
                self.transcript.push((
                    "Marlin".into(),
                    "Commands must start with /. Draft retained.".into(),
                ));
                return;
            }
            Some(input)
        } else if self.composer.text().trim_start().starts_with('/') {
            let command = self.composer.take();
            self.prompt_history.restore(&mut self.composer);
            Some(command)
        } else {
            None
        };
        if let Some(input) = command {
            match Command::parse(&input) {
                Some(Command::Help) => self.transcript.push(("Help".into(), "Enter: send · Tab / Shift-Tab: focus · Ctrl-N: newline · Ctrl-O: command without losing draft · Ctrl-Z: suspend · arrows: edit · Home/End or Ctrl-A/Ctrl-E: line start/end · Alt-P/Alt-N: prompt history · Ctrl-R: search prompts · PageUp/PageDown: scroll · Esc/Ctrl-C: cancel · Ctrl-C twice or Ctrl-Q: quit · /models: list · /model: select · /status: worktree · /diff: unstaged · /diff staged: staged · /sessions: list · /session: switch · /inspect: read-only history · /history older or newest: page · /context: budget · /runtime: stream-open policy".into())),
                Some(Command::Sessions(next)) => self.show_sessions(runtime, next),
                Some(Command::Models) => self.show_models(),
                Some(Command::Workspace(command)) => {
                    let message = match parse_query(command) {
                        Err(error) => error.to_owned(),
                        Ok(query) => match env::current_dir() {
                            Ok(path) => if self.workspace.start(runtime, path, query) { "Reading workspace…".to_owned() } else { "A workspace read is already running.".to_owned() },
                            Err(error) => format!("Cannot determine current directory: {error}"),
                        },
                    };
                    self.transcript.push(("Workspace".into(), message));
                }
                Some(Command::Model(model)) => self.select_model(model),
                Some(Command::History(older)) => self.request_history_page(runtime, older),
                Some(Command::Context) => {
                    let message = if self.history_position.is_some() {
                        "Context budget unavailable in read-only history; load a full session first.".to_owned()
                    } else if let Some(usage) = self.context_budget.measure(&self.history, self.composer.text()) {
                        format!("Completed turns: {} / {}. Next request accounting: {} / {} bytes, including a {}-byte draft. Budget: {}. Counts include message structures and text; they are not token counts or total process memory.", usage.completed_turns, self.context_budget.turns, usage.request_bytes, self.context_budget.bytes, self.composer.text().len(), if usage.allowed { "within limit" } else { "exceeded" })
                    } else {
                        "Context accounting overflowed; the request cannot be admitted.".to_owned()
                    };
                    self.transcript.push(("Context".into(), message));
                }
                Some(Command::Runtime) => {
                    if self.runtime_edge.is_some() {
                        self.runtime_receipt_open = true;
                    } else {
                        self.transcript.push(("Runtime".into(), "Runtime stream-open policy is unavailable.".into()));
                    }
                }
                Some(Command::Clear) => self.transcript.clear(),
                Some(Command::Session(id)) => self.request_session_switch(runtime, id),
                Some(Command::Inspect(id)) => {
                    match serde_json::from_str::<String>(id).ok().and_then(|value| SessionId::new(value).ok()) {
                        Some(id) => self.request_history(runtime, id, None),
                        None => self.transcript.push(("History".into(), "Use /inspect followed by a nonempty JSON-quoted session ID.".into())),
                    }
                }
                None => self.transcript.push(("Marlin".into(), "Unknown command or arguments. Use /help to see available commands.".into())),
            }
            return;
        }
        if self.history_position.is_some() {
            self.transcript.push((
                "History".into(),
                "Read-only history; load a full session before sending. Draft retained.".into(),
            ));
            return;
        }
        if self.needs_replay {
            self.transcript.push(("Error".into(), "Session state is uncertain. Reopen it to inspect the journal; unfinished turns require recovery.".into()));
            return;
        }
        if self.unverified_thinking {
            let message = if self.persistent {
                "Prior thinking cannot be verified for continuation. Select another session; draft retained."
            } else {
                "Prior thinking cannot be verified for continuation. Restart to begin a new session; draft retained."
            };
            self.transcript.push(("Error".into(), message.into()));
            return;
        }
        if self.has_incompatible_continuation() {
            self.transcript.push(("Error".into(), "A provider continuation belongs to a different model route. Select its original route or open another session; draft retained.".into()));
            return;
        }
        if self.history.iter().any(|message| {
            message
                .continuation
                .as_ref()
                .is_some_and(|state| state.destination.is_none())
        }) {
            self.transcript.push(("Error".into(), "Historical provider continuation has no destination binding. Open another session or reconcile its origin; draft retained.".into()));
            return;
        }
        if !self
            .context_budget
            .allows(&self.history, self.composer.text())
        {
            self.transcript.push(("Error".into(), "Full model context exceeds the TUI budget. Draft retained; Ctrl-O opens commands without losing it.".into()));
            return;
        }
        let input = self.composer.take();
        self.prompt_history.restore(&mut self.composer);
        self.transcript.push(("You".into(), input.clone()));
        let history = Arc::clone(&self.history);
        let endpoint = self.endpoint.clone();
        let prompt = input.clone();
        let gateway = gateway.clone();
        self.next_id += 1;
        let id = self.next_id;
        let tx = tx.clone();
        let handle = runtime.spawn(async move {
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
                let terminal = matches!(event, Ok(ModelStreamEvent::Completed { .. }) | Err(_));
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
                    self.transcript.push(("Assistant".into(), String::new()));
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
                    self.transcript.push(("Thinking".into(), String::new()));
                    pending.segments += 1;
                    let index = self.transcript.len() - 1;
                    pending.thinking = Some(index);
                    pending.thinking_segments.push(index);
                    index
                };
                self.transcript.append_content(index, &chunk.content);
            }
            Ok(ModelStreamEvent::Completed { response }) => match response
                .choices
                .into_iter()
                .find(|choice| choice.message.role == ModelGatewayMessageRole::Assistant)
            {
                Some(choice) => {
                    let pending = self.pending.as_ref().expect("matching pending turn");
                    let mut remainder = Some(choice.message.content.as_str());
                    for index in &pending.output_segments {
                        remainder = remainder
                            .and_then(|text| text.strip_prefix(self.transcript[*index].1.as_str()));
                    }
                    if !pending.output_segments.is_empty() && remainder != Some("") {
                        self.fail_turn("model stream completion does not match its chunks".into());
                        return;
                    }
                    let thinking_verified = !pending.had_thinking || {
                        let thinking: String = pending
                            .thinking_segments
                            .iter()
                            .map(|index| self.transcript[*index].1.as_str())
                            .collect();
                        matches_observed_anthropic_thinking(&choice.message, &thinking)
                    };
                    let pending = self.pending.take().expect("matching pending turn");
                    if pending.output_segments.is_empty() {
                        self.transcript
                            .push(("Assistant".into(), choice.message.content.clone()));
                    }
                    let history = Arc::make_mut(&mut self.history);
                    history.push(user_gateway_message(pending.prompt));
                    history.push(choice.message);
                    self.gateway_observation = GatewayObservation::Online;
                    if !thinking_verified {
                        self.unverified_thinking = true;
                        self.transcript.push(("Error".into(), "Thinking was observed but has not been verified against a provider continuation; further requests are blocked.".into()));
                    }
                }
                None => self.fail_turn("Gateway returned no assistant choice.".into()),
            },
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
                self.transcript.push((
                    "Error".into(),
                    format!("Request not dispatched: {message}. {explanation}"),
                ));
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
        self.transcript.push(("Error".into(), message));
    }

    pub(crate) fn cancel(&mut self) {
        self.command_selection = None;
        if let Some(draft) = self.command_draft.take() {
            self.composer = draft;
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
            self.transcript.push(("Marlin".into(), message.into()));
        } else if self
            .session_browser
            .as_mut()
            .is_some_and(SessionBrowser::cancel_open)
        {
            self.transcript.push((
                "Sessions".into(),
                "Session switch cancelled; current session retained.".into(),
            ));
        } else if self.workspace.cancel() {
            self.transcript
                .push(("Workspace".into(), "Workspace read cancelled.".into()));
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
        if let Some(view) = self.workspace.view_mut() {
            view.scroll_from_top = if up {
                view.scroll_from_top.saturating_sub(10)
            } else {
                view.scroll_from_top.saturating_add(10)
            };
        } else {
            self.scroll_up = if up {
                self.scroll_up.saturating_add(10)
            } else {
                self.scroll_up.saturating_sub(10)
            };
        }
    }

    pub(super) fn set_effective_scroll(&mut self, value: usize) {
        match self.workspace.view_mut() {
            Some(view) => view.scroll_from_top = value,
            None => self.scroll_up = value,
        }
    }
}
