// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Typed reads of the journal's existing committed records.

use marlin_agent_storage::SessionEventCursor;

use super::*;

#[derive(Debug)]
pub enum ConversationEvent {
    Started(ConversationTurnStart),
    Checkpoint {
        channel: ConversationCheckpointChannel,
        text: String,
    },
    Outcome(ConversationTurnOutcome),
}

#[derive(Debug)]
pub struct ConversationEventEntry {
    pub cursor: SessionEventCursor,
    pub event: ConversationEvent,
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
        } else {
            let Some(sequence) = event_id
                .as_str()
                .rsplit(':')
                .next()
                .and_then(|value| value.parse::<u64>().ok())
            else {
                return Ok(None);
            };
            if sequence == 0 || event_id != id.checkpoint_event_id(sequence)? {
                return Ok(None);
            }
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
    if record.causality_parent_event_id != Some(start_id) {
        return Err(invalid());
    }
    match record.event_kind.as_str() {
        CHECKPOINT_KIND | THINKING_CHECKPOINT_KIND => {
            let sequence = record
                .event_id
                .as_str()
                .rsplit(':')
                .next()
                .and_then(|value| value.parse::<u64>().ok())
                .filter(|n| *n > 0)
                .ok_or_else(invalid)?;
            if record.event_id != id.checkpoint_event_id(sequence)? {
                return Err(invalid());
            }
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

fn invalid() -> ConversationJournalError {
    ConversationJournalError::InvalidEvent("invalid conversation event page".into())
}
