// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Runtime-owned child process visibility and cleanup state.

mod cleanup;
mod ledger;
mod managed;
mod managed_reaper;
mod reconcile;
mod registry;
mod shutdown;
mod types;
mod watchdog;

pub use cleanup::{
    RuntimeCommandCleanupEntry, RuntimeCommandCleanupOutcome, RuntimeCommandCleanupReceipt,
    RuntimeProcessCleanupController, RuntimeProcessCleanupFailure, RuntimeProcessCleanupPolicy,
    RuntimeProcessCleanupSweep, RuntimeProcessLiveness, RuntimeProcessTerminator,
    SysinfoRuntimeProcessController,
};
pub use ledger::{
    FileProcessLedgerStore, InMemoryProcessLedgerStore, ManagedProcessLedgerEntry,
    ManagedProcessLedgerRecordRequest, ProcessGroupId, ProcessLedgerStore, ProcessStartTimeSeconds,
    RuntimeHostRunId,
};
pub use managed::{
    AsyncManagedChildProcess, DEFAULT_MAX_CAPTURED_PIPE_BYTES, ManagedChildProcessSpec,
    ManagedChildStdin, ManagedChildStdout, ManagedStdoutError,
    RuntimeProcessContainmentRequirement, WatchdogCleanup,
};
pub use reconcile::{
    OsProcessProbeResult, PersistedProcessReconciliationReport, ReapSurvivedProcessRequest,
    SurvivedProcessObservation, SurvivedProcessReapReceipt, probe_persisted_process,
    reap_survived_entry, reconcile_persisted_ledger, sample_os_start_time_secs,
};
pub use registry::{RuntimeProcessRegistry, RuntimeProcessRegistryHandle};
pub use shutdown::{
    DEFAULT_COOPERATIVE_SHUTDOWN_TIMEOUT, DEFAULT_GRACEFUL_SHUTDOWN_TIMEOUT,
    ProcessGracefulShutdownDurationMs, ProcessGracefulShutdownPolicy, ProcessShutdownHook,
    ProcessTerminationStage, SignalDeliveryOutcome, is_process_group_alive,
    signal_process_group_kill, signal_process_group_term,
};
pub use types::{
    RuntimeCommandKind, RuntimeCommandObservation, RuntimeProcessExitStatus, RuntimeProcessHandle,
    RuntimeProcessKind, RuntimeProcessKindCounts, RuntimeProcessObservation,
    RuntimeProcessObservationTimestampMs, RuntimeProcessOutput, RuntimeProcessRegistrationError,
    RuntimeProcessRegistrySnapshot, RuntimeProcessStatus, RuntimeProcessStatusCounts,
};
pub use watchdog::{
    HostHeartbeat, ProcessCommandWatchdog, ProcessDeadlineElapsed, ProcessHealthCondition,
    ProcessHealthMonitor, ProcessHealthPolicy, ProcessHealthReport, ProcessWatchdog,
    ProcessWatchdogReport,
};
