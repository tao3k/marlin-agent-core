// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Scheme-selected Mini Buffer Ask bound to one durable tool request.

use super::publish_error;
use super::{
    App, MessageEvent, MessageSeverity, MessageSource, ModelStreamingGateway, ReplySender,
    Transcript, UiHost,
};
use marlin_agent_sessions::{
    ConversationToolReviewDecision, ConversationToolReviewRequest, ConversationToolReviewStatus,
};
use marlin_agent_ui::UiToolReviewDecision;

pub(super) fn open_tool_review(app: &mut App, ui: &UiHost<'_>) -> bool {
    if !app.pending_tool_request
        || app.history_position.is_some()
        || app.tool_review_prompt.is_some()
        || app.tool_review_inflight.is_some()
        || app.tool_review_suppressed
    {
        return false;
    }
    let Some(pending) = app
        .tool_review
        .as_ref()
        .and_then(|review| review.try_pending())
    else {
        return false;
    };
    if pending.status() != ConversationToolReviewStatus::AwaitingReview {
        app.tool_review_suppressed = true;
        return false;
    }
    let menu = match ui.tool_review_menu() {
        Ok(menu) => menu,
        Err(error) => {
            publish_error(app, MessageSource::Ui, "Tool review menu failed", &error);
            app.tool_review_suppressed = true;
            return true;
        }
    };
    let mut entries = Vec::with_capacity(pending.calls().len());
    let mut calls = Vec::with_capacity(pending.calls().len());
    for call in pending.calls() {
        let arguments = extract_structured_arguments(&call.arguments);
        let compact_args = compact_single_line(
            &serde_json::to_string(&call.arguments)
                .unwrap_or_else(|e| format!("<invalid-arguments-json: {e}>")),
            96,
        );
        let role = format!("Tool · {} [{}]", call.name.as_str(), call.id.as_str());
        let body = format_compact_ask_arguments(&arguments);
        calls.push(super::state::ToolReviewCallSummary {
            call_id: call.id.as_str().to_owned(),
            tool_name: call.name.as_str().to_owned(),
            arguments,
            compact_args,
        });
        entries.push((role, body));
    }
    app.tool_review_prompt = Some(super::state::ToolReviewPrompt {
        key: pending.key(),
        calls,
        menu,
        details: Transcript::new(entries),
        selected: 0,
        scroll: 0,
    });
    true
}

pub(super) fn submit_tool_review(app: &mut App, runtime: &tokio::runtime::Runtime) {
    let Some(prompt) = app.tool_review_prompt.take() else {
        return;
    };
    let Some(review) = app.tool_review.clone() else {
        return;
    };
    let Some(choice) = prompt.menu.choices.get(prompt.selected) else {
        return;
    };
    let decision = match choice.decision {
        UiToolReviewDecision::ApproveOnce => ConversationToolReviewDecision::ApproveOnce,
        UiToolReviewDecision::Reject => ConversationToolReviewDecision::Reject,
    };
    let now = std::time::Instant::now();
    let initial_state = if decision == ConversationToolReviewDecision::ApproveOnce {
        "Running"
    } else {
        "Rejected"
    };
    let mut names = Vec::with_capacity(prompt.calls.len());
    for call in &prompt.calls {
        names.push(format!("{} [{}]", call.tool_name, call.call_id));
        if app.tool_executions.len() >= 64 {
            app.tool_executions.remove(0);
        }
        app.tool_executions.push(super::state::ToolExecutionRecord {
            call_id: call.call_id.clone(),
            tool_name: call.tool_name.clone(),
            arguments: call.arguments.clone(),
            state: initial_state,
            started_at: now,
            duration_ms: (initial_state == "Rejected").then_some(0),
            otel_span: "runtime.tool.execute",
            output_summary: call.compact_args.clone(),
            output_lines: Vec::new(),
        });
    }
    app.tool_executions_revision = app.tool_executions_revision.saturating_add(1);
    if decision == ConversationToolReviewDecision::ApproveOnce && !names.is_empty() {
        app.publish_message(MessageEvent {
            source: MessageSource::Runtime,
            severity: MessageSeverity::Info,
            summary: "Tool execution started",
            detail: &format!("span=runtime.tool.execute · {}", names.join(", ")),
        });
    }
    let request = ConversationToolReviewRequest::new(prompt.key, decision);
    let context = app.runtime_owner.context();
    let (sender, receiver) = tokio::sync::oneshot::channel();
    app.tool_review_inflight = Some(receiver);
    app.tool_review_suppressed = true;
    app.tool_review_task = Some(app.runtime_owner.spawn_on(runtime, async move {
        let _ = sender.send(review.decide(request, context).await);
    }));
}

pub(super) fn poll_tool_review<G: ModelStreamingGateway + Clone>(
    app: &mut App,
    gateway: &G,
    runtime: &tokio::runtime::Runtime,
    tx: &ReplySender,
) -> bool {
    let Some(receiver) = app.tool_review_inflight.as_mut() else {
        if app.pending_tool_request
            && app.pending.is_none()
            && !app.tool_review_suppressed
            && let Some(messages) = app
                .tool_review
                .as_ref()
                .and_then(|review| review.try_pending())
                .and_then(|pending| pending.continuation_messages())
        {
            app.tool_review_suppressed = true;
            app.continue_tool_turn(gateway, runtime, tx, &messages);
            return true;
        }
        return false;
    };
    let result = match receiver.try_recv() {
        Ok(result) => result,
        Err(tokio::sync::oneshot::error::TryRecvError::Empty) => return false,
        Err(tokio::sync::oneshot::error::TryRecvError::Closed) => {
            app.tool_review_inflight = None;
            app.tool_review_task = None;
            app.publish_message(MessageEvent {
                source: MessageSource::Gateway,
                severity: MessageSeverity::Error,
                summary: "Tool review interrupted",
                detail: "Inspect the session journal before another action.",
            });
            return true;
        }
    };
    app.tool_review_inflight = None;
    app.tool_review_task = None;
    match result {
        Ok(super::super::journal_gateway::JournalToolReviewOutcome::Rejected(receipt)) => {
            app.publish_message(MessageEvent {
                source: MessageSource::Gateway,
                severity: MessageSeverity::Info,
                summary: "Tool request denied",
                detail: &format!(
                    "Request in turn {} was denied; continuing the model turn.",
                    receipt.key().turn().get()
                ),
            });
            if let Some(messages) = receipt.denial_messages() {
                app.continue_tool_turn(gateway, runtime, tx, &messages);
            }
        }
        Ok(super::super::journal_gateway::JournalToolReviewOutcome::Results(receipt)) => {
            app.sync_search_index_background(runtime.handle());
            let mut summaries = Vec::with_capacity(receipt.results().len());
            for result in receipt.results() {
                let (state, preview) = match &result.outcome {
                    marlin_agent_sessions::ConversationToolResultOutcome::Completed(output) => {
                        ("Completed", summarize_tool_output(output))
                    }
                    marlin_agent_sessions::ConversationToolResultOutcome::Failed(error) => {
                        ("Failed", compact_single_line(error, 110))
                    }
                };
                if let Some(record) = app
                    .tool_executions
                    .iter_mut()
                    .rev()
                    .find(|rec| rec.call_id == result.id.as_str())
                {
                    let elapsed =
                        u64::try_from(record.started_at.elapsed().as_millis()).unwrap_or(u64::MAX);
                    record.state = state;
                    record.duration_ms = Some(elapsed);
                    record.output_summary = preview.clone();
                    let raw_body = match &result.outcome {
                        marlin_agent_sessions::ConversationToolResultOutcome::Completed(output) => {
                            record.output_lines = output
                                .lines()
                                .take(24)
                                .map(super::presentation::safe_text)
                                .collect();
                            format_structured_tool_output(output)
                        }
                        marlin_agent_sessions::ConversationToolResultOutcome::Failed(error) => {
                            let msg = format!("error: {}", super::presentation::safe_text(error));
                            record.output_lines = vec![msg.clone()];
                            msg
                        }
                    };
                    let collapsed_role = format!(
                        "▸ Tool · {} [{}] · {state} ({elapsed}ms) · {preview}  (*Processes* for details)",
                        record.tool_name, record.call_id
                    );
                    let expanded_role = format!(
                        "▾ Tool · {} [{}] · {state} ({elapsed}ms)",
                        record.tool_name, record.call_id
                    );
                    // Collapsed by default in Chat so raw tool receipts do not clutter
                    // the conversation; full structured trace lives in `*Processes*`.
                    app.transcript
                        .push_collapsible(expanded_role, collapsed_role, raw_body, true);
                    summaries.push(format!(
                        "{} [{}] {state} ({elapsed}ms): {preview}",
                        record.tool_name, record.call_id
                    ));
                } else {
                    summaries.push(format!("{} {state}: {preview}", result.id.as_str()));
                }
            }
            app.tool_executions_revision = app.tool_executions_revision.saturating_add(1);
            app.publish_message(MessageEvent {
                source: MessageSource::Gateway,
                severity: MessageSeverity::Info,
                summary: "Tool results committed",
                detail: &format!(
                    "{} result(s) committed · {}",
                    receipt.results().len(),
                    summaries.join(" | ")
                ),
            });
            app.continue_tool_turn(gateway, runtime, tx, &receipt.gateway_messages());
        }
        Err(error) => publish_error(app, MessageSource::Gateway, "Tool review failed", &error),
    }
    true
}

fn compact_single_line(raw: &str, max_chars: usize) -> String {
    let mut out = String::new();
    let mut count = 0usize;
    let mut prev_space = false;
    for ch in raw.chars() {
        let mapped = if ch.is_control() || ch.is_whitespace() {
            ' '
        } else {
            ch
        };
        if mapped == ' ' {
            if prev_space || out.is_empty() {
                continue;
            }
            prev_space = true;
        } else {
            prev_space = false;
        }
        if count >= max_chars {
            out.push('…');
            break;
        }
        out.push(mapped);
        count += 1;
    }
    out.trim_end().to_owned()
}

fn summarize_tool_output(output: &str) -> String {
    let mut parts = Vec::new();
    for line in output.lines().take(8) {
        let trimmed = line.trim();
        if trimmed.starts_with("mode:")
            || trimmed.starts_with("hits:")
            || trimmed.starts_with("results:")
            || trimmed.starts_with("worlds:")
            || trimmed.starts_with("elapsed_us:")
        {
            parts.push(trimmed);
        }
    }
    if parts.is_empty() {
        compact_single_line(output, 110)
    } else {
        compact_single_line(&parts.join(" · "), 110)
    }
}

fn extract_structured_arguments(
    arguments: &serde_json::Map<String, serde_json::Value>,
) -> Vec<(String, String)> {
    arguments
        .iter()
        .map(|(key, value)| {
            let rendered = match value {
                serde_json::Value::String(s) => super::presentation::safe_text(s),
                other => super::presentation::safe_text(
                    &serde_json::to_string_pretty(other)
                        .unwrap_or_else(|e| format!("<invalid-json: {e}>")),
                ),
            };
            (key.clone(), rendered)
        })
        .collect()
}

fn format_compact_ask_arguments(arguments: &[(String, String)]) -> String {
    arguments
        .iter()
        .map(|(key, value)| format!("  {key}: {}", compact_single_line(value, 140)))
        .collect::<Vec<_>>()
        .join("\n")
}

fn format_structured_tool_output(output: &str) -> String {
    const MAX_PREVIEW_LINES: usize = 12;
    let total = output.lines().count();
    let mut shown: Vec<String> = output
        .lines()
        .take(MAX_PREVIEW_LINES)
        .map(super::presentation::safe_text)
        .collect();
    if total > MAX_PREVIEW_LINES {
        shown.push(format!(
            "… (+{} more lines; open M-x list-processes for full structured receipt)",
            total - MAX_PREVIEW_LINES
        ));
    }
    shown.join("\n")
}
