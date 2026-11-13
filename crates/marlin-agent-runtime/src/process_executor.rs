// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Runtime-owned execution of bounded process requests.
use crate::{
    RuntimeContext, RuntimeOperationId,
    process::{
        AsyncManagedChildProcess, HostHeartbeat, ManagedChildProcessSpec, ManagedStdoutError,
        ProcessHealthPolicy, ProcessHealthReport, ProcessWatchdogReport, RuntimeProcessKind,
        WatchdogCleanup,
    },
};
use marlin_agent_protocol::{
    BoundedProcessExecutor, BoundedProcessOutput, BoundedProcessRequest, ProcessEnvironmentPolicy,
    ProcessExecutionError,
};
use std::{future::Future, pin::Pin, process::Stdio};
use tokio::process::Command;
use tokio::sync::mpsc;
use tokio::time::{Instant, timeout_at};

/// Execute bounded tool requests with the Runtime's process and pipe ownership.
#[derive(Clone, Debug)]
pub struct ManagedProcessExecutor {
    context: RuntimeContext,
    kind: RuntimeProcessKind,
    operation: RuntimeOperationId,
    health: Option<(
        HostHeartbeat,
        ProcessHealthPolicy,
        mpsc::Sender<ProcessHealthReport>,
    )>,
}
impl ManagedProcessExecutor {
    pub fn new(
        context: RuntimeContext,
        kind: RuntimeProcessKind,
        operation: RuntimeOperationId,
    ) -> Self {
        Self {
            context,
            kind,
            operation,
            health: None,
        }
    }

    /// Attach an advisory health policy to each process admitted by this executor.
    pub fn with_health_monitor(
        mut self,
        host: HostHeartbeat,
        policy: ProcessHealthPolicy,
        reports: mpsc::Sender<ProcessHealthReport>,
    ) -> Self {
        self.health = Some((host, policy, reports));
        self
    }
}
impl BoundedProcessExecutor for ManagedProcessExecutor {
    fn execute(
        &self,
        request: BoundedProcessRequest,
    ) -> Pin<
        Box<dyn Future<Output = Result<BoundedProcessOutput, ProcessExecutionError>> + Send + '_>,
    > {
        Box::pin(async move {
            let deadline = Instant::now().checked_add(request.timeout).ok_or_else(|| {
                ProcessExecutionError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "process deadline out of range",
                ))
            })?;
            let mut command = Command::new(request.program);
            if request.environment == ProcessEnvironmentPolicy::Clean {
                command.env_clear();
                command.env("PATH", ProcessEnvironmentPolicy::CLEAN_PATH);
            }
            command
                .args(request.arguments)
                .current_dir(request.working_directory)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::null());
            for name in request.remove_environment {
                command.env_remove(name);
            }
            let spec = ManagedChildProcessSpec::new(self.kind.clone(), self.operation.as_str());
            let mut child = timeout_at(
                deadline,
                AsyncManagedChildProcess::spawn_with_spec(&self.context, command, spec),
            )
            .await
            .map_err(|_| ProcessExecutionError::TimedOut)?
            .map_err(ProcessExecutionError::Io)?;
            let observation = {
                let command_watchdog = child.command_watchdog(deadline);
                if let Some((host, policy, reports)) = &self.health {
                    let mut monitor = child.watchdog().monitor_health(host.clone(), *policy);
                    // Polling the reader and sampling progress share this task.
                    let last_output = parking_lot::Mutex::new(None);
                    let mut output =
                        std::pin::pin!(child.collect_bounded_stdout(request.stdout_limit, || {
                            *last_output.lock() = Some(Instant::now());
                        }));
                    let period = policy.sample_period();
                    let mut interval = tokio::time::interval_at(Instant::now() + period, period);
                    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                    let observed = command_watchdog.observe_until(output.as_mut());
                    tokio::pin!(observed);
                    loop {
                        tokio::select! {
                            biased;
                            result = &mut observed => break result,
                            _ = interval.tick() => {
                                for report in monitor.sample(*last_output.lock()) {
                                    tracing::warn!(target: "marlin.runtime.health",
                                        resource = ?report.resource_id(), condition = ?report.condition(),
                                        "managed process health observation");
                                    // Advisory delivery must not delay command progress or cleanup.
                                    let _ = reports.try_send(report);
                                }
                            }
                        }
                    }
                } else {
                    let mut output =
                        std::pin::pin!(child.collect_bounded_stdout(request.stdout_limit, || {}));
                    command_watchdog.observe_until(output.as_mut()).await
                }
            };
            let output = match observation {
                Ok(result) => result.map_err(|error| match error {
                    ManagedStdoutError::Io(error) => ProcessExecutionError::Io(error),
                    ManagedStdoutError::LimitExceeded => ProcessExecutionError::OutputLimit,
                })?,
                Err(elapsed) => match child
                    .apply_watchdog_report(ProcessWatchdogReport::CommandDeadline(&elapsed))
                {
                    WatchdogCleanup::Started => return Err(ProcessExecutionError::TimedOut),
                    WatchdogCleanup::Stale(_) | WatchdogCleanup::Advisory(_) => {
                        unreachable!("watchdog was bound to this owned child")
                    }
                },
            };
            Ok(BoundedProcessOutput {
                stdout: output.stdout,
                exit_code: output.status.code(),
                success: output.status.success(),
            })
        })
    }
}
