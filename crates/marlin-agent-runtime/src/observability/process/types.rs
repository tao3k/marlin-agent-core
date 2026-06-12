//! Shared process observation, handle, status, and output value types.

use std::{
    fmt,
    process::{ExitStatus as StdExitStatus, Output as StdOutput},
};

use marlin_agent_protocol::SubAgentSpawnProfile;

/// Runtime-visible process kind.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuntimeProcessKind {
    Tool,
    SubAgent,
    Provider,
    Hook,
    Other(String),
}

/// Runtime-visible process lifecycle state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuntimeProcessStatus {
    Running,
    Finished,
    Failed,
    Orphaned,
    CleanupRequested,
}

/// Millisecond timestamp observed by the runtime process registry.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct RuntimeProcessObservationTimestampMs(u64);

impl RuntimeProcessObservationTimestampMs {
    pub fn new(value: u64) -> Self {
        Self(value)
    }

    pub fn as_millis(self) -> u64 {
        self.0
    }
}

impl From<u64> for RuntimeProcessObservationTimestampMs {
    fn from(value: u64) -> Self {
        Self::new(value)
    }
}

/// Runtime-owned handle used to correlate a process across events and cleanup.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct RuntimeProcessHandle(String);

impl RuntimeProcessHandle {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for RuntimeProcessHandle {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for RuntimeProcessHandle {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// Registration failure for a runtime-owned process observation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuntimeProcessRegistrationError {
    DuplicateActiveHandle {
        handle: RuntimeProcessHandle,
        existing_pid: u32,
        new_pid: u32,
    },
    DuplicateActivePid {
        pid: u32,
    },
}

impl fmt::Display for RuntimeProcessRegistrationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateActiveHandle {
                handle,
                existing_pid,
                new_pid,
            } => write!(
                formatter,
                "runtime process handle '{}' is already active for pid {}; refused pid {}",
                handle.as_str(),
                existing_pid,
                new_pid
            ),
            Self::DuplicateActivePid { pid } => write!(
                formatter,
                "runtime process pid {pid} is already active in the registry"
            ),
        }
    }
}

impl std::error::Error for RuntimeProcessRegistrationError {}

/// Count summary for active runtime-owned process statuses.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RuntimeProcessStatusCounts {
    pub running: usize,
    pub finished: usize,
    pub failed: usize,
    pub orphaned: usize,
    pub cleanup_requested: usize,
}

impl RuntimeProcessStatusCounts {
    pub fn record(&mut self, status: &RuntimeProcessStatus) {
        match status {
            RuntimeProcessStatus::Running => self.running += 1,
            RuntimeProcessStatus::Finished => self.finished += 1,
            RuntimeProcessStatus::Failed => self.failed += 1,
            RuntimeProcessStatus::Orphaned => self.orphaned += 1,
            RuntimeProcessStatus::CleanupRequested => self.cleanup_requested += 1,
        }
    }
}

/// Count summary for active runtime-owned process kinds.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RuntimeProcessKindCounts {
    pub tool: usize,
    pub sub_agent: usize,
    pub provider: usize,
    pub hook: usize,
    pub other: usize,
}

impl RuntimeProcessKindCounts {
    pub fn record(&mut self, kind: &RuntimeProcessKind) {
        match kind {
            RuntimeProcessKind::Tool => self.tool += 1,
            RuntimeProcessKind::SubAgent => self.sub_agent += 1,
            RuntimeProcessKind::Provider => self.provider += 1,
            RuntimeProcessKind::Hook => self.hook += 1,
            RuntimeProcessKind::Other(_) => self.other += 1,
        }
    }
}

/// Count-only snapshot of runtime-owned process registry state.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RuntimeProcessRegistrySnapshot {
    pub observed_at_ms: u64,
    pub active_count: usize,
    pub cleanup_candidate_count: usize,
    pub status_counts: RuntimeProcessStatusCounts,
    pub kind_counts: RuntimeProcessKindCounts,
}

/// Active or terminal process observation owned by the runtime.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeProcessObservation {
    pub pid: u32,
    pub handle: RuntimeProcessHandle,
    pub kind: RuntimeProcessKind,
    pub owner_reference: String,
    pub command: Option<RuntimeCommandObservation>,
    pub status: RuntimeProcessStatus,
    pub started_at_ms: Option<RuntimeProcessObservationTimestampMs>,
    pub last_observed_at_ms: Option<RuntimeProcessObservationTimestampMs>,
    pub cleanup_attempts: u32,
    pub last_cleanup_attempt_at_ms: Option<RuntimeProcessObservationTimestampMs>,
}

impl RuntimeProcessObservation {
    pub fn new(pid: u32, kind: RuntimeProcessKind, owner_reference: impl Into<String>) -> Self {
        let owner_reference = owner_reference.into();
        Self {
            pid,
            handle: RuntimeProcessHandle::new(owner_reference.clone()),
            kind,
            owner_reference,
            command: None,
            status: RuntimeProcessStatus::Running,
            started_at_ms: None,
            last_observed_at_ms: None,
            cleanup_attempts: 0,
            last_cleanup_attempt_at_ms: None,
        }
    }

    pub fn with_handle(mut self, handle: impl Into<RuntimeProcessHandle>) -> Self {
        self.handle = handle.into();
        self
    }

    pub fn with_command(mut self, command: RuntimeCommandObservation) -> Self {
        self.command = Some(command);
        self
    }

    pub fn with_started_at_ms(mut self, started_at_ms: u64) -> Self {
        let started_at_ms = RuntimeProcessObservationTimestampMs::new(started_at_ms);
        self.started_at_ms = Some(started_at_ms);
        self.last_observed_at_ms = Some(started_at_ms);
        self
    }

    pub fn observed_at(mut self, observed_at_ms: u64) -> Self {
        self.last_observed_at_ms = Some(RuntimeProcessObservationTimestampMs::new(observed_at_ms));
        self
    }

    pub fn started_at_ms(&self) -> Option<u64> {
        self.started_at_ms
            .map(RuntimeProcessObservationTimestampMs::as_millis)
    }

    pub fn last_observed_at_ms(&self) -> Option<u64> {
        self.last_observed_at_ms
            .map(RuntimeProcessObservationTimestampMs::as_millis)
    }

    pub fn last_cleanup_attempt_at_ms(&self) -> Option<u64> {
        self.last_cleanup_attempt_at_ms
            .map(RuntimeProcessObservationTimestampMs::as_millis)
    }

    pub fn request_cleanup_at_ms(&mut self, observed_at_ms: u64) {
        let observed_at_ms = RuntimeProcessObservationTimestampMs::new(observed_at_ms);
        self.status = RuntimeProcessStatus::CleanupRequested;
        self.last_observed_at_ms = Some(observed_at_ms);
        self.cleanup_attempts = self.cleanup_attempts.saturating_add(1);
        self.last_cleanup_attempt_at_ms = Some(observed_at_ms);
    }

    pub fn update_status_at_ms(&mut self, status: RuntimeProcessStatus, observed_at_ms: u64) {
        self.status = status;
        self.last_observed_at_ms = Some(RuntimeProcessObservationTimestampMs::new(observed_at_ms));
    }
}

/// Typed command category carried by a runtime process observation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeCommandKind(String);

impl RuntimeCommandKind {
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

impl fmt::Display for RuntimeCommandKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl From<&str> for RuntimeCommandKind {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for RuntimeCommandKind {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

/// Command-level metadata carried by a runtime process observation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeCommandObservation {
    pub command_kind: RuntimeCommandKind,
    pub argv: Vec<String>,
    pub cwd: Option<String>,
    pub sub_agent_source: Option<String>,
    pub sub_agent_role: Option<String>,
    pub sub_agent_profile: Option<SubAgentSpawnProfile>,
}

impl RuntimeCommandObservation {
    pub fn new(command_kind: impl Into<RuntimeCommandKind>) -> Self {
        Self {
            command_kind: command_kind.into(),
            argv: Vec::new(),
            cwd: None,
            sub_agent_source: None,
            sub_agent_role: None,
            sub_agent_profile: None,
        }
    }

    pub fn with_argv(mut self, argv: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.argv = argv.into_iter().map(Into::into).collect();
        self
    }

    pub fn with_cwd(mut self, cwd: impl Into<String>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }

    pub fn with_sub_agent_source(mut self, sub_agent_source: impl Into<String>) -> Self {
        self.sub_agent_source = Some(sub_agent_source.into());
        self
    }

    pub fn with_sub_agent_role(mut self, sub_agent_role: impl Into<String>) -> Self {
        self.sub_agent_role = Some(sub_agent_role.into());
        self
    }

    pub fn with_sub_agent_profile(mut self, profile: SubAgentSpawnProfile) -> Self {
        self.sub_agent_role = Some(profile.role.clone());
        self.sub_agent_profile = Some(profile);
        self
    }
}

/// Runtime-owned process exit status.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeProcessExitStatus {
    code: Option<i32>,
    success: bool,
}

impl RuntimeProcessExitStatus {
    pub fn code(&self) -> Option<i32> {
        self.code
    }

    pub fn success(&self) -> bool {
        self.success
    }
}

impl From<StdExitStatus> for RuntimeProcessExitStatus {
    fn from(status: StdExitStatus) -> Self {
        Self {
            code: status.code(),
            success: status.success(),
        }
    }
}

/// Runtime-owned process output packet.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeProcessOutput {
    pub status: RuntimeProcessExitStatus,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

impl From<StdOutput> for RuntimeProcessOutput {
    fn from(output: StdOutput) -> Self {
        Self {
            status: output.status.into(),
            stdout: output.stdout,
            stderr: output.stderr,
        }
    }
}
