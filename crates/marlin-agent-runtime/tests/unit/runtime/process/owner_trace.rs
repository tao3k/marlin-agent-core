// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Replay the Runtime ownership model through actual macOS subprocesses.
use marlin_agent_runtime::{
    AgentSessionContext, AsyncManagedChildProcess, ContextVisibility, ManagedChildProcessSpec,
    RuntimeAdmissionError, RuntimeContext, RuntimeResourceId, RuntimeTask, RuntimeTaskJoinError,
    RuntimeTaskShutdownReceipt, RuntimeTaskShutdownStatus, RuntimeTaskTrackerPolicy,
    TokioAgentRuntime,
    process::{
        HostHeartbeat, ProcessHealthCondition, ProcessHealthPolicy, ProcessWatchdogReport,
        RuntimeProcessKind, RuntimeProcessStatus, WatchdogCleanup,
    },
};
use serde::Deserialize;
use std::{fs, path::Path, process::Stdio, time::Duration};
use tokio::{
    process::Command,
    sync::oneshot,
    task::JoinHandle,
    time::{Instant, timeout},
};

const DEADLINE: Duration = Duration::from_secs(10);
const OPERATION: &str = "test.tool";
const OTHER_OPERATION: &str = "test.other-tool";

#[derive(Deserialize)]
struct Trace {
    states: Vec<Frame>,
}

#[derive(Deserialize)]
struct Frame {
    state: State,
}

#[derive(Deserialize)]
struct Int {
    #[serde(rename = "#bigint")]
    value: String,
}

impl Int {
    fn get(&self) -> u64 {
        self.value.parse().unwrap()
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct State {
    closed: bool,
    a: Int,
    b: Int,
    c: Int,
    last_a: Int,
    held: bool,
    task: bool,
    next: Int,
    done: bool,
    report: Int,
    target: Int,
    event: String,
}

fn context(runtime: &TokioAgentRuntime, name: &str) -> RuntimeContext {
    runtime
        .context()
        .with_session_context(AgentSessionContext::root(
            name,
            ContextVisibility::default_runtime(),
        ))
}

async fn launch(
    context: &RuntimeContext,
    operation: &str,
    marker: Option<&Path>,
) -> std::io::Result<AsyncManagedChildProcess> {
    let mut command = if let Some(marker) = marker {
        let mut command = Command::new("/bin/sh");
        command.args([
            "-c",
            "printf launched > \"$1\"; exec /bin/sleep 30",
            "fixture",
        ]);
        command.arg(marker);
        command
    } else {
        let mut command = Command::new("/bin/sleep");
        command.arg("30");
        command
    };
    command.stdin(Stdio::piped());
    AsyncManagedChildProcess::spawn_with_spec(
        context,
        command,
        ManagedChildProcessSpec::new(RuntimeProcessKind::Tool, operation),
    )
    .await
}

fn os_alive(pid: u32) -> bool {
    // SAFETY: signal 0 probes existence without delivering a signal.
    if unsafe { libc::kill(pid.try_into().unwrap(), 0) } == 0 {
        return true;
    }
    assert_eq!(
        std::io::Error::last_os_error().raw_os_error(),
        Some(libc::ESRCH)
    );
    false
}

async fn reap(child: AsyncManagedChildProcess, runtime: &TokioAgentRuntime) -> RuntimeResourceId {
    let pid = child.pid();
    let id = child.resource_id().clone();
    // Exercise the real reaper from a thread without a Tokio runtime context.
    std::thread::spawn(move || drop(child)).join().unwrap();
    wait_for_reap(pid, &id, runtime).await;
    id
}

async fn wait_for_reap(pid: u32, id: &RuntimeResourceId, runtime: &TokioAgentRuntime) {
    timeout(DEADLINE, async {
        while os_alive(pid) || runtime.process_registry().get_by_resource(id).is_some() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("managed child did not reap");
}

fn assert_rejection(error: std::io::Error, expected: RuntimeAdmissionError) {
    assert_eq!(
        error
            .get_ref()
            .and_then(|cause| cause.downcast_ref::<RuntimeAdmissionError>()),
        Some(&expected),
        "{error:?}",
    );
}

fn check_process(
    process: Option<&AsyncManagedChildProcess>,
    generation: &Int,
    issued: &std::collections::BTreeMap<u64, RuntimeResourceId>,
    runtime: &TokioAgentRuntime,
) {
    let number = generation.get();
    match process {
        None => assert_eq!(number, 0),
        Some(process) => {
            assert_eq!(process.resource_id(), &issued[&number]);
            assert!(
                os_alive(process.pid()),
                "registered child must still be alive"
            );
            assert_eq!(
                runtime
                    .process_registry()
                    .get_by_resource(process.resource_id())
                    .unwrap()
                    .status,
                RuntimeProcessStatus::Running,
            );
        }
    }
}

#[tokio::test]
#[ignore = "requires generated Quint runtime-process-owner ITF files"]
async fn generated_owner_traces_match_macos_runtime() {
    let directory =
        std::env::var_os("MARLIN_OWNER_TRACE_DIR").expect("MARLIN_OWNER_TRACE_DIR required");
    let mut files: Vec<_> = fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .collect();
    files.sort();
    assert_eq!(
        files.len(),
        23,
        "seven directed and sixteen simulated traces"
    );
    for file in files {
        eprintln!("replaying {}", file.display());
        let trace: Trace = serde_json::from_slice(&fs::read(file).unwrap()).unwrap();
        let (runtime, _) = TokioAgentRuntime::new(4);
        let a_context = context(&runtime, "trace.session.a");
        let b_context = context(&runtime, "trace.session.b");
        let mut a = None;
        let mut b = None;
        let mut c = None;
        let mut pipe = None;
        let mut report = None;
        let mut health_report = None;
        let mut issued = std::collections::BTreeMap::<u64, RuntimeResourceId>::new();
        let mut task: Option<(RuntimeTask<()>, oneshot::Sender<()>)> = None;
        let mut shutdown: Option<JoinHandle<RuntimeTaskShutdownReceipt>> = None;
        let mut done = false;
        let temp = tempfile::tempdir().unwrap();
        let marker = temp.path().join("rejected-process");
        for frame in trace.states {
            let state = frame.state;
            match state.event.as_str() {
                "init" => {}
                "startA" | "startB" | "startC" => {
                    let (scope, operation, slot, generation) = match state.event.as_str() {
                        "startA" => (&a_context, OPERATION, &mut a, state.a.get()),
                        "startB" => (&b_context, OPERATION, &mut b, state.b.get()),
                        _ => (&a_context, OTHER_OPERATION, &mut c, state.c.get()),
                    };
                    let process = launch(scope, operation, None).await.unwrap();
                    let id = process.resource_id();
                    assert_eq!(id.session(), scope.session().session_id());
                    assert_eq!(id.operation().as_str(), operation);
                    if let Some((_, previous)) = issued.last_key_value() {
                        assert!(previous.generation() < id.generation());
                    }
                    assert!(issued.insert(generation, id.clone()).is_none());
                    *slot = Some(process);
                }
                "duplicateA" | "rejectProcess" => {
                    let error = launch(&a_context, OPERATION, Some(&marker))
                        .await
                        .unwrap_err();
                    let expected = if state.closed {
                        RuntimeAdmissionError::Closed
                    } else {
                        RuntimeAdmissionError::OperationActive
                    };
                    assert_rejection(error, expected);
                    assert!(!marker.exists(), "rejected command ran");
                }
                "borrowA" => pipe = a.as_mut().unwrap().take_stdin(),
                "reapA" => {
                    let retired = reap(a.take().unwrap(), &runtime).await;
                    assert_eq!(retired, issued[&state.last_a.get()]);
                }
                "reapB" => {
                    reap(b.take().unwrap(), &runtime).await;
                }
                "reapC" => {
                    reap(c.take().unwrap(), &runtime).await;
                }
                "releasePipe" => drop(pipe.take().unwrap()),
                "staleA" => {
                    let old = &issued[&state.last_a.get()];
                    assert!(
                        runtime
                            .process_registry()
                            .request_cleanup_for(old, 0)
                            .is_none()
                    );
                }
                "observeA" => {
                    let process = a.as_ref().unwrap();
                    let mut pending = std::pin::pin!(std::future::pending::<()>());
                    let observation = process
                        .command_watchdog(Instant::now())
                        .observe_until(pending.as_mut())
                        .await
                        .unwrap_err();
                    assert_eq!(observation.resource_id(), &issued[&state.report.get()]);
                    report = Some(observation);
                    health_report = None;
                }
                "observeHost" | "observeIdle" => {
                    let process = a.as_ref().unwrap();
                    let (policy, expected) = if state.event == "observeHost" {
                        (
                            ProcessHealthPolicy::new(Duration::ZERO, DEADLINE),
                            ProcessHealthCondition::HostHeartbeatMissed,
                        )
                    } else {
                        (
                            ProcessHealthPolicy::new(DEADLINE, Duration::ZERO),
                            ProcessHealthCondition::TaskOutputIdle,
                        )
                    };
                    let mut monitor = process
                        .watchdog()
                        .monitor_health(HostHeartbeat::new(), policy);
                    let reports: Vec<_> = monitor.sample(None).collect();
                    assert_eq!(reports.len(), 1);
                    assert_eq!(reports[0].condition(), expected);
                    assert_eq!(reports[0].resource_id(), &issued[&state.report.get()]);
                    assert!(monitor.sample(None).next().is_none());
                    health_report = reports.into_iter().next();
                    report = None;
                }
                "applyA" | "applyB" | "applyC" => {
                    let slot = match state.event.as_str() {
                        "applyA" => &mut a,
                        "applyB" => &mut b,
                        _ => &mut c,
                    };
                    let child = slot.take().unwrap();
                    let pid = child.pid();
                    let id = child.resource_id().clone();
                    let evidence = if let Some(health) = &health_report {
                        ProcessWatchdogReport::Health(health)
                    } else {
                        ProcessWatchdogReport::CommandDeadline(report.as_ref().unwrap())
                    };
                    match child.apply_watchdog_report(evidence) {
                        WatchdogCleanup::Started => {
                            wait_for_reap(pid, &id, &runtime).await;
                            assert!(
                                health_report.is_none(),
                                "advisory watchdog report reaped a healthy process"
                            );
                            assert_ne!(
                                state.target.get(),
                                0,
                                "stale watchdog report reaped a live replacement"
                            );
                            assert_eq!(id, issued[&state.target.get()]);
                        }
                        WatchdogCleanup::Stale(child) => {
                            assert_eq!(state.target.get(), 0);
                            if let Some(health) = &health_report {
                                assert_ne!(health.resource_id(), &id);
                            }
                            assert!(os_alive(pid));
                            *slot = Some(child);
                        }
                        WatchdogCleanup::Advisory(child) => {
                            assert_eq!(state.target.get(), 0);
                            assert_eq!(health_report.as_ref().unwrap().resource_id(), &id);
                            assert!(os_alive(pid));
                            *slot = Some(child);
                        }
                    }
                }
                "startTask" => {
                    let (release, wait) = oneshot::channel();
                    let admitted = runtime.spawn(async move { wait.await.unwrap() });
                    let id = admitted.admission().unwrap().clone();
                    if let Some((_, previous)) = issued.last_key_value() {
                        assert!(previous.generation() < id.generation());
                    }
                    assert!(issued.insert(state.next.get() - 1, id).is_none());
                    task = Some((admitted, release));
                }
                "releaseTask" => {
                    let (admitted, release) = task.take().unwrap();
                    release.send(()).unwrap();
                    admitted.join().await.unwrap();
                }
                "rejectTask" => {
                    let rejected = runtime.spawn(async { panic!("rejected task ran") });
                    assert_eq!(rejected.admission(), Err(RuntimeAdmissionError::Closed));
                    assert!(matches!(
                        rejected.join().await,
                        Err(RuntimeTaskJoinError::Admission(
                            RuntimeAdmissionError::Closed
                        ))
                    ));
                }
                "close" => {
                    let owner = runtime.clone();
                    shutdown = Some(tokio::spawn(async move {
                        owner
                            .shutdown_tasks(&RuntimeTaskTrackerPolicy::wait_on_shutdown(Some(
                                10_000,
                            )))
                            .await
                    }));
                    timeout(DEADLINE, runtime.task_tracker().closed())
                        .await
                        .unwrap();
                }
                "finish" => {
                    if let Some(waiter) = shutdown.take() {
                        let receipt = timeout(DEADLINE, waiter).await.unwrap().unwrap();
                        assert_eq!(receipt.status(), &RuntimeTaskShutdownStatus::Completed);
                        assert_eq!(receipt.remaining_resource_count(), 0);
                    }
                    done = true;
                }
                event => panic!("unknown Quint action {event}"),
            }
            check_process(a.as_ref(), &state.a, &issued, &runtime);
            check_process(b.as_ref(), &state.b, &issued, &runtime);
            check_process(c.as_ref(), &state.c, &issued, &runtime);
            assert_eq!(runtime.task_tracker().is_closed(), state.closed);
            assert_eq!(
                runtime.task_tracker().active_task_count(),
                usize::from(state.task)
            );
            assert_eq!(pipe.is_some(), state.held);
            assert_eq!(done, state.done);
            let owned = usize::from(state.a.get() != 0 || state.held)
                + usize::from(state.b.get() != 0)
                + usize::from(state.c.get() != 0)
                + usize::from(state.task);
            assert_eq!(runtime.task_tracker().active_resource_count(), owned);
            if owned != 0
                && let Some(waiter) = &shutdown
            {
                assert!(
                    !waiter.is_finished(),
                    "shutdown finished with live ownership"
                );
            }
        }
        drop(pipe);
        for child in [a, b, c].into_iter().flatten() {
            reap(child, &runtime).await;
        }
        if let Some((admitted, release)) = task {
            release.send(()).unwrap();
            admitted.join().await.unwrap();
        }
        if let Some(waiter) = shutdown {
            assert_eq!(
                waiter.await.unwrap().status(),
                &RuntimeTaskShutdownStatus::Completed
            );
        }
        assert_eq!(runtime.task_tracker().active_resource_count(), 0);
    }
}
