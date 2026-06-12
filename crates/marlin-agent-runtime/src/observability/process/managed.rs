//! Tokio-backed managed child process wrapper for runtime-owned processes.

use std::{
    io,
    time::{SystemTime, UNIX_EPOCH},
};

#[cfg(unix)]
use process_wrap::tokio::ProcessGroup;
use process_wrap::tokio::{ChildWrapper, CommandWrap, KillOnDrop};
use tokio::process::{ChildStdin as AsyncChildStdin, Command as AsyncCommand};

use super::registry::RuntimeProcessRegistryHandle;
use super::types::{
    RuntimeCommandObservation, RuntimeProcessExitStatus, RuntimeProcessHandle, RuntimeProcessKind,
    RuntimeProcessObservation, RuntimeProcessOutput,
};
use crate::tokio_runtime::RuntimeContext;

/// Named spawn request for a runtime-owned child process.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManagedChildProcessSpec {
    pub kind: RuntimeProcessKind,
    pub handle: RuntimeProcessHandle,
    pub owner_reference: String,
    pub command: Option<RuntimeCommandObservation>,
    pub started_at_ms: Option<u64>,
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
}

impl AsyncManagedChildProcess {
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
        let mut command = CommandWrap::from(command);
        command.wrap(KillOnDrop);
        #[cfg(unix)]
        command.wrap(ProcessGroup::leader());

        let mut child = command.spawn()?;
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
            .with_started_at_ms(started_at_ms);
        if let Some(command) = spec.command {
            observation = observation.with_command(command);
        }
        let registration = registry.try_track(observation);
        if let Err(error) = registration {
            let _ = child.start_kill();
            let _ = child.wait().await;
            return Err(io::Error::new(io::ErrorKind::AlreadyExists, error));
        }

        Ok(Self {
            child: Some(child),
            registry,
            pid,
            completed: false,
        })
    }

    pub fn pid(&self) -> u32 {
        self.pid
    }

    pub fn take_stdin(&mut self) -> Option<AsyncChildStdin> {
        self.child.as_mut()?.stdin().take()
    }

    pub fn request_cleanup(&self) {
        self.request_cleanup_observed_at(current_time_millis());
    }

    pub fn request_cleanup_observed_at(&self, observed_at_ms: u64) {
        self.registry.request_cleanup(self.pid, observed_at_ms);
    }

    pub async fn wait(self) -> io::Result<RuntimeProcessExitStatus> {
        self.wait_observed_at(current_time_millis()).await
    }

    pub async fn wait_observed_at(
        mut self,
        observed_at_ms: u64,
    ) -> io::Result<RuntimeProcessExitStatus> {
        let status = self
            .child
            .as_mut()
            .expect("managed child process must own a child")
            .wait()
            .await
            .map(RuntimeProcessExitStatus::from);
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
        let child = self
            .child
            .take()
            .expect("managed child process must own a child");
        let output = Box::into_pin(child.wait_with_output())
            .await
            .map(RuntimeProcessOutput::from);
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
        self.record_terminal_status(&status, observed_at_ms);
        status
    }

    fn record_terminal_status(
        &mut self,
        status: &io::Result<RuntimeProcessExitStatus>,
        observed_at_ms: u64,
    ) {
        match status {
            Ok(exit_status) if exit_status.success() => {
                self.registry.finish(self.pid, observed_at_ms);
                self.completed = true;
                self.child = None;
            }
            Ok(_) => {
                self.registry.fail(self.pid, observed_at_ms);
                self.completed = true;
                self.child = None;
            }
            Err(_) => {
                self.registry.request_cleanup(self.pid, observed_at_ms);
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
                self.registry.finish(self.pid, observed_at_ms);
                self.completed = true;
                self.child = None;
            }
            Ok(_) => {
                self.registry.fail(self.pid, observed_at_ms);
                self.completed = true;
                self.child = None;
            }
            Err(_) => {
                self.registry.request_cleanup(self.pid, observed_at_ms);
            }
        }
    }
}

impl Drop for AsyncManagedChildProcess {
    fn drop(&mut self) {
        if !self.completed {
            if let Some(child) = self.child.as_mut() {
                let _ = child.start_kill();
            }
            self.registry
                .request_cleanup(self.pid, current_time_millis());
        }
    }
}

fn current_time_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or_default()
}
