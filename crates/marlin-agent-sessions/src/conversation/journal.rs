// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Admission, durable writes, and ordered replay over the session event store.

use std::future::Future;

use marlin_agent_storage::{
    AgentId, EventId, ProjectId, SessionEventPageRequest, SessionEventRecord, SessionEventStore,
    SessionId, StoragePageLimit, TurnId,
};

use super::{
    events::{self, ConversationEvent},
    tool_review::ConversationToolReviewStatus,
    turn::{
        ConversationCheckpointChannel, ConversationJournalError, ConversationPendingToolRequest,
        ConversationSubmissionId, ConversationTerminalObservation,
        ConversationToolContinuationStart, ConversationTurn, ConversationTurnId,
        ConversationTurnOutcome, ConversationTurnSource, ConversationTurnStart,
        ConversationUncertaintyReason, TURN_PREFIX, pending_tool_request_from_outcome,
        validate_checkpoint_prefix, validate_outcome,
    },
};

pub(super) const START_KIND: &str = "marlin.conversation.turn.started.v1";
pub(super) const TOOL_CONTINUATION_START_KIND: &str =
    "marlin.conversation.turn.tool-continuation.v1";
pub(super) const END_KIND: &str = "marlin.conversation.turn.finished.v1";
pub(super) const CHECKPOINT_KIND: &str = "marlin.conversation.turn.checkpoint.v1";
pub(super) const THINKING_CHECKPOINT_KIND: &str = "marlin.conversation.turn.thinking-checkpoint.v1";
pub(super) const UNCERTAIN_KIND: &str = "marlin.conversation.turn.uncertain.v1";
pub(super) const SUBMISSION_KIND: &str = "marlin.conversation.submission.v1";
pub(super) const MAX_CHECKPOINT_BYTES: usize = 128 * 1024;
const MAX_PARTIAL_BYTES: usize = 16 * 1024 * 1024;
#[cfg(test)]
#[path = "../../tests/unit/authority_write.rs"]
mod authority_write_tests;

pub(super) async fn write_with_authority<E>(
    owned: &mut bool,
    write: impl Future<Output = Result<(), E>>,
) -> Result<(), E> {
    *owned = false;
    write.await?;
    *owned = true;
    Ok(())
}

/// Single-client journal over a shared runtime storage backend.
///
/// A started-event key is claimed before dispatch; concurrent stale writers
/// cannot claim the same ordinal. This is not a process lease: a replayed
/// unfinished turn must not be automatically retried or marked dead.
pub struct ConversationJournal<S> {
    pub(super) storage: S,
    project: ProjectId,
    session: SessionId,
    agent: AgentId,
    next: u64,
    active: Option<ConversationTurn>,
    deferred_tool_turn: Option<ConversationTurn>,
    owns_active: bool,
    next_checkpoint: u64,
    legacy_outcome_unresolved: bool,
    pub(super) pending_tool_request: Option<ConversationPendingToolRequest>,
    pub(super) owns_pending_review: bool,
    pub(super) owns_pending_result: bool,
}

impl<S: SessionEventStore> ConversationJournal<S> {
    /// Return only a request whose terminal event was committed or completely
    /// replayed from storage. Presentation must not treat this as approval.
    pub fn pending_tool_request(&self) -> Option<&ConversationPendingToolRequest> {
        self.pending_tool_request.as_ref()
    }

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
        visit: impl FnMut(&ConversationTurn) -> bool,
    ) -> Result<Self, ConversationJournalError> {
        Self::replay(storage, project, session, agent, None, visit).await
    }

    /// Replay for admission with a byte limit on each storage page.
    pub async fn open_controlled_bounded(
        storage: S,
        project: ProjectId,
        session: SessionId,
        agent: AgentId,
        body_budget: std::num::NonZeroU32,
        visit: impl FnMut(&ConversationTurn) -> bool,
    ) -> Result<Self, ConversationJournalError> {
        Self::replay(storage, project, session, agent, Some(body_budget), visit).await
    }

    async fn replay(
        storage: S,
        project: ProjectId,
        session: SessionId,
        agent: AgentId,
        body_budget: Option<std::num::NonZeroU32>,
        mut visit: impl FnMut(&ConversationTurn) -> bool,
    ) -> Result<Self, ConversationJournalError> {
        let mut journal = Self {
            storage,
            project,
            session,
            agent,
            next: 1,
            active: None,
            deferred_tool_turn: None,
            owns_active: false,
            next_checkpoint: 1,
            legacy_outcome_unresolved: false,
            pending_tool_request: None,
            owns_pending_review: false,
            owns_pending_result: false,
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
            if let Some(budget) = body_budget {
                request = request.with_body_budget(budget);
            }
            let page = journal.storage.list_session_events_page(request).await?;
            if page.items.len() > 16 {
                return Err(ConversationJournalError::InvalidEvent(
                    "storage page exceeds requested limit".into(),
                ));
            }
            let has_items = !page.items.is_empty();
            let mut remaining = body_budget.map_or(usize::MAX, |budget| budget.get() as usize);
            for event in page.items {
                remaining = remaining.checked_sub(event.body.len()).ok_or_else(|| {
                    ConversationJournalError::InvalidEvent(
                        "storage page exceeds requested byte budget".into(),
                    )
                })?;
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
        if let Some(turn) = &journal.deferred_tool_turn
            && !visit(turn)
        {
            return Err(ConversationJournalError::ReplayStopped);
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
        self.commit_start(
            ConversationTurnSource::User(start),
            None,
            created_at_unix_ms,
        )
        .await
    }

    /// Claim the next provider request after the exact review decision and,
    /// when approved, its tool results are durable. No user input is fabricated.
    pub async fn start_tool_continuation(
        &mut self,
        start: ConversationToolContinuationStart,
        created_at_unix_ms: i64,
    ) -> Result<ConversationTurnId, ConversationJournalError> {
        self.commit_start(
            ConversationTurnSource::ToolResults(start),
            None,
            created_at_unix_ms,
        )
        .await
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
        self.commit_start(
            ConversationTurnSource::User(start),
            Some(submission),
            created_at_unix_ms,
        )
        .await
    }

    async fn commit_start(
        &mut self,
        start: ConversationTurnSource,
        submission: Option<&ConversationSubmissionId>,
        created_at_unix_ms: i64,
    ) -> Result<ConversationTurnId, ConversationJournalError> {
        if self.active.is_some() {
            return Err(ConversationJournalError::UnfinishedTurn);
        }
        if self.legacy_outcome_unresolved {
            return Err(ConversationJournalError::LegacyOutcomeUnresolved);
        }
        start
            .endpoint()
            .validate_contract()
            .map_err(|error| ConversationJournalError::InvalidEvent(error.to_string()))?;
        let (kind, body) = match &start {
            ConversationTurnSource::User(user) => {
                if self.pending_tool_request.is_some() {
                    return Err(ConversationJournalError::ToolRequestPending);
                }
                if user.user_text.trim().is_empty() {
                    return Err(ConversationJournalError::InvalidEvent(
                        "user text is empty".into(),
                    ));
                }
                (START_KIND, serde_json::to_vec(user)?)
            }
            ConversationTurnSource::ToolResults(tool) => {
                if submission.is_some()
                    || !self.pending_tool_request.as_ref().is_some_and(|pending| {
                        matches!(
                            pending.status,
                            ConversationToolReviewStatus::ResultsCommitted
                                | ConversationToolReviewStatus::Rejected
                        ) && pending.key() == tool.source
                            && pending.endpoint == tool.endpoint
                    })
                {
                    return Err(ConversationJournalError::ToolRequestPending);
                }
                (TOOL_CONTINUATION_START_KIND, serde_json::to_vec(tool)?)
            }
        };
        let id = ConversationTurnId(self.next);
        let next = self
            .next
            .checked_add(1)
            .ok_or(ConversationJournalError::OrdinalExhausted)?;
        let event = self.record(
            id,
            id.start_event_id()?,
            kind,
            None,
            body,
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
            tool_results: None,
            tool_denial: None,
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
        let active = self.active.take().expect("active turn checked above");
        let pending_tool_request =
            pending_tool_request_from_outcome(&self.session, id, &active.start, &outcome);
        if pending_tool_request.is_some()
            || !matches!(
                outcome,
                ConversationTurnOutcome::RejectedBeforeDispatch { .. }
            )
        {
            self.pending_tool_request = pending_tool_request;
        }
        self.owns_active = false;
        self.owns_pending_review = self
            .pending_tool_request
            .as_ref()
            .is_some_and(|pending| pending.status == ConversationToolReviewStatus::AwaitingReview);
        self.owns_pending_result = false;
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

    pub(super) fn record(
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
        let id = ConversationTurnId::from_storage_id(&event.turn_id)
            .ok_or_else(|| ConversationJournalError::InvalidEvent("invalid turn ordinal".into()))?;
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
                if let Some(turn) = self.deferred_tool_turn.take()
                    && !visit(&turn)
                {
                    return Err(ConversationJournalError::ReplayStopped);
                }
                let allowed = match &start {
                    ConversationTurnSource::User(_) => self.pending_tool_request.is_none(),
                    ConversationTurnSource::ToolResults(tool) => {
                        self.pending_tool_request.as_ref().is_some_and(|pending| {
                            matches!(
                                pending.status,
                                ConversationToolReviewStatus::ResultsCommitted
                                    | ConversationToolReviewStatus::Rejected
                            ) && pending.key() == tool.source
                                && pending.endpoint == tool.endpoint
                        })
                    }
                };
                if !allowed || self.active.is_some() || id.get() != self.next {
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
                    tool_results: None,
                    tool_denial: None,
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
                let next_pending = turn.outcome.as_ref().and_then(|outcome| {
                    pending_tool_request_from_outcome(&self.session, id, &turn.start, outcome)
                });
                if next_pending.is_some()
                    || !matches!(
                        turn.outcome,
                        Some(ConversationTurnOutcome::RejectedBeforeDispatch { .. })
                    )
                {
                    self.pending_tool_request = next_pending;
                }
                self.owns_pending_review =
                    self.pending_tool_request.as_ref().is_some_and(|pending| {
                        pending.status == ConversationToolReviewStatus::AwaitingReview
                    });
                self.owns_pending_result = false;
                if matches!(
                    turn.outcome,
                    Some(ConversationTurnOutcome::ToolRequested { .. })
                ) {
                    self.deferred_tool_turn = Some(turn);
                } else if !visit(&turn) {
                    return Err(ConversationJournalError::ReplayStopped);
                }
            }
            ConversationEvent::ToolReview(_) => {
                self.replay_tool_review(&event, id)?;
                let denial = self
                    .pending_tool_request
                    .as_ref()
                    .filter(|pending| pending.status == ConversationToolReviewStatus::Rejected)
                    .and_then(|pending| pending.review_receipt.clone());
                if let Some(denial) = denial {
                    let deferred = self
                        .deferred_tool_turn
                        .as_mut()
                        .filter(|turn| turn.id == id)
                        .ok_or_else(|| {
                            ConversationJournalError::InvalidEvent(
                                "tool denial without deferred request".into(),
                            )
                        })?;
                    deferred.tool_denial = Some(denial);
                }
            }
            ConversationEvent::ToolExecutionClaim(_) => self.replay_tool_claim(&event, id)?,
            ConversationEvent::ToolResults(_) => {
                self.replay_tool_results(&event, id)?;
                let receipt = self
                    .pending_tool_request
                    .as_ref()
                    .and_then(|pending| pending.results_receipt.clone());
                let deferred = self
                    .deferred_tool_turn
                    .as_mut()
                    .filter(|turn| turn.id == id)
                    .ok_or_else(|| {
                        ConversationJournalError::InvalidEvent(
                            "tool results without deferred request".into(),
                        )
                    })?;
                deferred.tool_results = receipt;
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
