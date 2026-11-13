// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Scheme-selected Mini Buffer Ask bound to one durable tool request.

use super::runner::publish_error;
use super::{App, MessageEvent, MessageSeverity, MessageSource, Transcript, UiHost};
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
    let mut details =
        String::from("Model tool request · inspect every argument before approval\n\n");
    for call in pending.calls() {
        details.push_str(call.name.as_str());
        details.push_str("  [");
        details.push_str(call.id.as_str());
        details.push_str("]\n");
        let arguments = serde_json::to_string_pretty(&call.arguments)
            .expect("validated JSON tool arguments serialize");
        details.push_str(&super::presentation::safe_text(&arguments));
        details.push_str("\n\n");
    }
    app.tool_review_prompt = Some(super::state::ToolReviewPrompt {
        key: pending.key(),
        menu,
        details: Transcript::new(vec![(String::new(), details)]),
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
    let request = ConversationToolReviewRequest::new(prompt.key, decision);
    let context = app.runtime_owner.context();
    let (sender, receiver) = tokio::sync::oneshot::channel();
    app.tool_review_inflight = Some(receiver);
    app.tool_review_suppressed = true;
    app.tool_review_task = Some(app.runtime_owner.spawn_on(runtime, async move {
        let _ = sender.send(review.decide(request, context).await);
    }));
}

pub(super) fn poll_tool_review(app: &mut App) -> bool {
    let Some(receiver) = app.tool_review_inflight.as_mut() else {
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
        Ok(super::super::journal_gateway::JournalToolReviewOutcome::Rejected(receipt)) => app
            .publish_message(MessageEvent {
                source: MessageSource::Gateway,
                severity: MessageSeverity::Info,
                summary: "Tool request denied",
                detail: &format!("Request in turn {} was denied.", receipt.key().turn().get()),
            }),
        Ok(super::super::journal_gateway::JournalToolReviewOutcome::Results(receipt)) => app
            .publish_message(MessageEvent {
                source: MessageSource::Gateway,
                severity: MessageSeverity::Info,
                summary: "Tool results committed",
                detail: &format!(
                    "{} result(s) are recorded for provider continuation.",
                    receipt.results().len()
                ),
            }),
        Err(error) => publish_error(app, MessageSource::Gateway, "Tool review failed", &error),
    }
    true
}
