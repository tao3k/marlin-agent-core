//! Storage projection adapters for runtime session events.

use std::fmt::{self, Display, Formatter};

use marlin_agent_storage::{
    AgentId as StorageAgentId, AgentStorage, EventId as StorageEventId,
    ProjectId as StorageProjectId, SessionEventRecord, SessionId as StorageSessionId, StorageError,
    StorageResult, TurnId as StorageTurnId,
};

use crate::AgentSessionContext;

/// Runtime session event ready to be projected into `marlin-agent-storage`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionStorageEvent {
    project_id: String,
    agent_id: String,
    turn_id: String,
    event_id: String,
    event_kind: String,
    causality_parent_event_id: Option<String>,
    body: Vec<u8>,
    created_at_unix_ms: i64,
}

impl SessionStorageEvent {
    /// Start a named builder for a session event projection.
    pub fn builder() -> SessionStorageEventBuilder {
        SessionStorageEventBuilder::default()
    }

    pub fn project_id(&self) -> &str {
        &self.project_id
    }

    pub fn event_id(&self) -> &str {
        &self.event_id
    }
}

/// Named builder for `SessionStorageEvent`.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SessionStorageEventBuilder {
    project_id: Option<String>,
    agent_id: Option<String>,
    turn_id: Option<String>,
    event_id: Option<String>,
    event_kind: Option<String>,
    causality_parent_event_id: Option<String>,
    body: Vec<u8>,
    created_at_unix_ms: i64,
}

impl SessionStorageEventBuilder {
    /// Set the project that owns this session event.
    pub fn with_project_id(mut self, project_id: impl Into<String>) -> Self {
        self.project_id = Some(project_id.into());
        self
    }

    /// Set the agent that emitted this session event.
    pub fn with_agent_id(mut self, agent_id: impl Into<String>) -> Self {
        self.agent_id = Some(agent_id.into());
        self
    }

    /// Set the turn identifier for this event.
    pub fn with_turn_id(mut self, turn_id: impl Into<String>) -> Self {
        self.turn_id = Some(turn_id.into());
        self
    }

    /// Set the event identifier.
    pub fn with_event_id(mut self, event_id: impl Into<String>) -> Self {
        self.event_id = Some(event_id.into());
        self
    }

    /// Set the event kind.
    pub fn with_event_kind(mut self, event_kind: impl Into<String>) -> Self {
        self.event_kind = Some(event_kind.into());
        self
    }

    /// Set the causality parent event identifier.
    pub fn with_parent_event_id(mut self, event_id: impl Into<String>) -> Self {
        self.causality_parent_event_id = Some(event_id.into());
        self
    }

    /// Set the event payload bytes.
    pub fn with_body(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.body = body.into();
        self
    }

    /// Set the event creation timestamp in Unix milliseconds.
    pub fn with_created_at_unix_ms(mut self, created_at_unix_ms: i64) -> Self {
        self.created_at_unix_ms = created_at_unix_ms;
        self
    }

    /// Build the event projection request.
    pub fn build(self) -> Result<SessionStorageEvent, SessionStorageProjectionError> {
        Ok(SessionStorageEvent {
            project_id: required_field(self.project_id, "project_id")?,
            agent_id: required_field(self.agent_id, "agent_id")?,
            turn_id: required_field(self.turn_id, "turn_id")?,
            event_id: required_field(self.event_id, "event_id")?,
            event_kind: required_field(self.event_kind, "event_kind")?,
            causality_parent_event_id: self.causality_parent_event_id,
            body: self.body,
            created_at_unix_ms: self.created_at_unix_ms,
        })
    }
}

/// Adapter that projects session contexts into an `AgentStorage` implementation.
#[derive(Clone, Debug)]
pub struct SessionStorageAdapter<S> {
    storage: S,
}

impl<S> SessionStorageAdapter<S> {
    /// Wrap an existing storage backend.
    pub fn new(storage: S) -> Self {
        Self { storage }
    }

    /// Return the wrapped storage backend.
    pub fn storage(&self) -> &S {
        &self.storage
    }
}

impl<S> SessionStorageAdapter<S>
where
    S: AgentStorage,
{
    pub async fn append_context_event(
        &self,
        context: &AgentSessionContext,
        event: SessionStorageEvent,
    ) -> Result<(), SessionStorageProjectionError> {
        self.storage
            .append_session_event(
                to_session_event_record(context, event)
                    .map_err(SessionStorageProjectionError::Storage)?,
            )
            .await
            .map_err(SessionStorageProjectionError::Storage)
    }

    pub async fn list_context_events(
        &self,
        project_id: impl Into<String>,
        context: &AgentSessionContext,
    ) -> Result<Vec<SessionEventRecord>, SessionStorageProjectionError> {
        let project_id = StorageProjectId::new(project_id.into())
            .map_err(SessionStorageProjectionError::Storage)?;
        let session_id = StorageSessionId::new(context.session_id().as_str())
            .map_err(SessionStorageProjectionError::Storage)?;
        self.storage
            .list_session_events(&project_id, &session_id)
            .await
            .map_err(SessionStorageProjectionError::Storage)
    }
}

/// Error returned while projecting runtime session events into storage.
#[derive(Debug)]
pub enum SessionStorageProjectionError {
    /// A required builder field was not set.
    MissingField { field: &'static str },
    /// The storage backend rejected the projected record.
    Storage(StorageError),
}

impl Display for SessionStorageProjectionError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingField { field } => {
                write!(formatter, "session storage event is missing {field}")
            }
            Self::Storage(error) => write!(formatter, "session storage projection failed: {error}"),
        }
    }
}

impl std::error::Error for SessionStorageProjectionError {}

fn to_session_event_record(
    context: &AgentSessionContext,
    event: SessionStorageEvent,
) -> StorageResult<SessionEventRecord> {
    Ok(SessionEventRecord {
        project_id: StorageProjectId::new(event.project_id)?,
        session_id: StorageSessionId::new(context.session_id().as_str())?,
        agent_id: StorageAgentId::new(event.agent_id)?,
        turn_id: StorageTurnId::new(event.turn_id)?,
        event_id: StorageEventId::new(event.event_id)?,
        event_kind: event.event_kind,
        causality_parent_event_id: event
            .causality_parent_event_id
            .map(StorageEventId::new)
            .transpose()?,
        body: event.body,
        created_at_unix_ms: event.created_at_unix_ms,
    })
}

fn required_field(
    value: Option<String>,
    field: &'static str,
) -> Result<String, SessionStorageProjectionError> {
    value.ok_or(SessionStorageProjectionError::MissingField { field })
}
