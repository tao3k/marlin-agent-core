//! Hook protocol contracts for runtime interception and observation.

use serde::{Deserialize, Serialize};

/// Runtime event that can trigger configured hook handlers.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum HookEventName {
    PreToolUse,
    PermissionRequest,
    PostToolUse,
    PreCompact,
    PostCompact,
    SessionStart,
    UserPromptSubmit,
    SubAgentStart,
    SubAgentStop,
    Stop,
}

/// Kind of handler used to execute a hook.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum HookHandlerType {
    Command,
    Prompt,
    Agent,
}

/// Scheduling mode for a hook handler.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum HookExecutionMode {
    Sync,
    Async,
}

/// Lifetime scope for hook state.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum HookScope {
    Thread,
    Turn,
}

/// Configuration source that provided a hook.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum HookSource {
    System,
    User,
    Project,
    SessionFlags,
    Plugin,
    Unknown,
}

/// Trust state assigned to the hook source.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum HookTrustStatus {
    Managed,
    Untrusted,
    Trusted,
    Modified,
}

/// Runtime status for one hook invocation.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum HookRunStatus {
    Running,
    Completed,
    Failed,
    Blocked,
    Stopped,
}

/// Structured hook output entry category.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum HookOutputEntryKind {
    Warning,
    Stop,
    Feedback,
    Context,
    Error,
}

/// Structured output emitted by one hook run.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct HookOutputEntry {
    pub kind: HookOutputEntryKind,
    pub text: String,
}

impl HookOutputEntry {
    pub fn new(kind: HookOutputEntryKind, text: impl Into<String>) -> Self {
        Self {
            kind,
            text: text.into(),
        }
    }
}

/// Stable identifier for one hook run.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct HookRunId(String);

impl HookRunId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

impl From<String> for HookRunId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for HookRunId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// Source path string recorded in a hook protocol receipt.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct HookSourcePath(String);

impl HookSourcePath {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

impl From<String> for HookSourcePath {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for HookSourcePath {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// Millisecond timestamp in hook protocol receipts.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct HookTimestampMs(i64);

impl HookTimestampMs {
    pub fn new(value: i64) -> Self {
        Self(value)
    }

    pub fn as_i64(self) -> i64 {
        self.0
    }
}

impl From<i64> for HookTimestampMs {
    fn from(value: i64) -> Self {
        Self::new(value)
    }
}

/// Millisecond duration in hook protocol receipts.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct HookDurationMs(i64);

impl HookDurationMs {
    pub fn new(value: i64) -> Self {
        Self(value)
    }

    pub fn as_i64(self) -> i64 {
        self.0
    }
}

impl From<i64> for HookDurationMs {
    fn from(value: i64) -> Self {
        Self::new(value)
    }
}

/// Protocol receipt for one hook run.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct HookRunSummary {
    pub id: HookRunId,
    pub event_name: HookEventName,
    pub handler_type: HookHandlerType,
    pub execution_mode: HookExecutionMode,
    pub scope: HookScope,
    pub source_path: Option<HookSourcePath>,
    pub source: HookSource,
    pub trust: HookTrustStatus,
    pub display_order: i64,
    pub status: HookRunStatus,
    pub status_message: Option<String>,
    pub started_at_ms: HookTimestampMs,
    pub completed_at_ms: Option<HookTimestampMs>,
    pub duration_ms: Option<HookDurationMs>,
    pub entries: Vec<HookOutputEntry>,
}

impl HookRunSummary {
    pub fn running(
        id: impl Into<HookRunId>,
        event_name: HookEventName,
        handler_type: HookHandlerType,
    ) -> Self {
        Self {
            id: id.into(),
            event_name,
            handler_type,
            execution_mode: HookExecutionMode::Sync,
            scope: HookScope::Turn,
            source_path: None,
            source: HookSource::Unknown,
            trust: HookTrustStatus::Untrusted,
            display_order: 0,
            status: HookRunStatus::Running,
            status_message: None,
            started_at_ms: HookTimestampMs::new(0),
            completed_at_ms: None,
            duration_ms: None,
            entries: Vec::new(),
        }
    }

    pub fn completed(mut self) -> Self {
        self.status = HookRunStatus::Completed;
        self
    }

    pub fn with_entry(mut self, entry: HookOutputEntry) -> Self {
        self.entries.push(entry);
        self
    }
}
