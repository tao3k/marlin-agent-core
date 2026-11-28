// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Bounded storage pages and typed cursors shared by both backends.

use std::num::NonZeroU32;

use serde::{Deserialize, Serialize};

use super::contract::{
    AgentId, EventId, MemoryKey, MemoryProposalId, MemoryProposalRecord, ProjectId,
    SessionEventRecord, SessionId, StorageError, StorageResult, TopologyEdgeId, TopologyEdgeRecord,
    TurnId, VisibilityReceipt,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u16")]
pub struct StoragePageLimit(u16);

impl TryFrom<u16> for StoragePageLimit {
    type Error = StorageError;

    fn try_from(limit: u16) -> Result<Self, Self::Error> {
        Self::new(limit)
    }
}

impl StoragePageLimit {
    pub const MAX: u16 = 1_000;
    pub const MAXIMUM: Self = Self(Self::MAX);

    pub fn new(limit: u16) -> StorageResult<Self> {
        if !(1..=Self::MAX).contains(&limit) {
            return Err(StorageError::InvalidPageLimit { limit });
        }
        Ok(Self(limit))
    }

    pub fn get(self) -> usize {
        usize::from(self.0)
    }

    #[cfg(feature = "turso")]
    pub(crate) fn fetch_count(self) -> i64 {
        i64::from(self.0) + 1
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoragePage<T, C> {
    pub items: Vec<T>,
    pub next_cursor: Option<C>,
}

/// Lists session IDs with at least one event for the specified project and agent.
/// The cursor is exclusive; ordering is by session ID, not recency.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionPageRequest {
    project_id: ProjectId,
    agent_id: AgentId,
    after: Option<SessionId>,
    limit: StoragePageLimit,
}

impl SessionPageRequest {
    pub fn new(project_id: ProjectId, agent_id: AgentId, limit: StoragePageLimit) -> Self {
        Self {
            project_id,
            agent_id,
            after: None,
            limit,
        }
    }
    pub fn after(mut self, session: SessionId) -> Self {
        self.after = Some(session);
        self
    }
    pub fn project_id(&self) -> &ProjectId {
        &self.project_id
    }
    pub fn agent_id(&self) -> &AgentId {
        &self.agent_id
    }
    pub fn cursor(&self) -> Option<&SessionId> {
        self.after.as_ref()
    }
    pub fn limit(&self) -> StoragePageLimit {
        self.limit
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionEventCursor {
    turn_id: TurnId,
    event_id: EventId,
}

impl SessionEventCursor {
    pub fn new(turn_id: TurnId, event_id: EventId) -> Self {
        Self { turn_id, event_id }
    }

    pub fn from_record(record: &SessionEventRecord) -> Self {
        Self {
            turn_id: record.turn_id.clone(),
            event_id: record.event_id.clone(),
        }
    }

    pub fn turn_id(&self) -> &TurnId {
        &self.turn_id
    }

    pub fn event_id(&self) -> &EventId {
        &self.event_id
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionEventPageRequest {
    project_id: ProjectId,
    session_id: SessionId,
    after: Option<SessionEventCursor>,
    limit: StoragePageLimit,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    turn_body_budget: Option<NonZeroU32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    session_body_budget: Option<NonZeroU32>,
}

impl SessionEventPageRequest {
    pub fn new(project_id: ProjectId, session_id: SessionId, limit: StoragePageLimit) -> Self {
        Self {
            project_id,
            session_id,
            after: None,
            limit,
            turn_body_budget: None,
            session_body_budget: None,
        }
    }

    pub fn after(mut self, cursor: SessionEventCursor) -> Self {
        self.after = Some(cursor);
        self
    }

    /// Read only the cursor's turn and return a contiguous prefix within the
    /// combined budget for raw event bodies. The cursor is exclusive.
    pub fn within_turn_after(
        mut self,
        cursor: SessionEventCursor,
        body_budget: NonZeroU32,
    ) -> Self {
        self.after = Some(cursor);
        self.turn_body_budget = Some(body_budget);
        self.session_body_budget = None;
        self
    }

    /// Bound the raw event bodies in a page spanning the Session.
    pub fn with_body_budget(mut self, body_budget: NonZeroU32) -> Self {
        self.session_body_budget = Some(body_budget);
        self.turn_body_budget = None;
        self
    }

    /// The budget for pages restricted to the cursor's turn.
    pub fn turn_body_budget(&self) -> Option<NonZeroU32> {
        self.turn_body_budget
    }

    /// Stores enforce this budget before copying bodies into the page.
    pub fn body_budget(&self) -> Option<NonZeroU32> {
        self.turn_body_budget.or(self.session_body_budget)
    }

    pub(crate) fn turn_bound(&self) -> StorageResult<Option<(&SessionEventCursor, NonZeroU32)>> {
        self.turn_body_budget
            .map(|budget| {
                self.cursor()
                    .map(|cursor| (cursor, budget))
                    .ok_or(StorageError::MissingTurnPageCursor)
            })
            .transpose()
    }

    pub fn project_id(&self) -> &ProjectId {
        &self.project_id
    }

    pub fn session_id(&self) -> &SessionId {
        &self.session_id
    }

    pub fn cursor(&self) -> Option<&SessionEventCursor> {
        self.after.as_ref()
    }

    pub fn limit(&self) -> StoragePageLimit {
        self.limit
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisibilityCursor {
    created_at_unix_ms: i64,
    receipt_id: String,
}

impl VisibilityCursor {
    pub fn from_record(record: &VisibilityReceipt) -> Self {
        Self {
            created_at_unix_ms: record.created_at_unix_ms,
            receipt_id: record.receipt_id.clone(),
        }
    }

    pub fn created_at_unix_ms(&self) -> i64 {
        self.created_at_unix_ms
    }

    pub fn receipt_id(&self) -> &str {
        &self.receipt_id
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisibilityPageRequest {
    project_id: ProjectId,
    after: Option<VisibilityCursor>,
    limit: StoragePageLimit,
}

impl VisibilityPageRequest {
    pub fn new(project_id: ProjectId, limit: StoragePageLimit) -> Self {
        Self {
            project_id,
            after: None,
            limit,
        }
    }

    pub fn after(mut self, cursor: VisibilityCursor) -> Self {
        self.after = Some(cursor);
        self
    }

    pub fn project_id(&self) -> &ProjectId {
        &self.project_id
    }

    pub fn cursor(&self) -> Option<&VisibilityCursor> {
        self.after.as_ref()
    }

    pub fn limit(&self) -> StoragePageLimit {
        self.limit
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryProposalCursor {
    created_at_unix_ms: i64,
    proposal_id: MemoryProposalId,
}

impl MemoryProposalCursor {
    pub fn from_record(record: &MemoryProposalRecord) -> Self {
        Self {
            created_at_unix_ms: record.created_at_unix_ms,
            proposal_id: record.proposal_id.clone(),
        }
    }

    pub fn created_at_unix_ms(&self) -> i64 {
        self.created_at_unix_ms
    }

    pub fn proposal_id(&self) -> &MemoryProposalId {
        &self.proposal_id
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryProposalPageRequest {
    project_id: ProjectId,
    memory_key: Option<MemoryKey>,
    after: Option<MemoryProposalCursor>,
    limit: StoragePageLimit,
}

impl MemoryProposalPageRequest {
    pub fn new(project_id: ProjectId, limit: StoragePageLimit) -> Self {
        Self {
            project_id,
            memory_key: None,
            after: None,
            limit,
        }
    }

    pub fn for_memory_key(mut self, memory_key: MemoryKey) -> Self {
        self.memory_key = Some(memory_key);
        self
    }

    pub fn after(mut self, cursor: MemoryProposalCursor) -> Self {
        self.after = Some(cursor);
        self
    }

    pub fn project_id(&self) -> &ProjectId {
        &self.project_id
    }

    pub fn memory_key(&self) -> Option<&MemoryKey> {
        self.memory_key.as_ref()
    }

    pub fn cursor(&self) -> Option<&MemoryProposalCursor> {
        self.after.as_ref()
    }

    pub fn limit(&self) -> StoragePageLimit {
        self.limit
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopologyEdgeCursor {
    created_at_unix_ms: i64,
    edge_id: TopologyEdgeId,
}

impl TopologyEdgeCursor {
    pub fn from_record(record: &TopologyEdgeRecord) -> Self {
        Self {
            created_at_unix_ms: record.created_at_unix_ms,
            edge_id: record.edge_id.clone(),
        }
    }

    pub fn created_at_unix_ms(&self) -> i64 {
        self.created_at_unix_ms
    }

    pub fn edge_id(&self) -> &TopologyEdgeId {
        &self.edge_id
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopologyEdgePageRequest {
    project_id: ProjectId,
    after: Option<TopologyEdgeCursor>,
    limit: StoragePageLimit,
}

impl TopologyEdgePageRequest {
    pub fn new(project_id: ProjectId, limit: StoragePageLimit) -> Self {
        Self {
            project_id,
            after: None,
            limit,
        }
    }

    pub fn after(mut self, cursor: TopologyEdgeCursor) -> Self {
        self.after = Some(cursor);
        self
    }

    pub fn project_id(&self) -> &ProjectId {
        &self.project_id
    }

    pub fn cursor(&self) -> Option<&TopologyEdgeCursor> {
        self.after.as_ref()
    }

    pub fn limit(&self) -> StoragePageLimit {
        self.limit
    }
}
