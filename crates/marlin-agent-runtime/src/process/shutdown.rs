// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Graceful two-phase process shutdown policy, cooperative hooks, and OS signaling.

use std::{fmt, future::Future, io, pin::Pin, sync::Arc, time::Duration};

use serde::{Deserialize, Serialize};

/// Default grace period given to a managed process after `SIGTERM` (or a
/// cooperative shutdown hook) before escalating to `SIGKILL`.
pub const DEFAULT_GRACEFUL_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(3);

/// Default timeout for an optional cooperative shutdown hook (e.g. HTTP `/shutdown`).
pub const DEFAULT_COOPERATIVE_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(2);

/// Typed graceful shutdown window duration stored in the persistent ledger.
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProcessGracefulShutdownDurationMs(u64);

impl ProcessGracefulShutdownDurationMs {
    /// Construct a typed millisecond duration for graceful shutdown accounting.
    pub const fn new(millis: u64) -> Self {
        Self(millis)
    }

    /// Return the duration in milliseconds.
    pub const fn as_millis(self) -> u64 {
        self.0
    }
}

/// Lifecycle termination stage recorded in both the persistent ledger and
/// OpenTelemetry process spans.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessTerminationStage {
    #[default]
    Running,
    CooperativeShutdownRequested,
    GracefulTermSent,
    ForceKillEscalated,
    ExitedCleanly,
    ExitedFailed,
    SurvivedRestart,
    OrphanReaped,
    ReconciledDeadAfterRestart,
}

impl ProcessTerminationStage {
    /// Stable low-cardinality tracing and ledger label.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::CooperativeShutdownRequested => "cooperative_shutdown_requested",
            Self::GracefulTermSent => "graceful_term_sent",
            Self::ForceKillEscalated => "force_kill_escalated",
            Self::ExitedCleanly => "exited_cleanly",
            Self::ExitedFailed => "exited_failed",
            Self::SurvivedRestart => "survived_restart",
            Self::OrphanReaped => "orphan_reaped",
            Self::ReconciledDeadAfterRestart => "reconciled_dead_after_restart",
        }
    }

    /// Returns true once the process is confirmed no longer running.
    pub const fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::ExitedCleanly
                | Self::ExitedFailed
                | Self::OrphanReaped
                | Self::ReconciledDeadAfterRestart
        )
    }
}

/// Typed graceful shutdown policy for a managed process and its process group.
///
/// Shutdown always runs in ordered stages:
/// 1. Optional cooperative shutdown hook (bounded by `cooperative_timeout`),
/// 2. Graceful `SIGTERM` to the process group (bounded by `graceful_timeout`),
/// 3. `SIGKILL` escalation to the process group only if still alive and
///    `escalate_to_sigkill` is enabled.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProcessGracefulShutdownPolicy {
    pub cooperative_timeout: Duration,
    pub graceful_timeout: Duration,
    pub escalate_to_sigkill: bool,
}

impl ProcessGracefulShutdownPolicy {
    /// Construct a policy with an explicit graceful `SIGTERM` window.
    pub const fn new(graceful_timeout: Duration) -> Self {
        Self {
            cooperative_timeout: DEFAULT_COOPERATIVE_SHUTDOWN_TIMEOUT,
            graceful_timeout,
            escalate_to_sigkill: true,
        }
    }

    /// Construct an immediate force-kill policy with zero grace period.
    pub const fn immediate_kill() -> Self {
        Self {
            cooperative_timeout: Duration::ZERO,
            graceful_timeout: Duration::ZERO,
            escalate_to_sigkill: true,
        }
    }

    /// Override the cooperative hook timeout.
    pub const fn with_cooperative_timeout(mut self, timeout: Duration) -> Self {
        self.cooperative_timeout = timeout;
        self
    }

    /// Override the graceful `SIGTERM` wait window.
    pub const fn with_graceful_timeout(mut self, timeout: Duration) -> Self {
        self.graceful_timeout = timeout;
        self
    }

    /// Control whether `SIGKILL` escalation is allowed after the grace period.
    pub const fn with_force_kill_escalation(mut self, escalate: bool) -> Self {
        self.escalate_to_sigkill = escalate;
        self
    }

    /// Graceful timeout encoded as a typed millisecond duration.
    pub fn graceful_timeout_duration_ms(&self) -> ProcessGracefulShutdownDurationMs {
        let millis = u64::try_from(self.graceful_timeout.as_millis()).unwrap_or(u64::MAX);
        ProcessGracefulShutdownDurationMs::new(millis)
    }
}

impl Default for ProcessGracefulShutdownPolicy {
    fn default() -> Self {
        Self::new(DEFAULT_GRACEFUL_SHUTDOWN_TIMEOUT)
    }
}

type ShutdownCallback =
    Arc<dyn Fn() -> Pin<Box<dyn Future<Output = io::Result<()>> + Send>> + Send + Sync>;

/// Optional cooperative shutdown callback invoked before `SIGTERM` / `SIGKILL`.
#[derive(Clone)]
pub struct ProcessShutdownHook {
    label: &'static str,
    callback: ShutdownCallback,
}

impl ProcessShutdownHook {
    /// Wrap an async shutdown callback with a low-cardinality diagnostic label.
    pub fn new<F, Fut>(label: &'static str, callback: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = io::Result<()>> + Send + 'static,
    {
        Self {
            label,
            callback: Arc::new(move || Box::pin(callback())),
        }
    }

    /// Wrap a synchronous shutdown callback on a dedicated helper thread so the
    /// Tokio worker loop is never blocked.
    pub fn from_blocking<F>(label: &'static str, callback: F) -> Self
    where
        F: Fn() -> io::Result<()> + Send + Sync + 'static,
    {
        let callback = Arc::new(callback);
        Self::new(label, move || {
            let callback = Arc::clone(&callback);
            async move {
                let (tx, rx) = tokio::sync::oneshot::channel();
                std::thread::Builder::new()
                    .name(format!("marlin-shutdown-{label}"))
                    .spawn(move || {
                        let _ = tx.send(callback());
                    })?;
                rx.await.map_err(|_| {
                    io::Error::other(format!(
                        "shutdown hook '{label}' helper exited unexpectedly"
                    ))
                })?
            }
        })
    }

    /// Low-cardinality label identifying this shutdown hook.
    pub const fn label(&self) -> &'static str {
        self.label
    }

    /// Invoke the cooperative shutdown hook with a hard upper time bound.
    pub async fn invoke(&self, timeout: Duration) -> io::Result<()> {
        if timeout.is_zero() {
            return Ok(());
        }
        match tokio::time::timeout(timeout, (self.callback)()).await {
            Ok(result) => result,
            Err(_) => Err(io::Error::new(
                io::ErrorKind::TimedOut,
                format!("cooperative shutdown hook '{}' timed out", self.label),
            )),
        }
    }
}

impl fmt::Debug for ProcessShutdownHook {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProcessShutdownHook")
            .field("label", &self.label)
            .finish()
    }
}

/// Outcome of signaling a managed process or its Unix process group.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SignalDeliveryOutcome {
    Delivered,
    AlreadyExited,
}

/// Send `SIGTERM` to a Unix process group (falling back to the leader PID).
pub fn signal_process_group_term(pid: u32, pgid: Option<u32>) -> io::Result<SignalDeliveryOutcome> {
    #[cfg(unix)]
    {
        signal_unix_target(pid, pgid, libc::SIGTERM)
    }
    #[cfg(not(unix))]
    {
        let _ = (pid, pgid);
        Ok(SignalDeliveryOutcome::Delivered)
    }
}

/// Send `SIGKILL` to a Unix process group (falling back to the leader PID).
pub fn signal_process_group_kill(pid: u32, pgid: Option<u32>) -> io::Result<SignalDeliveryOutcome> {
    #[cfg(unix)]
    {
        signal_unix_target(pid, pgid, libc::SIGKILL)
    }
    #[cfg(not(unix))]
    {
        let _ = (pid, pgid);
        Ok(SignalDeliveryOutcome::Delivered)
    }
}

#[cfg(unix)]
pub(super) fn current_host_pgid() -> Option<u32> {
    // SAFETY: getpgrp takes no arguments and returns the current process group id.
    let pgid = unsafe { libc::getpgrp() };
    u32::try_from(pgid).ok().filter(|&value| value > 1)
}

#[cfg(unix)]
pub(super) fn validate_unix_pgid(pgid: u32) -> io::Result<libc::pid_t> {
    if pgid <= 1 || current_host_pgid() == Some(pgid) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("refusing to target protected or host process group id {pgid}"),
        ));
    }
    i32::try_from(pgid)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid process group id"))
}

#[cfg(unix)]
pub(super) fn validate_signalable_unix_target(
    pid: u32,
    pgid: Option<u32>,
) -> io::Result<(libc::pid_t, libc::pid_t)> {
    if pid <= 1 || pid == std::process::id() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("refusing to signal protected or host process id {pid}"),
        ));
    }
    let pid_i32 = i32::try_from(pid)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid process id"))?;
    let target_pgid = pgid.unwrap_or(pid);
    let pgid_i32 = validate_unix_pgid(target_pgid)?;
    Ok((pid_i32, pgid_i32))
}

/// Probe whether any process remains in the Unix process group `pgid`.
pub fn is_process_group_alive(pgid: u32) -> io::Result<bool> {
    #[cfg(unix)]
    {
        let pgid_i32 = validate_unix_pgid(pgid)?;
        // SAFETY: signal 0 probes existence without delivering a signal; pgid_i32 > 1.
        if unsafe { libc::kill(-pgid_i32, 0) } == 0 {
            return Ok(true);
        }
        let error = io::Error::last_os_error();
        match error.raw_os_error() {
            Some(libc::ESRCH) => Ok(false),
            Some(libc::EPERM) => Ok(true),
            _ => Err(error),
        }
    }
    #[cfg(not(unix))]
    {
        let _ = pgid;
        Ok(false)
    }
}

/// Wait until a Unix process group is observed absent (`ESRCH`).
pub(super) async fn wait_for_process_group_exit(pgid: u32) -> io::Result<()> {
    #[cfg(unix)]
    {
        let pgid_i32 = validate_unix_pgid(pgid)?;
        loop {
            // SAFETY: signal 0 does not signal processes; validated pgid_i32 > 1 probes the group.
            if unsafe { libc::kill(-pgid_i32, 0) } == -1 {
                let error = io::Error::last_os_error();
                if error.raw_os_error() == Some(libc::ESRCH) {
                    return Ok(());
                }
                if error.raw_os_error() != Some(libc::EPERM) {
                    return Err(error);
                }
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }
    #[cfg(not(unix))]
    {
        let _ = pgid;
        Ok(())
    }
}

#[cfg(unix)]
fn signal_unix_target(
    pid: u32,
    pgid: Option<u32>,
    signal: libc::c_int,
) -> io::Result<SignalDeliveryOutcome> {
    let (pid_i32, pgid_i32) = validate_signalable_unix_target(pid, pgid)?;
    // SAFETY: validated pgid_i32 > 1 and != host pgid targets only the child process group.
    if unsafe { libc::kill(-pgid_i32, signal) } == 0 {
        return Ok(SignalDeliveryOutcome::Delivered);
    }
    let group_error = io::Error::last_os_error();
    if group_error.raw_os_error() == Some(libc::ESRCH) {
        // SAFETY: validated pid_i32 > 1 and != host pid targets only that process.
        if unsafe { libc::kill(pid_i32, signal) } == 0 {
            return Ok(SignalDeliveryOutcome::Delivered);
        }
        let pid_error = io::Error::last_os_error();
        if pid_error.raw_os_error() == Some(libc::ESRCH) {
            return Ok(SignalDeliveryOutcome::AlreadyExited);
        }
        return Err(pid_error);
    }
    Err(group_error)
}
