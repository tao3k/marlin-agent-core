// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Durable review and pre-effect claim for a pending model tool batch.

use marlin_agent_protocol::{
    ModelGatewayMessage, ModelToolCall, ModelToolCallId, tool_gateway_message,
};
use marlin_agent_storage::{SessionEventRecord, SessionEventStore, SessionId};
use serde::{Deserialize, Serialize};

use super::{
    ConversationJournal, ConversationJournalError, ConversationPendingToolRequest,
    ConversationTurnId, write_with_authority,
};

pub(super) const REVIEW_KIND: &str = "marlin.conversation.tool.review.v1";
pub(super) const CLAIM_KIND: &str = "marlin.conversation.tool.claim.v1";
pub(super) const RESULTS_KIND: &str = "marlin.conversation.tool.results.v1";
const MAX_RESULTS_BODY_BYTES: usize = 4 * 1024 * 1024;
const MAX_RESULT_TEXT_BYTES: usize = 1024 * 1024;

/// Durable state of the one review for a pending model tool batch.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConversationToolReviewStatus {
    AwaitingReview,
    Approved,
    Rejected,
    ExecutionClaimed,
    /// Replay cannot know whether a claimed external effect ran.
    ExecutionUncertain,
    ResultsCommitted,
}

/// A review decision. Approval alone never permits tool execution.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ConversationToolReviewDecision {
    ApproveOnce,
    Reject,
}

/// Exact durable request identity presented by the Mini Buffer.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ConversationToolRequestKey {
    session: SessionId,
    turn: ConversationTurnId,
    call_ids: Vec<ModelToolCallId>,
}

impl ConversationToolRequestKey {
    pub fn session(&self) -> &SessionId {
        &self.session
    }

    pub fn turn(&self) -> ConversationTurnId {
        self.turn
    }

    pub fn call_ids(&self) -> &[ModelToolCallId] {
        &self.call_ids
    }
}

/// User selection tied to the request that was actually displayed.
pub struct ConversationToolReviewRequest {
    key: ConversationToolRequestKey,
    decision: ConversationToolReviewDecision,
}

impl ConversationToolReviewRequest {
    pub fn new(key: ConversationToolRequestKey, decision: ConversationToolReviewDecision) -> Self {
        Self { key, decision }
    }
}

/// Committed review event. Its approval still requires an execution claim.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ConversationToolReviewReceipt {
    key: ConversationToolRequestKey,
    decision: ConversationToolReviewDecision,
}

impl ConversationToolReviewReceipt {
    pub fn key(&self) -> &ConversationToolRequestKey {
        &self.key
    }

    pub fn decision(&self) -> ConversationToolReviewDecision {
        self.decision
    }

    /// A rejected batch has one correlated denial for each requested call.
    /// No execution claim or handler result is implied by these messages.
    pub fn denial_messages(&self) -> Option<Vec<ModelGatewayMessage>> {
        (self.decision == ConversationToolReviewDecision::Reject).then(|| {
            self.key
                .call_ids
                .iter()
                .map(|id| tool_gateway_message(id.clone(), "Tool execution denied by the user."))
                .collect()
        })
    }
}

/// Unique committed claim that must be consumed before invoking handlers.
/// It cannot be reconstructed by the Mini Buffer or cloned for a retry.
pub struct ConversationToolExecutionClaim {
    session: SessionId,
    turn: ConversationTurnId,
    calls: Vec<ModelToolCall>,
}

/// A bounded outcome for one provider call, in the original request order.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ConversationToolResultOutcome {
    Completed(String),
    Failed(String),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ConversationToolCallResult {
    pub id: ModelToolCallId,
    pub outcome: ConversationToolResultOutcome,
}

/// The committed post-effect observation. It is valid only for the exact
/// previously claimed batch and is never inferred from a restarted process.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ConversationToolResultsReceipt {
    key: ConversationToolRequestKey,
    results: Vec<ConversationToolCallResult>,
}

impl ConversationToolResultsReceipt {
    pub fn key(&self) -> &ConversationToolRequestKey {
        &self.key
    }

    pub fn results(&self) -> &[ConversationToolCallResult] {
        &self.results
    }

    /// Provider-neutral correlated messages in the committed call order.
    pub fn gateway_messages(&self) -> Vec<ModelGatewayMessage> {
        self.results
            .iter()
            .map(|result| {
                let content = match &result.outcome {
                    ConversationToolResultOutcome::Completed(output) => output.clone(),
                    ConversationToolResultOutcome::Failed(error) => format!("Tool error: {error}"),
                };
                tool_gateway_message(result.id.clone(), content)
            })
            .collect()
    }
}

impl ConversationToolExecutionClaim {
    pub fn key(&self) -> ConversationToolRequestKey {
        ConversationToolRequestKey {
            session: self.session.clone(),
            turn: self.turn,
            call_ids: self.calls.iter().map(|call| call.id.clone()).collect(),
        }
    }

    pub fn session(&self) -> &SessionId {
        &self.session
    }

    pub fn turn(&self) -> ConversationTurnId {
        self.turn
    }

    pub fn into_calls(self) -> Vec<ModelToolCall> {
        self.calls
    }
}

impl ConversationPendingToolRequest {
    /// Snapshot the committed request identity without copying tool arguments.
    pub fn key(&self) -> ConversationToolRequestKey {
        ConversationToolRequestKey {
            session: self.session.clone(),
            turn: self.turn,
            call_ids: self.calls.iter().map(|call| call.id.clone()).collect(),
        }
    }

    pub fn status(&self) -> ConversationToolReviewStatus {
        self.status
    }

    pub fn review_receipt(&self) -> Option<&ConversationToolReviewReceipt> {
        self.review_receipt.as_ref()
    }

    pub fn results_receipt(&self) -> Option<&ConversationToolResultsReceipt> {
        self.results_receipt.as_ref()
    }

    pub fn continuation_messages(&self) -> Option<Vec<ModelGatewayMessage>> {
        match self.status {
            ConversationToolReviewStatus::Rejected => self
                .review_receipt
                .as_ref()
                .and_then(ConversationToolReviewReceipt::denial_messages),
            ConversationToolReviewStatus::ResultsCommitted => self
                .results_receipt
                .as_ref()
                .map(ConversationToolResultsReceipt::gateway_messages),
            _ => None,
        }
    }
}

fn validate_results(
    pending: &ConversationPendingToolRequest,
    receipt: &ConversationToolResultsReceipt,
) -> Result<(), ConversationJournalError> {
    if pending.key() != receipt.key || pending.calls.len() != receipt.results.len() {
        return Err(ConversationJournalError::InvalidEvent(
            "tool results do not match the claimed batch".into(),
        ));
    }
    for (call, result) in pending.calls.iter().zip(&receipt.results) {
        let text = match &result.outcome {
            ConversationToolResultOutcome::Completed(text)
            | ConversationToolResultOutcome::Failed(text) => text,
        };
        if call.id != result.id || text.len() > MAX_RESULT_TEXT_BYTES {
            return Err(ConversationJournalError::InvalidEvent(
                "invalid tool result identity or size".into(),
            ));
        }
    }
    Ok(())
}

impl<S: SessionEventStore> ConversationJournal<S> {
    /// Commit a decision only for the exact displayed request.
    /// A failed or cancelled write relinquishes review authority until replay.
    pub async fn review_tool_request(
        &mut self,
        request: ConversationToolReviewRequest,
        created_at_unix_ms: i64,
    ) -> Result<ConversationToolReviewReceipt, ConversationJournalError> {
        let pending = self.pending_tool_request.as_ref().ok_or_else(|| {
            ConversationJournalError::InvalidEvent("no pending tool request".into())
        })?;
        if !self.owns_pending_review {
            return Err(ConversationJournalError::UnownedTurn);
        }
        if pending.status != ConversationToolReviewStatus::AwaitingReview
            || pending.key() != request.key
        {
            return Err(ConversationJournalError::InvalidEvent(
                "stale or already reviewed tool request".into(),
            ));
        }
        let receipt = ConversationToolReviewReceipt {
            key: request.key,
            decision: request.decision,
        };
        let id = pending.turn;
        let event = self.record(
            id,
            id.review_event_id()?,
            REVIEW_KIND,
            Some(id.finish_event_id()?),
            serde_json::to_vec(&receipt)?,
            created_at_unix_ms,
        )?;
        write_with_authority(
            &mut self.owns_pending_review,
            self.storage.append_session_event(event),
        )
        .await?;
        let pending = self.pending_tool_request.as_mut().expect("validated above");
        pending.status = match receipt.decision {
            ConversationToolReviewDecision::ApproveOnce => ConversationToolReviewStatus::Approved,
            ConversationToolReviewDecision::Reject => ConversationToolReviewStatus::Rejected,
        };
        pending.review_receipt = Some(receipt.clone());
        Ok(receipt)
    }

    /// Persist an at-most-once claim before any handler side effect.
    /// A replayed claim is uncertain and cannot issue another claim.
    pub async fn claim_tool_execution(
        &mut self,
        receipt: &ConversationToolReviewReceipt,
        created_at_unix_ms: i64,
    ) -> Result<ConversationToolExecutionClaim, ConversationJournalError> {
        let pending = self.pending_tool_request.as_ref().ok_or_else(|| {
            ConversationJournalError::InvalidEvent("no pending tool request".into())
        })?;
        if !self.owns_pending_review {
            return Err(ConversationJournalError::UnownedTurn);
        }
        if pending.status != ConversationToolReviewStatus::Approved
            || pending.review_receipt.as_ref() != Some(receipt)
            || receipt.decision != ConversationToolReviewDecision::ApproveOnce
        {
            return Err(ConversationJournalError::InvalidEvent(
                "tool execution lacks a matching approval".into(),
            ));
        }
        let id = pending.turn;
        let event = self.record(
            id,
            id.claim_event_id()?,
            CLAIM_KIND,
            Some(id.review_event_id()?),
            serde_json::to_vec(receipt)?,
            created_at_unix_ms,
        )?;
        write_with_authority(
            &mut self.owns_pending_review,
            self.storage.append_session_event(event),
        )
        .await?;
        self.owns_pending_review = false;
        self.owns_pending_result = true;
        let pending = self.pending_tool_request.as_mut().expect("validated above");
        pending.status = ConversationToolReviewStatus::ExecutionClaimed;
        Ok(ConversationToolExecutionClaim {
            session: pending.session.clone(),
            turn: pending.turn,
            calls: pending.calls.clone(),
        })
    }

    /// Record the exact result batch after the claimed handlers have returned.
    /// A failed write leaves the claim uncertain; no caller may infer success.
    pub async fn record_tool_results(
        &mut self,
        key: ConversationToolRequestKey,
        results: Vec<ConversationToolCallResult>,
        created_at_unix_ms: i64,
    ) -> Result<ConversationToolResultsReceipt, ConversationJournalError> {
        let pending = self.pending_tool_request.as_ref().ok_or_else(|| {
            ConversationJournalError::InvalidEvent("no claimed tool request".into())
        })?;
        if !self.owns_pending_result
            || pending.status != ConversationToolReviewStatus::ExecutionClaimed
        {
            return Err(ConversationJournalError::UnownedTurn);
        }
        let receipt = ConversationToolResultsReceipt { key, results };
        validate_results(pending, &receipt)?;
        let body = serde_json::to_vec(&receipt)?;
        if body.len() > MAX_RESULTS_BODY_BYTES {
            return Err(ConversationJournalError::InvalidEvent(
                "tool results exceed journal limit".into(),
            ));
        }
        let id = pending.turn;
        let event = self.record(
            id,
            id.tool_results_event_id()?,
            RESULTS_KIND,
            Some(id.claim_event_id()?),
            body,
            created_at_unix_ms,
        )?;
        write_with_authority(
            &mut self.owns_pending_result,
            self.storage.append_session_event(event),
        )
        .await?;
        self.owns_pending_result = false;
        let pending = self.pending_tool_request.as_mut().expect("validated above");
        pending.status = ConversationToolReviewStatus::ResultsCommitted;
        pending.results_receipt = Some(receipt.clone());
        Ok(receipt)
    }

    pub(super) fn replay_tool_review(
        &mut self,
        event: &SessionEventRecord,
        id: ConversationTurnId,
    ) -> Result<(), ConversationJournalError> {
        if event.causality_parent_event_id != Some(id.finish_event_id()?) {
            return Err(ConversationJournalError::InvalidEvent(
                "invalid tool review parent".into(),
            ));
        }
        let receipt: ConversationToolReviewReceipt = serde_json::from_slice(&event.body)?;
        let pending = self.pending_tool_request.as_mut().ok_or_else(|| {
            ConversationJournalError::InvalidEvent("tool review without a request".into())
        })?;
        if pending.status != ConversationToolReviewStatus::AwaitingReview
            || pending.key() != receipt.key
            || pending.turn != id
        {
            return Err(ConversationJournalError::InvalidEvent(
                "stale or duplicate tool review".into(),
            ));
        }
        pending.status = match receipt.decision {
            ConversationToolReviewDecision::ApproveOnce => ConversationToolReviewStatus::Approved,
            ConversationToolReviewDecision::Reject => ConversationToolReviewStatus::Rejected,
        };
        pending.review_receipt = Some(receipt);
        self.owns_pending_review = true;
        Ok(())
    }

    pub(super) fn replay_tool_claim(
        &mut self,
        event: &SessionEventRecord,
        id: ConversationTurnId,
    ) -> Result<(), ConversationJournalError> {
        if event.causality_parent_event_id != Some(id.review_event_id()?) {
            return Err(ConversationJournalError::InvalidEvent(
                "invalid tool claim parent".into(),
            ));
        }
        let receipt: ConversationToolReviewReceipt = serde_json::from_slice(&event.body)?;
        let pending = self.pending_tool_request.as_mut().ok_or_else(|| {
            ConversationJournalError::InvalidEvent("tool claim without a request".into())
        })?;
        if pending.turn != id
            || pending.status != ConversationToolReviewStatus::Approved
            || pending.review_receipt.as_ref() != Some(&receipt)
            || receipt.decision != ConversationToolReviewDecision::ApproveOnce
        {
            return Err(ConversationJournalError::InvalidEvent(
                "stale or duplicate tool execution claim".into(),
            ));
        }
        pending.status = ConversationToolReviewStatus::ExecutionUncertain;
        self.owns_pending_review = false;
        self.owns_pending_result = false;
        Ok(())
    }

    pub(super) fn replay_tool_results(
        &mut self,
        event: &SessionEventRecord,
        id: ConversationTurnId,
    ) -> Result<(), ConversationJournalError> {
        if event.causality_parent_event_id != Some(id.claim_event_id()?)
            || event.body.len() > MAX_RESULTS_BODY_BYTES
        {
            return Err(ConversationJournalError::InvalidEvent(
                "invalid tool result record".into(),
            ));
        }
        let receipt: ConversationToolResultsReceipt = serde_json::from_slice(&event.body)?;
        let pending = self.pending_tool_request.as_mut().ok_or_else(|| {
            ConversationJournalError::InvalidEvent("tool results without a request".into())
        })?;
        if pending.turn != id || pending.status != ConversationToolReviewStatus::ExecutionUncertain
        {
            return Err(ConversationJournalError::InvalidEvent(
                "stale or duplicate tool results".into(),
            ));
        }
        validate_results(pending, &receipt)?;
        pending.status = ConversationToolReviewStatus::ResultsCommitted;
        pending.results_receipt = Some(receipt);
        Ok(())
    }
}
