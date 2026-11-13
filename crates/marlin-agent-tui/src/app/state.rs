// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Terminal application state and transcript navigation.

use super::{
    Arc, BufferView, Command, Composer, ContextBudget, ConversationCheckpointChannel,
    ConversationTurnId, GatewayObservation, HistorySearch, JournalGateway,
    MAX_COMPLETION_HEAP_BYTES, MessageEvent, MessageSeverity, MessageSource, ModelAlias,
    ModelCatalog, ModelEndpoint, ModelGatewayMessage, OpenedView, PromptHistory, ReplayOutcome,
    ReplayTurn, RuntimeEdgePolicyReceipt, RuntimeStreamGateway, SessionBrowser, SessionId,
    SessionPageRequestError, SharedGateway, Transcript, TursoAgentStorage, UiBufferId, UiCommand,
    UiCommandName, UiContent, UiEvent, UiTranscriptPolicy, UiWorkbench, WorkspaceStatus,
    matches_observed_anthropic_thinking, user_gateway_message,
};

pub(crate) const REPLY_BATCH_LIMIT: usize = 16;
pub(crate) const REPLY_QUEUE_ITEMS: usize = 64;
pub(crate) const REPLY_QUEUE_BYTES: u32 = MAX_COMPLETION_HEAP_BYTES as u32;
pub(crate) const INPUT_QUEUE_ITEMS: usize = 8;
pub(crate) const RUNTIME_EVENT_BATCH_LIMIT: usize = 16;
pub(crate) const MAX_TURN_SEGMENTS: usize =
    marlin_agent_sessions::CONVERSATION_MAX_CHECKPOINT_SEGMENTS;

pub(crate) struct PendingTurn {
    pub(super) id: u64,
    pub(super) task: marlin_agent_runtime::RuntimeTask<()>,
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

pub(crate) struct CommandMenu {
    pub(crate) prefix: String,
    pub(crate) prompt: String,
}

pub(crate) struct App {
    pub(super) runtime_owner: marlin_agent_runtime::TokioAgentRuntime,
    pub(super) runtime_events: marlin_agent_runtime::RuntimeEventStream,
    pub(crate) composer: Composer,
    pub(crate) minibuffer_hovered: bool,
    pub(crate) gateway_observation: GatewayObservation,
    pub(super) prompt_history: PromptHistory,
    pub(crate) history_search: Option<HistorySearch>,
    pub(crate) command_draft: Option<Composer>,
    pub(crate) command_menu: Option<CommandMenu>,
    pub(crate) command_selection: Option<(String, UiCommandName)>,
    pub(crate) command_query: Option<String>,
    pub(crate) command_candidates_pending: Option<bool>,
    pub(crate) command_candidates: Vec<UiCommandName>,
    pub(crate) command_catalog: Vec<UiCommand>,
    pub(crate) ui_request: Option<UiEvent>,
    pub(crate) transient_request: Option<super::UiTransientId>,
    pub(crate) quit_armed: bool,
    pub(crate) transcript: Transcript,
    pub(crate) transcript_policy: Option<UiTranscriptPolicy>,
    pub(crate) messages: Transcript,
    pub(crate) buffer_view: BufferView,
    pub(crate) minibuffer_notice: Option<String>,
    pub(crate) transient_hint: Option<marlin_agent_ui::UiElement>,
    pub(crate) history: Arc<Vec<ModelGatewayMessage>>,
    pub(crate) context_budget: ContextBudget,
    pub(super) endpoint: ModelEndpoint,
    pub(super) models: Option<ModelCatalog>,
    pub(crate) pending: Option<PendingTurn>,
    pub(super) next_id: u64,
    pub(crate) scroll_up: usize,
    pub(crate) message_scroll_up: usize,
    pub(crate) static_buffer_scroll: usize,
    pub(super) persistent: bool,
    pub(super) needs_replay: bool,
    pub(super) pending_tool_request: bool,
    pub(super) unverified_thinking: bool,
    pub(crate) runtime_edge: Option<RuntimeEdgePolicyReceipt>,
    pub(crate) runtime_content: Vec<UiContent>,
    pub(crate) runtime_receipt_open: bool,
    pub(crate) dynamic_buffer_content: std::collections::HashMap<UiBufferId, Vec<UiContent>>,
    pub(super) process_snapshot: Option<(marlin_agent_runtime::SessionId, u64)>,
    pub(super) session_browser: Option<SessionBrowser<TursoAgentStorage>>,
    pub(super) current_session: Option<SessionId>,
    pub(super) history_position: Option<HistoryPosition>,
    pub(crate) workspace: WorkspaceStatus,
}

impl App {
    pub(crate) fn new(endpoint: ModelEndpoint) -> Self {
        let (runtime_owner, runtime_events) = marlin_agent_runtime::TokioAgentRuntime::new(64);
        Self::with_runtime(endpoint, runtime_owner, runtime_events)
    }

    pub(crate) fn with_runtime(
        endpoint: ModelEndpoint,
        runtime_owner: marlin_agent_runtime::TokioAgentRuntime,
        runtime_events: marlin_agent_runtime::RuntimeEventStream,
    ) -> Self {
        Self {
            workspace: WorkspaceStatus::new(runtime_owner.clone()),
            runtime_owner,
            runtime_events,
            composer: Composer::default(),
            minibuffer_hovered: false,
            gateway_observation: GatewayObservation::Unchecked,
            prompt_history: PromptHistory::default(),
            history_search: None,
            command_draft: None,
            command_menu: None,
            command_selection: None,
            command_query: None,
            command_candidates_pending: None,
            command_candidates: Vec::new(),
            command_catalog: Vec::new(),
            ui_request: None,
            transient_request: None,
            quit_armed: false,
            transcript: Transcript::new(vec![]),
            transcript_policy: None,
            messages: Transcript::new(vec![]),
            buffer_view: BufferView::Conversation,
            minibuffer_notice: None,
            transient_hint: None,
            history: Arc::new(Vec::new()),
            context_budget: ContextBudget::default(),
            endpoint,
            models: None,
            pending: None,
            next_id: 0,
            scroll_up: 0,
            message_scroll_up: 0,
            static_buffer_scroll: 0,
            persistent: false,
            needs_replay: false,
            pending_tool_request: false,
            unverified_thinking: false,
            runtime_edge: None,
            runtime_content: Vec::new(),
            runtime_receipt_open: false,
            dynamic_buffer_content: std::collections::HashMap::new(),
            process_snapshot: None,
            session_browser: None,
            current_session: None,
            history_position: None,
        }
    }

    pub(crate) fn push_prompt(&mut self, text: String) {
        if let Some(policy) = &self.transcript_policy {
            self.transcript
                .push((String::new(), format!("{}{}", policy.user_prefix, text)));
        } else {
            self.transcript.push(("You".into(), text));
        }
    }

    pub(crate) fn push_answer(&mut self, role: &str, content: String) {
        let display_role = if role == "Assistant" {
            self.transcript_policy
                .as_ref()
                .map_or_else(|| role.to_owned(), |policy| policy.assistant_role.clone())
        } else {
            role.to_owned()
        };
        self.transcript.push((display_role, content));
    }

    pub(crate) fn selected_command(&self) -> Option<&UiCommandName> {
        self.command_selection.as_ref().and_then(|(query, name)| {
            (query == self.composer.text() && self.composer.cursor_at_end()).then_some(name)
        })
    }

    pub(crate) fn command_lookup_query(&self) -> String {
        match &self.command_menu {
            Some(menu) => format!("{}{}", menu.prefix, self.composer.text()),
            None => self.composer.text().to_owned(),
        }
    }

    pub(crate) fn command_query_is_current(&self) -> bool {
        match (self.command_menu.as_ref(), self.command_query.as_deref()) {
            (Some(menu), Some(query)) => {
                query.strip_prefix(&menu.prefix) == Some(self.composer.text())
            }
            (None, Some(query)) => query == self.composer.text(),
            _ => false,
        }
    }

    /// Keep the first visible Scheme-ordered candidate as the real selection.
    /// The renderer and first arrow key must agree on the same anchor.
    pub(crate) fn menu_navigation_anchor(&self) -> Option<UiCommandName> {
        self.selected_command()
            .cloned()
            .or_else(|| self.command_candidates.first().cloned())
    }

    pub(crate) fn seed_command_selection(&mut self) {
        if self.command_draft.is_some() {
            self.command_selection = self
                .menu_navigation_anchor()
                .map(|name| (self.composer.text().to_owned(), name));
        }
    }

    pub(crate) fn set_commands(&mut self, ui: &UiWorkbench) {
        self.command_catalog = ui.commands().to_vec();
        self.command_query = None;
        self.command_candidates_pending = None;
    }

    /// Scheme orders candidates; native runtime readiness narrows effects.
    pub(crate) fn admit_available_commands(&self, candidates: &mut Vec<UiCommandName>) {
        if self.pending.is_some() {
            candidates.retain(|name| {
                self.command_catalog.iter().any(|descriptor| {
                    descriptor.name == *name
                        && (matches!(
                            descriptor.capability,
                            marlin_agent_ui::UiCommandCapability::ToggleThinking
                                | marlin_agent_ui::UiCommandCapability::KillProcess
                        ) || (descriptor.capability
                            == marlin_agent_ui::UiCommandCapability::SelectBuffer
                            && descriptor.value.as_deref() == Some("processes")))
                })
            });
        }
    }

    pub(crate) fn command_descriptor(&self, input: &str) -> Option<&UiCommand> {
        let name = input
            .trim()
            .split_once(' ')
            .map_or(input.trim(), |(name, _)| name);
        self.command_catalog
            .iter()
            .find(|command| command.name.as_str() == name)
    }

    /// Publish a menu completion and say whether its current form is runnable.
    pub(crate) fn accept_command_completion(&mut self, completed: &str) -> bool {
        let runnable = self
            .command_descriptor(completed)
            .is_some_and(|descriptor| Command::parse(descriptor, completed).is_some());
        let display = self
            .command_menu
            .as_ref()
            .and_then(|menu| completed.strip_prefix(&menu.prefix))
            .unwrap_or(completed);
        self.composer.take();
        self.composer.insert_text(display);
        self.command_selection = None;
        runnable
    }

    pub(crate) fn endpoint(&self) -> &ModelEndpoint {
        &self.endpoint
    }

    pub(crate) fn session_id(&self) -> Option<&SessionId> {
        self.current_session.as_ref()
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
        text.push_str("Use M-x model <alias> to select. Credentials are read at startup.");
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
                let detail = format!(
                    "Selected {alias}: {}/{}. {continuation_status}",
                    self.endpoint.provider.as_str(),
                    self.endpoint.model.as_str()
                );
                self.publish_message(MessageEvent {
                    source: MessageSource::Ui,
                    severity: MessageSeverity::Info,
                    summary: "Model selected",
                    detail: &detail,
                });
            }
            Err(reason) => self.publish_message(MessageEvent {
                source: MessageSource::Ui,
                severity: MessageSeverity::Warning,
                summary: "Model selection failed",
                detail: &format!("Current model retained: {reason}."),
            }),
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

    /// Open or close the command Mini Buffer without consuming the message draft.
    pub(crate) fn toggle_command(&mut self) {
        self.command_selection = None;
        self.command_query = None;
        self.command_candidates_pending = None;
        self.command_candidates.clear();
        self.command_menu = None;
        if let Some(draft) = self.command_draft.take() {
            self.composer = draft;
            return;
        }
        if self
            .session_browser
            .as_ref()
            .is_some_and(SessionBrowser::is_opening)
        {
            self.publish_message(MessageEvent {
                source: MessageSource::Ui,
                severity: MessageSeverity::Warning,
                summary: "Commands unavailable",
                detail: "Wait for the current request or session switch before opening commands.",
            });
            return;
        }
        self.command_draft = Some(std::mem::take(&mut self.composer));
    }

    pub(super) fn show_sessions(&mut self, runtime: &tokio::runtime::Runtime, next: bool) {
        let (severity, message) = match self.session_browser.as_mut() {
            None => (
                MessageSeverity::Warning,
                "Session listing requires a persistent session configuration.",
            ),
            Some(browser) => match browser.request(runtime, next) {
                Ok(()) => (MessageSeverity::Info, "Loading sessions…"),
                Err(SessionPageRequestError::Busy) => (
                    MessageSeverity::Warning,
                    "A session listing is already in progress.",
                ),
                Err(SessionPageRequestError::NoNextPage) => (
                    MessageSeverity::Info,
                    "No next page. Use M-x sessions to refresh.",
                ),
            },
        };
        self.publish_message(MessageEvent {
            source: MessageSource::Ui,
            severity,
            summary: "Sessions",
            detail: message,
        });
    }

    pub(super) fn poll_sessions(&mut self) -> bool {
        let Some(result) = self.session_browser.as_mut().and_then(SessionBrowser::poll) else {
            return false;
        };
        let text = match result {
            Err(error) => {
                self.publish_message(MessageEvent {
                    source: MessageSource::Ui,
                    severity: MessageSeverity::Error,
                    summary: "Session listing failed",
                    detail: &error.to_string(),
                });
                return true;
            }
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
                    output.push_str("Use M-x sessions-next for the next page.\n");
                }
                output.push_str("Use M-x session followed by a quoted ID to switch; listing does not verify recovery status.");
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
                self.publish_message(MessageEvent {
                    source: MessageSource::Input,
                    severity: MessageSeverity::Warning,
                    summary: "Invalid session ID",
                    detail: "Use M-x session followed by a nonempty JSON-quoted session ID.",
                });
                return;
            }
        };
        let (severity, message) = match self.session_browser.as_mut() {
            None => (
                MessageSeverity::Warning,
                "Session switching requires a persistent session configuration.",
            ),
            Some(browser) => match browser.open(runtime, id) {
                Ok(()) => (MessageSeverity::Info, "Loading selected session…"),
                Err(_) => (
                    MessageSeverity::Warning,
                    "A session switch is already in progress.",
                ),
            },
        };
        self.publish_message(MessageEvent {
            source: MessageSource::Ui,
            severity,
            summary: "Session switch",
            detail: message,
        });
    }

    pub(super) fn request_history(
        &mut self,
        runtime: &tokio::runtime::Runtime,
        id: SessionId,
        before: Option<ConversationTurnId>,
    ) {
        let (severity, message) = match self.session_browser.as_mut() {
            None => (
                MessageSeverity::Warning,
                "History inspection requires a persistent session configuration.",
            ),
            Some(browser) => match browser.inspect(runtime, id, before) {
                Ok(()) => (MessageSeverity::Info, "Loading read-only history…"),
                Err(_) => (
                    MessageSeverity::Warning,
                    "A session load is already in progress.",
                ),
            },
        };
        self.publish_message(MessageEvent {
            source: MessageSource::Ui,
            severity,
            summary: "History inspection",
            detail: message,
        });
    }

    pub(super) fn request_history_page(&mut self, runtime: &tokio::runtime::Runtime, older: bool) {
        let Some(position) = &self.history_position else {
            self.publish_message(MessageEvent {
                source: MessageSource::Input,
                severity: MessageSeverity::Warning,
                summary: "No history buffer selected",
                detail: "Use M-x inspect followed by a JSON-quoted session ID first.",
            });
            return;
        };
        if older && !position.has_earlier {
            self.publish_message(MessageEvent {
                source: MessageSource::Ui,
                severity: MessageSeverity::Info,
                summary: "No earlier history page",
                detail: "The current history window is already at the oldest page.",
            });
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
            Err(error) => self.publish_message(MessageEvent {
                source: MessageSource::Ui,
                severity: MessageSeverity::Error,
                summary: "Session switch failed",
                detail: &format!("Current session retained: {error}"),
            }),
            Ok(opened) => {
                self.prompt_history.restore(&mut self.composer);
                *gateway = SharedGateway::new(
                    JournalGateway::new(base_gateway.clone(), opened.journal)
                        .with_tool_offer(marlin_agent_runtime::ModelToolCatalog::new().offer()),
                );
                self.transcript.clear();
                self.history = Arc::new(Vec::new());
                self.needs_replay = false;
                self.pending_tool_request = false;
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
                            "Read-only history: model requests are disabled. Use M-x session with this ID to load full context if it fits.",
                        );
                        if position.has_earlier {
                            text.push_str(" M-x history-older loads earlier turns.");
                        }
                        if position.has_newer {
                            text.push_str(" M-x history returns to recent turns.");
                        }
                        self.publish_message(MessageEvent {
                            source: MessageSource::Ui,
                            severity: MessageSeverity::Info,
                            summary: "Read-only history",
                            detail: &text,
                        });
                    }
                }
            }
        }
        true
    }

    pub(crate) fn restore(&mut self, turns: Vec<ReplayTurn>) {
        self.persistent = true;
        for turn in turns {
            self.push_prompt(turn.prompt.clone());
            let Some(answer_end) = self.restore_checkpoints(&turn) else {
                self.needs_replay = true;
                self.publish_message(MessageEvent {
                    source: MessageSource::Gateway,
                    severity: MessageSeverity::Error,
                    summary: "Invalid checkpoint ranges",
                    detail: "The session cannot be continued.",
                });
                continue;
            };
            let unverified_thinking = !turn.thinking.is_empty()
                && self.history_position.is_none()
                && !matches!(&turn.outcome,
                    ReplayOutcome::Completed(message) | ReplayOutcome::ToolRequested(message)
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
                ReplayOutcome::ToolRequested(message) => {
                    self.pending_tool_request = true;
                    self.restore_answer_suffix(
                        "Assistant",
                        &message.content[answer_end..],
                        merge_answer,
                        false,
                    );
                    self.publish_message(MessageEvent {
                        source: MessageSource::Gateway,
                        severity: MessageSeverity::Warning,
                        summary: "Tool review pending",
                        detail: &format!(
                            "The model requested {} tool(s). Execution and further turns await authorization.",
                            message.tool_calls.len(),
                        ),
                    });
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
                    self.publish_message(MessageEvent {
                        source: MessageSource::Gateway,
                        severity: MessageSeverity::Error,
                        summary: "Historical request failed",
                        detail: &diagnostic,
                    });
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
                    self.publish_message(MessageEvent {
                        source: MessageSource::Gateway,
                        severity: MessageSeverity::Error,
                        summary: "Historical failure",
                        detail: &format!("{diagnostic}. Remote retirement is unverified; further turns are blocked."),
                    });
                }
                ReplayOutcome::LegacyCancelled { partial } => {
                    self.needs_replay = true;
                    self.restore_answer_suffix(
                        "Assistant",
                        &partial[answer_end..],
                        merge_answer,
                        false,
                    );
                    self.publish_message(MessageEvent {
                        source: MessageSource::Gateway,
                        severity: MessageSeverity::Error,
                        summary: "Historical cancellation uncertain",
                        detail: "Historical local cancellation has no remote retirement evidence; further turns are blocked.",
                    });
                }
                ReplayOutcome::Unfinished { partial } => {
                    self.needs_replay = true;
                    self.restore_answer_suffix(
                        "Assistant (recovered prefix)",
                        &partial[answer_end..],
                        merge_answer,
                        false,
                    );
                    self.publish_message(MessageEvent {
                        source: MessageSource::Gateway,
                        severity: MessageSeverity::Error,
                        summary: "Unfinished prior turn",
                        detail: "Ownership is unknown; automatic retry is disabled.",
                    });
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
                    self.publish_message(MessageEvent {
                        source: MessageSource::Gateway,
                        severity: MessageSeverity::Error,
                        summary: "Uncertain prior turn",
                        detail: &message,
                    });
                }
                ReplayOutcome::Omitted => self.publish_message(MessageEvent {
                    source: MessageSource::Ui,
                    severity: MessageSeverity::Warning,
                    summary: "History turn omitted",
                    detail: "Turn contents exceed the history page budget and were not displayed.",
                }),
            }
            if unverified_thinking {
                self.publish_message(MessageEvent {
                    source: MessageSource::Gateway,
                    severity: MessageSeverity::Error,
                    summary: "Recorded thinking unverified",
                    detail: "Recorded thinking is not yet verified against a provider continuation; further requests are blocked.",
                });
            }
        }
        if self.history_position.is_none() && self.has_incompatible_continuation() {
            self.publish_message(MessageEvent {
                source: MessageSource::Gateway,
                severity: MessageSeverity::Warning,
                summary: "Restored continuation route mismatch",
                detail: "Select the original model route before sending.",
            });
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
                self.push_answer(role, String::new());
            }
        } else if merge {
            let index = self.transcript.len() - 1;
            self.transcript.append_content(index, suffix);
        } else {
            self.push_answer(role, suffix.to_owned());
        }
    }

    pub(super) fn restore_checkpoints(&mut self, turn: &ReplayTurn) -> Option<usize> {
        let answer = match &turn.outcome {
            ReplayOutcome::Completed(message) | ReplayOutcome::ToolRequested(message) => {
                message.content.as_str()
            }
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
            let content = source[span.start..span.end].to_owned();
            if matches!(span.channel, ConversationCheckpointChannel::Thinking) {
                if let Some(policy) = &self.transcript_policy {
                    self.transcript.push_thinking(
                        policy.thinking_expanded_role.clone(),
                        &policy.thinking_collapsed_role,
                        content,
                        policy.default_collapsed,
                    );
                } else {
                    self.transcript.push((role.into(), content));
                }
            } else {
                self.push_answer(role, content);
            }
        }
        Some(answer_end)
    }
}
