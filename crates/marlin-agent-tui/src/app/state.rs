// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Terminal application state and transcript navigation.

use super::{
    Arc, Command, Composer, ContextBudget, ConversationCheckpointChannel, ConversationTurnId,
    HistorySearch, JournalGateway, MAX_COMPLETION_HEAP_BYTES, ModelAlias, ModelCatalog,
    ModelEndpoint, ModelGatewayMessage, OpenedView, PromptHistory, ReplayOutcome, ReplayTurn,
    RuntimeEdgePolicyReceipt, RuntimeStreamGateway, SessionBrowser, SessionId,
    SessionPageRequestError, SharedGateway, Transcript, TursoAgentStorage, UiCommandName,
    WorkspaceStatus, matches_observed_anthropic_thinking, user_gateway_message,
};

pub(crate) const REPLY_BATCH_LIMIT: usize = 16;
pub(crate) const REPLY_QUEUE_ITEMS: usize = 64;
pub(crate) const REPLY_QUEUE_BYTES: u32 = MAX_COMPLETION_HEAP_BYTES as u32;
pub(crate) const INPUT_QUEUE_ITEMS: usize = 8;
pub(crate) const MAX_TURN_SEGMENTS: usize =
    marlin_agent_sessions::CONVERSATION_MAX_CHECKPOINT_SEGMENTS;

pub(crate) struct PendingTurn {
    pub(super) id: u64,
    pub(super) task: tokio::task::JoinHandle<()>,
    pub(super) prompt: String,
    pub(super) output: Option<usize>,
    pub(super) output_segments: Vec<usize>,
    pub(super) thinking: Option<usize>,
    pub(super) thinking_segments: Vec<usize>,
    pub(super) had_thinking: bool,
    pub(super) observed_stream: bool,
    pub(super) segments: usize,
    pub(super) next_sequence: u64,
    pub(super) next_thinking_sequence: u64,
}

pub(super) struct HistoryPosition {
    pub(super) oldest: Option<ConversationTurnId>,
    pub(super) has_earlier: bool,
    pub(super) has_newer: bool,
}

pub(crate) struct App {
    pub(crate) composer: Composer,
    pub(crate) minibuffer_hovered: bool,
    pub(crate) gateway_observation: GatewayObservation,
    pub(super) prompt_history: PromptHistory,
    pub(crate) history_search: Option<HistorySearch>,
    pub(crate) command_draft: Option<Composer>,
    pub(crate) command_selection: Option<(String, UiCommandName)>,
    pub(crate) quit_armed: bool,
    pub(crate) transcript: Transcript,
    pub(crate) history: Arc<Vec<ModelGatewayMessage>>,
    pub(crate) context_budget: ContextBudget,
    pub(super) endpoint: ModelEndpoint,
    pub(super) models: Option<ModelCatalog>,
    pub(crate) pending: Option<PendingTurn>,
    pub(super) next_id: u64,
    pub(crate) scroll_up: usize,
    pub(super) persistent: bool,
    pub(super) needs_replay: bool,
    pub(super) unverified_thinking: bool,
    pub(super) runtime_edge: Option<RuntimeEdgePolicyReceipt>,
    pub(super) session_browser: Option<SessionBrowser<TursoAgentStorage>>,
    pub(super) current_session: Option<SessionId>,
    pub(super) history_position: Option<HistoryPosition>,
    pub(crate) workspace: WorkspaceStatus,
}

/// The most recent local transport observation, not a persistent HTTP link.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum GatewayObservation {
    Unchecked,
    Online,
    Offline,
    Unknown,
}

impl App {
    pub(crate) fn new(endpoint: ModelEndpoint) -> Self {
        Self {
            composer: Composer::default(),
            minibuffer_hovered: false,
            gateway_observation: GatewayObservation::Unchecked,
            prompt_history: PromptHistory::default(),
            history_search: None,
            command_draft: None,
            command_selection: None,
            quit_armed: false,
            transcript: Transcript::new(vec![]),
            history: Arc::new(Vec::new()),
            context_budget: ContextBudget::default(),
            endpoint,
            models: None,
            pending: None,
            next_id: 0,
            scroll_up: 0,
            persistent: false,
            needs_replay: false,
            unverified_thinking: false,
            runtime_edge: None,
            session_browser: None,
            current_session: None,
            history_position: None,
            workspace: WorkspaceStatus::default(),
        }
    }

    pub(crate) fn selected_command(&self) -> Option<&UiCommandName> {
        self.command_selection.as_ref().and_then(|(query, name)| {
            (query == self.composer.text() && self.composer.cursor_at_end()).then_some(name)
        })
    }

    /// Publish a menu completion and say whether its current form is runnable.
    pub(crate) fn accept_command_completion(&mut self, completed: &str) -> bool {
        let runnable = Command::parse(completed).is_some();
        self.composer.take();
        self.composer.insert_text(completed);
        self.command_selection = None;
        runnable
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
        self.command_selection = None;
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

    pub(super) fn show_models(&mut self) {
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

    pub(super) fn select_model(&mut self, alias: &str) {
        let result = self
            .models
            .as_ref()
            .ok_or("model catalog is unavailable")
            .and_then(|models| models.select(&ModelAlias::new(alias)));
        match result {
            Ok(endpoint) => {
                self.endpoint = endpoint;
                // A successful observation belongs to the selected route, not
                // to every configured model in the catalog.
                self.gateway_observation = GatewayObservation::Unchecked;
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

    pub(super) fn has_incompatible_continuation(&self) -> bool {
        self.history.iter().any(|message| {
            message
                .continuation
                .as_ref()
                .is_some_and(|state| state.endpoint != self.endpoint)
        })
    }

    /// Temporarily edit a slash command without consuming the message draft.
    pub(crate) fn toggle_command(&mut self) {
        self.command_selection = None;
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

    pub(super) fn show_sessions(&mut self, runtime: &tokio::runtime::Runtime, next: bool) {
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

    pub(super) fn poll_sessions(&mut self) -> bool {
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

    pub(super) fn request_session_switch(
        &mut self,
        runtime: &tokio::runtime::Runtime,
        value: &str,
    ) {
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

    pub(super) fn request_history(
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

    pub(super) fn request_history_page(&mut self, runtime: &tokio::runtime::Runtime, older: bool) {
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

    pub(super) fn poll_session_switch(
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

    pub(super) fn restore_answer_suffix(
        &mut self,
        role: &str,
        suffix: &str,
        merge: bool,
        show_empty: bool,
    ) {
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

    pub(super) fn restore_checkpoints(&mut self, turn: &ReplayTurn) -> Option<usize> {
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
