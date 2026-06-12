//! Cleanup controllers and sweep receipts for `RuntimeProcessRegistry`.

use super::types::{
    RuntimeCommandObservation, RuntimeProcessHandle, RuntimeProcessKind, RuntimeProcessObservation,
    RuntimeProcessStatus,
};

/// Failed cleanup operation for a runtime-owned process.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeProcessCleanupFailure {
    pub process: RuntimeProcessObservation,
    pub error: String,
}

/// Cleanup receipt policy for retained cleanup candidates.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeProcessCleanupPolicy {
    pub graceful_termination_attempts_before_force: u32,
}

impl RuntimeProcessCleanupPolicy {
    pub fn new(graceful_termination_attempts_before_force: u32) -> Self {
        Self {
            graceful_termination_attempts_before_force: graceful_termination_attempts_before_force
                .max(1),
        }
    }

    pub fn force_cleanup_recommended(
        &self,
        outcome: &RuntimeCommandCleanupOutcome,
        cleanup_attempts: u32,
    ) -> bool {
        matches!(outcome, RuntimeCommandCleanupOutcome::TerminationFailed)
            && cleanup_attempts >= self.graceful_termination_attempts_before_force
    }
}

impl Default for RuntimeProcessCleanupPolicy {
    fn default() -> Self {
        Self::new(1)
    }
}

/// Cleanup sweep result for runtime-owned orphaned or cleanup-requested processes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeProcessCleanupSweep {
    pub observed_at_ms: u64,
    pub removed_stale: Vec<RuntimeProcessObservation>,
    pub termination_requested: Vec<RuntimeProcessObservation>,
    pub termination_failed: Vec<RuntimeProcessCleanupFailure>,
}

impl RuntimeProcessCleanupSweep {
    pub fn new(observed_at_ms: u64) -> Self {
        Self {
            observed_at_ms,
            removed_stale: Vec::new(),
            termination_requested: Vec::new(),
            termination_failed: Vec::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.removed_stale.is_empty()
            && self.termination_requested.is_empty()
            && self.termination_failed.is_empty()
    }

    pub fn command_receipt(&self) -> RuntimeCommandCleanupReceipt {
        self.command_receipt_with_policy(&RuntimeProcessCleanupPolicy::default())
    }

    pub fn command_receipt_with_policy(
        &self,
        policy: &RuntimeProcessCleanupPolicy,
    ) -> RuntimeCommandCleanupReceipt {
        let removed_stale = self.removed_stale.iter().map(|process| {
            RuntimeCommandCleanupEntry::from_process(
                process,
                RuntimeCommandCleanupOutcome::RemovedStale,
                None,
                policy,
            )
        });
        let termination_requested = self.termination_requested.iter().map(|process| {
            RuntimeCommandCleanupEntry::from_process(
                process,
                RuntimeCommandCleanupOutcome::TerminationRequested,
                None,
                policy,
            )
        });
        let termination_failed = self.termination_failed.iter().map(|failure| {
            RuntimeCommandCleanupEntry::from_process(
                &failure.process,
                RuntimeCommandCleanupOutcome::TerminationFailed,
                Some(failure.error.clone()),
                policy,
            )
        });

        RuntimeCommandCleanupReceipt {
            observed_at_ms: self.observed_at_ms,
            entries: removed_stale
                .chain(termination_requested)
                .chain(termination_failed)
                .collect(),
        }
    }
}

/// Command-facing cleanup receipt for runtime-owned child processes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeCommandCleanupReceipt {
    pub observed_at_ms: u64,
    pub entries: Vec<RuntimeCommandCleanupEntry>,
}

impl RuntimeCommandCleanupReceipt {
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// One command-level cleanup outcome projected from a process sweep.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeCommandCleanupEntry {
    pub pid: u32,
    pub handle: RuntimeProcessHandle,
    pub kind: RuntimeProcessKind,
    pub owner_reference: String,
    pub status: RuntimeProcessStatus,
    pub command: Option<RuntimeCommandObservation>,
    pub outcome: RuntimeCommandCleanupOutcome,
    pub error: Option<String>,
    pub cleanup_attempts: u32,
    pub last_cleanup_attempt_at_ms: Option<u64>,
    pub retained_in_registry: bool,
    pub requires_follow_up: bool,
    pub force_cleanup_recommended: bool,
}

impl RuntimeCommandCleanupEntry {
    fn from_process(
        process: &RuntimeProcessObservation,
        outcome: RuntimeCommandCleanupOutcome,
        error: Option<String>,
        policy: &RuntimeProcessCleanupPolicy,
    ) -> Self {
        let retained_in_registry = outcome.retained_in_registry();
        let requires_follow_up = outcome.requires_follow_up();
        let force_cleanup_recommended =
            policy.force_cleanup_recommended(&outcome, process.cleanup_attempts);
        Self {
            pid: process.pid,
            handle: process.handle.clone(),
            kind: process.kind.clone(),
            owner_reference: process.owner_reference.clone(),
            status: process.status.clone(),
            command: process.command.clone(),
            outcome,
            error,
            cleanup_attempts: process.cleanup_attempts,
            last_cleanup_attempt_at_ms: process.last_cleanup_attempt_at_ms(),
            retained_in_registry,
            requires_follow_up,
            force_cleanup_recommended,
        }
    }
}

/// Command-facing cleanup outcome for a runtime-owned child process.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuntimeCommandCleanupOutcome {
    RemovedStale,
    TerminationRequested,
    TerminationFailed,
}

impl RuntimeCommandCleanupOutcome {
    pub fn retained_in_registry(&self) -> bool {
        matches!(self, Self::TerminationRequested | Self::TerminationFailed)
    }

    pub fn requires_follow_up(&self) -> bool {
        matches!(self, Self::TerminationRequested | Self::TerminationFailed)
    }
}

/// Process liveness backend used by cleanup sweeps.
pub trait RuntimeProcessLiveness {
    fn is_process_alive(&mut self, pid: u32) -> bool;
}

/// Process termination backend used by cleanup sweeps.
pub trait RuntimeProcessTerminator {
    fn request_termination(&mut self, pid: u32) -> Result<bool, String>;
}

/// Runtime process cleanup backend used by registry cleanup sweeps.
pub trait RuntimeProcessCleanupController:
    RuntimeProcessLiveness + RuntimeProcessTerminator
{
}

impl<T> RuntimeProcessCleanupController for T where
    T: RuntimeProcessLiveness + RuntimeProcessTerminator
{
}

/// `sysinfo`-backed runtime process controller for cleanup sweeps.
#[derive(Debug, Default)]
pub struct SysinfoRuntimeProcessController {
    system: sysinfo::System,
}

impl SysinfoRuntimeProcessController {
    pub fn new() -> Self {
        Self::default()
    }

    fn refresh_process(&mut self, pid: u32) -> Option<&sysinfo::Process> {
        let pid = sysinfo::Pid::from_u32(pid);
        self.system
            .refresh_processes(sysinfo::ProcessesToUpdate::Some(&[pid]), true);
        self.system.process(pid)
    }
}

impl RuntimeProcessLiveness for SysinfoRuntimeProcessController {
    fn is_process_alive(&mut self, pid: u32) -> bool {
        self.refresh_process(pid).is_some()
    }
}

impl RuntimeProcessTerminator for SysinfoRuntimeProcessController {
    fn request_termination(&mut self, pid: u32) -> Result<bool, String> {
        Ok(self
            .refresh_process(pid)
            .map(sysinfo::Process::kill)
            .unwrap_or(false))
    }
}
