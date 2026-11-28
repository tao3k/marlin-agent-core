// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Real-OS verification of graceful two-phase shutdown, persistent process ledger,
//! post-restart orphan detection (with PID-reuse protection), and orphan reaping.

use std::{
    fs, io,
    path::Path,
    process::Stdio,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use marlin_agent_runtime::{
    AsyncManagedChildProcess, FileProcessLedgerStore, ManagedChildProcessSpec,
    ManagedProcessLedgerEntry, ManagedProcessLedgerRecordRequest,
    ProcessGracefulShutdownDurationMs, ProcessGracefulShutdownPolicy, ProcessGroupId,
    ProcessLedgerStore, ProcessShutdownHook, ProcessStartTimeSeconds, ProcessTerminationStage,
    RuntimeHostRunId, RuntimeTaskTrackerPolicy, TokioAgentRuntime, TokioAgentRuntimeBuildRequest,
    process::{
        RuntimeProcessKind, RuntimeProcessObservation, RuntimeProcessObservationTimestampMs,
        RuntimeProcessStatus,
    },
};
use tokio::{process::Command as AsyncCommand, time::timeout};

fn os_pid_alive(pid: u32) -> bool {
    let Ok(pid_i32) = i32::try_from(pid) else {
        return false;
    };
    // SAFETY: signal 0 probes existence without delivering a signal.
    unsafe { libc::kill(pid_i32, 0) == 0 }
}

async fn wait_for_file_pid(path: &Path) -> u32 {
    timeout(Duration::from_secs(5), async {
        loop {
            if let Ok(raw) = fs::read_to_string(path)
                && let Ok(pid) = raw.trim().parse::<u32>()
            {
                return pid;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("timed out waiting for child pid file")
}

/// Real User Case regression:
/// Previously, dropping a gateway guard sent immediate SIGKILL right after
/// `POST /shutdown`, killing the gateway before it could clean up its own
/// sub-processes (even those in separate sessions/PGIDs).
///
/// With `AsyncManagedChildProcess` + `ProcessShutdownHook` + `ProcessGracefulShutdownPolicy`:
/// Dropping the handle invokes the cooperative hook and gives the parent a
/// graceful `SIGTERM` window so it cleans up its own grandchild cleanly without
/// `SIGKILL` escalation, while recording `ExitedCleanly` in the persistent ledger.
#[tokio::test]
async fn drop_gives_parent_graceful_window_to_cleanup_grandchild_without_sigkill() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ledger_path = tmp.path().join("managed-processes.json");
    let grandchild_pid_file = tmp.path().join("grandchild.pid");
    let cleanup_marker_file = tmp.path().join("cleaned.marker");
    let shutdown_file = tmp.path().join("cooperative.shutdown");

    let (runtime, _events) = TokioAgentRuntime::from_build_request(
        TokioAgentRuntimeBuildRequest::runtime_root(16).with_process_ledger_path(&ledger_path),
    );
    let context = runtime.context();

    // Parent launches a grandchild, does NOT trap TERM, and relies solely on the
    // cooperative shutdown hook completing its cleanup sequence (sleeping 150ms
    // before killing grandchild and exiting 0). If Marlin sent SIGTERM immediately
    // after the cooperative hook returned, parent would die before cleaning up
    // its grandchild and `cleanup_marker_file` would never be written!
    let script = format!(
        r#"
set -eu
/bin/sleep 60 &
GC_PID=$!
printf '%s' "$GC_PID" > "{gc}"
while [ ! -f "{hook_file}" ]; do
  sleep 0.02
done
sleep 0.15
kill -TERM "$GC_PID" 2>/dev/null || true
wait "$GC_PID" 2>/dev/null || true
printf 'cleaned' > "{marker}"
exit 0
"#,
        gc = grandchild_pid_file.display(),
        marker = cleanup_marker_file.display(),
        hook_file = shutdown_file.display(),
    );

    let hook_called = Arc::new(AtomicBool::new(false));
    let hook_called_clone = Arc::clone(&hook_called);
    let shutdown_file_for_hook = shutdown_file.clone();

    let mut command = AsyncCommand::new("/bin/sh");
    command
        .args(["-c", &script])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let spec = ManagedChildProcessSpec::new(RuntimeProcessKind::Provider, "gateway-fixture")
        .with_graceful_shutdown(
            ProcessGracefulShutdownPolicy::new(Duration::from_secs(3))
                .with_cooperative_timeout(Duration::from_secs(2)),
        )
        .with_shutdown_hook(ProcessShutdownHook::from_blocking(
            "gateway-cooperative-shutdown",
            move || {
                hook_called_clone.store(true, Ordering::SeqCst);
                // Return immediately (just like HTTP POST /shutdown returning 200 OK
                // while the server tears down its sub-processes asynchronously).
                fs::write(&shutdown_file_for_hook, b"shutdown")
            },
        ));

    let child = AsyncManagedChildProcess::spawn_with_spec(&context, command, spec)
        .await
        .expect("spawn managed gateway fixture");
    let parent_pid = child.pid();
    let grandchild_pid = wait_for_file_pid(&grandchild_pid_file).await;
    assert!(os_pid_alive(parent_pid));
    assert!(os_pid_alive(grandchild_pid));

    // Simulate `GatewayGuard::drop`!
    drop(child);

    // Runtime shutdown waits for the graceful reaper (which holds ProcessLease).
    let receipt = runtime
        .shutdown_tasks(&RuntimeTaskTrackerPolicy::cancel_on_shutdown(Some(5_000)))
        .await;
    assert_eq!(
        receipt.status(),
        &marlin_agent_runtime::RuntimeTaskShutdownStatus::Completed
    );

    assert!(
        hook_called.load(Ordering::SeqCst),
        "cooperative hook must run"
    );
    assert!(
        cleanup_marker_file.exists(),
        "parent cleanup trap must complete before exit (no premature SIGKILL)"
    );
    assert!(!os_pid_alive(parent_pid), "parent must be dead");
    assert!(
        !os_pid_alive(grandchild_pid),
        "grandchild must be cleaned up"
    );

    // Verify durable persistent ledger recorded clean exit without SIGKILL escalation.
    let entries = runtime
        .process_registry()
        .persisted_entries()
        .expect("load ledger");
    let entry = entries
        .iter()
        .find(|e| e.pid == parent_pid)
        .expect("ledger entry for gateway fixture");
    assert_eq!(
        entry.termination_stage,
        ProcessTerminationStage::ExitedCleanly
    );
    assert!(!entry.escalated_to_sigkill);
    assert_eq!(entry.exit_code, Some(0));
}

#[tokio::test]
async fn stubborn_process_ignoring_sigterm_escalates_to_sigkill_after_grace_window() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ledger_path = tmp.path().join("managed-processes.json");
    let ready_file = tmp.path().join("ready");

    let (runtime, _events) = TokioAgentRuntime::from_build_request(
        TokioAgentRuntimeBuildRequest::runtime_root(16).with_process_ledger_path(&ledger_path),
    );
    let context = runtime.context();

    let script = format!(
        "trap '' TERM; printf 'ready' > \"{}\"; while true; do sleep 0.1; done",
        ready_file.display()
    );
    let mut command = AsyncCommand::new("/bin/sh");
    command
        .args(["-c", &script])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let grace = Duration::from_millis(250);
    let spec = ManagedChildProcessSpec::new(RuntimeProcessKind::Tool, "stubborn-worker")
        .with_graceful_shutdown(ProcessGracefulShutdownPolicy::new(grace));

    let child = AsyncManagedChildProcess::spawn_with_spec(&context, command, spec)
        .await
        .expect("spawn stubborn worker");
    let pid = child.pid();

    timeout(Duration::from_secs(5), async {
        while !ready_file.exists() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("stubborn worker ready");

    let t0 = Instant::now();
    let _status = child
        .shutdown_gracefully()
        .await
        .expect("shutdown stubborn worker");
    let elapsed = t0.elapsed();

    assert!(
        elapsed >= Duration::from_millis(200),
        "must wait grace window before SIGKILL escalation; elapsed={elapsed:?}"
    );
    assert!(
        !os_pid_alive(pid),
        "SIGKILL escalation must reap stubborn process"
    );

    let entries = runtime
        .process_registry()
        .persisted_entries()
        .expect("load ledger");
    let entry = entries.iter().find(|e| e.pid == pid).expect("ledger entry");
    assert_eq!(
        entry.termination_stage,
        ProcessTerminationStage::ForceKillEscalated
    );
    assert!(entry.escalated_to_sigkill);
}

#[tokio::test]
async fn restart_reconciliation_detects_unkilled_process_guards_pid_reuse_and_reaps_gracefully() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ledger_path = tmp.path().join("managed-processes.json");
    let store = Arc::new(FileProcessLedgerStore::new(&ledger_path));

    // Boot #1: spawn a managed process and simulate an abrupt Marlin crash
    // (`mem::forget` bypasses Drop, just like `SIGKILL` of the Marlin host).
    let run1_id = RuntimeHostRunId::new("marlin-boot-1");
    let (runtime1, _events1) = TokioAgentRuntime::from_build_request(
        TokioAgentRuntimeBuildRequest::runtime_root(16)
            .with_host_run_id(run1_id.clone())
            .with_process_ledger(store.clone()),
    );
    let mut cmd = AsyncCommand::new("/bin/sleep");
    cmd.arg("60")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let leaked_child = AsyncManagedChildProcess::spawn_with_spec(
        &runtime1.context(),
        cmd,
        ManagedChildProcessSpec::new(RuntimeProcessKind::Provider, "leaked-provider"),
    )
    .await
    .expect("spawn leaked provider");
    let leaked_pid = leaked_child.pid();
    let leaked_pgid = leaked_child.pgid().expect("unix pgid");

    // Simulate abrupt host crash without Drop/reap.
    std::mem::forget(leaked_child);
    assert!(os_pid_alive(leaked_pid));

    // Also plant a PID-reuse entry pointing to our OWN current test PID with a
    // mismatched OS start-time fingerprint (epoch = 1) to verify PID-reuse safety!
    let self_pid = std::process::id();
    let bogus_reuse_obs =
        RuntimeProcessObservation::new(self_pid, RuntimeProcessKind::Tool, "stale-reused-pid")
            .with_started_at_ms(100);
    let bogus_reuse_entry =
        ManagedProcessLedgerEntry::from_request(ManagedProcessLedgerRecordRequest {
            host_run_id: RuntimeHostRunId::new("marlin-boot-0"),
            host_pid: 1,
            observation: &bogus_reuse_obs,
            pgid: Some(ProcessGroupId::new(self_pid)),
            os_start_time_secs: Some(ProcessStartTimeSeconds::new(1)),
            graceful_timeout: ProcessGracefulShutdownDurationMs::new(1000),
            observed_at_ms: RuntimeProcessObservationTimestampMs::new(100),
        });
    store
        .upsert(&bogus_reuse_entry)
        .expect("seed bogus reuse entry");

    // Boot #2: new Marlin host restarts against the same persistent ledger.
    let run2_id = RuntimeHostRunId::new("marlin-boot-2");
    let (runtime2, _events2) = TokioAgentRuntime::from_build_request(
        TokioAgentRuntimeBuildRequest::runtime_root(16)
            .with_host_run_id(run2_id)
            .with_process_ledger(store.clone()),
    );

    let report = runtime2
        .reconcile_persisted_processes()
        .expect("reconcile persisted ledger on boot #2");
    assert_eq!(report.inspected_count, 2);
    assert!(report.has_survivors());
    assert_eq!(report.survived.len(), 1);
    assert_eq!(report.reconciled_dead.len(), 1);
    assert_eq!(
        report.reconciled_dead[0].pid, self_pid,
        "mismatched start-time PID reuse must be reconciled dead, never treated as orphan"
    );

    let survivor = &report.survived[0];
    assert_eq!(survivor.entry.pid, leaked_pid);
    assert_eq!(survivor.entry.pgid, Some(leaked_pgid));
    assert_eq!(survivor.entry.host_run_id, run1_id);
    assert_eq!(survivor.entry.status, RuntimeProcessStatus::Orphaned);
    assert_eq!(
        survivor.entry.termination_stage,
        ProcessTerminationStage::SurvivedRestart
    );
    assert!(survivor.leader_alive);

    // Registry immediately exposes the survived orphan to UI/gRPC/operators!
    assert_eq!(runtime2.process_registry().survived_processes().len(), 1);
    assert!(runtime2.process_registry().get(leaked_pid).is_some());

    // Reap the survived orphan gracefully after restart.
    let reap = runtime2
        .process_registry()
        .reap_survived_process(
            leaked_pid,
            &ProcessGracefulShutdownPolicy::new(Duration::from_secs(2)),
            2000,
        )
        .expect("reap survived process")
        .expect("survivor should be found");
    assert!(reap.graceful_term_sent);
    assert!(!reap.escalated_to_sigkill);
    assert!(reap.confirmed_absent);
    assert_eq!(
        reap.termination_stage,
        ProcessTerminationStage::OrphanReaped
    );
    // Because both simulated boots share this test OS parent (and we called
    // `mem::forget` on the child handle), reap the zombie exit status here and
    // verify the kernel terminated it via SIGTERM rather than SIGKILL.
    let mut status: libc::c_int = 0;
    // SAFETY: waitpid on our own forgotten child PID with valid status pointer.
    let waited = unsafe { libc::waitpid(leaked_pid as libc::pid_t, &mut status, 0) };
    assert_eq!(waited, leaked_pid as libc::pid_t);
    assert!(libc::WIFSIGNALED(status));
    assert_eq!(libc::WTERMSIG(status), libc::SIGTERM);
    assert!(!os_pid_alive(leaked_pid));
    assert!(runtime2.process_registry().survived_processes().is_empty());

    // Confirm durable ledger reflects `OrphanReaped` after restart.
    let persisted = store.load_all().expect("load ledger after reap");
    let reaped_row = persisted
        .iter()
        .find(|e| e.pid == leaked_pid && e.host_run_id == run1_id)
        .expect("persisted row");
    assert_eq!(
        reaped_row.termination_stage,
        ProcessTerminationStage::OrphanReaped
    );
    assert!(!reaped_row.is_non_terminal());

    // Ensure our cooperative hook helper type compiles cleanly for io errors too.
    let _ = io::ErrorKind::Other;
}
