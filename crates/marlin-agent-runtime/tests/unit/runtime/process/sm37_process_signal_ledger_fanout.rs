// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! SM37 (ADV-L4) adversarial confinement tests for runtime process signaling,
//! anti-PID-reuse orphan reconciliation, symlink-safe bounded process ledgers,
//! and bounded sub-agent fanout parallelism with abort-drain quiescence.

use std::{
    fs, io,
    os::unix::fs::symlink,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use marlin_agent_gitops::working_copy::{
    WorkingCopyCreateRequest, WorkingCopyHandle, WorkingCopyIsolationProvider,
    WorkingCopyIsolationReceipt, WorkingCopyIsolationRequest,
};
use marlin_agent_runtime::{
    CancellationToken, FileProcessLedgerStore, ManagedProcessLedgerEntry,
    ManagedProcessLedgerRecordRequest, ProcessGracefulShutdownDurationMs,
    ProcessGracefulShutdownPolicy, ProcessGroupId, ProcessLedgerStore, RuntimeContext,
    RuntimeEnvironment, RuntimeFanoutJoinPolicy, RuntimeFanoutTaskStatus, RuntimeFuture,
    RuntimeHostRunId, SubAgentRuntime, TokioAgentRuntime, WorkingCopySubAgentFanoutItem,
    process::{
        ReapSurvivedProcessRequest, RuntimeProcessKind, RuntimeProcessObservation,
        RuntimeProcessObservationTimestampMs, RuntimeProcessTerminator,
        SysinfoRuntimeProcessController, is_process_group_alive, probe_persisted_process,
        reap_survived_entry, signal_process_group_kill, signal_process_group_term,
    },
};

fn current_host_pgid() -> u32 {
    // SAFETY: getpgrp takes no arguments and always succeeds on POSIX.
    let pgid = unsafe { libc::getpgrp() };
    u32::try_from(pgid).expect("positive host pgid")
}

#[test]
fn sm37_protected_pid_and_pgid_rejected_before_posix_kill() {
    let host_pid = std::process::id();
    let host_pgid = current_host_pgid();

    // Probe signal=0 first so an unhardened build fails closed here BEFORE any
    // SIGTERM/SIGKILL helper is invoked on pgid=0 (kill(0)) or pgid=1 (kill(-1)).
    for protected_pgid in [0u32, 1u32, host_pgid] {
        let probe = is_process_group_alive(protected_pgid);
        assert!(
            probe.is_err(),
            "SM37: is_process_group_alive({protected_pgid}) must reject protected/host pgid, got {probe:?}"
        );
    }

    // With guard proven live, verify SIGTERM / SIGKILL helpers refuse all protected targets.
    for (pid, pgid) in [
        (0, None),
        (1, None),
        (99_999_999, Some(0)),
        (99_999_999, Some(1)),
        (host_pid, None),
        (99_999_999, Some(host_pgid)),
        (u32::MAX, Some(u32::MAX)),
    ] {
        assert_eq!(
            signal_process_group_term(pid, pgid)
                .expect_err("SM37: term must refuse protected target")
                .kind(),
            io::ErrorKind::InvalidData,
            "SM37: signal_process_group_term({pid}, {pgid:?}) must return InvalidData"
        );
        assert_eq!(
            signal_process_group_kill(pid, pgid)
                .expect_err("SM37: kill must refuse protected target")
                .kind(),
            io::ErrorKind::InvalidData,
            "SM37: signal_process_group_kill({pid}, {pgid:?}) must return InvalidData"
        );
    }

    let mut controller = SysinfoRuntimeProcessController::new();
    for protected_pid in [0u32, 1u32, host_pid] {
        assert!(
            controller.request_termination(protected_pid).is_err(),
            "SM37: SysinfoRuntimeProcessController::request_termination({protected_pid}) must fail closed"
        );
        assert!(
            controller.force_kill(protected_pid).is_err(),
            "SM37: SysinfoRuntimeProcessController::force_kill({protected_pid}) must fail closed"
        );
    }
}

#[test]
fn sm37_orphan_reconciliation_fails_closed_without_start_time_or_on_protected_pid() {
    let host_pid = std::process::id();
    let host_pgid = current_host_pgid();

    let missing_fingerprint_obs =
        RuntimeProcessObservation::new(host_pid, RuntimeProcessKind::Tool, "unfingerprinted-self")
            .with_started_at_ms(100);
    let missing_fingerprint_entry =
        ManagedProcessLedgerEntry::from_request(ManagedProcessLedgerRecordRequest {
            host_run_id: RuntimeHostRunId::new("marlin-prior-run"),
            host_pid: 42,
            observation: &missing_fingerprint_obs,
            pgid: Some(ProcessGroupId::new(host_pgid)),
            os_start_time_secs: None,
            graceful_timeout: ProcessGracefulShutdownDurationMs::new(1000),
            observed_at_ms: RuntimeProcessObservationTimestampMs::new(100),
        });

    let probe = probe_persisted_process(&missing_fingerprint_entry);
    assert!(
        !probe.is_alive(),
        "SM37: ledger entry without os_start_time_secs or pointing at host PID/PGID must never probe alive: {probe:?}"
    );

    let init_obs = RuntimeProcessObservation::new(1, RuntimeProcessKind::Tool, "init-spoof")
        .with_started_at_ms(100);
    let init_entry = ManagedProcessLedgerEntry::from_request(ManagedProcessLedgerRecordRequest {
        host_run_id: RuntimeHostRunId::new("marlin-prior-run"),
        host_pid: 42,
        observation: &init_obs,
        pgid: Some(ProcessGroupId::new(1)),
        os_start_time_secs: None,
        graceful_timeout: ProcessGracefulShutdownDurationMs::new(1000),
        observed_at_ms: RuntimeProcessObservationTimestampMs::new(100),
    });
    assert!(
        !probe_persisted_process(&init_entry).is_alive(),
        "SM37: init PID=1 entry must never probe alive"
    );

    let reap_err = reap_survived_entry(ReapSurvivedProcessRequest {
        store: None,
        current_host_run_id: &RuntimeHostRunId::new("marlin-current-run"),
        entry: init_entry,
        policy: &ProcessGracefulShutdownPolicy::new(Duration::from_millis(50)),
        observed_at_ms: RuntimeProcessObservationTimestampMs::new(200),
    })
    .expect_err("SM37: reap_survived_entry must fail closed on protected PID=1");
    assert_eq!(reap_err.kind(), io::ErrorKind::InvalidData);
}

#[test]
fn sm37_file_process_ledger_refuses_symlinks_bad_schema_and_oversize_files() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let outside = tmp.path().join("outside");
    let ledger_dir = tmp.path().join("ledger");
    fs::create_dir_all(&outside).expect("outside dir");
    fs::create_dir_all(&ledger_dir).expect("ledger dir");

    // 1. Leaf symlink to outside valid JSON file must be refused fail-closed.
    let outside_ledger = outside.join("victim.json");
    fs::write(&outside_ledger, r#"{"schema_version":1,"entries":[]}"#).expect("write victim");
    let symlink_ledger = ledger_dir.join("managed-processes.json");
    symlink(&outside_ledger, &symlink_ledger).expect("create leaf symlink");
    let symlink_store = FileProcessLedgerStore::new(&symlink_ledger);
    assert!(
        symlink_store.load_all().is_err(),
        "SM37: FileProcessLedgerStore::load_all must refuse symlinked ledger file"
    );

    // 2. Parent directory symlink escape must be refused fail-closed.
    let symlink_parent = tmp.path().join("escaped_dir");
    symlink(&outside, &symlink_parent).expect("create parent symlink");
    let parent_escape_store = FileProcessLedgerStore::new(symlink_parent.join("new-ledger.json"));
    let sample_obs = RuntimeProcessObservation::new(42_424, RuntimeProcessKind::Tool, "probe");
    let sample_entry = ManagedProcessLedgerEntry::from_request(ManagedProcessLedgerRecordRequest {
        host_run_id: RuntimeHostRunId::new("run-symlink-test"),
        host_pid: 42_000,
        observation: &sample_obs,
        pgid: Some(ProcessGroupId::new(42_424)),
        os_start_time_secs: None,
        graceful_timeout: ProcessGracefulShutdownDurationMs::new(1000),
        observed_at_ms: RuntimeProcessObservationTimestampMs::new(100),
    });
    assert!(
        parent_escape_store.upsert(&sample_entry).is_err(),
        "SM37: FileProcessLedgerStore::upsert must refuse symlinked parent directory"
    );

    // 3. Unknown schema_version must fail closed with InvalidData.
    let bad_schema_path = ledger_dir.join("bad-schema.json");
    fs::write(&bad_schema_path, r#"{"schema_version":999,"entries":[]}"#)
        .expect("write bad schema");
    let bad_schema_store = FileProcessLedgerStore::new(&bad_schema_path);
    let schema_err = bad_schema_store
        .load_all()
        .expect_err("SM37: unsupported ledger schema_version must fail closed");
    assert_eq!(schema_err.kind(), io::ErrorKind::InvalidData);

    // 4. Oversize (> 4 MiB) ledger file must fail closed before JSON deserialization.
    let oversize_path = ledger_dir.join("oversize.json");
    let mut big_json = String::with_capacity(4 * 1024 * 1024 + 128);
    big_json.push_str(r#"{"schema_version":1,"entries":[]}"#);
    big_json.push_str(&" ".repeat(4 * 1024 * 1024 + 64));
    fs::write(&oversize_path, big_json).expect("write oversize valid json");
    let oversize_store = FileProcessLedgerStore::new(&oversize_path);
    let oversize_err = oversize_store
        .load_all()
        .expect_err("SM37: oversize ledger file (> 4 MiB) must be refused fail-closed");
    assert_eq!(oversize_err.kind(), io::ErrorKind::InvalidData);
}

#[derive(Clone, Debug)]
struct ConcurrencyMeasuringSubAgent {
    active: Arc<AtomicUsize>,
    peak: Arc<AtomicUsize>,
}

impl SubAgentRuntime for ConcurrencyMeasuringSubAgent {
    type Input = usize;
    type Output = usize;

    fn run_sub_agent(
        &self,
        input: Self::Input,
        _context: RuntimeContext,
    ) -> RuntimeFuture<Self::Output> {
        let active = Arc::clone(&self.active);
        let peak = Arc::clone(&self.peak);
        Box::pin(async move {
            let now = active.fetch_add(1, Ordering::SeqCst) + 1;
            peak.fetch_max(now, Ordering::SeqCst);
            tokio::time::sleep(Duration::from_millis(40)).await;
            active.fetch_sub(1, Ordering::SeqCst);
            input
        })
    }
}

#[derive(Clone, Debug)]
struct PanicAndSlowSiblingSubAgent {
    live_siblings: Arc<AtomicUsize>,
}

struct LiveSiblingGuard(Arc<AtomicUsize>);

impl Drop for LiveSiblingGuard {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

impl SubAgentRuntime for PanicAndSlowSiblingSubAgent {
    type Input = usize;
    type Output = usize;

    fn run_sub_agent(
        &self,
        input: Self::Input,
        _context: RuntimeContext,
    ) -> RuntimeFuture<Self::Output> {
        let live_siblings = Arc::clone(&self.live_siblings);
        Box::pin(async move {
            if input == 0 {
                tokio::time::sleep(Duration::from_millis(25)).await;
                panic!("SM37 intentional worker panic at index 0");
            }
            live_siblings.fetch_add(1, Ordering::SeqCst);
            let _guard = LiveSiblingGuard(live_siblings);
            tokio::time::sleep(Duration::from_secs(30)).await;
            input
        })
    }
}

fn make_fanout_item(index: usize) -> WorkingCopySubAgentFanoutItem<usize> {
    let request = WorkingCopyIsolationRequest::Create(WorkingCopyCreateRequest::new(
        "marlin-core",
        WorkingCopyIsolationProvider::GitWorktree,
        "/repo",
        WorkingCopyHandle::new(format!("slot-{index}"), format!("/repo.slot-{index}")),
    ));
    WorkingCopySubAgentFanoutItem::new(
        index,
        RuntimeEnvironment::default().with_cwd(format!("/repo.slot-{index}")),
        WorkingCopyIsolationReceipt::applied(&request),
    )
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn sm37_working_copy_fanout_enforces_hard_parallelism_cap_and_drains_on_panic() {
    // 1. Unbounded / huge parallelism request must clamp to hard ceiling (<= 64).
    let (runtime, _events) = TokioAgentRuntime::with_environment(
        256,
        CancellationToken::new(),
        RuntimeEnvironment::default().with_cwd("/repo"),
    );
    let active = Arc::new(AtomicUsize::new(0));
    let peak = Arc::new(AtomicUsize::new(0));
    let items: Vec<_> = (0..80).map(make_fanout_item).collect();
    let output = runtime
        .spawn_sub_agents_with_working_copy_environments_with_policy(
            Arc::new(ConcurrencyMeasuringSubAgent {
                active: Arc::clone(&active),
                peak: Arc::clone(&peak),
            }),
            items,
            RuntimeFanoutJoinPolicy::bounded(512),
        )
        .join()
        .await
        .expect("fanout task should finish")
        .expect("fanout outputs should succeed");
    assert_eq!(output.outputs().len(), 80);
    let observed_peak = peak.load(Ordering::SeqCst);
    assert!(
        observed_peak <= 64,
        "SM37: sub-agent fanout peak concurrency must never exceed hard cap 64, observed {observed_peak}"
    );

    // 2. Worker panic with cancel_on_first_error must attribute exact input_index and drain
    //    aborted siblings to quiescence BEFORE returning Err(receipt).
    let live_siblings = Arc::new(AtomicUsize::new(0));
    let panic_items: Vec<_> = (0..4).map(make_fanout_item).collect();
    let receipt = runtime
        .spawn_sub_agents_with_working_copy_environments_with_policy(
            Arc::new(PanicAndSlowSiblingSubAgent {
                live_siblings: Arc::clone(&live_siblings),
            }),
            panic_items,
            RuntimeFanoutJoinPolicy::bounded(2),
        )
        .join()
        .await
        .expect("fanout wrapper should finish")
        .expect_err("panicking worker must produce Err(receipt)");

    assert_eq!(
        live_siblings.load(Ordering::SeqCst),
        0,
        "SM37: cancel_remaining_work must await aborted sibling drain before returning Err(receipt)"
    );
    assert!(
        receipt.task_receipts().iter().any(|task| {
            task.status() == &RuntimeFanoutTaskStatus::JoinError && task.input_index() == Some(0)
        }),
        "SM37: JoinError receipt must preserve exact panicked input_index=Some(0), got {:?}",
        receipt.task_receipts()
    );
}
