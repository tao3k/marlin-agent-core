//! Process registry state and shared `RuntimeProcessRegistryHandle` accessors.

use std::{collections::BTreeMap, sync::Arc};

use parking_lot::Mutex;

use super::cleanup::{
    RuntimeCommandCleanupOutcome, RuntimeCommandCleanupReceipt, RuntimeProcessCleanupController,
    RuntimeProcessCleanupFailure, RuntimeProcessCleanupPolicy, RuntimeProcessCleanupSweep,
};
use super::types::{
    RuntimeProcessHandle, RuntimeProcessKind, RuntimeProcessObservation,
    RuntimeProcessRegistrationError, RuntimeProcessRegistrySnapshot, RuntimeProcessStatus,
};
use crate::observability::{
    PROCESS_EVENT_CLEANUP_REQUESTED, PROCESS_EVENT_CLEANUP_SWEEP, PROCESS_EVENT_FAILED,
    PROCESS_EVENT_FINISHED, PROCESS_EVENT_ORPHANED, PROCESS_EVENT_REMOVED_STALE,
    PROCESS_EVENT_TERMINATION_FAILED, PROCESS_EVENT_TERMINATION_REQUESTED, PROCESS_EVENT_TRACKED,
    RUNTIME_KIND_GENERIC, RUNTIME_KIND_HOOK, RUNTIME_KIND_PROVIDER, RUNTIME_KIND_SUB_AGENT,
    RUNTIME_KIND_TOOL, SUB_AGENT_SOURCE_UNSPECIFIED, TARGET_RUNTIME_PROCESS,
};

/// Tracks currently active runtime-owned child processes.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RuntimeProcessRegistry {
    active: BTreeMap<u32, RuntimeProcessObservation>,
    active_handles: BTreeMap<RuntimeProcessHandle, u32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RuntimeCleanupProbe {
    candidate: RuntimeProcessObservation,
    outcome: RuntimeCleanupProbeOutcome,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum RuntimeCleanupProbeOutcome {
    RemovedStale,
    TerminationRequested,
    TerminationFailed(String),
}

impl RuntimeProcessRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn track(&mut self, observation: RuntimeProcessObservation) {
        self.try_track(observation)
            .expect("runtime process observation must not duplicate an active pid or handle");
    }

    pub fn try_track(
        &mut self,
        observation: RuntimeProcessObservation,
    ) -> Result<(), RuntimeProcessRegistrationError> {
        if let Some(existing_pid) = self.active_handles.get(&observation.handle) {
            return Err(RuntimeProcessRegistrationError::DuplicateActiveHandle {
                handle: observation.handle,
                existing_pid: *existing_pid,
                new_pid: observation.pid,
            });
        }
        if self.active.contains_key(&observation.pid) {
            return Err(RuntimeProcessRegistrationError::DuplicateActivePid {
                pid: observation.pid,
            });
        }
        emit_process_lifecycle_event(PROCESS_EVENT_TRACKED, &observation);
        self.active_handles
            .insert(observation.handle.clone(), observation.pid);
        self.active.insert(observation.pid, observation);
        Ok(())
    }

    pub fn active_processes(&self) -> Vec<&RuntimeProcessObservation> {
        self.active.values().collect()
    }

    pub fn get(&self, pid: u32) -> Option<&RuntimeProcessObservation> {
        self.active.get(&pid)
    }

    pub fn get_by_handle(
        &self,
        handle: &RuntimeProcessHandle,
    ) -> Option<&RuntimeProcessObservation> {
        self.active_handles
            .get(handle)
            .and_then(|pid| self.active.get(pid))
    }

    pub fn finish(&mut self, pid: u32, observed_at_ms: u64) -> Option<RuntimeProcessObservation> {
        self.complete(pid, RuntimeProcessStatus::Finished, observed_at_ms)
    }

    pub fn fail(&mut self, pid: u32, observed_at_ms: u64) -> Option<RuntimeProcessObservation> {
        self.complete(pid, RuntimeProcessStatus::Failed, observed_at_ms)
    }

    pub fn mark_orphaned(
        &mut self,
        pid: u32,
        observed_at_ms: u64,
    ) -> Option<&RuntimeProcessObservation> {
        self.update_active(pid, RuntimeProcessStatus::Orphaned, observed_at_ms)
    }

    pub fn request_cleanup(
        &mut self,
        pid: u32,
        observed_at_ms: u64,
    ) -> Option<&RuntimeProcessObservation> {
        self.update_active(pid, RuntimeProcessStatus::CleanupRequested, observed_at_ms)
    }

    pub fn cleanup_candidates(&self) -> Vec<&RuntimeProcessObservation> {
        self.active
            .values()
            .filter(|observation| {
                matches!(
                    observation.status,
                    RuntimeProcessStatus::Orphaned | RuntimeProcessStatus::CleanupRequested
                )
            })
            .collect()
    }

    pub fn snapshot(&self, observed_at_ms: u64) -> RuntimeProcessRegistrySnapshot {
        let mut snapshot = RuntimeProcessRegistrySnapshot {
            observed_at_ms,
            active_count: self.active.len(),
            ..RuntimeProcessRegistrySnapshot::default()
        };
        for observation in self.active.values() {
            snapshot.status_counts.record(&observation.status);
            snapshot.kind_counts.record(&observation.kind);
            if matches!(
                observation.status,
                RuntimeProcessStatus::Orphaned | RuntimeProcessStatus::CleanupRequested
            ) {
                snapshot.cleanup_candidate_count += 1;
            }
        }
        snapshot
    }

    pub fn remove(&mut self, pid: u32) -> Option<RuntimeProcessObservation> {
        let observation = self.active.remove(&pid)?;
        self.active_handles.remove(&observation.handle);
        Some(observation)
    }

    pub fn sweep_cleanup_candidates<C>(
        &mut self,
        controller: &mut C,
        observed_at_ms: u64,
    ) -> RuntimeProcessCleanupSweep
    where
        C: RuntimeProcessCleanupController,
    {
        let candidates = self.cleanup_candidate_observations();
        let probes = collect_cleanup_probes(candidates, controller);
        self.apply_cleanup_probes(probes, observed_at_ms)
    }

    pub fn sweep_cleanup_command_receipt<C>(
        &mut self,
        controller: &mut C,
        observed_at_ms: u64,
    ) -> RuntimeCommandCleanupReceipt
    where
        C: RuntimeProcessCleanupController,
    {
        self.sweep_cleanup_candidates(controller, observed_at_ms)
            .command_receipt()
    }

    fn cleanup_candidate_observations(&self) -> Vec<RuntimeProcessObservation> {
        self.cleanup_candidates().into_iter().cloned().collect()
    }

    fn apply_cleanup_probes(
        &mut self,
        probes: Vec<RuntimeCleanupProbe>,
        observed_at_ms: u64,
    ) -> RuntimeProcessCleanupSweep {
        let cleanup_candidate_count = probes.len();
        let mut sweep = RuntimeProcessCleanupSweep::new(observed_at_ms);

        for probe in probes {
            if !self.is_current_cleanup_candidate(&probe.candidate) {
                continue;
            }

            match probe.outcome {
                RuntimeCleanupProbeOutcome::RemovedStale => {
                    if let Some(removed) =
                        self.remove_cleanup_candidate(probe.candidate.pid, observed_at_ms)
                    {
                        sweep.removed_stale.push(removed);
                    }
                }
                RuntimeCleanupProbeOutcome::TerminationRequested => {
                    if let Some(process) =
                        self.record_cleanup_attempt(probe.candidate.pid, observed_at_ms)
                    {
                        emit_process_cleanup_event(
                            PROCESS_EVENT_TERMINATION_REQUESTED,
                            &process,
                            RuntimeCommandCleanupOutcome::TerminationRequested,
                            None,
                        );
                        sweep.termination_requested.push(process);
                    }
                }
                RuntimeCleanupProbeOutcome::TerminationFailed(error) => {
                    if let Some(process) =
                        self.record_cleanup_attempt(probe.candidate.pid, observed_at_ms)
                    {
                        emit_process_cleanup_event(
                            PROCESS_EVENT_TERMINATION_FAILED,
                            &process,
                            RuntimeCommandCleanupOutcome::TerminationFailed,
                            Some(error.as_str()),
                        );
                        sweep
                            .termination_failed
                            .push(RuntimeProcessCleanupFailure { process, error });
                    }
                }
            }
        }

        emit_process_cleanup_sweep_event(cleanup_candidate_count, &sweep);
        sweep
    }

    fn is_current_cleanup_candidate(&self, candidate: &RuntimeProcessObservation) -> bool {
        self.active.get(&candidate.pid).is_some_and(|current| {
            current.handle == candidate.handle
                && matches!(
                    current.status,
                    RuntimeProcessStatus::Orphaned | RuntimeProcessStatus::CleanupRequested
                )
        })
    }

    fn complete(
        &mut self,
        pid: u32,
        status: RuntimeProcessStatus,
        observed_at_ms: u64,
    ) -> Option<RuntimeProcessObservation> {
        let mut observation = self.active.remove(&pid)?;
        self.active_handles.remove(&observation.handle);
        observation.update_status_at_ms(status, observed_at_ms);
        emit_process_lifecycle_event(process_event_for_status(&observation.status), &observation);
        Some(observation)
    }

    fn remove_cleanup_candidate(
        &mut self,
        pid: u32,
        observed_at_ms: u64,
    ) -> Option<RuntimeProcessObservation> {
        let mut observation = self.remove(pid)?;
        observation.update_status_at_ms(observation.status.clone(), observed_at_ms);
        emit_process_cleanup_event(
            PROCESS_EVENT_REMOVED_STALE,
            &observation,
            RuntimeCommandCleanupOutcome::RemovedStale,
            None,
        );
        Some(observation)
    }

    fn record_cleanup_attempt(
        &mut self,
        pid: u32,
        observed_at_ms: u64,
    ) -> Option<RuntimeProcessObservation> {
        let observation = self.active.get_mut(&pid)?;
        observation.request_cleanup_at_ms(observed_at_ms);
        emit_process_lifecycle_event(process_event_for_status(&observation.status), observation);
        Some(observation.clone())
    }

    fn update_active(
        &mut self,
        pid: u32,
        status: RuntimeProcessStatus,
        observed_at_ms: u64,
    ) -> Option<&RuntimeProcessObservation> {
        let observation = self.active.get_mut(&pid)?;
        observation.update_status_at_ms(status, observed_at_ms);
        emit_process_lifecycle_event(process_event_for_status(&observation.status), observation);
        Some(observation)
    }
}

fn collect_cleanup_probes<C>(
    candidates: Vec<RuntimeProcessObservation>,
    controller: &mut C,
) -> Vec<RuntimeCleanupProbe>
where
    C: RuntimeProcessCleanupController,
{
    candidates
        .into_iter()
        .map(|candidate| {
            let outcome = if !controller.is_process_alive(candidate.pid) {
                RuntimeCleanupProbeOutcome::RemovedStale
            } else {
                match controller.request_termination(candidate.pid) {
                    Ok(true) => RuntimeCleanupProbeOutcome::TerminationRequested,
                    Ok(false) => RuntimeCleanupProbeOutcome::RemovedStale,
                    Err(error) => RuntimeCleanupProbeOutcome::TerminationFailed(error),
                }
            };
            RuntimeCleanupProbe { candidate, outcome }
        })
        .collect()
}

fn emit_process_lifecycle_event(event: &'static str, process: &RuntimeProcessObservation) {
    tracing::debug!(
        target: TARGET_RUNTIME_PROCESS,
        process_event = event,
        pid = process.pid,
        runtime_kind = runtime_kind_for_process(&process.kind),
        process_kind = ?process.kind,
        process_handle = process.handle.as_str(),
        owner_reference = process.owner_reference.as_str(),
        agent_reference = process_agent_reference(process),
        sub_agent_source = process_sub_agent_source(process),
        process_status = ?process.status,
        cleanup_attempts = process.cleanup_attempts,
        last_cleanup_attempt_at_ms = ?process.last_cleanup_attempt_at_ms,
    );
}

fn emit_process_cleanup_event(
    event: &'static str,
    process: &RuntimeProcessObservation,
    outcome: RuntimeCommandCleanupOutcome,
    error: Option<&str>,
) {
    let retained_in_registry = outcome.retained_in_registry();
    let requires_follow_up = outcome.requires_follow_up();
    let force_cleanup_recommended = RuntimeProcessCleanupPolicy::default()
        .force_cleanup_recommended(&outcome, process.cleanup_attempts);
    if let Some(error) = error {
        tracing::warn!(
            target: TARGET_RUNTIME_PROCESS,
            process_event = event,
            pid = process.pid,
            runtime_kind = runtime_kind_for_process(&process.kind),
            process_kind = ?process.kind,
            process_handle = process.handle.as_str(),
            owner_reference = process.owner_reference.as_str(),
            agent_reference = process_agent_reference(process),
            sub_agent_source = process_sub_agent_source(process),
            process_status = ?process.status,
            cleanup_outcome = ?outcome,
            cleanup_attempts = process.cleanup_attempts,
            last_cleanup_attempt_at_ms = ?process.last_cleanup_attempt_at_ms,
            retained_in_registry,
            requires_follow_up,
            force_cleanup_recommended,
            error,
        );
    } else {
        tracing::info!(
            target: TARGET_RUNTIME_PROCESS,
            process_event = event,
            pid = process.pid,
            runtime_kind = runtime_kind_for_process(&process.kind),
            process_kind = ?process.kind,
            process_handle = process.handle.as_str(),
            owner_reference = process.owner_reference.as_str(),
            agent_reference = process_agent_reference(process),
            sub_agent_source = process_sub_agent_source(process),
            process_status = ?process.status,
            cleanup_outcome = ?outcome,
            cleanup_attempts = process.cleanup_attempts,
            last_cleanup_attempt_at_ms = ?process.last_cleanup_attempt_at_ms,
            retained_in_registry,
            requires_follow_up,
            force_cleanup_recommended,
        );
    }
}

fn emit_process_cleanup_sweep_event(
    cleanup_candidate_count: usize,
    sweep: &RuntimeProcessCleanupSweep,
) {
    tracing::debug!(
        target: TARGET_RUNTIME_PROCESS,
        process_event = PROCESS_EVENT_CLEANUP_SWEEP,
        observed_at_ms = sweep.observed_at_ms,
        cleanup_candidate_count,
        removed_stale_count = sweep.removed_stale.len(),
        termination_requested_count = sweep.termination_requested.len(),
        termination_failed_count = sweep.termination_failed.len(),
    );
}

fn process_event_for_status(status: &RuntimeProcessStatus) -> &'static str {
    match status {
        RuntimeProcessStatus::Running => PROCESS_EVENT_TRACKED,
        RuntimeProcessStatus::Finished => PROCESS_EVENT_FINISHED,
        RuntimeProcessStatus::Failed => PROCESS_EVENT_FAILED,
        RuntimeProcessStatus::Orphaned => PROCESS_EVENT_ORPHANED,
        RuntimeProcessStatus::CleanupRequested => PROCESS_EVENT_CLEANUP_REQUESTED,
    }
}

fn runtime_kind_for_process(kind: &RuntimeProcessKind) -> &'static str {
    match kind {
        RuntimeProcessKind::Tool => RUNTIME_KIND_TOOL,
        RuntimeProcessKind::SubAgent => RUNTIME_KIND_SUB_AGENT,
        RuntimeProcessKind::Provider => RUNTIME_KIND_PROVIDER,
        RuntimeProcessKind::Hook => RUNTIME_KIND_HOOK,
        RuntimeProcessKind::Other(_) => RUNTIME_KIND_GENERIC,
    }
}

fn process_agent_reference(process: &RuntimeProcessObservation) -> &str {
    process
        .command
        .as_ref()
        .and_then(|command| command.sub_agent_role.as_deref())
        .unwrap_or(process.owner_reference.as_str())
}

fn process_sub_agent_source(process: &RuntimeProcessObservation) -> &str {
    process
        .command
        .as_ref()
        .and_then(|command| command.sub_agent_source.as_deref())
        .unwrap_or(SUB_AGENT_SOURCE_UNSPECIFIED)
}

/// Shared runtime-owned process registry handle.
///
/// This keeps the lock primitive out of runtime API signatures while preserving cheap cloned
/// handles for child contexts.
#[derive(Clone, Debug)]
pub struct RuntimeProcessRegistryHandle {
    inner: Arc<Mutex<RuntimeProcessRegistry>>,
}

impl RuntimeProcessRegistryHandle {
    pub fn new() -> Self {
        Self::from_registry(RuntimeProcessRegistry::new())
    }

    pub fn from_registry(registry: RuntimeProcessRegistry) -> Self {
        Self {
            inner: Arc::new(Mutex::new(registry)),
        }
    }

    pub fn track(&self, observation: RuntimeProcessObservation) {
        self.inner.lock().track(observation);
    }

    pub fn try_track(
        &self,
        observation: RuntimeProcessObservation,
    ) -> Result<(), RuntimeProcessRegistrationError> {
        self.inner.lock().try_track(observation)
    }

    pub fn active_processes(&self) -> Vec<RuntimeProcessObservation> {
        self.inner
            .lock()
            .active_processes()
            .into_iter()
            .cloned()
            .collect()
    }

    pub fn get(&self, pid: u32) -> Option<RuntimeProcessObservation> {
        self.inner.lock().get(pid).cloned()
    }

    pub fn get_by_handle(
        &self,
        handle: &RuntimeProcessHandle,
    ) -> Option<RuntimeProcessObservation> {
        self.inner.lock().get_by_handle(handle).cloned()
    }

    pub fn finish(&self, pid: u32, observed_at_ms: u64) -> Option<RuntimeProcessObservation> {
        self.inner.lock().finish(pid, observed_at_ms)
    }

    pub fn fail(&self, pid: u32, observed_at_ms: u64) -> Option<RuntimeProcessObservation> {
        self.inner.lock().fail(pid, observed_at_ms)
    }

    pub fn mark_orphaned(
        &self,
        pid: u32,
        observed_at_ms: u64,
    ) -> Option<RuntimeProcessObservation> {
        self.inner
            .lock()
            .mark_orphaned(pid, observed_at_ms)
            .cloned()
    }

    pub fn request_cleanup(
        &self,
        pid: u32,
        observed_at_ms: u64,
    ) -> Option<RuntimeProcessObservation> {
        self.inner
            .lock()
            .request_cleanup(pid, observed_at_ms)
            .cloned()
    }

    pub fn cleanup_candidates(&self) -> Vec<RuntimeProcessObservation> {
        self.inner
            .lock()
            .cleanup_candidates()
            .into_iter()
            .cloned()
            .collect()
    }

    pub fn snapshot(&self, observed_at_ms: u64) -> RuntimeProcessRegistrySnapshot {
        self.inner.lock().snapshot(observed_at_ms)
    }

    pub fn remove(&self, pid: u32) -> Option<RuntimeProcessObservation> {
        self.inner.lock().remove(pid)
    }

    pub fn sweep_cleanup_candidates<C>(
        &self,
        controller: &mut C,
        observed_at_ms: u64,
    ) -> RuntimeProcessCleanupSweep
    where
        C: RuntimeProcessCleanupController,
    {
        let candidates = self.inner.lock().cleanup_candidate_observations();
        let probes = collect_cleanup_probes(candidates, controller);
        self.inner
            .lock()
            .apply_cleanup_probes(probes, observed_at_ms)
    }

    pub fn sweep_cleanup_command_receipt<C>(
        &self,
        controller: &mut C,
        observed_at_ms: u64,
    ) -> RuntimeCommandCleanupReceipt
    where
        C: RuntimeProcessCleanupController,
    {
        self.sweep_cleanup_candidates(controller, observed_at_ms)
            .command_receipt()
    }
}

impl Default for RuntimeProcessRegistryHandle {
    fn default() -> Self {
        Self::new()
    }
}
