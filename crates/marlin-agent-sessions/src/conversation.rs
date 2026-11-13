// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Ordered conversation turns over the runtime-owned session event store.

use std::{error::Error, fmt, future::Future};

use marlin_agent_protocol::{
    ModelEndpoint, ModelGatewayCompletionResponse, ModelGatewayMessageRole, ModelStreamInterruption,
};
use marlin_agent_storage::{
    AgentId, EventId, ProjectId, SessionEventPageRequest, SessionEventRecord, SessionEventStore,
    SessionId, StorageError, StoragePageLimit, TurnId,
};
use serde::{Deserialize, Serialize};

#[path = "conversation_events.rs"]
mod events;
pub use events::{ConversationEvent, ConversationEventEntry, ConversationEventReader};

const START_KIND: &str = "marlin.conversation.turn.started.v1";
const END_KIND: &str = "marlin.conversation.turn.finished.v1";
const CHECKPOINT_KIND: &str = "marlin.conversation.turn.checkpoint.v1";
const THINKING_CHECKPOINT_KIND: &str = "marlin.conversation.turn.thinking-checkpoint.v1";
const UNCERTAIN_KIND: &str = "marlin.conversation.turn.uncertain.v1";
const TURN_PREFIX: &str = "conversation:";
const SUBMISSION_KIND: &str = "marlin.conversation.submission.v1";
const MAX_CHECKPOINT_BYTES: usize = 128 * 1024;
const MAX_PARTIAL_BYTES: usize = 16 * 1024 * 1024;
/// Maximum contiguous channel runs in one recorded turn.
pub const CONVERSATION_MAX_CHECKPOINT_SEGMENTS: usize = 4096;

#[cfg(test)]
#[path = "../tests/unit/authority_write.rs"]
mod authority_write_tests;

async fn write_with_authority<E>(
    owned: &mut bool,
    write: impl Future<Output = Result<(), E>>,
) -> Result<(), E> {
    *owned = false;
    write.await?;
    *owned = true;
    Ok(())
}

/// Ordinal in a single conversation, encoded as a fixed-width storage key.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct ConversationTurnId(u64);

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

    fn record_turn(&self) -> Result<TurnId, StorageError> {
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

    fn storage_id(self) -> Result<TurnId, StorageError> {
        TurnId::new(format!("{TURN_PREFIX}{:020}", self.0))
    }

    fn start_event_id(self) -> Result<EventId, StorageError> {
        EventId::new(format!("{TURN_PREFIX}{:020}:00-start", self.0))
    }

    fn finish_event_id(self) -> Result<EventId, StorageError> {
        EventId::new(format!("{TURN_PREFIX}{:020}:01-finish", self.0))
    }

    fn uncertain_event_id(self) -> Result<EventId, StorageError> {
        EventId::new(format!("{TURN_PREFIX}{:020}:01-uncertain", self.0))
    }

    fn checkpoint_event_id(self, sequence: u64) -> Result<EventId, StorageError> {
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
    pub start: ConversationTurnStart,
    pub outcome: Option<ConversationTurnOutcome>,
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
            .field("partial_text", &self.partial_text)
            .field("partial_thinking_bytes", &self.partial_thinking.len())
            .field("checkpoint_spans", &self.checkpoint_spans)
            .finish()
    }
}

impl ConversationTurn {
    fn can_append_checkpoint(&self, channel: ConversationCheckpointChannel) -> bool {
        self.checkpoint_spans
            .last()
            .is_some_and(|span| span.channel == channel)
            || self.checkpoint_spans.len() < CONVERSATION_MAX_CHECKPOINT_SEGMENTS
    }

    fn append_checkpoint(&mut self, channel: ConversationCheckpointChannel, delta: &str) {
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

/// Single-client journal over a shared runtime storage backend.
///
/// A started-event key is claimed before dispatch; concurrent stale writers
/// cannot claim the same ordinal. This is not a process lease: a replayed
/// unfinished turn must not be automatically retried or marked dead.
pub struct ConversationJournal<S> {
    storage: S,
    project: ProjectId,
    session: SessionId,
    agent: AgentId,
    next: u64,
    active: Option<ConversationTurn>,
    owns_active: bool,
    next_checkpoint: u64,
    legacy_outcome_unresolved: bool,
    pending_tool_request: bool,
}

impl<S: SessionEventStore> ConversationJournal<S> {
    /// Scan pages in storage order and visit each validated turn once.
    /// The callback controls whether and how replayed history is retained;
    /// its observations remain provisional until this method returns `Ok`.
    /// The storage interface does not promise a cross-page snapshot. An
    /// append-only count check rejects a scan if committed events change.
    pub async fn open(
        storage: S,
        project: ProjectId,
        session: SessionId,
        agent: AgentId,
        mut visit: impl FnMut(&ConversationTurn),
    ) -> Result<Self, ConversationJournalError> {
        Self::open_controlled(storage, project, session, agent, |turn| {
            visit(turn);
            true
        })
        .await
    }

    /// Replay until complete or until the visitor rejects a turn. Rejection
    /// stops the scan and returns no journal; prior visits are provisional.
    pub async fn open_controlled(
        storage: S,
        project: ProjectId,
        session: SessionId,
        agent: AgentId,
        mut visit: impl FnMut(&ConversationTurn) -> bool,
    ) -> Result<Self, ConversationJournalError> {
        let mut journal = Self {
            storage,
            project,
            session,
            agent,
            next: 1,
            active: None,
            owns_active: false,
            next_checkpoint: 1,
            legacy_outcome_unresolved: false,
            pending_tool_request: false,
        };
        let before = journal
            .storage
            .count_session_events(&journal.project, &journal.session)
            .await?;
        let mut observed = 0u64;
        let mut cursor = None;
        let mut last_key: Option<(TurnId, EventId)> = None;
        loop {
            let mut request = SessionEventPageRequest::new(
                journal.project.clone(),
                journal.session.clone(),
                StoragePageLimit::new(16)?,
            );
            if let Some(after) = cursor {
                request = request.after(after);
            }
            let page = journal.storage.list_session_events_page(request).await?;
            if page.items.len() > 16 {
                return Err(ConversationJournalError::InvalidEvent(
                    "storage page exceeds requested limit".into(),
                ));
            }
            let has_items = !page.items.is_empty();
            for event in page.items {
                observed = observed
                    .checked_add(1)
                    .ok_or(ConversationJournalError::OrdinalExhausted)?;
                if event.project_id != journal.project || event.session_id != journal.session {
                    return Err(ConversationJournalError::InvalidEvent(
                        "storage page crosses session scope".into(),
                    ));
                }
                let key = (event.turn_id.clone(), event.event_id.clone());
                if last_key.as_ref().is_some_and(|last| last >= &key) {
                    return Err(ConversationJournalError::InvalidEvent(
                        "storage page order is not strictly increasing".into(),
                    ));
                }
                last_key = Some(key);
                if !event.turn_id.as_str().starts_with(TURN_PREFIX) {
                    continue;
                }
                journal.replay_record(event, &mut visit)?;
            }
            match page.next_cursor {
                Some(next) => {
                    if !has_items
                        || last_key.as_ref().is_none_or(|(turn, event)| {
                            turn != next.turn_id() || event != next.event_id()
                        })
                    {
                        return Err(ConversationJournalError::InvalidEvent(
                            "storage cursor does not match page tail".into(),
                        ));
                    }
                    cursor = Some(next);
                }
                None => break,
            }
        }
        if let Some(turn) = &journal.active
            && !visit(turn)
        {
            return Err(ConversationJournalError::ReplayStopped);
        }
        let after = journal
            .storage
            .count_session_events(&journal.project, &journal.session)
            .await?;
        if before != after || observed != after.get() {
            return Err(ConversationJournalError::ReplayInconsistent);
        }
        Ok(journal)
    }

    /// Return an unfinished turn. Its prior owner may still be running.
    pub fn unfinished(&self) -> Option<&ConversationTurn> {
        self.active.as_ref()
    }

    /// Persist and claim a new turn before any external model request.
    pub async fn start(
        &mut self,
        start: ConversationTurnStart,
        created_at_unix_ms: i64,
    ) -> Result<ConversationTurnId, ConversationJournalError> {
        self.commit_start(start, None, created_at_unix_ms).await
    }

    /// Atomically claim both the submission identity and its new turn ordinal.
    pub async fn start_submission(
        &mut self,
        submission: &ConversationSubmissionId,
        start: ConversationTurnStart,
        after_turn: u64,
        created_at_unix_ms: i64,
    ) -> Result<ConversationTurnId, ConversationJournalError> {
        if self.next - 1 != after_turn {
            return Err(ConversationJournalError::StaleTurn);
        }
        self.commit_start(start, Some(submission), created_at_unix_ms)
            .await
    }

    async fn commit_start(
        &mut self,
        start: ConversationTurnStart,
        submission: Option<&ConversationSubmissionId>,
        created_at_unix_ms: i64,
    ) -> Result<ConversationTurnId, ConversationJournalError> {
        if self.active.is_some() {
            return Err(ConversationJournalError::UnfinishedTurn);
        }
        if self.legacy_outcome_unresolved {
            return Err(ConversationJournalError::LegacyOutcomeUnresolved);
        }
        if self.pending_tool_request {
            return Err(ConversationJournalError::ToolRequestPending);
        }
        start
            .endpoint
            .validate_contract()
            .map_err(|error| ConversationJournalError::InvalidEvent(error.to_string()))?;
        if start.user_text.trim().is_empty() {
            return Err(ConversationJournalError::InvalidEvent(
                "user text is empty".into(),
            ));
        }
        let id = ConversationTurnId(self.next);
        let next = self
            .next
            .checked_add(1)
            .ok_or(ConversationJournalError::OrdinalExhausted)?;
        let event = self.record(
            id,
            id.start_event_id()?,
            START_KIND,
            None,
            serde_json::to_vec(&start)?,
            created_at_unix_ms,
        )?;
        if let Some(submission) = submission {
            // This Session-wide key and the normal turn start key are claimed
            // in one transaction. Only a successful writer can dispatch.
            let claim = SessionEventRecord {
                project_id: event.project_id.clone(),
                session_id: event.session_id.clone(),
                agent_id: event.agent_id.clone(),
                turn_id: submission.record_turn()?,
                event_id: EventId::new("submission")?,
                event_kind: SUBMISSION_KIND.into(),
                causality_parent_event_id: None,
                body: serde_json::to_vec(&id)?,
                created_at_unix_ms,
            };
            self.storage
                .append_session_events_atomically(vec![event, claim])
                .await?;
        } else {
            self.storage.append_session_event(event).await?;
        }
        self.next = next;
        self.active = Some(ConversationTurn {
            id,
            start,
            outcome: None,
            partial_text: String::new(),
            partial_thinking: String::new(),
            checkpoint_spans: Vec::new(),
        });
        self.owns_active = true;
        self.next_checkpoint = 1;
        Ok(id)
    }

    /// Persist an ordered UTF-8 delta from the active owner's observed output.
    /// A failed storage write leaves the turn active and relinquishes authority.
    pub async fn checkpoint(
        &mut self,
        id: ConversationTurnId,
        delta: &str,
        created_at_unix_ms: i64,
    ) -> Result<(), ConversationJournalError> {
        self.checkpoint_kind(
            id,
            delta,
            created_at_unix_ms,
            ConversationCheckpointChannel::Text,
        )
        .await
    }

    /// Persist provider-returned thinking in the same ordered event sequence
    /// as answer text. This does not validate a provider continuation.
    pub async fn checkpoint_thinking(
        &mut self,
        id: ConversationTurnId,
        delta: &str,
        created_at_unix_ms: i64,
    ) -> Result<(), ConversationJournalError> {
        self.checkpoint_kind(
            id,
            delta,
            created_at_unix_ms,
            ConversationCheckpointChannel::Thinking,
        )
        .await
    }

    async fn checkpoint_kind(
        &mut self,
        id: ConversationTurnId,
        delta: &str,
        created_at_unix_ms: i64,
        kind: ConversationCheckpointChannel,
    ) -> Result<(), ConversationJournalError> {
        let active = self
            .active
            .as_ref()
            .filter(|turn| turn.id == id)
            .ok_or(ConversationJournalError::WrongActiveTurn)?;
        if !self.owns_active {
            return Err(ConversationJournalError::UnownedTurn);
        }
        if !active.can_append_checkpoint(kind) {
            return Err(ConversationJournalError::InvalidEvent(
                "too many alternating checkpoint segments".into(),
            ));
        }
        if delta.is_empty()
            || delta.len() > MAX_CHECKPOINT_BYTES
            || active
                .partial_text
                .len()
                .checked_add(active.partial_thinking.len())
                .and_then(|n| n.checked_add(delta.len()))
                .is_none_or(|n| n > MAX_PARTIAL_BYTES)
        {
            return Err(ConversationJournalError::InvalidEvent(
                "invalid checkpoint size".into(),
            ));
        }
        let next = self
            .next_checkpoint
            .checked_add(1)
            .ok_or(ConversationJournalError::OrdinalExhausted)?;
        let event = self.record(
            id,
            id.checkpoint_event_id(self.next_checkpoint)?,
            match kind {
                ConversationCheckpointChannel::Text => CHECKPOINT_KIND,
                ConversationCheckpointChannel::Thinking => THINKING_CHECKPOINT_KIND,
            },
            Some(id.start_event_id()?),
            serde_json::to_vec(delta)?,
            created_at_unix_ms,
        )?;
        write_with_authority(
            &mut self.owns_active,
            self.storage.append_session_event(event),
        )
        .await?;
        let active = self.active.as_mut().expect("active turn checked above");
        active.append_checkpoint(kind, delta);
        self.next_checkpoint = next;
        Ok(())
    }

    /// Append a terminal observation. Local cancellation is not retirement evidence.
    /// A failed storage write relinquishes authority.
    pub async fn finish(
        &mut self,
        id: ConversationTurnId,
        observation: ConversationTerminalObservation,
        created_at_unix_ms: i64,
    ) -> Result<(), ConversationJournalError> {
        if self.active.as_ref().map(|turn| turn.id) != Some(id) {
            return Err(ConversationJournalError::WrongActiveTurn);
        }
        if !self.owns_active {
            return Err(ConversationJournalError::UnownedTurn);
        }
        let outcome = match observation {
            ConversationTerminalObservation::Completed { response } => {
                ConversationTurnOutcome::Completed { response }
            }
            ConversationTerminalObservation::ToolRequested { response } => {
                ConversationTurnOutcome::ToolRequested { response }
            }
            ConversationTerminalObservation::RejectedBeforeDispatch { diagnostic } => {
                if self.next_checkpoint != 1 {
                    return Err(ConversationJournalError::InvalidEvent(
                        "pre-dispatch rejection follows a checkpoint".into(),
                    ));
                }
                ConversationTurnOutcome::RejectedBeforeDispatch { diagnostic }
            }
        };
        validate_outcome(&outcome)?;
        validate_checkpoint_prefix(
            &self
                .active
                .as_ref()
                .expect("active turn checked above")
                .partial_text,
            &outcome,
        )?;
        let event = self.record(
            id,
            id.finish_event_id()?,
            END_KIND,
            Some(id.start_event_id()?),
            serde_json::to_vec(&outcome)?,
            created_at_unix_ms,
        )?;
        write_with_authority(
            &mut self.owns_active,
            self.storage.append_session_event(event),
        )
        .await?;
        self.active = None;
        self.owns_active = false;
        self.pending_tool_request =
            matches!(outcome, ConversationTurnOutcome::ToolRequested { .. });
        Ok(())
    }

    /// Record a local uncertainty without declaring the turn retired.
    /// A failed write also relinquishes local authority because commit may be uncertain.
    pub async fn mark_uncertain(
        &mut self,
        id: ConversationTurnId,
        reason: ConversationUncertaintyReason,
        created_at_unix_ms: i64,
    ) -> Result<(), ConversationJournalError> {
        if self.active.as_ref().map(|turn| turn.id) != Some(id) {
            return Err(ConversationJournalError::WrongActiveTurn);
        }
        if !self.owns_active {
            return Err(ConversationJournalError::UnownedTurn);
        }
        if self.next_checkpoint != 1
            && !matches!(
                reason,
                ConversationUncertaintyReason::StreamInterrupted { .. }
            )
        {
            return Err(ConversationJournalError::InvalidEvent(
                "stream-open uncertainty follows a checkpoint".into(),
            ));
        }
        let event = self.record(
            id,
            id.uncertain_event_id()?,
            UNCERTAIN_KIND,
            Some(id.start_event_id()?),
            serde_json::to_vec(&reason)?,
            created_at_unix_ms,
        )?;
        self.owns_active = false;
        self.storage.append_session_event(event).await?;
        self.active
            .as_mut()
            .expect("active turn checked above")
            .outcome = Some(ConversationTurnOutcome::Uncertain { reason });
        Ok(())
    }

    fn record(
        &self,
        id: ConversationTurnId,
        event_id: EventId,
        event_kind: &str,
        parent: Option<EventId>,
        body: Vec<u8>,
        timestamp: i64,
    ) -> Result<SessionEventRecord, ConversationJournalError> {
        Ok(SessionEventRecord {
            project_id: self.project.clone(),
            session_id: self.session.clone(),
            agent_id: self.agent.clone(),
            turn_id: id.storage_id()?,
            event_id,
            event_kind: event_kind.into(),
            causality_parent_event_id: parent,
            body,
            created_at_unix_ms: timestamp,
        })
    }

    fn replay_record(
        &mut self,
        event: SessionEventRecord,
        visit: &mut impl FnMut(&ConversationTurn) -> bool,
    ) -> Result<(), ConversationJournalError> {
        if event.agent_id != self.agent {
            return Err(ConversationJournalError::InvalidEvent(
                "unexpected agent identity".into(),
            ));
        }
        let ordinal = event
            .turn_id
            .as_str()
            .strip_prefix(TURN_PREFIX)
            .and_then(|value| {
                (value.len() == 20 && value.bytes().all(|byte| byte.is_ascii_digit()))
                    .then(|| value.parse::<u64>().ok())
                    .flatten()
            })
            .ok_or_else(|| ConversationJournalError::InvalidEvent("invalid turn ordinal".into()))?;
        let id = ConversationTurnId(ordinal);
        if self.active.as_ref().is_some_and(|turn| {
            matches!(
                turn.outcome,
                Some(ConversationTurnOutcome::Uncertain { .. })
            )
        }) {
            return Err(ConversationJournalError::InvalidEvent(
                "events follow an uncertain turn".into(),
            ));
        }
        match events::decode_event(id, &event)? {
            ConversationEvent::Started(start) => {
                if self.pending_tool_request || self.active.is_some() || id.0 != self.next {
                    return Err(ConversationJournalError::InvalidEvent(
                        "noncontiguous or overlapping start".into(),
                    ));
                }
                self.next = self
                    .next
                    .checked_add(1)
                    .ok_or(ConversationJournalError::OrdinalExhausted)?;
                self.active = Some(ConversationTurn {
                    id,
                    start,
                    outcome: None,
                    partial_text: String::new(),
                    partial_thinking: String::new(),
                    checkpoint_spans: Vec::new(),
                });
                self.next_checkpoint = 1;
            }
            ConversationEvent::Checkpoint {
                channel,
                text: delta,
            } if event.event_id == id.checkpoint_event_id(self.next_checkpoint)? => {
                let turn = self
                    .active
                    .as_mut()
                    .filter(|turn| turn.id == id)
                    .ok_or(ConversationJournalError::WrongActiveTurn)?;
                if !turn.can_append_checkpoint(channel) {
                    return Err(ConversationJournalError::InvalidEvent(
                        "too many alternating checkpoint segments".into(),
                    ));
                }
                if delta.is_empty()
                    || delta.len() > MAX_CHECKPOINT_BYTES
                    || turn
                        .partial_text
                        .len()
                        .checked_add(turn.partial_thinking.len())
                        .and_then(|n| n.checked_add(delta.len()))
                        .is_none_or(|n| n > MAX_PARTIAL_BYTES)
                {
                    return Err(ConversationJournalError::InvalidEvent(
                        "invalid checkpoint size".into(),
                    ));
                }
                turn.append_checkpoint(channel, &delta);
                self.next_checkpoint = self
                    .next_checkpoint
                    .checked_add(1)
                    .ok_or(ConversationJournalError::OrdinalExhausted)?;
            }
            ConversationEvent::Outcome(outcome)
                if !matches!(outcome, ConversationTurnOutcome::Uncertain { .. }) =>
            {
                if matches!(
                    outcome,
                    ConversationTurnOutcome::RejectedBeforeDispatch { .. }
                ) && self.next_checkpoint != 1
                {
                    return Err(ConversationJournalError::InvalidEvent(
                        "pre-dispatch rejection follows a checkpoint".into(),
                    ));
                }
                let mut turn = self
                    .active
                    .take()
                    .filter(|turn| turn.id == id)
                    .ok_or(ConversationJournalError::WrongActiveTurn)?;
                validate_checkpoint_prefix(&turn.partial_text, &outcome)?;
                if matches!(
                    outcome,
                    ConversationTurnOutcome::Failed { .. }
                        | ConversationTurnOutcome::CancelledLocally { .. }
                ) {
                    self.legacy_outcome_unresolved = true;
                }
                turn.outcome = Some(outcome);
                self.pending_tool_request = matches!(
                    turn.outcome,
                    Some(ConversationTurnOutcome::ToolRequested { .. })
                );
                if !visit(&turn) {
                    return Err(ConversationJournalError::ReplayStopped);
                }
            }
            ConversationEvent::Outcome(ConversationTurnOutcome::Uncertain { reason }) => {
                if self.next_checkpoint != 1
                    && !matches!(
                        reason,
                        ConversationUncertaintyReason::StreamInterrupted { .. }
                    )
                {
                    return Err(ConversationJournalError::InvalidEvent(
                        "stream-open uncertainty follows a checkpoint".into(),
                    ));
                }
                let turn = self
                    .active
                    .as_mut()
                    .filter(|turn| turn.id == id)
                    .ok_or(ConversationJournalError::WrongActiveTurn)?;
                turn.outcome = Some(ConversationTurnOutcome::Uncertain { reason });
                self.owns_active = false;
            }
            _ => {
                return Err(ConversationJournalError::InvalidEvent(
                    "unknown conversation event".into(),
                ));
            }
        }
        Ok(())
    }
}

fn validate_checkpoint_prefix(
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

fn validate_outcome(outcome: &ConversationTurnOutcome) -> Result<(), ConversationJournalError> {
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
