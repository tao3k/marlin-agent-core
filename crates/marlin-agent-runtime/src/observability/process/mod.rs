// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

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
