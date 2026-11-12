// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! A terminal client for Marlin's typed model gateway.

use std::{
    env,
    io::{self, IsTerminal, Write},
    path::PathBuf,
    sync::{Arc, mpsc},
    thread,
    time::Instant,
};

use super::{
    cli::{CliAction, list_sessions, parse_cli},
    command::Command,
    composer::Composer,
    config,
    context_budget::ContextBudget,
    edge_gateway::{RuntimeStreamGateway, SharedGateway},
    event_queue::{QueuedReply, Reply, ReplyError, ReplySender, reply_channel},
    frame_schedule::FrameSchedule,
    input::{BracketedPaste, InputDecoder, TerminalInput},
    journal_gateway::JournalGateway,
    model_catalog::ModelCatalog,
    output_limit::{MAX_RESPONSE_BYTES, OutputLimit},
    prompt_history::{HistorySearch, PromptHistory},
    render::draw,
    replay::{ReplayOutcome, ReplayTurn, load_session},
    session_browser::{OpenedView, SessionBrowser, SessionPageRequestError},
    signals::SignalMonitor,
    ui_host::{UiHost, UiKeyAction},
    viewport::{ScrollAnchor, Transcript, TranscriptViewportCache},
    workspace::{WorkspaceStatus, parse_query},
};
use marlin_agent_protocol::{
    ModelAlias, ModelEndpoint, ModelGatewayError, ModelGatewayMessage, ModelGatewayMessageRole,
    ModelGatewayRequest, user_gateway_message,
};
use marlin_agent_runtime::{RuntimeEdgePolicy, RuntimeEdgePolicyReceipt};
use marlin_agent_sessions::{ConversationCheckpointChannel, ConversationTurnId};
use marlin_agent_storage::{
    AgentId, ProjectId, SessionId, TursoAgentStorage, TursoAgentStorageConfig,
    TursoBatchTransactionMode, TursoOptimizationProfile,
};
use marlin_agent_stream::{
    ModelStreamEvent, ModelStreamingGateway, matches_observed_anthropic_thinking,
};
use marlin_agent_ui::UiOpenFacts;
use ratatui::{Terminal, backend::TermionBackend};
use termion::{
    event::Key, input::TermReadEventsAndRaw, raw::IntoRawMode, screen::IntoAlternateScreen,
};
use tokio_stream::StreamExt;

const REPLY_BATCH_LIMIT: usize = 16;
const REPLY_QUEUE_ITEMS: usize = 64;
pub(crate) const REPLY_QUEUE_BYTES: u32 = super::output_limit::MAX_COMPLETION_HEAP_BYTES as u32;
const INPUT_QUEUE_ITEMS: usize = 8;
const MAX_TURN_SEGMENTS: usize = marlin_agent_sessions::CONVERSATION_MAX_CHECKPOINT_SEGMENTS;

pub(crate) struct PendingTurn {
    id: u64,
    task: tokio::task::JoinHandle<()>,
    prompt: String,
    output: Option<usize>,
    output_segments: Vec<usize>,
    thinking: Option<usize>,
    thinking_segments: Vec<usize>,
    had_thinking: bool,
    observed_stream: bool,
    segments: usize,
    next_sequence: u64,
    next_thinking_sequence: u64,
}

struct HistoryPosition {
    oldest: Option<ConversationTurnId>,
    has_earlier: bool,
    has_newer: bool,
}

pub(crate) struct App {
    pub(crate) composer: Composer,
    prompt_history: PromptHistory,
    pub(crate) history_search: Option<HistorySearch>,
    pub(crate) command_draft: Option<Composer>,
    pub(crate) quit_armed: bool,
    pub(crate) transcript: Transcript,
    pub(crate) history: Arc<Vec<ModelGatewayMessage>>,
    pub(crate) context_budget: ContextBudget,
    endpoint: ModelEndpoint,
    models: Option<ModelCatalog>,
    pub(crate) pending: Option<PendingTurn>,
    next_id: u64,
    pub(crate) scroll_up: usize,
    persistent: bool,
    needs_replay: bool,
    unverified_thinking: bool,
    runtime_edge: Option<RuntimeEdgePolicyReceipt>,
    session_browser: Option<SessionBrowser<TursoAgentStorage>>,
    current_session: Option<SessionId>,
    history_position: Option<HistoryPosition>,
    pub(crate) workspace: WorkspaceStatus,
}

impl App {
    pub(crate) fn new(endpoint: ModelEndpoint) -> Self {
        Self { composer: Composer::default(), prompt_history: PromptHistory::default(), history_search: None, command_draft: None, quit_armed: false, transcript: Transcript::new(vec![("Marlin".into(), "Model gateway configured. Tools and approvals are not yet available. /help for keys.".into())]), history: Arc::new(Vec::new()), context_budget: ContextBudget::default(), endpoint, models: None, pending: None, next_id: 0, scroll_up: 0, persistent: false, needs_replay: false, unverified_thinking: false, runtime_edge: None, session_browser: None, current_session: None, history_position: None, workspace: WorkspaceStatus::default() }
    }

    pub(crate) fn endpoint(&self) -> &ModelEndpoint {
        &self.endpoint
    }

    pub(crate) fn session_id(&self) -> Option<&SessionId> {
        self.current_session.as_ref()
    }

    pub(crate) fn activity_label(&self) -> &'static str {
        if self.needs_replay {
            "Recovery required"
        } else if self.unverified_thinking {
            "Continuation unavailable"
        } else if self.history_position.is_some() {
            "History read-only"
        } else if self.pending.is_some() {
            "Working"
        } else if self.workspace.is_pending() {
            "Reading workspace"
        } else {
            "Idle"
        }
    }

    pub(crate) fn set_runtime_edge(&mut self, receipt: RuntimeEdgePolicyReceipt) {
        self.runtime_edge = Some(receipt);
    }

    pub(crate) fn navigate_prompts(&mut self, previous: bool) {
        if self.pending.is_some() || self.command_draft.is_some() || self.history_position.is_some()
        {
            return;
        }
        if previous {
            self.prompt_history
                .previous(&mut self.composer, &self.history);
        } else {
            self.prompt_history.next(&mut self.composer, &self.history);
        }
    }

    pub(crate) fn start_history_search(&mut self) {
        if self.pending.is_some()
            || self.command_draft.is_some()
            || self.history_position.is_some()
            || self
                .session_browser
                .as_ref()
                .is_some_and(SessionBrowser::is_opening)
        {
            return;
        }
        let mut search = HistorySearch::default();
        search.previous(&self.history);
        self.history_search = Some(search);
    }

    pub(crate) fn accept_history_search(&mut self) {
        if self
            .history_search
            .as_ref()
            .is_some_and(HistorySearch::is_searching)
        {
            return;
        }
        if let Some(preview) = self.history_search.take().and_then(HistorySearch::accept) {
            self.prompt_history.discard();
            self.composer = preview;
        }
    }

    pub(crate) fn set_models(&mut self, models: ModelCatalog) {
        self.models = Some(models);
    }

    fn show_models(&mut self) {
        let Some(models) = &self.models else {
            self.transcript
                .push(("Models".into(), "Model catalog is unavailable.".into()));
            return;
        };
        let mut text = String::new();
        for (alias, endpoint, available) in models.entries() {
            let marker = if endpoint == &self.endpoint { "*" } else { " " };
            let status = if available {
                "available"
            } else {
                "credential unavailable"
            };
            text.push_str(&format!(
                "{marker} {}: {}/{} ({status})\n",
                alias.as_str(),
                endpoint.provider.as_str(),
                endpoint.model.as_str()
            ));
        }
        text.push_str("Use /model <alias> to select. Credentials are read at startup.");
        self.transcript.push(("Models".into(), text));
    }

    fn select_model(&mut self, alias: &str) {
        let result = self
            .models
            .as_ref()
            .ok_or("model catalog is unavailable")
            .and_then(|models| models.select(&ModelAlias::new(alias)));
        match result {
            Ok(endpoint) => {
                self.endpoint = endpoint;
                let continuation_status = if self.has_incompatible_continuation() {
                    "Current session has continuation from another route; sending is blocked until you restore that route or switch sessions."
                } else {
                    "Next request includes current conversation."
                };
                self.transcript.push((
                    "Models".into(),
                    format!(
                        "Selected {alias}: {}/{}. {continuation_status}",
                        self.endpoint.provider.as_str(),
                        self.endpoint.model.as_str()
                    ),
                ));
            }
            Err(reason) => self.transcript.push((
                "Models".into(),
                format!("Model selection failed; current model retained: {reason}."),
            )),
        }
    }

    fn has_incompatible_continuation(&self) -> bool {
        self.history.iter().any(|message| {
            message
                .continuation
                .as_ref()
                .is_some_and(|state| state.endpoint != self.endpoint)
        })
    }

    /// Temporarily edit a slash command without consuming the message draft.
    pub(crate) fn toggle_command(&mut self) {
        if let Some(draft) = self.command_draft.take() {
            self.composer = draft;
            return;
        }
        if self.pending.is_some()
            || self
                .session_browser
                .as_ref()
                .is_some_and(SessionBrowser::is_opening)
        {
            self.transcript.push((
                "Marlin".into(),
                "Wait for the current request or session switch before opening commands.".into(),
            ));
            return;
        }
        self.command_draft = Some(std::mem::take(&mut self.composer));
        self.composer.insert('/');
    }

    fn show_sessions(&mut self, runtime: &tokio::runtime::Runtime, next: bool) {
        let message = match self.session_browser.as_mut() {
            None => "Session listing requires a persistent session configuration.",
            Some(browser) => match browser.request(runtime, next) {
                Ok(()) => "Loading sessions…",
                Err(SessionPageRequestError::Busy) => "A session listing is already in progress.",
                Err(SessionPageRequestError::NoNextPage) => {
                    "No next page. Use /sessions to refresh."
                }
            },
        };
        self.transcript.push(("Sessions".into(), message.into()));
    }

    fn poll_sessions(&mut self) -> bool {
        let Some(result) = self.session_browser.as_mut().and_then(SessionBrowser::poll) else {
            return false;
        };
        let text = match result {
            Err(error) => format!("Session listing failed: {error}"),
            Ok(page) => {
                let mut output = String::from("Sessions with recorded events (ordered by ID):\n");
                if page.items.is_empty() {
                    output.push_str("(none)\n");
                }
                for id in &page.items {
                    let is_current = self.current_session.as_ref() == Some(id);
                    if id.as_str().len() > 512 {
                        output.push_str("[ID too long to display; use --list-sessions]");
                    } else {
                        output.push_str(
                            &serde_json::to_string(id.as_str()).expect("string serialization"),
                        );
                    }
                    if is_current {
                        output.push_str(" (current)");
                    }
                    output.push('\n');
                }
                if page.next_cursor.is_some() {
                    output.push_str("Use /sessions next for the next page.\n");
                }
                output.push_str("Use /session followed by a quoted ID to switch; listing does not verify recovery status.");
                output
            }
        };
        self.transcript.push(("Sessions".into(), text));
        true
    }

    fn request_session_switch(&mut self, runtime: &tokio::runtime::Runtime, value: &str) {
        let id = match serde_json::from_str::<String>(value)
            .ok()
            .and_then(|value| SessionId::new(value).ok())
        {
            Some(id) => id,
            None => {
                self.transcript.push((
                    "Sessions".into(),
                    "Use /session followed by a nonempty JSON-quoted session ID.".into(),
                ));
                return;
            }
        };
        let message = match self.session_browser.as_mut() {
            None => "Session switching requires a persistent session configuration.",
            Some(browser) => match browser.open(runtime, id) {
                Ok(()) => "Loading selected session…",
                Err(_) => "A session switch is already in progress.",
            },
        };
        self.transcript.push(("Sessions".into(), message.into()));
    }

    fn request_history(
        &mut self,
        runtime: &tokio::runtime::Runtime,
        id: SessionId,
        before: Option<ConversationTurnId>,
    ) {
        let message = match self.session_browser.as_mut() {
            None => "History inspection requires a persistent session configuration.",
            Some(browser) => match browser.inspect(runtime, id, before) {
                Ok(()) => "Loading read-only history…",
                Err(_) => "A session load is already in progress.",
            },
        };
        self.transcript.push(("History".into(), message.into()));
    }

    fn request_history_page(&mut self, runtime: &tokio::runtime::Runtime, older: bool) {
        let Some(position) = &self.history_position else {
            self.transcript.push((
                "History".into(),
                "Use /inspect followed by a JSON-quoted session ID first.".into(),
            ));
            return;
        };
        if older && !position.has_earlier {
            self.transcript
                .push(("History".into(), "No earlier history page.".into()));
            return;
        }
        let before = if older { position.oldest } else { None };
        let id = self.current_session.clone().expect("history has a session");
        self.request_history(runtime, id, before);
    }

    fn poll_session_switch(
        &mut self,
        gateway: &mut SharedGateway,
        base_gateway: &RuntimeStreamGateway,
    ) -> bool {
        let Some(result) = self
            .session_browser
            .as_mut()
            .and_then(SessionBrowser::poll_open)
        else {
            return false;
        };
        match result {
            Err(error) => self.transcript.push((
                "Error".into(),
                format!("Session switch failed; current session retained: {error}"),
            )),
            Ok(opened) => {
                self.prompt_history.restore(&mut self.composer);
                *gateway =
                    SharedGateway::new(JournalGateway::new(base_gateway.clone(), opened.journal));
                self.transcript.clear();
                self.history = Arc::new(Vec::new());
                self.needs_replay = false;
                self.unverified_thinking = false;
                self.scroll_up = 0;
                self.current_session = Some(opened.id.clone());
                self.transcript
                    .push(("Session".into(), opened.id.as_str().to_owned()));
                match opened.view {
                    OpenedView::Full(turns) => {
                        self.history_position = None;
                        self.restore(turns);
                    }
                    OpenedView::History(window) => {
                        self.history_position = Some(HistoryPosition {
                            oldest: window.turns.first().map(|turn| turn.id),
                            has_earlier: window.has_earlier,
                            has_newer: window.before.is_some(),
                        });
                        self.restore(window.turns);
                        let position = self
                            .history_position
                            .as_ref()
                            .expect("history position set");
                        let mut text = String::from(
                            "Read-only history: model requests are disabled. Use /session with this ID to load full context if it fits.",
                        );
                        if position.has_earlier {
                            text.push_str(" /history older loads earlier turns.");
                        }
                        if position.has_newer {
                            text.push_str(" /history newest returns to recent turns.");
                        }
                        self.transcript.push(("History".into(), text));
                    }
                }
            }
        }
        true
    }

    pub(crate) fn restore(&mut self, turns: Vec<ReplayTurn>) {
        self.persistent = true;
        for turn in turns {
            self.transcript.push(("You".into(), turn.prompt.clone()));
            let Some(answer_end) = self.restore_checkpoints(&turn) else {
                self.needs_replay = true;
                self.transcript.push((
                    "Error".into(),
                    "Invalid checkpoint ranges; session cannot be continued.".into(),
                ));
                continue;
            };
            let unverified_thinking = !turn.thinking.is_empty()
                && self.history_position.is_none()
                && !matches!(&turn.outcome, ReplayOutcome::Completed(message)
                    if matches_observed_anthropic_thinking(message, &turn.thinking));
            self.unverified_thinking |= unverified_thinking;
            let merge_answer = turn
                .checkpoint_spans
                .last()
                .is_some_and(|span| span.channel == ConversationCheckpointChannel::Text);
            match turn.outcome {
                ReplayOutcome::Completed(message) => {
                    self.restore_answer_suffix(
                        "Assistant",
                        &message.content[answer_end..],
                        merge_answer,
                        true,
                    );
                    if self.history_position.is_none() {
                        let history = Arc::make_mut(&mut self.history);
                        history.push(user_gateway_message(turn.prompt));
                        history.push(message);
                    }
                }
                ReplayOutcome::Failed {
                    diagnostic,
                    partial,
                } => {
                    self.restore_answer_suffix(
                        "Assistant",
                        &partial[answer_end..],
                        merge_answer,
                        false,
                    );
                    self.transcript.push(("Error".into(), diagnostic));
                }
                ReplayOutcome::LegacyFailed {
                    diagnostic,
                    partial,
                } => {
                    self.needs_replay = true;
                    self.restore_answer_suffix(
                        "Assistant",
                        &partial[answer_end..],
                        merge_answer,
                        false,
                    );
                    self.transcript.push(("Error".into(), format!("Historical failure: {diagnostic}. Remote retirement is unverified; further turns are blocked.")));
                }
                ReplayOutcome::LegacyCancelled { partial } => {
                    self.needs_replay = true;
                    self.restore_answer_suffix(
                        "Assistant",
                        &partial[answer_end..],
                        merge_answer,
                        false,
                    );
                    self.transcript
                        .push(("Marlin".into(), "Historical local cancellation has no remote retirement evidence; further turns are blocked.".into()));
                }
                ReplayOutcome::Unfinished { partial } => {
                    self.needs_replay = true;
                    self.restore_answer_suffix(
                        "Assistant (recovered prefix)",
                        &partial[answer_end..],
                        merge_answer,
                        false,
                    );
                    self.transcript.push((
                        "Error".into(),
                        "Unfinished prior turn: ownership is unknown; automatic retry is disabled."
                            .into(),
                    ));
                }
                ReplayOutcome::Uncertain { reason, partial } => {
                    self.needs_replay = true;
                    self.restore_answer_suffix(
                        "Assistant (recovered prefix)",
                        &partial[answer_end..],
                        merge_answer,
                        false,
                    );
                    let message = match reason {
                        marlin_agent_sessions::ConversationUncertaintyReason::StreamOpenTimeout => "Stream-open deadline elapsed; remote outcome and ownership are unknown. Further turns are blocked.".to_owned(),
                        marlin_agent_sessions::ConversationUncertaintyReason::StreamOpenTransport => "Stream-open transport failed; remote outcome and ownership are unknown. Further turns are blocked.".to_owned(),
                        marlin_agent_sessions::ConversationUncertaintyReason::StreamOpenUnclassified => "Stream-open failed; remote outcome and ownership are unknown. Further turns are blocked.".to_owned(),
                        marlin_agent_sessions::ConversationUncertaintyReason::StreamInterrupted { reason } => format!("{reason}; remote outcome and ownership are unknown. Further turns are blocked."),
                    };
                    self.transcript.push(("Error".into(), message));
                }
                ReplayOutcome::Omitted => self.transcript.push((
                    "History".into(),
                    "Turn contents exceed the history page budget and were not displayed.".into(),
                )),
            }
            if unverified_thinking {
                self.transcript.push(("Error".into(), "Recorded thinking is not yet verified against a provider continuation; further requests are blocked.".into()));
            }
        }
        if self.history_position.is_none() && self.has_incompatible_continuation() {
            self.transcript.push((
                "Models".into(),
                "Restored session has continuation from another model route; select its original route before sending.".into(),
            ));
        }
    }

    fn restore_answer_suffix(&mut self, role: &str, suffix: &str, merge: bool, show_empty: bool) {
        if suffix.is_empty() {
            if show_empty && !merge {
                self.transcript.push((role.into(), String::new()));
            }
        } else if merge {
            let index = self.transcript.len() - 1;
            self.transcript.append_content(index, suffix);
        } else {
            self.transcript.push((role.into(), suffix.to_owned()));
        }
    }

    fn restore_checkpoints(&mut self, turn: &ReplayTurn) -> Option<usize> {
        let answer = match &turn.outcome {
            ReplayOutcome::Completed(message) => message.content.as_str(),
            ReplayOutcome::Failed { partial, .. }
            | ReplayOutcome::LegacyFailed { partial, .. }
            | ReplayOutcome::LegacyCancelled { partial }
            | ReplayOutcome::Unfinished { partial }
            | ReplayOutcome::Uncertain { partial, .. } => partial.as_str(),
            ReplayOutcome::Omitted => "",
        };
        let (mut answer_end, mut thinking_end) = (0, 0);
        for span in &turn.checkpoint_spans {
            let (source, end) = match span.channel {
                ConversationCheckpointChannel::Text => (answer, &mut answer_end),
                ConversationCheckpointChannel::Thinking => {
                    (turn.thinking.as_str(), &mut thinking_end)
                }
            };
            if span.start != *end
                || span.start >= span.end
                || source.get(span.start..span.end).is_none()
            {
                return None;
            }
            *end = span.end;
        }
        if thinking_end != turn.thinking.len() {
            return None;
        }
        let answer_role = if matches!(
            &turn.outcome,
            ReplayOutcome::Unfinished { .. } | ReplayOutcome::Uncertain { .. }
        ) {
            "Assistant (recovered prefix)"
        } else {
            "Assistant"
        };
        for span in &turn.checkpoint_spans {
            let (role, source) = match span.channel {
                ConversationCheckpointChannel::Text => (answer_role, answer),
                ConversationCheckpointChannel::Thinking => {
                    ("Thinking (recorded)", turn.thinking.as_str())
                }
            };
            self.transcript
                .push((role.into(), source[span.start..span.end].to_owned()));
        }
        Some(answer_end)
    }

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
                Some(Command::Help) => self.transcript.push(("Help".into(), "Enter: send · Tab / Shift-Tab: focus · Ctrl-N: newline · Ctrl-O: command without losing draft · arrows: edit · Home/End or Ctrl-A/Ctrl-E: line start/end · Alt-P/Alt-N: prompt history · Ctrl-R: search prompts · PageUp/PageDown: scroll · Esc/Ctrl-C: cancel · Ctrl-C twice or Ctrl-Q: quit · /models: list · /model: select · /status: worktree · /diff: unstaged · /diff staged: staged · /sessions: list · /session: switch · /inspect: read-only history · /history older or newest: page · /context: budget · /runtime: stream-open policy".into())),
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
                    let text = self.runtime_edge.as_ref().map_or_else(|| "Runtime stream-open policy is unavailable.".into(), |receipt| {
                        let timeout = receipt.timeout_ms().map_or_else(|| "disabled".into(), |ms| format!("{ms} ms"));
                        format!("Stream-open timeout: {timeout}; layers: {}. This policy does not cover stream duration, tools, approvals, or remote cancellation.", receipt.layers().iter().map(|layer| layer.as_str()).collect::<Vec<_>>().join(", "))
                    });
                    self.transcript.push(("Runtime".into(), text));
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
            Err(error) => self.fail_turn(error.to_string()),
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
        }
    }

    /// Ctrl-C cancels active work or command entry; when idle it requires a
    /// second consecutive press to exit, preserving the draft on the first.
    pub(crate) fn interrupt(&mut self) -> bool {
        if self.command_draft.is_some()
            || self.pending.is_some()
            || self.workspace.is_pending()
            || self.workspace.view().is_some()
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

    fn set_effective_scroll(&mut self, value: usize) {
        match self.workspace.view_mut() {
            Some(view) => view.scroll_from_top = value,
            None => self.scroll_up = value,
        }
    }
}

pub(crate) fn process_replies(
    app: &mut App,
    receiver: &mut tokio::sync::mpsc::Receiver<QueuedReply>,
    limit: usize,
) -> usize {
    let mut processed = 0;
    while processed < limit {
        let Ok(event) = receiver.try_recv() else {
            break;
        };
        let (id, reply) = event.into_parts();
        app.receive(id, reply);
        processed += 1;
    }
    processed
}

// Never pass untrusted terminal escape sequences to the renderer.
pub(crate) fn safe_text(text: &str) -> String {
    text.chars()
        .map(|c| {
            if c == '\n' {
                c
            } else if c == '\t' {
                ' '
            } else if c.is_control() {
                '�'
            } else {
                c
            }
        })
        .collect()
}

fn run(
    path: PathBuf,
    selected_session: Option<SessionId>,
) -> Result<Option<i32>, Box<dyn std::error::Error>> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err("marlin-tui requires a terminal on stdin and stdout".into());
    }
    let config = config::Config::load(&path)?;
    if selected_session.is_some() && config.session.is_none() {
        return Err("--session requires a [session] storage configuration".into());
    }
    let (endpoint, catalog) = config.catalog()?;
    // Package resolution uses SDK-owned process environment during startup;
    // perform it before starting Tokio workers or the terminal input thread.
    let _ = writeln!(io::stderr(), "Starting interface…");
    let scheme_runtime = gerbil_scheme::GerbilRuntime::initialize()?;
    scheme_runtime
        .set_package_resolution_progress(gerbil_scheme::PackageResolutionProgress::Quiet)?;
    let directory = env::current_dir()?;
    let directory = directory
        .to_str()
        .ok_or("UI package directory is not UTF-8")?;
    let model_label = format!("{}/{}", endpoint.provider.as_str(), endpoint.model.as_str());
    let session_label = selected_session
        .as_ref()
        .map(SessionId::as_str)
        .or_else(|| {
            config
                .session
                .as_ref()
                .map(|session| session.session.as_str())
        })
        .unwrap_or("temporary");
    let mut ui = UiHost::open(
        &scheme_runtime,
        directory,
        &UiOpenFacts {
            model_label: &model_label,
            session_label,
            status: &[],
            extensions: &[],
            method_bundles: &[],
        },
    )?;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    let mut app = App::new(endpoint);
    app.set_models(catalog.clone());
    let open_timeout_ms = config
        .timeout_seconds
        .checked_mul(1000)
        .ok_or("stream-open timeout is too large")?;
    let gateway = RuntimeStreamGateway::new(
        catalog,
        RuntimeEdgePolicy::new().with_timeout_ms(open_timeout_ms),
    )?;
    app.set_runtime_edge(gateway.receipt().clone());
    let base_gateway = gateway.clone();
    let mut gateway = if let Some(session) = config.session {
        let session_id = match selected_session {
            Some(id) => id,
            None => SessionId::new(session.session)?,
        };
        let (journal, turns, browser) = runtime.block_on(async {
            let storage = TursoAgentStorage::open_local(TursoAgentStorageConfig {
                path: session.path,
                optimization_profile: TursoOptimizationProfile::AsyncIoOnlyCompatibility,
                batch_transaction_mode: TursoBatchTransactionMode::Immediate,
            })
            .await?;
            let project = ProjectId::new(session.project)?;
            let agent = AgentId::new(session.agent)?;
            let browser = SessionBrowser::new(storage.clone(), project.clone(), agent.clone());
            let (journal, turns) =
                load_session(storage, project, session_id.clone(), agent).await?;
            Ok::<_, Box<dyn std::error::Error>>((journal, turns, browser))
        })?;
        app.session_browser = Some(browser);
        app.current_session = Some(session_id.clone());
        app.transcript
            .push(("Session".into(), session_id.as_str().to_owned()));
        app.restore(turns);
        SharedGateway::new(JournalGateway::new(gateway, journal))
    } else {
        SharedGateway::new(gateway)
    };
    let signals = SignalMonitor::new(&runtime)?;
    let stdout = io::stdout().into_raw_mode()?.into_alternate_screen()?;
    let stdout = BracketedPaste::enable(stdout)?;
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let (keys_tx, keys_rx) = mpsc::sync_channel(INPUT_QUEUE_ITEMS);
    thread::spawn(move || {
        let mut decoder = InputDecoder::default();
        for event in io::stdin().events_and_raw() {
            match event {
                Ok((event, raw)) => {
                    if let Some(input) = decoder.accept(event, &raw)
                        && keys_tx.send(Ok(input)).is_err()
                    {
                        break;
                    }
                }
                Err(error) => {
                    let _ = keys_tx.send(Err(error));
                    break;
                }
            }
        }
    });
    let (reply_tx, mut reply_rx) = reply_channel(REPLY_QUEUE_ITEMS, REPLY_QUEUE_BYTES);
    let mut schedule = FrameSchedule::new();
    let mut viewport_cache = TranscriptViewportCache::default();
    let mut scroll_anchor = ScrollAnchor::default();
    let mut last_size = None;
    let mut failed_ui_labels: Option<(String, String)> = None;
    let mut signal_exit = None;
    let result = (|| -> io::Result<()> {
        loop {
            signal_exit = signals.observed_exit_code();
            if signal_exit.is_some() {
                break;
            }
            let processed = process_replies(&mut app, &mut reply_rx, REPLY_BATCH_LIMIT);
            let sessions_changed = app.poll_sessions();
            let workspace_changed = if let Some(result) = app.workspace.poll() {
                match result {
                    Ok(view) => app.workspace.show(view),
                    Err(error) => app.transcript.push((
                        "Workspace".into(),
                        format!("Workspace read failed: {error}"),
                    )),
                }
                true
            } else {
                false
            };
            let switched = app.poll_session_switch(&mut gateway, &base_gateway);
            let search_finished = app
                .history_search
                .as_mut()
                .is_some_and(|search| search.step(&app.history));
            if processed > 0 || sessions_changed || switched || workspace_changed || search_finished
            {
                schedule.mark_dirty();
            }
            let size = termion::terminal_size()?;
            if last_size != Some(size) {
                last_size = Some(size);
                schedule.mark_dirty();
            }
            let now = Instant::now();
            if schedule.should_draw(now) {
                let endpoint = app.endpoint();
                let model = format!("{}/{}", endpoint.provider.as_str(), endpoint.model.as_str());
                let session = app.session_id().map_or("temporary", |id| id.as_str());
                if ui.view().model_label == model && ui.view().session_label == session {
                    failed_ui_labels = None;
                }
                if (ui.view().model_label != model || ui.view().session_label != session)
                    && !failed_ui_labels
                        .as_ref()
                        .is_some_and(|(old_model, old_session)| {
                            old_model == &model && old_session == session
                        })
                {
                    if let Err(error) = ui.refresh_labels(&model, session) {
                        let attempted = (model, session.to_owned());
                        app.transcript.push((
                            "UI".into(),
                            safe_text(&format!("UI label refresh failed: {error}")),
                        ));
                        failed_ui_labels = Some(attempted);
                    } else {
                        failed_ui_labels = None;
                    }
                }
                let mut effective_scroll = app.scroll_up;
                terminal.draw(|frame| {
                    effective_scroll = draw(
                        frame,
                        &app,
                        ui.view(),
                        &mut viewport_cache,
                        &mut scroll_anchor,
                    )
                })?;
                app.set_effective_scroll(effective_scroll);
                schedule.drawn(Instant::now());
            }
            let wait = if processed == REPLY_BATCH_LIMIT
                || app
                    .history_search
                    .as_ref()
                    .is_some_and(HistorySearch::is_searching)
            {
                std::time::Duration::ZERO
            } else {
                schedule.wait_duration(Instant::now())
            };
            let received = keys_rx.recv_timeout(wait);
            signal_exit = signals.observed_exit_code();
            if signal_exit.is_some() {
                break;
            }
            if matches!(&received, Ok(Ok(_))) {
                schedule.mark_dirty();
            }
            if matches!(&received, Ok(Ok(input)) if !matches!(input, TerminalInput::Key(Key::Ctrl('c'))))
            {
                app.disarm_exit();
            }
            if let Some(search) = app.history_search.as_mut() {
                match received {
                    Ok(Ok(TerminalInput::Key(Key::Ctrl('q')))) => break,
                    Ok(Ok(TerminalInput::Key(Key::Ctrl('c') | Key::Esc))) => {
                        app.history_search = None;
                        app.disarm_exit();
                    }
                    Ok(Ok(TerminalInput::Key(Key::Char('\n')))) => app.accept_history_search(),
                    Ok(Ok(TerminalInput::Key(Key::Ctrl('r')))) => search.previous(&app.history),
                    Ok(Ok(TerminalInput::Key(Key::Backspace))) => search.backspace(&app.history),
                    Ok(Ok(TerminalInput::Key(Key::Char(c)))) if !c.is_control() => {
                        search.insert(&c.to_string(), &app.history)
                    }
                    Ok(Ok(TerminalInput::Paste(text))) => search.insert(&text, &app.history),
                    Ok(Err(e)) => return Err(e),
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    _ => {}
                }
                continue;
            }
            if let Ok(Ok(TerminalInput::Key(key))) = &received {
                if *key == Key::Char('\t')
                    && app.composer.cursor_at_end()
                    && ui.accepts_text(app.command_draft.is_some())
                    && ui.has_command_prefix(app.composer.text())
                {
                    if let Some(completed) = ui.complete_command(app.composer.text()) {
                        app.composer.take();
                        app.composer.insert_text(&completed);
                    }
                    continue;
                }
                match ui.route_key(
                    *key,
                    app.command_draft.is_some(),
                    app.composer.text().trim_start().starts_with('/'),
                ) {
                    Ok(UiKeyAction::Handled) => continue,
                    Ok(UiKeyAction::Submit) => {
                        app.submit(&gateway, &runtime, &reply_tx);
                        continue;
                    }
                    Ok(UiKeyAction::OpenCommands) => {
                        app.toggle_command();
                        continue;
                    }
                    Ok(UiKeyAction::Scroll(up)) => {
                        app.scroll(up);
                        continue;
                    }
                    Ok(UiKeyAction::Unhandled) => {}
                    Err(error) => {
                        app.transcript
                            .push(("UI".into(), safe_text(&format!("UI event failed: {error}"))));
                        continue;
                    }
                }
            }
            let accepts_text = ui.accepts_text(app.command_draft.is_some());
            match received {
                Ok(Ok(TerminalInput::Key(Key::Ctrl('q')))) => break,
                Ok(Ok(TerminalInput::Key(Key::Ctrl('c')))) if app.interrupt() => break,
                Ok(Ok(TerminalInput::Key(Key::Ctrl('c')))) => {}
                Ok(Ok(TerminalInput::Key(Key::Ctrl('o')))) => app.toggle_command(),
                Ok(Ok(TerminalInput::Key(Key::Ctrl('r')))) => app.start_history_search(),
                Ok(Ok(TerminalInput::Key(Key::Alt('p')))) if accepts_text => {
                    app.navigate_prompts(true)
                }
                Ok(Ok(TerminalInput::Key(Key::Alt('n')))) if accepts_text => {
                    app.navigate_prompts(false)
                }
                Ok(Ok(TerminalInput::Key(Key::Esc))) => app.cancel(),
                Ok(Ok(TerminalInput::Key(Key::Ctrl('n')))) if accepts_text => {
                    app.composer.insert('\n')
                }
                Ok(Ok(TerminalInput::Key(Key::Char(c)))) if !c.is_control() && accepts_text => {
                    app.composer.insert(c)
                }
                Ok(Ok(TerminalInput::Key(Key::Left))) if accepts_text => app.composer.left(),
                Ok(Ok(TerminalInput::Key(Key::Right))) if accepts_text => app.composer.right(),
                Ok(Ok(TerminalInput::Key(Key::Home | Key::Ctrl('a')))) if accepts_text => {
                    app.composer.line_start()
                }
                Ok(Ok(TerminalInput::Key(Key::End | Key::Ctrl('e')))) if accepts_text => {
                    app.composer.line_end()
                }
                Ok(Ok(TerminalInput::Key(Key::Up))) if accepts_text => {
                    app.composer.move_vertical(size.0.saturating_sub(2), false)
                }
                Ok(Ok(TerminalInput::Key(Key::Down))) if accepts_text => {
                    app.composer.move_vertical(size.0.saturating_sub(2), true)
                }
                Ok(Ok(TerminalInput::Key(Key::Backspace))) if accepts_text => {
                    app.composer.backspace()
                }
                Ok(Ok(TerminalInput::Key(Key::Delete))) if accepts_text => app.composer.delete(),
                Ok(Ok(TerminalInput::Key(Key::PageUp))) => app.scroll(true),
                Ok(Ok(TerminalInput::Key(Key::PageDown))) => app.scroll(false),
                Ok(Ok(TerminalInput::Paste(text))) if accepts_text => {
                    app.composer.insert_text(&text)
                }
                Ok(Ok(TerminalInput::PasteRejected)) => app.transcript.push((
                    "Error".into(),
                    "Paste rejected: invalid UTF-8 or larger than 1 MiB.".into(),
                )),
                Ok(Err(e)) => return Err(e),
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
                _ => {}
            }
        }
        Ok(())
    })();
    app.cancel();
    terminal.show_cursor()?;
    drop(terminal); // alternate-screen and raw-mode guards restore the terminal
    result?;
    Ok(signal_exit)
}

/// Start the terminal client with the process argument list.
pub fn run_cli() {
    let args = match parse_cli(env::args_os().skip(1)) {
        Ok(args) => args,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(2);
        }
    };
    let result = match args.action {
        CliAction::Open { session } => run(args.config, session),
        CliAction::List { after } => list_sessions(&args.config, after).map(|()| None),
    };
    match result {
        Ok(Some(code)) => std::process::exit(code),
        Ok(None) => {}
        Err(error) => {
            let _ = writeln!(io::stderr(), "marlin-tui: {error}");
            std::process::exit(1);
        }
    }
}
