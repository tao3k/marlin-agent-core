// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Tokio-backed managed child process wrapper for runtime-owned processes.

use std::{
    io,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context, Poll},
    time::{SystemTime, UNIX_EPOCH},
};

#[cfg(unix)]
use process_wrap::tokio::ProcessGroup;
use process_wrap::tokio::{ChildWrapper, CommandWrap, KillOnDrop};
use tokio::io::{AsyncReadExt, AsyncWrite};
use tokio::process::{ChildStdin as AsyncChildStdin, Command as AsyncCommand};
use tokio::sync::OwnedSemaphorePermit;

use super::registry::RuntimeProcessRegistryHandle;
use super::types::{
    RuntimeCommandObservation, RuntimeProcessExitStatus, RuntimeProcessHandle, RuntimeProcessKind,
    RuntimeProcessObservation, RuntimeProcessOutput,
};
use super::watchdog::{ProcessCommandWatchdog, ProcessWatchdog, ProcessWatchdogReport};
use crate::tokio_runtime::{
    RuntimeAdmissionError, RuntimeContext, RuntimeOperationId, RuntimeResourceId, RuntimeTaskGuard,
};

/// Required operating-system containment for a managed process.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum RuntimeProcessContainmentRequirement {
    /// Use the platform's existing managed-process boundary.
    #[default]
    PlatformDefault,
    /// Require an OS-enforced boundary that prevents descendants escaping.
    EnforcedDescendantTree,
}

/// Named spawn request for a runtime-owned child process.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManagedChildProcessSpec {
    pub kind: RuntimeProcessKind,
    pub handle: RuntimeProcessHandle,
    pub owner_reference: String,
    pub command: Option<RuntimeCommandObservation>,
    pub started_at_ms: Option<u64>,
    pub containment: RuntimeProcessContainmentRequirement,
}

impl ManagedChildProcessSpec {
    pub fn new(kind: RuntimeProcessKind, owner_reference: impl Into<String>) -> Self {
        let owner_reference = owner_reference.into();
        Self {
            kind,
            handle: RuntimeProcessHandle::new(owner_reference.clone()),
            owner_reference,
            command: None,
            started_at_ms: None,
            containment: RuntimeProcessContainmentRequirement::default(),
        }
    }

    pub fn with_containment(mut self, containment: RuntimeProcessContainmentRequirement) -> Self {
        self.containment = containment;
        self
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
        self.started_at_ms = Some(started_at_ms);
        self
    }
}

/// Runtime-owned async child process wrapper backed by Tokio and process-wrap.
#[derive(Debug)]
pub struct AsyncManagedChildProcess {
    child: Option<Box<dyn ChildWrapper>>,
    registry: RuntimeProcessRegistryHandle,
    pid: u32,
    completed: bool,
    permit: ProcessPermit,
    identity: RuntimeResourceId,
    stop: tokio_util::sync::CancellationToken,
    runtime: tokio::runtime::Handle,
}

// Shared with every pipe handed out or drained by the runtime. The last
// reference releases capacity only after process cleanup was confirmed.
#[derive(Debug)]
struct ProcessLease {
    permit: Option<OwnedSemaphorePermit>,
    confirmed: AtomicBool,
    ownership: Option<RuntimeTaskGuard>,
}

impl Drop for ProcessLease {
    fn drop(&mut self) {
        if !self.confirmed.load(Ordering::Acquire) {
            if let Some(permit) = self.permit.take() {
                permit.forget();
            }
            if let Some(ownership) = self.ownership.take() {
                ownership.leave_unresolved();
            }
        }
    }
}

#[derive(Debug)]
struct ProcessPermit(Option<Arc<ProcessLease>>);

impl ProcessPermit {
    fn release(&mut self) {
        if let Some(lease) = self.0.take() {
            lease.confirmed.store(true, Ordering::Release);
        }
    }
}

// Field order matters: close the actual pipe before dropping its lease.
#[derive(Debug)]
struct LeasedPipe<T> {
    inner: T,
    _lease: Option<Arc<ProcessLease>>,
}

/// An owned stdin writer that retains its process quota until dropped.
#[derive(Debug)]
pub struct ManagedChildStdin {
    pipe: LeasedPipe<AsyncChildStdin>,
}

/// Failure while collecting bounded stdout from a managed child.
#[derive(Debug)]
pub enum ManagedStdoutError {
    Io(io::Error),
    LimitExceeded,
}

/// The owner's decision on a watchdog report.
#[derive(Debug)]
#[must_use]
pub enum WatchdogCleanup {
    /// Ownership has moved to the Runtime reaper.
    Started,
    /// The report targeted a different resource. The current handle is returned.
    Stale(AsyncManagedChildProcess),
    /// An advisory observation leaves the process with its current owner.
    Advisory(AsyncManagedChildProcess),
}

impl AsyncWrite for ManagedChildStdin {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.pipe.inner).poll_write(cx, buf)
    }
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.pipe.inner).poll_flush(cx)
    }
    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.pipe.inner).poll_shutdown(cx)
    }
}

impl AsyncManagedChildProcess {
    /// Collect stdout up to `limit` bytes, terminating the child on overflow.
    /// Callers must not configure an undrained piped stderr.
    pub async fn wait_with_stdout_limit(
        mut self,
        limit: usize,
    ) -> Result<RuntimeProcessOutput, ManagedStdoutError> {
        self.collect_bounded_stdout(limit, || {}).await
    }

    pub(crate) async fn collect_bounded_stdout(
        &mut self,
        limit: usize,
        mut record_output: impl FnMut() + Send,
    ) -> Result<RuntimeProcessOutput, ManagedStdoutError> {
        let stop = self.stop.clone();
        tokio::select! {
            biased;
            () = stop.cancelled() => Err(ManagedStdoutError::Io(stop_requested())),
            output = self.read_bounded_stdout(limit, &mut record_output) => output,
        }
    }

    async fn read_bounded_stdout(
        &mut self,
        limit: usize,
        record_output: &mut (impl FnMut() + Send),
    ) -> Result<RuntimeProcessOutput, ManagedStdoutError> {
        let pipe = self
            .child
            .as_mut()
            .and_then(|child| child.stdout().take())
            .ok_or_else(|| {
                ManagedStdoutError::Io(io::Error::other("managed stdout unavailable"))
            })?;
        let mut output = Vec::new();
        let mut pipe = LeasedPipe {
            inner: pipe,
            _lease: self.permit.0.clone(),
        };
        let count = limit
            .checked_add(1)
            .ok_or_else(|| ManagedStdoutError::Io(io::Error::other("stdout limit overflow")))?;
        let mut reader = (&mut pipe.inner).take(count as u64);
        let mut buffer = [0; 8192];
        loop {
            let bytes = reader
                .read(&mut buffer)
                .await
                .map_err(ManagedStdoutError::Io)?;
            if bytes == 0 {
                break;
            }
            record_output();
            output.extend_from_slice(&buffer[..bytes]);
        }
        drop(pipe);
        if output.len() > limit {
            self.request_cleanup();
            let child = self
                .child
                .as_mut()
                .expect("managed child process must own a child");
            if let Err(error) = child.start_kill() {
                // The group may disappear between output EOF and the signal;
                // only accept that race if the direct child is actually reaped.
                match tokio::time::timeout(std::time::Duration::from_millis(100), child.wait())
                    .await
                {
                    Ok(Ok(_)) => {}
                    _ => return Err(ManagedStdoutError::Io(error)),
                }
            }
            self.wait_for_exit(current_time_millis())
                .await
                .map_err(ManagedStdoutError::Io)?;
            return Err(ManagedStdoutError::LimitExceeded);
        }
        let status = self
            .wait_for_exit(current_time_millis())
            .await
            .map_err(ManagedStdoutError::Io)?;
        Ok(RuntimeProcessOutput {
            status,
            stdout: output,
            stderr: Vec::new(),
        })
    }

    pub async fn spawn(
        context: &RuntimeContext,
        command: AsyncCommand,
        kind: RuntimeProcessKind,
        owner_reference: impl Into<String>,
    ) -> io::Result<Self> {
        Self::spawn_with_spec(
            context,
            command,
            ManagedChildProcessSpec::new(kind, owner_reference),
        )
        .await
    }

    pub async fn spawn_with_spec(
        context: &RuntimeContext,
        command: AsyncCommand,
        spec: ManagedChildProcessSpec,
    ) -> io::Result<Self> {
        if spec.containment == RuntimeProcessContainmentRequirement::EnforcedDescendantTree {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "enforced descendant-tree containment is unavailable",
            ));
        }
        let (permit, guard) =
            Self::reserve_process(context, RuntimeOperationId::new(spec.handle.as_str())).await?;
        Self::spawn_admitted(context, command, spec, permit, guard)
    }

    async fn reserve_process(
        context: &RuntimeContext,
        operation: RuntimeOperationId,
    ) -> io::Result<(Option<OwnedSemaphorePermit>, RuntimeTaskGuard)> {
        let permit = if let Some(quota) = &context.process_quota {
            let cancellation = context.cancellation_token();
            let permit = tokio::select! {
                biased;
                _ = context.task_tracker.closed() => return Err(io::Error::new(io::ErrorKind::Interrupted, RuntimeAdmissionError::Closed)),
                _ = cancellation.cancelled() => return Err(io::Error::new(io::ErrorKind::Interrupted, "process admission cancelled")),
                result = quota.clone().acquire_owned() => result.map_err(|_| io::Error::other("process quota closed"))?,
            };
            Some(permit)
        } else {
            None
        };
        let observed_memory = context
            .task_tracker
            .sample_process_memory(&context.process_registry())?;
        let guard = context
            .task_tracker
            .admit_process(context.session().session_id(), operation, observed_memory)
            .map_err(|error| {
                let kind = match error {
                    RuntimeAdmissionError::Closed => io::ErrorKind::Interrupted,
                    RuntimeAdmissionError::OperationActive => io::ErrorKind::AlreadyExists,
                    RuntimeAdmissionError::GenerationExhausted => io::ErrorKind::Other,
                    RuntimeAdmissionError::MemoryBudgetExceeded => io::ErrorKind::WouldBlock,
                };
                io::Error::new(kind, error)
            })?;
        if context.is_cancelled() {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "process admission cancelled",
            ));
        }
        Ok((permit, guard))
    }

    fn spawn_admitted(
        context: &RuntimeContext,
        command: AsyncCommand,
        spec: ManagedChildProcessSpec,
        permit: Option<OwnedSemaphorePermit>,
        guard: RuntimeTaskGuard,
    ) -> io::Result<Self> {
        let identity = guard.id().clone();
        let runtime = tokio::runtime::Handle::current();
        let mut permit = ProcessPermit(Some(Arc::new(ProcessLease {
            permit,
            confirmed: AtomicBool::new(false),
            ownership: Some(guard),
        })));
        let mut command = CommandWrap::from(command);
        command.wrap(KillOnDrop);
        #[cfg(unix)]
        command.wrap(ProcessGroup::leader());

        let child = match command.spawn() {
            Ok(child) => child,
            Err(error) => {
                permit.release();
                return Err(error);
            }
        };
        let pid = child.id().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "process-wrap child did not expose a process id",
            )
        })?;
        let registry = context.process_registry();
        let started_at_ms = spec.started_at_ms.unwrap_or_else(current_time_millis);
        let mut observation = RuntimeProcessObservation::new(pid, spec.kind, spec.owner_reference)
            .with_handle(spec.handle)
            .with_resource_id(identity.clone())
            .with_started_at_ms(started_at_ms);
        if let Some(command) = spec.command {
            observation = observation.with_command(command);
        }
        let stop = tokio_util::sync::CancellationToken::new();
        let registration = registry.track_managed(observation, stop.clone());
        let process = Self {
            child: Some(child),
            registry,
            pid,
            completed: false,
            permit,
            identity,
            stop,
            runtime,
        };
        registration.map_err(|error| io::Error::new(io::ErrorKind::AlreadyExists, error))?;
        Ok(process)
    }

    pub fn resource_id(&self) -> &RuntimeResourceId {
        &self.identity
    }

    /// Capture this generation for a watchdog without transferring cleanup authority.
    pub fn watchdog(&self) -> ProcessWatchdog {
        ProcessWatchdog(self.identity.clone())
    }

    /// Bind a command deadline to this owned generation.
    pub fn command_watchdog(&self, deadline: tokio::time::Instant) -> ProcessCommandWatchdog {
        ProcessCommandWatchdog {
            resource: self.identity.clone(),
            deadline,
        }
    }

    /// Apply a deadline report to this owned process. Matching reports start
    /// cleanup; the existing reaper retains ownership until cleanup is confirmed.
    pub fn apply_watchdog_report(self, report: ProcessWatchdogReport<'_>) -> WatchdogCleanup {
        let observation = match report {
            ProcessWatchdogReport::Health(observation) => {
                return if self.resource_id() == observation.resource_id() {
                    WatchdogCleanup::Advisory(self)
                } else {
                    WatchdogCleanup::Stale(self)
                };
            }
            ProcessWatchdogReport::CommandDeadline(observation) => observation,
        };
        if self.resource_id() != observation.resource_id() {
            return WatchdogCleanup::Stale(self);
        }
        drop(self);
        WatchdogCleanup::Started
    }

    pub fn pid(&self) -> u32 {
        self.pid
    }

    pub fn take_stdin(&mut self) -> Option<ManagedChildStdin> {
        let inner = self.child.as_mut()?.stdin().take()?;
        Some(ManagedChildStdin {
            pipe: LeasedPipe {
                inner,
                _lease: self.permit.0.clone(),
            },
        })
    }

    pub fn request_cleanup(&self) {
        self.request_cleanup_observed_at(current_time_millis());
    }

    pub fn request_cleanup_observed_at(&self, observed_at_ms: u64) {
        self.registry
            .request_cleanup_for(&self.identity, observed_at_ms);
    }

    pub async fn wait(self) -> io::Result<RuntimeProcessExitStatus> {
        self.wait_observed_at(current_time_millis()).await
    }

    pub async fn wait_observed_at(
        mut self,
        observed_at_ms: u64,
    ) -> io::Result<RuntimeProcessExitStatus> {
        let stop = self.stop.clone();
        tokio::select! {
            biased;
            () = stop.cancelled() => Err(stop_requested()),
            status = self.wait_for_exit(observed_at_ms) => status,
        }
    }

    async fn wait_for_exit(&mut self, observed_at_ms: u64) -> io::Result<RuntimeProcessExitStatus> {
        let status = self
            .child
            .as_mut()
            .expect("managed child process must own a child")
            .wait()
            .await
            .map(RuntimeProcessExitStatus::from);
        let status = self.confirm_process_group_exit(status).await;
        self.record_terminal_status(&status, observed_at_ms);
        status
    }

    pub async fn wait_with_output(self) -> io::Result<RuntimeProcessOutput> {
        self.wait_with_output_observed_at(current_time_millis())
            .await
    }

    pub async fn wait_with_output_observed_at(
        mut self,
        observed_at_ms: u64,
    ) -> io::Result<RuntimeProcessOutput> {
        let stop = self.stop.clone();
        tokio::select! {
            biased;
            () = stop.cancelled() => Err(stop_requested()),
            output = self.read_output(observed_at_ms) => output,
        }
    }

    async fn read_output(&mut self, observed_at_ms: u64) -> io::Result<RuntimeProcessOutput> {
        let lease = self.permit.0.clone();
        let child = self
            .child
            .as_mut()
            .expect("managed child process must own a child");
        let mut stdout = child.stdout().take().map(|inner| LeasedPipe {
            inner,
            _lease: lease.clone(),
        });
        let mut stderr = child.stderr().take().map(|inner| LeasedPipe {
            inner,
            _lease: lease,
        });
        let read_stdout = async {
            let mut bytes = Vec::new();
            if let Some(stream) = &mut stdout {
                stream.inner.read_to_end(&mut bytes).await?;
            }
            Ok::<_, io::Error>(bytes)
        };
        let read_stderr = async {
            let mut bytes = Vec::new();
            if let Some(stream) = &mut stderr {
                stream.inner.read_to_end(&mut bytes).await?;
            }
            Ok::<_, io::Error>(bytes)
        };
        let output = tokio::try_join!(child.wait(), read_stdout, read_stderr).map(
            |(status, stdout, stderr)| RuntimeProcessOutput {
                status: status.into(),
                stdout,
                stderr,
            },
        );
        // Close owned read ends before the permit can be returned.
        drop(stdout);
        drop(stderr);
        let output = match output {
            Ok(output) => self.confirm_process_group_exit(Ok(output)).await,
            Err(error) => Err(error),
        };
        self.record_output_status(&output, observed_at_ms);
        output
    }

    pub async fn kill(self) -> io::Result<RuntimeProcessExitStatus> {
        self.kill_observed_at(current_time_millis()).await
    }

    pub async fn kill_observed_at(
        mut self,
        observed_at_ms: u64,
    ) -> io::Result<RuntimeProcessExitStatus> {
        self.request_cleanup_observed_at(observed_at_ms);
        self.child
            .as_mut()
            .expect("managed child process must own a child")
            .start_kill()?;
        let status = self
            .child
            .as_mut()
            .expect("managed child process must own a child")
            .wait()
            .await
            .map(RuntimeProcessExitStatus::from);
        let status = self.confirm_process_group_exit(status).await;
        self.record_terminal_status(&status, observed_at_ms);
        status
    }

    async fn confirm_process_group_exit<T>(&self, result: io::Result<T>) -> io::Result<T> {
        let value = result?;
        wait_for_process_group_exit(self.pid).await?;
        Ok(value)
    }

    fn record_terminal_status(
        &mut self,
        status: &io::Result<RuntimeProcessExitStatus>,
        observed_at_ms: u64,
    ) {
        match status {
            Ok(exit_status) if exit_status.success() => {
                self.registry
                    .complete_managed(&self.identity, true, observed_at_ms);
                self.completed = true;
                self.child = None;
                self.permit.release();
            }
            Ok(_) => {
                self.registry
                    .complete_managed(&self.identity, false, observed_at_ms);
                self.completed = true;
                self.child = None;
                self.permit.release();
            }
            Err(_) => {
                self.registry
                    .request_cleanup_for(&self.identity, observed_at_ms);
            }
        }
    }

    fn record_output_status(
        &mut self,
        output: &io::Result<RuntimeProcessOutput>,
        observed_at_ms: u64,
    ) {
        match output {
            Ok(output) if output.status.success() => {
                self.registry
                    .complete_managed(&self.identity, true, observed_at_ms);
                self.completed = true;
                self.child = None;
                self.permit.release();
            }
            Ok(_) => {
                self.registry
                    .complete_managed(&self.identity, false, observed_at_ms);
                self.completed = true;
                self.child = None;
                self.permit.release();
            }
            Err(_) => {
                self.registry
                    .request_cleanup_for(&self.identity, observed_at_ms);
            }
        }
    }
}

fn stop_requested() -> io::Error {
    io::Error::new(io::ErrorKind::Interrupted, "managed process stop requested")
}

impl Drop for AsyncManagedChildProcess {
    fn drop(&mut self) {
        if !self.completed {
            self.registry
                .request_cleanup_for(&self.identity, current_time_millis());
            if let Some(mut child) = self.child.take() {
                // A completed wait may already have reaped the child while
                // output pipes were still draining; avoid signalling a reused PID.
                if child.id().is_some() {
                    let _ = child.start_kill();
                }
                let mut permit = std::mem::replace(&mut self.permit, ProcessPermit(None));
                let registry = self.registry.clone();
                let pid = self.pid;
                let identity = self.identity.clone();
                self.runtime.spawn(async move {
                    if let Ok(status) = child.wait().await {
                        if wait_for_process_group_exit(pid).await.is_err() {
                            return;
                        }
                        // Close the wrapper's pipe ends before releasing the lease.
                        drop(child);
                        registry.complete_managed(
                            &identity,
                            status.success(),
                            current_time_millis(),
                        );
                        permit.release();
                    }
                });
            }
        }
    }
}

// Unix process groups are the containment boundary supplied by process-wrap.
// An unknown probe result fails closed; escaped descendants are outside it.
#[cfg(unix)]
async fn wait_for_process_group_exit(pid: u32) -> io::Result<()> {
    let pgid = i32::try_from(pid)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid process group"))?;
    loop {
        // SAFETY: signal 0 does not signal processes; negative PID probes the group.
        if unsafe { libc::kill(-pgid, 0) } == -1 {
            let error = io::Error::last_os_error();
            if error.raw_os_error() == Some(libc::ESRCH) {
                return Ok(());
            }
            if error.raw_os_error() != Some(libc::EPERM) {
                return Err(error);
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
}

#[cfg(not(unix))]
async fn wait_for_process_group_exit(_pid: u32) -> io::Result<()> {
    Ok(())
}

fn current_time_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or_default()
}

#[cfg(test)]
#[path = "../../tests/unit/runtime/process/lease_release.rs"]
mod tests;
