// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Typed turn identities, recorded observations, and outcome validation.

use std::{error::Error, fmt};

use marlin_agent_protocol::{
    ModelEndpoint, ModelGatewayCompletionResponse, ModelGatewayMessage, ModelGatewayMessageRole,
    ModelStreamInterruption, ModelToolCall,
};
use marlin_agent_storage::{EventId, SessionId, StorageError, TurnId};
use serde::{Deserialize, Serialize};

use super::tool_review::{
    ConversationToolRequestKey, ConversationToolResultsReceipt, ConversationToolReviewReceipt,
    ConversationToolReviewStatus,
};

pub(super) const TURN_PREFIX: &str = "conversation:";

/// Maximum contiguous channel runs in one recorded turn.
pub const CONVERSATION_MAX_CHECKPOINT_SEGMENTS: usize = 4096;

/// Ordinal in a single conversation, encoded as a fixed-width storage key.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct ConversationTurnId(pub(super) u64);

/// Client-generated identity of one submission, scoped to a project and Session.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct ConversationSubmissionId(String);

impl ConversationSubmissionId {
    pub fn new(value: String) -> Option<Self> {
        (!value.is_empty()
            && value.len() <= 128
            && value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_')))
        .then_some(Self(value))
    }

    pub(super) fn record_turn(&self) -> Result<TurnId, StorageError> {
        TurnId::new(format!("conversation-submission:{}", self.0))
    }
}

impl ConversationTurnId {
    /// Returns the numeric ordinal, starting at one.
    pub fn get(self) -> u64 {
        self.0
    }

    pub fn from_ordinal(value: std::num::NonZeroU64) -> Self {
        Self(value.get())
    }

    pub(super) fn storage_id(self) -> Result<TurnId, StorageError> {
        TurnId::new(format!("{TURN_PREFIX}{:020}", self.0))
    }

    pub(super) fn from_storage_id(id: &TurnId) -> Option<Self> {
        let value = id.as_str().strip_prefix(TURN_PREFIX)?;
        if value.len() != 20 || !value.bytes().all(|byte| byte.is_ascii_digit()) {
            return None;
        }
        value.parse().ok().map(Self::from_ordinal)
    }

    pub(super) fn start_event_id(self) -> Result<EventId, StorageError> {
        EventId::new(format!("{TURN_PREFIX}{:020}:00-start", self.0))
    }

    pub(super) fn finish_event_id(self) -> Result<EventId, StorageError> {
        EventId::new(format!("{TURN_PREFIX}{:020}:01-finish", self.0))
    }

    pub(super) fn review_event_id(self) -> Result<EventId, StorageError> {
        EventId::new(format!("{TURN_PREFIX}{:020}:02-tool-review", self.0))
    }

    pub(super) fn claim_event_id(self) -> Result<EventId, StorageError> {
        EventId::new(format!("{TURN_PREFIX}{:020}:03-tool-claim", self.0))
    }

    pub(super) fn tool_results_event_id(self) -> Result<EventId, StorageError> {
        EventId::new(format!("{TURN_PREFIX}{:020}:04-tool-results", self.0))
    }

    pub(super) fn uncertain_event_id(self) -> Result<EventId, StorageError> {
        EventId::new(format!("{TURN_PREFIX}{:020}:01-uncertain", self.0))
    }

    pub(super) fn checkpoint_event_id(self, sequence: u64) -> Result<EventId, StorageError> {
        EventId::new(format!(
            "{TURN_PREFIX}{:020}:00-start:chunk:{sequence:020}",
            self.0
        ))
    }
}

/// A user request admitted before model dispatch.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ConversationTurnStart {
    pub endpoint: ModelEndpoint,
    pub user_text: String,
}

/// A new provider request caused by committed tool results, never by a
/// fabricated user prompt. Its source is the exact reviewed result batch.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ConversationToolContinuationStart {
    pub endpoint: ModelEndpoint,
    pub source: ConversationToolRequestKey,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ConversationTurnSource {
    User(ConversationTurnStart),
    ToolResults(ConversationToolContinuationStart),
}

impl ConversationTurnSource {
    pub fn endpoint(&self) -> &ModelEndpoint {
        match self {
            Self::User(start) => &start.endpoint,
            Self::ToolResults(start) => &start.endpoint,
        }
    }

    pub fn user_text(&self) -> Option<&str> {
        match self {
            Self::User(start) => Some(&start.user_text),
            Self::ToolResults(_) => None,
        }
    }
}

/// A committed model-tool request awaiting runtime review. This observation
/// grants no authority to execute a tool.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConversationPendingToolRequest {
    pub(super) session: SessionId,
    pub(super) turn: ConversationTurnId,
    pub(super) endpoint: ModelEndpoint,
    pub(super) assistant: ModelGatewayMessage,
    pub(super) calls: Vec<ModelToolCall>,
    pub(super) status: ConversationToolReviewStatus,
    pub(super) review_receipt: Option<ConversationToolReviewReceipt>,
    pub(super) results_receipt: Option<ConversationToolResultsReceipt>,
}

impl ConversationPendingToolRequest {
    pub fn session(&self) -> &SessionId {
        &self.session
    }

    pub fn turn(&self) -> ConversationTurnId {
        self.turn
    }

    pub fn calls(&self) -> &[ModelToolCall] {
        &self.calls
    }

    pub fn endpoint(&self) -> &ModelEndpoint {
        &self.endpoint
    }

    pub fn assistant(&self) -> &ModelGatewayMessage {
        &self.assistant
    }
}

/// A recorded observation; uncertainty does not claim runtime retirement.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ConversationTurnOutcome {
    Completed {
        response: ModelGatewayCompletionResponse,
    },
    /// Provider retired with a tool request; execution remains unapproved.
    ToolRequested {
        response: ModelGatewayCompletionResponse,
    },
    Failed {
        diagnostic: String,
        partial_text: String,
    },
    CancelledLocally {
        partial_text: String,
    },
    /// The gateway established that it did not dispatch the request.
    RejectedBeforeDispatch {
        diagnostic: String,
    },
    /// A local observation that does not establish remote retirement.
    Uncertain {
        reason: ConversationUncertaintyReason,
    },
}

/// Observations that may release a turn's local admission claim.
#[derive(Clone, Debug, PartialEq)]
pub enum ConversationTerminalObservation {
    Completed {
        response: ModelGatewayCompletionResponse,
    },
    ToolRequested {
        response: ModelGatewayCompletionResponse,
    },
    RejectedBeforeDispatch {
        diagnostic: String,
    },
}

/// Typed cause of an observed but unresolved turn.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ConversationUncertaintyReason {
    StreamOpenTimeout,
    StreamOpenTransport,
    StreamOpenUnclassified,
    StreamInterrupted { reason: ModelStreamInterruption },
}

/// One replayed turn. Missing outcome means it was not observed to finish.
#[derive(Clone, PartialEq)]
pub struct ConversationTurn {
    pub id: ConversationTurnId,
    pub start: ConversationTurnSource,
    pub outcome: Option<ConversationTurnOutcome>,
    pub tool_results: Option<ConversationToolResultsReceipt>,
    pub tool_denial: Option<ConversationToolReviewReceipt>,
    /// Durable prefix of streamed text, possibly shorter than observed output.
    pub partial_text: String,
    /// Recorded thinking prefix. It is an observation, not a validated provider continuation.
    pub partial_thinking: String,
    /// Ordered channel runs; offsets address the corresponding UTF-8 prefix.
    pub checkpoint_spans: Vec<ConversationCheckpointSpan>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConversationCheckpointChannel {
    Text,
    Thinking,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConversationCheckpointSpan {
    pub channel: ConversationCheckpointChannel,
    pub start: usize,
    pub end: usize,
}

impl fmt::Debug for ConversationTurn {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ConversationTurn")
            .field("id", &self.id)
            .field("start", &self.start)
            .field("outcome", &self.outcome)
            .field("tool_results", &self.tool_results)
            .field("tool_denial", &self.tool_denial)
            .field("partial_text", &self.partial_text)
            .field("partial_thinking_bytes", &self.partial_thinking.len())
            .field("checkpoint_spans", &self.checkpoint_spans)
            .finish()
    }
}

impl ConversationTurn {
    pub fn continuation_messages(&self) -> Option<Vec<ModelGatewayMessage>> {
        self.tool_results
            .as_ref()
            .map(ConversationToolResultsReceipt::gateway_messages)
            .or_else(|| {
                self.tool_denial
                    .as_ref()
                    .and_then(ConversationToolReviewReceipt::denial_messages)
            })
    }

    pub(super) fn can_append_checkpoint(&self, channel: ConversationCheckpointChannel) -> bool {
        self.checkpoint_spans
            .last()
            .is_some_and(|span| span.channel == channel)
            || self.checkpoint_spans.len() < CONVERSATION_MAX_CHECKPOINT_SEGMENTS
    }

    pub(super) fn append_checkpoint(
        &mut self,
        channel: ConversationCheckpointChannel,
        delta: &str,
    ) {
        let target = match channel {
            ConversationCheckpointChannel::Text => &mut self.partial_text,
            ConversationCheckpointChannel::Thinking => &mut self.partial_thinking,
        };
        let start = target.len();
        target.push_str(delta);
        let end = target.len();
        if let Some(last) = self
            .checkpoint_spans
            .last_mut()
            .filter(|span| span.channel == channel)
        {
            last.end = end;
        } else {
            self.checkpoint_spans.push(ConversationCheckpointSpan {
                channel,
                start,
                end,
            });
        }
    }
}

pub(super) fn validate_checkpoint_prefix(
    prefix: &str,
    outcome: &ConversationTurnOutcome,
) -> Result<(), ConversationJournalError> {
    let text = match outcome {
        ConversationTurnOutcome::Completed { response }
        | ConversationTurnOutcome::ToolRequested { response } => response
            .choices
            .iter()
            .find(|choice| choice.message.role == ModelGatewayMessageRole::Assistant)
            .map(|choice| choice.message.content.as_str())
            .unwrap_or(""),
        ConversationTurnOutcome::Failed { partial_text, .. }
        | ConversationTurnOutcome::CancelledLocally { partial_text } => partial_text,
        ConversationTurnOutcome::RejectedBeforeDispatch { .. } => "",
        ConversationTurnOutcome::Uncertain { .. } => {
            return Err(ConversationJournalError::InvalidEvent(
                "uncertain outcome has no terminal text".into(),
            ));
        }
    };
    if !text.starts_with(prefix) {
        return Err(ConversationJournalError::InvalidEvent(
            "terminal text contradicts checkpoint".into(),
        ));
    }
    Ok(())
}

pub(super) fn validate_outcome(
    outcome: &ConversationTurnOutcome,
) -> Result<(), ConversationJournalError> {
    match outcome {
        ConversationTurnOutcome::Completed { response } => {
            response
                .choices
                .iter()
                .find(|choice| choice.message.role == ModelGatewayMessageRole::Assistant)
                .ok_or_else(|| {
                    ConversationJournalError::InvalidEvent(
                        "completion has no assistant choice".into(),
                    )
                })?;
            if response
                .choices
                .iter()
                .any(|choice| !choice.message.tool_calls.is_empty())
            {
                return Err(ConversationJournalError::InvalidEvent(
                    "tool request recorded as completed answer".into(),
                ));
            }
        }
        ConversationTurnOutcome::ToolRequested { response }
            if response.choices.len() != 1
                || response.choices[0].message.role != ModelGatewayMessageRole::Assistant
                || response.choices[0].message.tool_calls.is_empty()
                || response.choices[0]
                    .message
                    .validate_tool_contract()
                    .is_err() =>
        {
            return Err(ConversationJournalError::InvalidEvent(
                "invalid pending tool request".into(),
            ));
        }
        _ => {}
    }
    Ok(())
}

pub(super) fn pending_tool_request_from_outcome(
    session: &SessionId,
    turn: ConversationTurnId,
    start: &ConversationTurnSource,
    outcome: &ConversationTurnOutcome,
) -> Option<ConversationPendingToolRequest> {
    let ConversationTurnOutcome::ToolRequested { response } = outcome else {
        return None;
    };
    let assistant = response.choices.first()?.message.clone();
    let calls = assistant.tool_calls.clone();
    Some(ConversationPendingToolRequest {
        session: session.clone(),
        turn,
        endpoint: start.endpoint().clone(),
        assistant,
        calls,
        status: ConversationToolReviewStatus::AwaitingReview,
        review_receipt: None,
        results_receipt: None,
    })
}

/// Storage, validation, or state-transition failure for a conversation journal.
#[derive(Debug)]
pub enum ConversationJournalError {
    Storage(StorageError),
    Encoding(serde_json::Error),
    InvalidEvent(String),
    UnfinishedTurn,
    /// Historical terminal record lacks evidence that remote work retired.
    LegacyOutcomeUnresolved,
    ToolRequestPending,
    StaleTurn,
    SubmissionConflict,
    UnownedTurn,
    WrongActiveTurn,
    OrdinalExhausted,
    ReplayStopped,
    ReplayInconsistent,
}

impl From<StorageError> for ConversationJournalError {
    fn from(value: StorageError) -> Self {
        Self::Storage(value)
    }
}
impl From<serde_json::Error> for ConversationJournalError {
    fn from(value: serde_json::Error) -> Self {
        Self::Encoding(value)
    }
}
impl fmt::Display for ConversationJournalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Storage(error) => write!(f, "conversation storage: {error}"),
            Self::Encoding(error) => write!(f, "conversation encoding: {error}"),
            Self::InvalidEvent(error) => write!(f, "invalid conversation event: {error}"),
            Self::UnfinishedTurn => f.write_str("conversation has an unfinished turn"),
            Self::LegacyOutcomeUnresolved => {
                f.write_str("historical failure or cancellation has no remote retirement evidence")
            }
            Self::ToolRequestPending => f.write_str("conversation has a pending tool request"),
            Self::StaleTurn => f.write_str("Session turn cursor is stale"),
            Self::SubmissionConflict => {
                f.write_str("submission ID was already used with different input")
            }
            Self::UnownedTurn => {
                f.write_str("replayed unfinished turn is owned by an unknown process")
            }
            Self::WrongActiveTurn => f.write_str("conversation turn is not active"),
            Self::OrdinalExhausted => f.write_str("conversation ordinal exhausted"),
            Self::ReplayStopped => f.write_str("conversation replay stopped by visitor"),
            Self::ReplayInconsistent => f.write_str(
                "session events changed or were incomplete during replay; reopen the session",
            ),
        }
    }
}
impl Error for ConversationJournalError {}
