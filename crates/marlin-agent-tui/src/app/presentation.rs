// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Scheme-selected Buffer views and bounded runtime presentation facts.

use super::state::App;
use super::{Command, RuntimeEdgePolicyReceipt, UiBufferId, UiContent, UiEvent, UiWorkbench};

const MAX_DYNAMIC_BUFFER_ITEMS: usize = 256;
const MAX_DYNAMIC_BUFFER_BYTES: usize = 32 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum BufferView {
    Conversation,
    Messages,
    Named(UiBufferId),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum MessageSource {
    Ui,
    Workspace,
    Input,
    Gateway,
    Runtime,
}

impl MessageSource {
    fn label(self) -> &'static str {
        match self {
            Self::Ui => "UI",
            Self::Workspace => "Workspace",
            Self::Input => "Input",
            Self::Gateway => "Gateway",
            Self::Runtime => "Runtime",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum MessageSeverity {
    Info,
    Warning,
    Error,
}

impl MessageSeverity {
    fn label(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct MessageEvent<'a> {
    pub(crate) source: MessageSource,
    pub(crate) severity: MessageSeverity,
    pub(crate) summary: &'a str,
    pub(crate) detail: &'a str,
}

/// The most recent local transport observation, not a persistent HTTP link.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum GatewayObservation {
    Unchecked,
    Online,
    Offline,
    Unknown,
}

/// Runtime facts select a Scheme-declared Modeline presentation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ModelineActivity {
    Idle,
    Runtime,
    Workspace,
    Recovery,
    ContinuationUnavailable,
    History,
    Working,
    ReadingWorkspace,
}

impl App {
    /// Read-only preview of a selected, admitted buffer command. The Scheme
    /// window binding changes only when the user confirms the command.
    pub(crate) fn preview_buffer(&self) -> Option<BufferView> {
        if self.command_draft.is_none()
            || self.history_search.is_some()
            || self.tool_review_prompt.is_some()
        {
            return None;
        }
        let selected = self
            .selected_command()
            .filter(|name| self.command_candidates.contains(name))
            .or_else(|| self.command_candidates.first())?;
        let command = self
            .command_catalog
            .iter()
            .find(|command| &command.name == selected)?;
        let Command::SelectUi(UiEvent::SelectBuffer(buffer)) =
            Command::parse(command, selected.as_str())?
        else {
            return None;
        };
        match buffer.as_str() {
            "conversation" => Some(BufferView::Conversation),
            "messages" => Some(BufferView::Messages),
            _ => Some(BufferView::Named(buffer)),
        }
    }

    /// Mirror the admitted Scheme window binding for native content painting.
    /// No command or key handler may select this view independently.
    pub(crate) fn sync_window(&mut self, ui: &UiWorkbench) -> Result<(), &'static str> {
        let next = match ui.frame.selected_buffer().map(UiBufferId::as_str) {
            Some("conversation") => BufferView::Conversation,
            Some("messages") => BufferView::Messages,
            Some(id)
                if ui.regions.iter().any(|region| {
                    matches!(region,
                marlin_agent_ui::UiRegion::Extension { id: region_id, focusable: false, .. }
                    if region_id.as_str() == id)
                }) =>
            {
                BufferView::Named(UiBufferId::new(id).map_err(|_| "invalid Scheme buffer ID")?)
            }
            _ => return Err("selected Scheme buffer has no native content owner"),
        };
        if self.buffer_view != next && next == BufferView::Messages {
            self.message_scroll_up = 0;
            self.minibuffer_notice = None;
        }
        if self.buffer_view != next && matches!(&next, BufferView::Named(_)) {
            self.static_buffer_scroll = 0;
            self.minibuffer_notice = None;
        }
        self.buffer_view = next;
        Ok(())
    }

    /// Replace a named Buffer's owner-supplied view atomically. Scheme owns
    /// its identity and title; this content does not enter conversation state.
    pub(crate) fn replace_dynamic_buffer_content(
        &mut self,
        id: UiBufferId,
        content: Vec<UiContent>,
    ) -> Result<bool, &'static str> {
        if content.len() > MAX_DYNAMIC_BUFFER_ITEMS {
            return Err("dynamic Buffer has too many rows");
        }
        let bytes = content.iter().try_fold(0usize, |total, row| {
            let row_bytes = match row {
                UiContent::Text(text) => text.len(),
                UiContent::Field { label, value } => label.len().saturating_add(value.len()),
            };
            total.checked_add(row_bytes)
        });
        if bytes.is_none_or(|bytes| bytes > MAX_DYNAMIC_BUFFER_BYTES) {
            return Err("dynamic Buffer exceeds its byte budget");
        }
        if self.dynamic_buffer_content.get(&id) == Some(&content) {
            return Ok(false);
        }
        self.dynamic_buffer_content.insert(id, content);
        Ok(true)
    }

    /// Publish bounded owner content, then ask the Scheme frame to select it.
    pub(crate) fn open_dynamic_buffer(
        &mut self,
        id: UiBufferId,
        content: Vec<UiContent>,
    ) -> Result<(), &'static str> {
        self.replace_dynamic_buffer_content(id.clone(), content)?;
        self.ui_request = Some(UiEvent::SelectBuffer(id));
        Ok(())
    }

    pub(crate) fn publish_message(&mut self, event: MessageEvent<'_>) {
        let summary: String = event
            .summary
            .chars()
            .take(256)
            .map(|ch| if ch.is_control() { '�' } else { ch })
            .collect();
        let bounded_detail: String = event.detail.chars().take(16_384).collect();
        let detail = format!("{}: {}", summary, safe_text(&bounded_detail));
        let role = format!("{} · {}", event.source.label(), event.severity.label());
        if !self
            .messages
            .last()
            .is_some_and(|(previous_role, previous)| previous_role == &role && previous == &detail)
        {
            self.messages.push((role, detail));
            self.messages.retain_latest(64);
        }
        self.minibuffer_notice = Some(summary);
    }

    /// Drain the runtime's bounded event bus into the Scheme-selected Messages buffer.
    /// Keeping the receiver alive also preserves producer backpressure semantics.
    pub(crate) fn poll_runtime_events(&mut self, limit: usize) -> usize {
        let mut processed = 0;
        while processed < limit {
            let Some(event) = self.runtime_events.try_next() else {
                break;
            };
            self.publish_message(MessageEvent {
                source: MessageSource::Runtime,
                severity: MessageSeverity::Info,
                summary: &event.topic,
                detail: &event.message,
            });
            processed += 1;
        }
        processed
    }

    pub(crate) fn modeline_activity(&self) -> ModelineActivity {
        if self.runtime_receipt_open {
            ModelineActivity::Runtime
        } else if self.workspace.view().is_some() {
            ModelineActivity::Workspace
        } else if self.needs_replay {
            ModelineActivity::Recovery
        } else if self.pending_tool_request || self.unverified_thinking {
            ModelineActivity::ContinuationUnavailable
        } else if self.history_position.is_some() {
            ModelineActivity::History
        } else if self.pending.is_some() {
            ModelineActivity::Working
        } else if self.workspace.is_pending() {
            ModelineActivity::ReadingWorkspace
        } else {
            ModelineActivity::Idle
        }
    }

    pub(crate) fn set_runtime_edge(&mut self, receipt: RuntimeEdgePolicyReceipt) {
        self.runtime_content = vec![
            UiContent::Field {
                label: "Stream-open timeout".into(),
                value: receipt.timeout_ms().map_or_else(|| "Disabled".into(), |ms| format!("{ms} ms")),
            },
            UiContent::Field {
                label: "Concurrency limit".into(),
                value: receipt.concurrency_limit().map_or_else(|| "Disabled".into(), |limit| limit.to_string()),
            },
            UiContent::Field {
                label: "Load shedding".into(),
                value: if receipt.load_shed() { "Enabled" } else { "Disabled" }.into(),
            },
            UiContent::Field {
                label: "Attached layers".into(),
                value: receipt.layers().iter().map(|layer| layer.as_str()).collect::<Vec<_>>().join(" · "),
            },
            UiContent::Text("Scope: stream opening. Stream duration, tools, approvals and remote cancellation are separate policies.".into()),
        ];
        self.runtime_edge = Some(receipt);
    }
}

// Never pass untrusted terminal escape sequences to the renderer.
pub(crate) fn safe_text(text: &str) -> String {
    let mut safe = String::with_capacity(text.len());
    append_safe_text(&mut safe, text);
    safe
}

/// Sanitize directly into an owned view to avoid a second full-size copy.
pub(crate) fn append_safe_text(output: &mut String, text: &str) {
    let mut start = 0;
    for (index, character) in text.char_indices() {
        if character == '\t' || (character != '\n' && character.is_control()) {
            output.push_str(&text[start..index]);
            output.push(if character == '\t' { ' ' } else { '�' });
            start = index + character.len_utf8();
        }
    }
    output.push_str(&text[start..]);
}
