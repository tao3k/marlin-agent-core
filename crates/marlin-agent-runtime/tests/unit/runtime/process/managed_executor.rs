// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use marlin_agent_protocol::{BoundedProcessExecutor, BoundedProcessRequest, ProcessExecutionError};
use marlin_agent_runtime::{
    ManagedProcessExecutor, RuntimeOperationId, RuntimeTaskTrackerPolicy, TokioAgentRuntime,
    process::RuntimeProcessKind,
};
use std::time::Duration;

fn request(script: &str, directory: &std::path::Path, limit: usize) -> BoundedProcessRequest {
    BoundedProcessRequest {
        program: "/bin/sh".into(),
        arguments: vec!["-c".into(), script.into()],
        working_directory: directory.into(),
        environment: marlin_agent_protocol::ProcessEnvironmentPolicy::Inherit,
        remove_environment: vec![],
        stdout_limit: limit,
        timeout: Duration::from_secs(30),
    }
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn managed_executor_budget_holds_until_process_and_pipe_are_reclaimed() {
    use marlin_agent_protocol::{ProcessMemorySample, ProcessMemorySampler};
    use marlin_agent_runtime::{
        AsyncManagedChildProcess, RuntimeAdmissionError, RuntimeProcessBudget,
        TokioAgentRuntimeBuildRequest,
    };
    use std::{io, process::Stdio, sync::Arc};
    use tokio::process::Command;

    #[derive(Debug)]
    struct FixtureMemory;
    impl ProcessMemorySampler for FixtureMemory {
        fn sample(&self, _: &[u32]) -> io::Result<ProcessMemorySample> {
            Ok(ProcessMemorySample {
                host_bytes: 40,
                child_bytes: 0,
            })
        }
    }

    let (owner, _events) = TokioAgentRuntime::from_build_request(
        TokioAgentRuntimeBuildRequest::runtime_root(8).with_process_budget(
            RuntimeProcessBudget {
                concurrent: 2.try_into().unwrap(),
                memory_limit_bytes: 100.try_into().unwrap(),
                command_reservation_bytes: 40.try_into().unwrap(),
            },
            Arc::new(FixtureMemory),
        ),
    );
    let context = owner.context();
    let launch = |operation: &'static str| {
        let mut command = Command::new("/bin/sleep");
        command.arg("30").stdin(Stdio::piped());
        AsyncManagedChildProcess::spawn(&context, command, RuntimeProcessKind::Tool, operation)
    };
    let (first, second) = tokio::join!(launch("test.first"), launch("test.second"));
    let (mut child, rejected) = match (first, second) {
        (Ok(child), Err(rejected)) | (Err(rejected), Ok(child)) => (child, rejected),
        outcomes => panic!("expected one process at admission: {outcomes:?}"),
    };
    let assert_budget = |error: io::Error| {
        assert_eq!(
            error.get_ref().and_then(|cause| cause.downcast_ref()),
            Some(&RuntimeAdmissionError::MemoryBudgetExceeded),
        );
    };
    assert_budget(rejected);
    let pipe = child.take_stdin().unwrap();
    let pid = child.pid();
    child.kill().await.unwrap();
    // SAFETY: signal zero only probes the fixture process.
    assert_eq!(unsafe { libc::kill(pid.try_into().unwrap(), 0) }, -1);
    assert_eq!(io::Error::last_os_error().raw_os_error(), Some(libc::ESRCH));
    assert_eq!(owner.task_tracker().reserved_process_memory_bytes(), 40);
    assert_budget(launch("test.retry").await.unwrap_err());
    drop(pipe);
    let next = launch("test.retry").await.unwrap();
    next.kill().await.unwrap();
    assert_eq!(owner.task_tracker().reserved_process_memory_bytes(), 0);
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn managed_executor_process_stop_is_scoped_and_rejects_old_generations() {
    use marlin_agent_runtime::{AsyncManagedChildProcess, SessionId};
    use std::io::ErrorKind;
    use tokio::{process::Command, time::timeout};

    let (owner, _events) = TokioAgentRuntime::new(8);
    let context = owner.context();
    let session = context.session().session_id();
    let registry = owner.process_registry();
    let before = registry.managed_revision();
    let launch = || {
        let mut command = Command::new("/bin/sleep");
        command.arg("30");
        AsyncManagedChildProcess::spawn(&context, command, RuntimeProcessKind::Tool, "test.stop")
    };
    let child = launch().await.unwrap();
    let tracked = registry.managed_revision();
    assert!(tracked > before);
    let id = child.resource_id().clone();
    let generation = id.generation().get().try_into().unwrap();
    assert!(!registry.stop_session_generation(&SessionId::new("other-session"), generation));
    assert!(registry.stop_session_generation(session, generation));
    let pid = child.pid();
    assert_eq!(
        child.wait().await.unwrap_err().kind(),
        ErrorKind::Interrupted
    );
    timeout(Duration::from_secs(5), async {
        while owner.process_registry().get_by_resource(&id).is_some() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    let completed = registry.managed_revision();
    assert!(completed > tracked);
    // SAFETY: signal zero only probes the fixture process.
    assert_eq!(unsafe { libc::kill(pid.try_into().unwrap(), 0) }, -1);
    assert_eq!(
        std::io::Error::last_os_error().raw_os_error(),
        Some(libc::ESRCH)
    );
    let replacement = launch().await.unwrap();
    assert!(registry.managed_revision() > completed);
    assert!(!registry.stop_session_generation(session, generation));
    assert!(
        owner
            .process_registry()
            .get_by_resource(replacement.resource_id())
            .is_some()
    );
    let rows = owner.process_registry().managed_processes(session);
    assert!(rows.iter().any(|row| row.resource_id() == Some(&id)
        && row.status == marlin_agent_runtime::process::RuntimeProcessStatus::Failed));
    replacement.kill().await.unwrap();
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn managed_executor_keeps_quiet_background_work_alive_and_rearms_health() {
    use marlin_agent_runtime::process::{
        HostHeartbeat, ProcessHealthCondition, ProcessHealthPolicy,
    };
    use tokio::{sync::mpsc, time::timeout};

    let (owner, _) = TokioAgentRuntime::new(8);
    let host = HostHeartbeat::new();
    let (reports, mut observed) = mpsc::channel(4);
    let executor = ManagedProcessExecutor::new(
        owner.context(),
        RuntimeProcessKind::Tool,
        RuntimeOperationId::new("test.health"),
    )
    .with_health_monitor(
        host.clone(),
        ProcessHealthPolicy::new(Duration::from_millis(150), Duration::from_millis(150)),
        reports,
    );
    let directory = tempfile::tempdir().unwrap();
    let command = request(
        "while [ ! -f progress ]; do /bin/sleep .02; done; printf progress; while [ ! -f finish ]; do /bin/sleep .02; done; printf done",
        directory.path(),
        1024,
    );
    let task = owner.spawn(async move { executor.execute(command).await });
    let observe = |condition, report: marlin_agent_runtime::process::ProcessHealthReport| {
        assert_eq!(report.condition(), condition);
        let process = owner
            .process_registry()
            .get_by_resource(report.resource_id())
            .unwrap();
        assert_eq!(
            process.status,
            marlin_agent_runtime::process::RuntimeProcessStatus::Running
        );
        // SAFETY: signal zero probes existence without delivering a signal.
        assert_eq!(unsafe { libc::kill(process.pid.try_into().unwrap(), 0) }, 0);
    };
    for condition in [
        ProcessHealthCondition::HostHeartbeatMissed,
        ProcessHealthCondition::TaskOutputIdle,
    ] {
        observe(
            condition,
            timeout(Duration::from_secs(5), observed.recv())
                .await
                .unwrap()
                .unwrap(),
        );
    }
    assert!(
        timeout(Duration::from_millis(250), observed.recv())
            .await
            .is_err()
    );

    host.pulse();
    std::fs::write(directory.path().join("progress"), []).unwrap();
    for condition in [
        ProcessHealthCondition::HostHeartbeatMissed,
        ProcessHealthCondition::TaskOutputIdle,
    ] {
        observe(
            condition,
            timeout(Duration::from_secs(5), observed.recv())
                .await
                .unwrap()
                .unwrap(),
        );
    }
    std::fs::write(directory.path().join("finish"), []).unwrap();
    let output = timeout(Duration::from_secs(5), task.join())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(output.success);
    assert_eq!(output.stdout, b"progressdone");
    assert!(owner.process_registry().active_processes().is_empty());
    assert_eq!(owner.task_tracker().active_resource_count(), 0);
}
#[tokio::test]
async fn managed_executor_cancellation_is_tracked_until_os_exit() {
    let (owner, _events) = TokioAgentRuntime::new(8);
    let executor = ManagedProcessExecutor::new(
        owner.context(),
        RuntimeProcessKind::Tool,
        RuntimeOperationId::new("test.process"),
    );
    let directory = tempfile::tempdir().unwrap();
    let pidfile = directory.path().join("pid");
    let command = request(
        "printf '%s' \"$$\" > pid; exec /bin/sleep 30",
        directory.path(),
        1024,
    );
    let task = owner.spawn(async move { executor.execute(command).await });
    let pid = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if let Ok(value) = std::fs::read_to_string(&pidfile)
                && let Ok(pid) = value.parse::<i32>()
            {
                break pid;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(unsafe { libc::kill(pid, 0) }, 0);
    assert_eq!(owner.process_registry().active_processes().len(), 1);
    task.abort();
    let _ = task.join().await;
    let receipt = owner
        .shutdown_tasks(&RuntimeTaskTrackerPolicy::cancel_on_shutdown(Some(5000)))
        .await;
    assert_eq!(receipt.remaining_resource_count(), 0);
    assert!(owner.process_registry().active_processes().is_empty());
    assert_eq!(
        unsafe { libc::kill(pid, 0) },
        -1,
        "child remains after runtime shutdown"
    );
    assert_eq!(
        std::io::Error::last_os_error().raw_os_error(),
        Some(libc::ESRCH)
    );
}
#[tokio::test]
async fn managed_executor_enforces_bounded_output() {
    let (owner, _events) = TokioAgentRuntime::new(8);
    let executor = ManagedProcessExecutor::new(
        owner.context(),
        RuntimeProcessKind::Tool,
        RuntimeOperationId::new("test.process"),
    );
    let directory = tempfile::tempdir().unwrap();
    for script in ["printf 123456789", "printf 123456789; exec /bin/sleep 30"] {
        let result = executor.execute(request(script, directory.path(), 4)).await;
        assert!(
            matches!(result, Err(ProcessExecutionError::OutputLimit)),
            "{result:?}"
        );
    }
    assert!(owner.process_registry().active_processes().is_empty());
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn managed_executor_deadline_reaps_group_with_inherited_stdout() {
    let (owner, _) = TokioAgentRuntime::new(8);
    let executor = ManagedProcessExecutor::new(
        owner.context(),
        RuntimeProcessKind::Tool,
        RuntimeOperationId::new("test.deadline"),
    );
    let directory = tempfile::tempdir().unwrap();
    let mut command = request(
        "/bin/sleep 30 & printf '%s %s\\n' \"$$\" \"$!\" > pids; wait",
        directory.path(),
        1024,
    );
    command.timeout = Duration::from_secs(1);
    assert!(matches!(
        executor.execute(command).await,
        Err(ProcessExecutionError::TimedOut)
    ));
    let pids = std::fs::read_to_string(directory.path().join("pids")).unwrap();
    let pids: Vec<i32> = pids
        .split_whitespace()
        .map(|pid| pid.parse().unwrap())
        .collect();
    assert_eq!(pids.len(), 2);
    let receipt = owner
        .shutdown_tasks(&RuntimeTaskTrackerPolicy::wait_on_shutdown(Some(5000)))
        .await;
    assert_eq!(receipt.remaining_resource_count(), 0);
    assert!(owner.process_registry().active_processes().is_empty());
    for pid in [pids[0], pids[1], -pids[0]] {
        // SAFETY: signal zero probes existence without delivering a signal.
        assert_eq!(unsafe { libc::kill(pid, 0) }, -1);
        assert_eq!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::ESRCH)
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn concurrent_process_starts_reserve_the_operation_before_launch() {
    let (owner, _) = TokioAgentRuntime::new(4);
    let directory = tempfile::tempdir().unwrap();
    let barrier = std::sync::Arc::new(tokio::sync::Barrier::new(2));
    let start = |marker: &str| {
        let request = request(
            &format!("printf started > {marker}; exec /bin/sleep 30"),
            directory.path(),
            1024,
        );
        let executor = ManagedProcessExecutor::new(
            owner.context(),
            RuntimeProcessKind::Tool,
            RuntimeOperationId::new("same-operation"),
        );
        let barrier = barrier.clone();
        owner.spawn(async move {
            barrier.wait().await;
            executor.execute(request).await
        })
    };
    let mut first = Some(start("first"));
    let mut second = Some(start("second"));
    let (rejected, active, rejected_marker) =
        tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                if first.as_ref().unwrap().is_finished() {
                    break (first.take().unwrap(), second.take().unwrap(), "first");
                }
                if second.as_ref().unwrap().is_finished() {
                    break (second.take().unwrap(), first.take().unwrap(), "second");
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .unwrap();
    let Err(ProcessExecutionError::Io(error)) = rejected.join().await.unwrap() else {
        panic!("concurrent launch must be rejected");
    };
    assert_eq!(
        error
            .get_ref()
            .and_then(|error| error.downcast_ref::<marlin_agent_runtime::RuntimeAdmissionError>()),
        Some(&marlin_agent_runtime::RuntimeAdmissionError::OperationActive),
    );
    assert!(!directory.path().join(rejected_marker).exists());
    tokio::time::timeout(Duration::from_secs(5), async {
        while owner.process_registry().active_processes().is_empty() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("admitted process should be registered");
    assert_eq!(owner.process_registry().active_processes().len(), 1);
    active.abort();
    let _ = active.join().await;
    let receipt = owner
        .shutdown_tasks(&RuntimeTaskTrackerPolicy::wait_on_shutdown(Some(5_000)))
        .await;
    assert_eq!(receipt.remaining_resource_count(), 0);
    assert!(owner.process_registry().active_processes().is_empty());
}

#[tokio::test]
async fn shutdown_wakes_a_process_waiting_for_quota_without_launching_it() {
    use marlin_agent_runtime::{
        AsyncManagedChildProcess, RuntimeAdmissionError, RuntimeTaskShutdownStatus,
        TokioAgentRuntimeBuildRequest,
    };
    use std::{future::Future, num::NonZeroUsize, task::Poll};

    let (runtime, _) = TokioAgentRuntime::from_build_request(
        TokioAgentRuntimeBuildRequest::runtime_root(4)
            .with_process_limit(NonZeroUsize::new(1).unwrap()),
    );
    let context = runtime.context();
    let mut command = tokio::process::Command::new("/bin/sleep");
    command.arg("30");
    let active =
        AsyncManagedChildProcess::spawn(&context, command, RuntimeProcessKind::Tool, "active")
            .await
            .unwrap();
    let directory = tempfile::tempdir().unwrap();
    let queued = ManagedProcessExecutor::new(
        context,
        RuntimeProcessKind::Tool,
        RuntimeOperationId::new("queued"),
    );
    let pending = queued.execute(request("printf started > marker", directory.path(), 1024));
    tokio::pin!(pending);
    std::future::poll_fn(|cx| {
        assert!(
            pending.as_mut().poll(cx).is_pending(),
            "quota should delay launch"
        );
        Poll::Ready(())
    })
    .await;

    let shutdown = tokio::spawn({
        let runtime = runtime.clone();
        async move {
            runtime
                .shutdown_tasks(&RuntimeTaskTrackerPolicy::wait_on_shutdown(Some(5_000)))
                .await
        }
    });
    let Err(ProcessExecutionError::Io(error)) =
        tokio::time::timeout(Duration::from_secs(2), pending)
            .await
            .unwrap()
    else {
        panic!("pending process must wake with a closed-admission error");
    };
    assert_eq!(
        error
            .get_ref()
            .and_then(|cause| cause.downcast_ref::<RuntimeAdmissionError>()),
        Some(&RuntimeAdmissionError::Closed),
    );
    assert!(!directory.path().join("marker").exists());
    assert!(!shutdown.is_finished());
    active.kill().await.unwrap();
    let receipt = shutdown.await.unwrap();
    assert_eq!(receipt.status(), &RuntimeTaskShutdownStatus::Completed);
    assert_eq!(receipt.tracked_resource_count_at_shutdown(), 1);
}
