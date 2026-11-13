// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Tokio task handle wrappers.

use std::{
    collections::BTreeSet,
    fmt,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

use marlin_agent_protocol::{ProcessMemorySample, ProcessMemorySampler};
use parking_lot::Mutex;
use tokio::{sync::Notify, task::JoinError};

use super::{
    CancellationToken, RuntimeTaskShutdownReceipt, RuntimeTaskShutdownReceiptInput,
    RuntimeTaskShutdownRequest, RuntimeTaskShutdownStatus, RuntimeTaskTrackerShutdownState,
    SessionId,
};

/// Host-owned process policy for every managed tool under this Runtime.
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeProcessBudget {
    pub concurrent: std::num::NonZeroUsize,
    pub memory_limit_bytes: std::num::NonZeroU64,
    pub command_reservation_bytes: std::num::NonZeroU64,
}

#[derive(Debug)]
struct ProcessMemoryAdmission {
    budget: RuntimeProcessBudget,
    sampler: Arc<dyn ProcessMemorySampler>,
}

enum AdmissionKind {
    Task,
    Process(Option<ProcessMemorySample>),
}

/// Operation within a runtime session.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct RuntimeOperationId(String);

impl RuntimeOperationId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Monotonic generation allocated within the host process.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct RuntimeResourceGeneration(u64);

impl RuntimeResourceGeneration {
    pub fn get(self) -> u64 {
        self.0
    }
}

/// Identity of an admitted task or managed process. A PID is an observation.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct RuntimeResourceId {
    session: SessionId,
    operation: RuntimeOperationId,
    generation: RuntimeResourceGeneration,
}

impl RuntimeResourceId {
    pub fn session(&self) -> &SessionId {
        &self.session
    }

    pub fn operation(&self) -> &RuntimeOperationId {
        &self.operation
    }

    pub fn generation(&self) -> RuntimeResourceGeneration {
        self.generation
    }
}

/// Reasons the Runtime may reject work before it is launched.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeAdmissionError {
    Closed,
    OperationActive,
    GenerationExhausted,
    MemoryBudgetExceeded,
}

impl fmt::Display for RuntimeAdmissionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Closed => "runtime admission is closed",
            Self::OperationActive => "the session already owns a process for this operation",
            Self::GenerationExhausted => "runtime resource generations are exhausted",
            Self::MemoryBudgetExceeded => "runtime process memory budget exceeded",
        })
    }
}

impl std::error::Error for RuntimeAdmissionError {}

/// Admission or join failure when awaiting a tracked Tokio task.
#[derive(Debug)]
pub enum RuntimeTaskJoinError {
    Admission(RuntimeAdmissionError),
    Join(JoinError),
}

impl fmt::Display for RuntimeTaskJoinError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Admission(error) => error.fmt(f),
            Self::Join(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for RuntimeTaskJoinError {}

/// Handle or typed admission rejection for runtime work.
#[derive(Debug)]
pub struct RuntimeTask<T> {
    state: Result<(RuntimeResourceId, tokio::task::JoinHandle<T>), RuntimeAdmissionError>,
}

/// Outcome for work spawned through Tokio cancellation-aware helpers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuntimeTaskOutcome<T> {
    Completed(T),
    Cancelled,
}

impl<T> RuntimeTask<T> {
    pub(crate) fn new(identity: RuntimeResourceId, handle: tokio::task::JoinHandle<T>) -> Self {
        Self {
            state: Ok((identity, handle)),
        }
    }

    pub(crate) fn rejected(error: RuntimeAdmissionError) -> Self {
        Self { state: Err(error) }
    }

    pub fn admission(&self) -> Result<&RuntimeResourceId, RuntimeAdmissionError> {
        self.state
            .as_ref()
            .map(|(id, _)| id)
            .map_err(|error| *error)
    }

    pub fn abort(&self) {
        if let Ok((_, handle)) = &self.state {
            handle.abort();
        }
    }

    pub fn is_finished(&self) -> bool {
        match &self.state {
            Ok((_, handle)) => handle.is_finished(),
            Err(_) => true,
        }
    }

    pub async fn join(self) -> Result<T, RuntimeTaskJoinError> {
        let (_, handle) = self.state.map_err(RuntimeTaskJoinError::Admission)?;
        handle.await.map_err(RuntimeTaskJoinError::Join)
    }
}

/// Shared admission and drain state for a runtime and its child contexts.
#[derive(Clone, Debug)]
pub struct RuntimeTaskTracker {
    state: Arc<RuntimeTaskTrackerState>,
}

#[derive(Debug, Default)]
struct AdmissionState {
    closed: bool,
    tasks: usize,
    processes: BTreeSet<(SessionId, RuntimeOperationId)>,
    unresolved: usize,
    reserved_process_memory: u64,
}

impl AdmissionState {
    fn active_count(&self) -> usize {
        self.tasks + self.processes.len()
    }
}

#[derive(Debug)]
struct RuntimeTaskTrackerState {
    admission: Mutex<AdmissionState>,
    memory: Option<ProcessMemoryAdmission>,
    closed: CancellationToken,
    notify: Notify,
}

#[derive(Debug)]
pub(crate) struct RuntimeTaskGuard {
    tracker: RuntimeTaskTracker,
    id: RuntimeResourceId,
    process: bool,
    release: bool,
    reserved_memory: u64,
}

impl RuntimeTaskGuard {
    pub(crate) fn id(&self) -> &RuntimeResourceId {
        &self.id
    }

    /// Leave unresolved ownership visible to shutdown without leaking the guard.
    pub(crate) fn leave_unresolved(mut self) {
        self.release = false;
        self.tracker.state.admission.lock().unresolved += 1;
        self.tracker.state.notify.notify_waiters();
    }
}

static NEXT_GENERATION: AtomicU64 = AtomicU64::new(1);

impl RuntimeTaskTracker {
    pub fn new() -> Self {
        Self::with_process_budget(None)
    }

    pub(crate) fn with_process_budget(
        policy: Option<(RuntimeProcessBudget, Arc<dyn ProcessMemorySampler>)>,
    ) -> Self {
        Self {
            state: Arc::new(RuntimeTaskTrackerState {
                admission: Mutex::new(AdmissionState::default()),
                memory: policy.map(|(budget, sampler)| ProcessMemoryAdmission { budget, sampler }),
                closed: CancellationToken::new(),
                notify: Notify::new(),
            }),
        }
    }

    /// Declared bytes currently held by managed processes and their cleanup leases.
    pub fn reserved_process_memory_bytes(&self) -> u64 {
        self.state.admission.lock().reserved_process_memory
    }

    pub(crate) fn sample_process_memory(
        &self,
        registry: &crate::process::RuntimeProcessRegistryHandle,
    ) -> std::io::Result<Option<ProcessMemorySample>> {
        self.state
            .memory
            .as_ref()
            .map(|memory| {
                let pids: Vec<_> = registry
                    .active_processes()
                    .into_iter()
                    .filter(|process| process.resource_id().is_some())
                    .map(|process| process.pid)
                    .collect();
                memory.sampler.sample(&pids)
            })
            .transpose()
    }

    pub fn active_task_count(&self) -> usize {
        self.state.admission.lock().tasks
    }

    pub fn active_resource_count(&self) -> usize {
        self.state.admission.lock().active_count()
    }

    pub fn is_closed(&self) -> bool {
        self.state.admission.lock().closed
    }

    pub fn close(&self) {
        self.close_admission();
    }

    pub(crate) fn close_admission(&self) -> usize {
        let count = {
            let mut admission = self.state.admission.lock();
            admission.closed = true;
            admission.active_count()
        };
        self.state.closed.cancel();
        count
    }

    pub async fn closed(&self) {
        self.state.closed.cancelled().await;
    }

    pub async fn wait(&self) {
        loop {
            let notified = self.state.notify.notified();
            if self.active_resource_count() == 0 {
                break;
            }
            notified.await;
        }
    }

    pub async fn close_and_wait(
        &self,
        timeout_duration: Option<Duration>,
        request: RuntimeTaskShutdownRequest,
    ) -> RuntimeTaskShutdownReceipt {
        let count = self.close_admission();
        self.wait_for_shutdown(timeout_duration, request, count)
            .await
    }

    pub(crate) async fn wait_for_shutdown(
        &self,
        timeout_duration: Option<Duration>,
        request: RuntimeTaskShutdownRequest,
        tracked_resource_count_at_shutdown: usize,
    ) -> RuntimeTaskShutdownReceipt {
        let drain = async {
            loop {
                let notified = self.state.notify.notified();
                {
                    let state = self.state.admission.lock();
                    if state.active_count() == state.unresolved {
                        return if state.unresolved == 0 {
                            RuntimeTaskShutdownStatus::Completed
                        } else {
                            RuntimeTaskShutdownStatus::CleanupUnconfirmed
                        };
                    }
                }
                notified.await;
            }
        };
        let status = match timeout_duration {
            Some(duration) => tokio::time::timeout(duration, drain)
                .await
                .unwrap_or(RuntimeTaskShutdownStatus::TimedOut),
            None => drain.await,
        };
        RuntimeTaskShutdownReceipt::from_input(RuntimeTaskShutdownReceiptInput {
            status,
            request,
            tracker_state: RuntimeTaskTrackerShutdownState::Closed,
            timeout_ms: duration_to_millis(timeout_duration),
            tracked_resource_count_at_shutdown,
            remaining_resource_count: self.active_resource_count(),
        })
    }

    pub(crate) fn admit(
        &self,
        session: &SessionId,
        operation: RuntimeOperationId,
    ) -> Result<RuntimeTaskGuard, RuntimeAdmissionError> {
        self.admit_resource(session, operation, AdmissionKind::Task)
    }

    pub(crate) fn admit_process(
        &self,
        session: &SessionId,
        operation: RuntimeOperationId,
        observed_memory: Option<ProcessMemorySample>,
    ) -> Result<RuntimeTaskGuard, RuntimeAdmissionError> {
        self.admit_resource(session, operation, AdmissionKind::Process(observed_memory))
    }

    fn admit_resource(
        &self,
        session: &SessionId,
        operation: RuntimeOperationId,
        kind: AdmissionKind,
    ) -> Result<RuntimeTaskGuard, RuntimeAdmissionError> {
        let mut admission = self.state.admission.lock();
        if admission.closed {
            return Err(RuntimeAdmissionError::Closed);
        }
        let process = matches!(kind, AdmissionKind::Process(_));
        let key = (session.clone(), operation.clone());
        if process && admission.processes.contains(&key) {
            return Err(RuntimeAdmissionError::OperationActive);
        }
        let mut reserved_memory = 0;
        if let (AdmissionKind::Process(sample), Some(memory)) = (kind, &self.state.memory) {
            let sample = sample.ok_or(RuntimeAdmissionError::MemoryBudgetExceeded)?;
            reserved_memory = memory.budget.command_reservation_bytes.get();
            let children = admission.reserved_process_memory.max(sample.child_bytes);
            let projected = sample
                .host_bytes
                .checked_add(children)
                .and_then(|used| used.checked_add(reserved_memory));
            if projected.is_none_or(|next| next > memory.budget.memory_limit_bytes.get()) {
                return Err(RuntimeAdmissionError::MemoryBudgetExceeded);
            }
        }
        let total_reserved = admission
            .reserved_process_memory
            .checked_add(reserved_memory)
            .ok_or(RuntimeAdmissionError::MemoryBudgetExceeded)?;
        let generation = NEXT_GENERATION
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |next| {
                next.checked_add(1)
            })
            .map_err(|_| RuntimeAdmissionError::GenerationExhausted)?;
        if process {
            admission.processes.insert(key);
            admission.reserved_process_memory = total_reserved;
        } else {
            admission.tasks += 1;
        }
        Ok(RuntimeTaskGuard {
            tracker: self.clone(),
            id: RuntimeResourceId {
                session: session.clone(),
                operation,
                generation: RuntimeResourceGeneration(generation),
            },
            process,
            release: true,
            reserved_memory,
        })
    }
}

impl Default for RuntimeTaskTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for RuntimeTaskGuard {
    fn drop(&mut self) {
        if !self.release {
            return;
        }
        let mut admission = self.tracker.state.admission.lock();
        if self.process {
            admission.reserved_process_memory -= self.reserved_memory;
            admission
                .processes
                .remove(&(self.id.session.clone(), self.id.operation.clone()));
        } else {
            admission.tasks -= 1;
        }
        if admission.active_count() == admission.unresolved {
            self.tracker.state.notify.notify_waiters();
        }
    }
}

fn duration_to_millis(duration: Option<Duration>) -> Option<u64> {
    duration.map(|duration| u64::try_from(duration.as_millis()).unwrap_or(u64::MAX))
}
