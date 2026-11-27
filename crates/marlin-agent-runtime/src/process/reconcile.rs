// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! OS process identity probing, post-restart reconciliation, and graceful orphan reaping.

use std::{
    io,
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};

use super::ledger::{
    ManagedProcessLedgerEntry, ProcessGroupId, ProcessLedgerStore, ProcessStartTimeSeconds,
    RuntimeHostRunId,
};
use super::shutdown::{
    ProcessGracefulShutdownPolicy, ProcessTerminationStage, SignalDeliveryOutcome,
    is_process_group_alive, signal_process_group_kill, signal_process_group_term,
};
use super::types::{RuntimeProcessObservationTimestampMs, RuntimeProcessStatus};
use crate::observability::{
    SPAN_RUNTIME_PROCESS_RECONCILE, TARGET_RUNTIME_PROCESS, TARGET_RUNTIME_SPAN,
};

/// Live process from a previous Marlin host run discovered on restart.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SurvivedProcessObservation {
    pub entry: ManagedProcessLedgerEntry,
    pub leader_alive: bool,
    pub group_alive: bool,
    pub surviving_descendant_pids: Vec<u32>,
}

/// Typed boot-time reconciliation report across all prior host runs.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct PersistedProcessReconciliationReport {
    pub host_run_id: Option<RuntimeHostRunId>,
    pub observed_at_ms: u64,
    pub inspected_count: usize,
    pub survived: Vec<SurvivedProcessObservation>,
    pub reconciled_dead: Vec<ManagedProcessLedgerEntry>,
}

impl PersistedProcessReconciliationReport {
    /// Returns true if any process from a previous run is still alive.
    pub fn has_survivors(&self) -> bool {
        !self.survived.is_empty()
    }
}

/// Named request for reaping a survived orphan process discovered after restart.
pub struct ReapSurvivedProcessRequest<'a> {
    pub store: Option<&'a dyn ProcessLedgerStore>,
    pub current_host_run_id: &'a RuntimeHostRunId,
    pub entry: ManagedProcessLedgerEntry,
    pub policy: &'a ProcessGracefulShutdownPolicy,
    pub observed_at_ms: RuntimeProcessObservationTimestampMs,
}

/// Typed receipt produced when reaping a survived process after restart.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SurvivedProcessReapReceipt {
    pub pid: u32,
    pub pgid: Option<ProcessGroupId>,
    pub origin_host_run_id: RuntimeHostRunId,
    pub observed_at_ms: RuntimeProcessObservationTimestampMs,
    pub graceful_term_sent: bool,
    pub escalated_to_sigkill: bool,
    pub confirmed_absent: bool,
    pub termination_stage: ProcessTerminationStage,
}

/// OS-level liveness and identity observation used during restart reconciliation.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OsProcessProbeResult {
    pub leader_alive: bool,
    pub group_alive: bool,
    pub surviving_descendant_pids: Vec<u32>,
}

impl OsProcessProbeResult {
    /// True if the leader, its Unix process group, or any direct descendant is alive.
    pub fn is_alive(&self) -> bool {
        self.leader_alive || self.group_alive || !self.surviving_descendant_pids.is_empty()
    }
}

/// Probe OS start-time fingerprint for a newly spawned process to guard against
/// PID reuse across Marlin restarts.
pub fn sample_os_start_time_secs(pid: u32) -> Option<ProcessStartTimeSeconds> {
    let mut system = sysinfo::System::new();
    let sys_pid = sysinfo::Pid::from_u32(pid);
    system.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[sys_pid]), true);
    let start = system.process(sys_pid)?.start_time();
    (start > 0).then_some(ProcessStartTimeSeconds::new(start))
}

/// Probe whether a previously persisted process (or its process group / direct
/// descendants) is still alive in the OS, checking `os_start_time_secs` to avoid
/// PID-reuse false positives.
pub fn probe_persisted_process(entry: &ManagedProcessLedgerEntry) -> OsProcessProbeResult {
    let mut system = sysinfo::System::new();
    system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

    let sys_pid = sysinfo::Pid::from_u32(entry.pid);
    let leader_matches = system
        .process(sys_pid)
        .is_some_and(|proc_info| matches_recorded_start_time(proc_info, entry.os_start_time_secs));

    let mut descendants: Vec<u32> = system
        .processes()
        .iter()
        .filter_map(|(&candidate_pid, proc_info)| {
            is_matching_descendant(candidate_pid, proc_info, entry, sys_pid, leader_matches)
        })
        .collect();
    descendants.sort_unstable();

    let group_alive = entry
        .pgid
        .is_some_and(|pgid| is_process_group_alive(pgid.as_u32()).unwrap_or(false))
        && (leader_matches || !descendants.is_empty());

    OsProcessProbeResult {
        leader_alive: leader_matches,
        group_alive,
        surviving_descendant_pids: descendants,
    }
}

/// Reconcile non-terminal entries from previous Marlin host runs against live OS
/// process state, updating the persistent ledger and emitting OpenTelemetry spans.
pub fn reconcile_persisted_ledger(
    store: &dyn ProcessLedgerStore,
    current_host_run_id: &RuntimeHostRunId,
    observed_at_ms: u64,
) -> io::Result<PersistedProcessReconciliationReport> {
    let entries = store.load_non_terminal()?;
    let mut report = PersistedProcessReconciliationReport {
        host_run_id: Some(current_host_run_id.clone()),
        observed_at_ms,
        inspected_count: 0,
        survived: Vec::new(),
        reconciled_dead: Vec::new(),
    };
    for entry in entries {
        if &entry.host_run_id == current_host_run_id {
            continue;
        }
        report.inspected_count += 1;
        reconcile_one_entry(
            store,
            current_host_run_id,
            entry,
            observed_at_ms,
            &mut report,
        )?;
    }
    Ok(report)
}

/// Gracefully reap a survived orphan process (`SIGTERM` -> grace window -> optional
/// `SIGKILL`), persisting the final state and emitting an OpenTelemetry span.
pub fn reap_survived_entry(
    request: ReapSurvivedProcessRequest<'_>,
) -> io::Result<SurvivedProcessReapReceipt> {
    let ReapSurvivedProcessRequest {
        store,
        current_host_run_id,
        mut entry,
        policy,
        observed_at_ms,
    } = request;
    let pgid_u32 = entry.pgid.map(ProcessGroupId::as_u32);
    let span = tracing::info_span!(
        target: TARGET_RUNTIME_SPAN,
        "runtime.process.reap_orphan",
        host_run_id = current_host_run_id.as_str(),
        origin_host_run_id = entry.host_run_id.as_str(),
        pid = entry.pid,
        pgid = pgid_u32.unwrap_or(entry.pid),
        process_handle = entry.handle.as_str(),
        owner_reference = entry.owner_reference.as_str(),
        graceful_timeout_ms = policy.graceful_timeout_duration_ms().as_millis(),
        termination_stage = tracing::field::Empty,
        escalated_to_sigkill = tracing::field::Empty,
    );
    let _entered = span.enter();

    if !probe_persisted_process(&entry).is_alive() {
        return finalize_already_dead_orphan(store, entry, observed_at_ms, &span);
    }

    let graceful_term_sent =
        send_graceful_term_and_wait(store, &mut entry, pgid_u32, policy, observed_at_ms)?;
    let escalated_to_sigkill = escalate_sigkill_if_alive(store, &mut entry, pgid_u32, policy)?;
    finalize_reaped_orphan(
        store,
        entry,
        observed_at_ms,
        graceful_term_sent,
        escalated_to_sigkill,
        &span,
    )
}

fn matches_recorded_start_time(
    proc_info: &sysinfo::Process,
    recorded: Option<ProcessStartTimeSeconds>,
) -> bool {
    if matches!(
        proc_info.status(),
        sysinfo::ProcessStatus::Zombie | sysinfo::ProcessStatus::Dead
    ) {
        return false;
    }
    match recorded {
        Some(expected) => {
            let current = proc_info.start_time();
            current > 0 && current.abs_diff(expected.as_u64()) <= 1
        }
        None => true,
    }
}

fn is_matching_descendant(
    candidate_pid: sysinfo::Pid,
    proc_info: &sysinfo::Process,
    entry: &ManagedProcessLedgerEntry,
    leader_sys_pid: sysinfo::Pid,
    leader_matches: bool,
) -> Option<u32> {
    if matches!(
        proc_info.status(),
        sysinfo::ProcessStatus::Zombie | sysinfo::ProcessStatus::Dead
    ) {
        return None;
    }
    let raw_pid = candidate_pid.as_u32();
    if raw_pid == entry.pid {
        return None;
    }
    let parent_matches = leader_matches && proc_info.parent() == Some(leader_sys_pid);
    let group_matches = process_in_group(raw_pid, entry.pgid);
    (parent_matches || group_matches).then_some(raw_pid)
}

fn process_in_group(raw_pid: u32, pgid: Option<ProcessGroupId>) -> bool {
    #[cfg(unix)]
    {
        pgid.is_some_and(|expected| {
            // SAFETY: getpgid is read-only and returns -1 on error.
            let candidate_pgid = unsafe { libc::getpgid(raw_pid as libc::pid_t) };
            candidate_pgid > 0 && candidate_pgid as u32 == expected.as_u32()
        })
    }
    #[cfg(not(unix))]
    {
        let _ = (raw_pid, pgid);
        false
    }
}

fn reconcile_one_entry(
    store: &dyn ProcessLedgerStore,
    current_host_run_id: &RuntimeHostRunId,
    mut entry: ManagedProcessLedgerEntry,
    observed_at_ms: u64,
    report: &mut PersistedProcessReconciliationReport,
) -> io::Result<()> {
    let probe = probe_persisted_process(&entry);
    let orphan_alive = probe.is_alive();
    let span = tracing::info_span!(
        target: TARGET_RUNTIME_SPAN,
        SPAN_RUNTIME_PROCESS_RECONCILE,
        host_run_id = current_host_run_id.as_str(),
        origin_host_run_id = entry.host_run_id.as_str(),
        pid = entry.pid,
        pgid = entry.pgid.map(ProcessGroupId::as_u32).unwrap_or(entry.pid),
        process_handle = entry.handle.as_str(),
        process_kind = ?entry.kind,
        owner_reference = entry.owner_reference.as_str(),
        orphan_alive,
        surviving_descendants = probe.surviving_descendant_pids.len() as u64,
        termination_stage = tracing::field::Empty,
    );
    let _entered = span.enter();
    let ts = RuntimeProcessObservationTimestampMs::new(observed_at_ms);
    entry.last_observed_at_ms = ts;

    if orphan_alive {
        entry.status = RuntimeProcessStatus::Orphaned;
        entry.termination_stage = ProcessTerminationStage::SurvivedRestart;
        span.record("termination_stage", entry.termination_stage.as_str());
        store.upsert(&entry)?;
        emit_orphan_detected_event(current_host_run_id, &entry, &probe);
        report.survived.push(SurvivedProcessObservation {
            entry,
            leader_alive: probe.leader_alive,
            group_alive: probe.group_alive,
            surviving_descendant_pids: probe.surviving_descendant_pids,
        });
    } else {
        entry.status = RuntimeProcessStatus::Failed;
        entry.termination_stage = ProcessTerminationStage::ReconciledDeadAfterRestart;
        entry.ended_at_ms = Some(ts);
        span.record("termination_stage", entry.termination_stage.as_str());
        store.upsert(&entry)?;
        report.reconciled_dead.push(entry);
    }
    Ok(())
}

fn emit_orphan_detected_event(
    current_host_run_id: &RuntimeHostRunId,
    entry: &ManagedProcessLedgerEntry,
    probe: &OsProcessProbeResult,
) {
    tracing::warn!(
        target: TARGET_RUNTIME_PROCESS,
        process_event = "orphan_detected_on_restart",
        host_run_id = current_host_run_id.as_str(),
        origin_host_run_id = entry.host_run_id.as_str(),
        pid = entry.pid,
        pgid = ?entry.pgid,
        process_handle = entry.handle.as_str(),
        owner_reference = entry.owner_reference.as_str(),
        leader_alive = probe.leader_alive,
        group_alive = probe.group_alive,
        surviving_descendants = ?probe.surviving_descendant_pids,
    );
}

fn finalize_already_dead_orphan(
    store: Option<&dyn ProcessLedgerStore>,
    mut entry: ManagedProcessLedgerEntry,
    observed_at_ms: RuntimeProcessObservationTimestampMs,
    span: &tracing::Span,
) -> io::Result<SurvivedProcessReapReceipt> {
    entry.status = RuntimeProcessStatus::Finished;
    entry.termination_stage = ProcessTerminationStage::ReconciledDeadAfterRestart;
    entry.last_observed_at_ms = observed_at_ms;
    entry.ended_at_ms = Some(observed_at_ms);
    span.record("termination_stage", entry.termination_stage.as_str());
    span.record("escalated_to_sigkill", false);
    if let Some(store) = store {
        store.upsert(&entry)?;
    }
    Ok(SurvivedProcessReapReceipt {
        pid: entry.pid,
        pgid: entry.pgid,
        origin_host_run_id: entry.host_run_id,
        observed_at_ms,
        graceful_term_sent: false,
        escalated_to_sigkill: false,
        confirmed_absent: true,
        termination_stage: entry.termination_stage,
    })
}

fn send_graceful_term_and_wait(
    store: Option<&dyn ProcessLedgerStore>,
    entry: &mut ManagedProcessLedgerEntry,
    pgid_u32: Option<u32>,
    policy: &ProcessGracefulShutdownPolicy,
    observed_at_ms: RuntimeProcessObservationTimestampMs,
) -> io::Result<bool> {
    if policy.graceful_timeout.is_zero() {
        return Ok(false);
    }
    entry.cleanup_attempts = entry.cleanup_attempts.saturating_add(1);
    entry.status = RuntimeProcessStatus::CleanupRequested;
    entry.termination_stage = ProcessTerminationStage::GracefulTermSent;
    entry.last_observed_at_ms = observed_at_ms;
    if let Some(store) = store {
        let _ = store.upsert(entry);
    }
    if !matches!(
        signal_process_group_term(entry.pid, pgid_u32)?,
        SignalDeliveryOutcome::Delivered
    ) {
        return Ok(false);
    }
    wait_until_entry_absent(entry, policy.graceful_timeout);
    Ok(true)
}

fn escalate_sigkill_if_alive(
    store: Option<&dyn ProcessLedgerStore>,
    entry: &mut ManagedProcessLedgerEntry,
    pgid_u32: Option<u32>,
    policy: &ProcessGracefulShutdownPolicy,
) -> io::Result<bool> {
    let after_term = probe_persisted_process(entry);
    if !after_term.is_alive() || !policy.escalate_to_sigkill {
        return Ok(false);
    }
    entry.escalated_to_sigkill = true;
    entry.termination_stage = ProcessTerminationStage::ForceKillEscalated;
    if let Some(store) = store {
        let _ = store.upsert(entry);
    }
    let _ = signal_process_group_kill(entry.pid, pgid_u32)?;
    for &desc_pid in &after_term.surviving_descendant_pids {
        let _ = signal_process_group_kill(desc_pid, Some(desc_pid));
    }
    wait_until_entry_absent(entry, Duration::from_secs(2));
    Ok(true)
}

fn wait_until_entry_absent(entry: &ManagedProcessLedgerEntry, limit: Duration) {
    let deadline = Instant::now() + limit;
    while Instant::now() < deadline {
        if !probe_persisted_process(entry).is_alive() {
            return;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn finalize_reaped_orphan(
    store: Option<&dyn ProcessLedgerStore>,
    mut entry: ManagedProcessLedgerEntry,
    observed_at_ms: RuntimeProcessObservationTimestampMs,
    graceful_term_sent: bool,
    escalated_to_sigkill: bool,
    span: &tracing::Span,
) -> io::Result<SurvivedProcessReapReceipt> {
    let confirmed_absent = !probe_persisted_process(&entry).is_alive();
    entry.last_observed_at_ms = observed_at_ms;
    if confirmed_absent {
        entry.status = RuntimeProcessStatus::Finished;
        entry.termination_stage = ProcessTerminationStage::OrphanReaped;
        entry.ended_at_ms = Some(observed_at_ms);
    } else {
        entry.status = RuntimeProcessStatus::Orphaned;
        entry.termination_stage = ProcessTerminationStage::SurvivedRestart;
    }
    span.record("termination_stage", entry.termination_stage.as_str());
    span.record("escalated_to_sigkill", escalated_to_sigkill);
    if let Some(store) = store {
        store.upsert(&entry)?;
    }
    Ok(SurvivedProcessReapReceipt {
        pid: entry.pid,
        pgid: entry.pgid,
        origin_host_run_id: entry.host_run_id,
        observed_at_ms,
        graceful_term_sent,
        escalated_to_sigkill,
        confirmed_absent,
        termination_stage: entry.termination_stage,
    })
}
