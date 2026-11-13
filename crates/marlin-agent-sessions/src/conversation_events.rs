// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Typed reads of the journal's existing committed records.

use marlin_agent_storage::SessionEventCursor;
use std::num::NonZeroU64;

use super::*;

const CANCELLATION_KIND: &str = "marlin.conversation.cancellation.v1";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ConversationRetirementStatus {
    Completed,
    TimedOut,
    CleanupUnconfirmed,
}

/// Local Runtime evidence recorded separately from the provider outcome.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ConversationCancellationEvidence {
    pub status: ConversationRetirementStatus,
    pub cancellation_requested: bool,
    pub tracker_closed: bool,
    pub timeout_ms: Option<u64>,
    pub tracked_tasks_at_shutdown: u64,
    pub remaining_tasks: u64,
}

impl ConversationCancellationEvidence {
    pub fn completed(&self) -> bool {
        self.status == ConversationRetirementStatus::Completed
    }
}

#[derive(Debug)]
pub enum ConversationEvent {
    Started(ConversationTurnStart),
    Checkpoint {
        channel: ConversationCheckpointChannel,
        text: String,
    },
    Outcome(ConversationTurnOutcome),
    ToolReview(ConversationToolReviewReceipt),
    ToolExecutionClaim(ConversationToolReviewReceipt),
}

#[derive(Debug)]
pub struct ConversationEventEntry {
    pub cursor: SessionEventCursor,
    pub event: ConversationEvent,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConversationTailState {
    Unfinished,
    Finished,
    Uncertain,
}

/// The latest committed conversation position, without transcript payloads.
#[derive(Debug)]
pub struct ConversationTail {
    pub turn: ConversationTurnId,
    pub event_id: EventId,
    pub state: ConversationTailState,
}

#[derive(Clone)]
pub struct ConversationEventReader<S> {
    storage: S,
    project: ProjectId,
    session: SessionId,
    agent: AgentId,
}

impl<S: SessionEventStore> ConversationEventReader<S> {
    pub fn new(storage: S, project: ProjectId, session: SessionId, agent: AgentId) -> Self {
        Self {
            storage,
            project,
            session,
            agent,
        }
    }

    /// Retain the first observation at each strength. A completed observation
    /// supersedes incomplete evidence, including out-of-order writes.
    pub async fn record_cancellation(
        &self,
        id: ConversationTurnId,
        evidence: &ConversationCancellationEvidence,
        created_at_unix_ms: i64,
    ) -> Result<(), ConversationJournalError> {
        let event = SessionEventRecord {
            project_id: self.project.clone(),
            session_id: self.session.clone(),
            agent_id: self.agent.clone(),
            turn_id: cancellation_key(id)?,
            event_id: cancellation_event_id(evidence.completed())?,
            event_kind: CANCELLATION_KIND.into(),
            causality_parent_event_id: Some(id.start_event_id()?),
            body: serde_json::to_vec(evidence)?,
            created_at_unix_ms,
        };
        match self.storage.append_session_event(event).await {
            Ok(()) => Ok(()),
            Err(StorageError::DuplicateSessionEvent { .. })
                if self
                    .cancellation_evidence(id)
                    .await?
                    .is_some_and(|stored| !evidence.completed() || stored.completed()) =>
            {
                Ok(())
            }
            Err(error) => Err(error.into()),
        }
    }

    pub async fn cancellation_evidence(
        &self,
        id: ConversationTurnId,
    ) -> Result<Option<ConversationCancellationEvidence>, ConversationJournalError> {
        let turn = cancellation_key(id)?;
        // Completed sorts before incomplete. One read chooses the strongest observation.
        let request = SessionEventPageRequest::new(
            self.project.clone(),
            self.session.clone(),
            StoragePageLimit::new(1)?,
        )
        .after(SessionEventCursor::new(turn.clone(), EventId::new("\0")?));
        let mut page = self.storage.list_session_events_page(request).await?;
        if page.items.len() > 1 {
            return Err(invalid());
        }
        let Some(record) = page.items.pop() else {
            return Ok(None);
        };
        if record.project_id != self.project
            || record.session_id != self.session
            || record.turn_id < turn
        {
            return Err(invalid());
        }
        if record.turn_id > turn {
            return Ok(None);
        }
        let completed = record.event_id == cancellation_event_id(true)?;
        if record.agent_id != self.agent
            || record.event_id != cancellation_event_id(completed)?
            || record.event_kind != CANCELLATION_KIND
            || record.causality_parent_event_id != Some(id.start_event_id()?)
        {
            return Err(invalid());
        }
        let evidence: ConversationCancellationEvidence = serde_json::from_slice(&record.body)?;
        if evidence.completed() != completed
            || !evidence.tracker_closed
            || (completed && evidence.remaining_tasks != 0)
        {
            return Err(invalid());
        }
        Ok(Some(evidence))
    }

    pub async fn committed_outcome(
        &self,
        id: ConversationTurnId,
    ) -> Result<Option<ConversationTurnOutcome>, ConversationJournalError> {
        let after = SessionEventCursor::new(id.storage_id()?, id.checkpoint_event_id(u64::MAX)?);
        let mut entries = self
            .page(id, Some(after), StoragePageLimit::new(1)?)
            .await?;
        match entries.pop().map(|entry| entry.event) {
            Some(ConversationEvent::Outcome(outcome)) => Ok(Some(outcome)),
            Some(_) => Err(invalid()),
            None => Ok(None),
        }
    }

    pub async fn has_committed_start(
        &self,
        id: ConversationTurnId,
    ) -> Result<bool, ConversationJournalError> {
        Ok(self
            .resolve_cursor(id, id.start_event_id()?)
            .await?
            .is_some())
    }

    /// Seek the conversation tail without scanning the transcript or loading event bodies.
    pub async fn latest_turn(&self) -> Result<Option<ConversationTail>, ConversationJournalError> {
        // Numeric conversation keys sort before the first key past `conversation:`.
        // Submission claims use `conversation-submission:` and are outside the range.
        let from = TurnId::new(TURN_PREFIX)?;
        let before = TurnId::new("conversation;")?;
        let Some(header) = self
            .storage
            .latest_session_event_header(&self.project, &self.session, &from, &before)
            .await?
        else {
            return Ok(None);
        };
        if header.agent_id != self.agent {
            return Err(invalid());
        }
        let turn =
            ConversationTurnId::from_storage_id(header.cursor.turn_id()).ok_or_else(invalid)?;
        let event_id = header.cursor.event_id();
        let start = turn.start_event_id()?;
        let state = match header.event_kind.as_str() {
            tool_review::REVIEW_KIND
                if *event_id == turn.review_event_id()?
                    && header.causality_parent_event_id == Some(turn.finish_event_id()?) =>
            {
                ConversationTailState::Uncertain
            }
            tool_review::CLAIM_KIND
                if *event_id == turn.claim_event_id()?
                    && header.causality_parent_event_id == Some(turn.review_event_id()?) =>
            {
                ConversationTailState::Uncertain
            }
            START_KIND if *event_id == start && header.causality_parent_event_id.is_none() => {
                ConversationTailState::Unfinished
            }
            kind if header.causality_parent_event_id.as_ref() == Some(&start) => match kind {
                END_KIND if *event_id == turn.finish_event_id()? => ConversationTailState::Finished,
                UNCERTAIN_KIND if *event_id == turn.uncertain_event_id()? => {
                    ConversationTailState::Uncertain
                }
                CHECKPOINT_KIND | THINKING_CHECKPOINT_KIND
                    if checkpoint_sequence(turn, event_id).is_some() =>
                {
                    ConversationTailState::Unfinished
                }
                _ => return Err(invalid()),
            },
            _ => return Err(invalid()),
        };
        Ok(Some(ConversationTail {
            turn,
            event_id: event_id.clone(),
            state,
        }))
    }

    /// Resolve a committed submission by its unique Session journal key.
    pub async fn resolve_submission(
        &self,
        submission: &ConversationSubmissionId,
        user_text: &str,
        after_turn: u64,
    ) -> Result<Option<ConversationTurnId>, ConversationJournalError> {
        let turn = submission.record_turn()?;
        let event = EventId::new("submission")?;
        let request = SessionEventPageRequest::new(
            self.project.clone(),
            self.session.clone(),
            StoragePageLimit::new(1)?,
        )
        .after(SessionEventCursor::new(turn.clone(), EventId::new("\0")?));
        let mut page = self.storage.list_session_events_page(request).await?;
        if page.items.len() > 1 {
            return Err(invalid());
        }
        let Some(record) = page.items.pop() else {
            return Ok(None);
        };
        if record.project_id != self.project || record.session_id != self.session {
            return Err(invalid());
        }
        if record.turn_id > turn {
            return Ok(None);
        }
        if record.turn_id != turn
            || record.event_id != event
            || record.agent_id != self.agent
            || record.event_kind != SUBMISSION_KIND
            || record.causality_parent_event_id.is_some()
        {
            return Err(invalid());
        }
        let id: ConversationTurnId = serde_json::from_slice(&record.body)?;
        let start = self.resolve_cursor(id, id.start_event_id()?).await?;
        let Some(ConversationEvent::Started(start)) = start.map(|entry| entry.event) else {
            return Err(invalid());
        };
        let expected_after = id.get().checked_sub(1).ok_or_else(invalid)?;
        if start.user_text != user_text || expected_after != after_turn {
            return Err(ConversationJournalError::SubmissionConflict);
        }
        Ok(Some(id))
    }

    /// Resolve a supplied cursor against a committed event in this turn.
    pub async fn resolve_cursor(
        &self,
        id: ConversationTurnId,
        event_id: EventId,
    ) -> Result<Option<ConversationEventEntry>, ConversationJournalError> {
        let lower = if event_id == id.start_event_id()? {
            EventId::new("\0")?
        } else if event_id == id.finish_event_id()? || event_id == id.uncertain_event_id()? {
            EventId::new(format!("{TURN_PREFIX}{:020}:01", id.get()))?
        } else if event_id == id.review_event_id()? {
            id.finish_event_id()?
        } else if event_id == id.claim_event_id()? {
            id.review_event_id()?
        } else {
            let Some(sequence) = checkpoint_sequence(id, &event_id) else {
                return Ok(None);
            };
            if sequence == 1 {
                id.start_event_id()?
            } else {
                id.checkpoint_event_id(sequence - 1)?
            }
        };
        let after = SessionEventCursor::new(id.storage_id()?, lower);
        Ok(self
            .page(id, Some(after), StoragePageLimit::new(1)?)
            .await?
            .into_iter()
            .find(|entry| entry.cursor.event_id() == &event_id))
    }

    /// Read a bounded, exclusive page within one conversation turn.
    pub async fn page(
        &self,
        id: ConversationTurnId,
        after: Option<SessionEventCursor>,
        limit: StoragePageLimit,
    ) -> Result<Vec<ConversationEventEntry>, ConversationJournalError> {
        let turn = id.storage_id()?;
        let mut cursor =
            after.unwrap_or(SessionEventCursor::new(turn.clone(), EventId::new("\0")?));
        if cursor.turn_id() != &turn {
            return Err(invalid());
        }
        let request =
            SessionEventPageRequest::new(self.project.clone(), self.session.clone(), limit)
                .after(cursor.clone());
        let page = self.storage.list_session_events_page(request).await?;
        if page.items.len() > limit.get() {
            return Err(invalid());
        }
        let mut entries = Vec::new();
        for record in page.items {
            if record.project_id != self.project
                || record.session_id != self.session
                || (&record.turn_id, &record.event_id) <= (cursor.turn_id(), cursor.event_id())
            {
                return Err(invalid());
            }
            if record.turn_id != turn {
                break;
            }
            if record.agent_id != self.agent {
                return Err(invalid());
            }
            let event = decode_event(id, &record)?;
            cursor = SessionEventCursor::from_record(&record);
            entries.push(ConversationEventEntry {
                cursor: cursor.clone(),
                event,
            });
        }
        Ok(entries)
    }
}

pub(super) fn decode_event(
    id: ConversationTurnId,
    record: &SessionEventRecord,
) -> Result<ConversationEvent, ConversationJournalError> {
    if record.turn_id != id.storage_id()? {
        return Err(invalid());
    }
    let start_id = id.start_event_id()?;
    if record.event_kind == START_KIND
        && record.event_id == start_id
        && record.causality_parent_event_id.is_none()
    {
        let start: ConversationTurnStart = serde_json::from_slice(&record.body)?;
        start
            .endpoint
            .validate_contract()
            .map_err(|error| ConversationJournalError::InvalidEvent(error.to_string()))?;
        if start.user_text.trim().is_empty() {
            return Err(invalid());
        }
        return Ok(ConversationEvent::Started(start));
    }
    if record.event_kind == tool_review::REVIEW_KIND
        && record.event_id == id.review_event_id()?
        && record.causality_parent_event_id == Some(id.finish_event_id()?)
    {
        let receipt: ConversationToolReviewReceipt = serde_json::from_slice(&record.body)?;
        if receipt.key().turn() != id || receipt.key().session() != &record.session_id {
            return Err(invalid());
        }
        return Ok(ConversationEvent::ToolReview(receipt));
    }
    if record.event_kind == tool_review::CLAIM_KIND
        && record.event_id == id.claim_event_id()?
        && record.causality_parent_event_id == Some(id.review_event_id()?)
    {
        let receipt: ConversationToolReviewReceipt = serde_json::from_slice(&record.body)?;
        if receipt.key().turn() != id || receipt.key().session() != &record.session_id {
            return Err(invalid());
        }
        return Ok(ConversationEvent::ToolExecutionClaim(receipt));
    }
    if record.causality_parent_event_id != Some(start_id) {
        return Err(invalid());
    }
    match record.event_kind.as_str() {
        CHECKPOINT_KIND | THINKING_CHECKPOINT_KIND => {
            checkpoint_sequence(id, &record.event_id).ok_or_else(invalid)?;
            let text: String = serde_json::from_slice(&record.body)?;
            if text.is_empty() || text.len() > MAX_CHECKPOINT_BYTES {
                return Err(invalid());
            }
            let channel = if record.event_kind == CHECKPOINT_KIND {
                ConversationCheckpointChannel::Text
            } else {
                ConversationCheckpointChannel::Thinking
            };
            Ok(ConversationEvent::Checkpoint { channel, text })
        }
        END_KIND if record.event_id == id.finish_event_id()? => {
            let outcome = serde_json::from_slice(&record.body)?;
            validate_outcome(&outcome)?;
            if matches!(outcome, ConversationTurnOutcome::Uncertain { .. }) {
                return Err(invalid());
            }
            Ok(ConversationEvent::Outcome(outcome))
        }
        UNCERTAIN_KIND if record.event_id == id.uncertain_event_id()? => Ok(
            ConversationEvent::Outcome(ConversationTurnOutcome::Uncertain {
                reason: serde_json::from_slice(&record.body)?,
            }),
        ),
        _ => Err(invalid()),
    }
}

fn checkpoint_sequence(turn: ConversationTurnId, event: &EventId) -> Option<u64> {
    let sequence = event
        .as_str()
        .rsplit(':')
        .next()?
        .parse::<NonZeroU64>()
        .ok()?
        .get();
    (turn.checkpoint_event_id(sequence).ok().as_ref() == Some(event)).then_some(sequence)
}

fn invalid() -> ConversationJournalError {
    ConversationJournalError::InvalidEvent("invalid conversation event page".into())
}

fn cancellation_key(id: ConversationTurnId) -> Result<TurnId, StorageError> {
    TurnId::new(format!("conversation-cancellation:{:020}", id.get()))
}

fn cancellation_event_id(completed: bool) -> Result<EventId, StorageError> {
    EventId::new(if completed { "completed" } else { "incomplete" })
}
