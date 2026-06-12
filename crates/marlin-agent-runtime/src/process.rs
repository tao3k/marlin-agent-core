//! Managed OS child process observations for runtime-owned tools.

use std::{
    io,
    process::{Child, Command, ExitStatus},
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    observability::{RuntimeProcessKind, RuntimeProcessObservation, RuntimeProcessRegistry},
    tokio_runtime::RuntimeContext,
};

/// Named spawn request for a runtime-owned child process.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManagedChildProcessSpec {
    pub kind: RuntimeProcessKind,
    pub owner_reference: String,
    pub started_at_ms: Option<u64>,
}

impl ManagedChildProcessSpec {
    pub fn new(kind: RuntimeProcessKind, owner_reference: impl Into<String>) -> Self {
        Self {
            kind,
            owner_reference: owner_reference.into(),
            started_at_ms: None,
        }
    }

    pub fn with_started_at_ms(mut self, started_at_ms: u64) -> Self {
        self.started_at_ms = Some(started_at_ms);
        self
    }
}

/// Runtime-owned child process wrapper that keeps PID lifecycle state observable.
#[derive(Debug)]
pub struct ManagedChildProcess {
    child: Option<Child>,
    registry: Arc<Mutex<RuntimeProcessRegistry>>,
    pid: u32,
    completed: bool,
}

impl ManagedChildProcess {
    pub fn spawn(
        context: &RuntimeContext,
        command: &mut Command,
        kind: RuntimeProcessKind,
        owner_reference: impl Into<String>,
    ) -> io::Result<Self> {
        Self::spawn_with_spec(
            context,
            command,
            ManagedChildProcessSpec::new(kind, owner_reference),
        )
    }

    pub fn spawn_with_spec(
        context: &RuntimeContext,
        command: &mut Command,
        spec: ManagedChildProcessSpec,
    ) -> io::Result<Self> {
        let child = command.spawn()?;
        let pid = child.id();
        let registry = context.process_registry();
        let started_at_ms = spec.started_at_ms.unwrap_or_else(current_time_millis);
        registry
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .track(
                RuntimeProcessObservation::new(pid, spec.kind, spec.owner_reference)
                    .with_started_at_ms(started_at_ms),
            );

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

    pub fn request_cleanup(&self) {
        self.request_cleanup_observed_at(current_time_millis());
    }

    pub fn request_cleanup_observed_at(&self, observed_at_ms: u64) {
        self.registry
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .request_cleanup(self.pid, observed_at_ms);
    }

    pub fn wait(self) -> io::Result<ExitStatus> {
        self.wait_observed_at(current_time_millis())
    }

    pub fn wait_observed_at(mut self, observed_at_ms: u64) -> io::Result<ExitStatus> {
        let status = self
            .child
            .as_mut()
            .expect("managed child process must own a child")
            .wait();
        self.record_terminal_status(&status, observed_at_ms);
        status
    }

    pub fn kill(self) -> io::Result<ExitStatus> {
        self.kill_observed_at(current_time_millis())
    }

    pub fn kill_observed_at(mut self, observed_at_ms: u64) -> io::Result<ExitStatus> {
        self.request_cleanup_observed_at(observed_at_ms);
        self.child
            .as_mut()
            .expect("managed child process must own a child")
            .kill()?;
        let status = self
            .child
            .as_mut()
            .expect("managed child process must own a child")
            .wait();
        self.record_terminal_status(&status, observed_at_ms);
        status
    }

    fn record_terminal_status(&mut self, status: &io::Result<ExitStatus>, observed_at_ms: u64) {
        let mut registry = self
            .registry
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        match status {
            Ok(exit_status) if exit_status.success() => {
                registry.finish(self.pid, observed_at_ms);
                self.completed = true;
                self.child = None;
            }
            Ok(_) => {
                registry.fail(self.pid, observed_at_ms);
                self.completed = true;
                self.child = None;
            }
            Err(_) => {
                registry.request_cleanup(self.pid, observed_at_ms);
            }
        }
    }
}

impl Drop for ManagedChildProcess {
    fn drop(&mut self) {
        if !self.completed {
            self.registry
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
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
