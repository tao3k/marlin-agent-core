// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Restore transcript and gateway state from recorded turns.

use super::{
    App, Arc, ConversationCheckpointChannel, MessageEvent, MessageSeverity, MessageSource,
    ReplayOutcome, ReplayTurn, matches_observed_anthropic_thinking, user_gateway_message,
};

impl App {
    pub(crate) fn restore(&mut self, turns: Vec<ReplayTurn>) {
        self.persistent = true;
        for turn in turns {
            if let Some(prompt) = &turn.prompt {
                self.push_prompt(prompt.clone());
            }
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
                    self.pending_tool_request = false;
                    self.restore_answer_suffix(
                        "Assistant",
                        &message.content[answer_end..],
                        merge_answer,
                        true,
                    );
                    if self.history_position.is_none() {
                        let history = Arc::make_mut(&mut self.history);
                        if let Some(prompt) = turn.prompt {
                            history.push(user_gateway_message(prompt));
                        }
                        history.push(message);
                    }
                }
                ReplayOutcome::ToolRequested(message) => {
                    self.pending_tool_request = true;
                    if self.history_position.is_none() {
                        let history = Arc::make_mut(&mut self.history);
                        if let Some(prompt) = turn.prompt {
                            history.push(user_gateway_message(prompt));
                        }
                        history.push(message.clone());
                        if let Some(messages) = &turn.tool_messages {
                            history.extend(messages.iter().cloned());
                        }
                    }
                    self.restore_answer_suffix(
                        "Assistant",
                        &message.content[answer_end..],
                        merge_answer,
                        false,
                    );
                    let detail = if turn.tool_messages.is_some() {
                        "Committed tool review awaits provider continuation.".to_owned()
                    } else {
                        format!(
                            "The model requested {} tool(s). Execution and further turns await authorization.",
                            message.tool_calls.len(),
                        )
                    };
                    self.publish_message(MessageEvent {
                        source: MessageSource::Gateway,
                        severity: MessageSeverity::Warning,
                        summary: if turn.tool_messages.is_some() {
                            "Tool continuation pending"
                        } else {
                            "Tool review pending"
                        },
                        detail: &detail,
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
