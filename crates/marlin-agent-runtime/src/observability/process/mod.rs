//! Runtime-owned child process visibility and cleanup state.

mod cleanup;
mod managed;
mod registry;
mod types;

pub use cleanup::{
    RuntimeCommandCleanupEntry, RuntimeCommandCleanupOutcome, RuntimeCommandCleanupReceipt,
    RuntimeProcessCleanupController, RuntimeProcessCleanupFailure, RuntimeProcessCleanupPolicy,
    RuntimeProcessCleanupSweep, RuntimeProcessLiveness, RuntimeProcessTerminator,
    SysinfoRuntimeProcessController,
};
pub use managed::{AsyncManagedChildProcess, ManagedChildProcessSpec};
pub use registry::{RuntimeProcessRegistry, RuntimeProcessRegistryHandle};
pub use types::{
    RuntimeCommandKind, RuntimeCommandObservation, RuntimeProcessExitStatus, RuntimeProcessHandle,
    RuntimeProcessKind, RuntimeProcessKindCounts, RuntimeProcessObservation,
    RuntimeProcessObservationTimestampMs, RuntimeProcessOutput, RuntimeProcessRegistrationError,
    RuntimeProcessRegistrySnapshot, RuntimeProcessStatus, RuntimeProcessStatusCounts,
};
