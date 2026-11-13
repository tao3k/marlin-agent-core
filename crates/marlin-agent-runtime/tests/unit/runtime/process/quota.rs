// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use std::{num::NonZeroUsize, process::Command, time::Duration};

use marlin_agent_runtime::{
    AsyncManagedChildProcess, ManagedChildProcessSpec, TokioAgentRuntime,
    TokioAgentRuntimeBuildRequest, process::RuntimeProcessKind,
};
use tokio::{process::Command as AsyncCommand, sync::oneshot, time::timeout};

fn os_process_exists(pid: u32) -> bool {
    Command::new("/bin/kill")
        .args(["-0", &pid.to_string()])
        .status()
        .expect("OS process probe")
        .success()
}

#[tokio::test]
async fn process_quota_waits_for_cancelled_tasks_child_to_exit() {
    let (runtime, _events) = TokioAgentRuntime::from_build_request(
        TokioAgentRuntimeBuildRequest::runtime_root(4)
            .with_process_limit(NonZeroUsize::new(1).unwrap()),
    );
    let context_a = runtime.context().child_context();
    let context_b = runtime.context().child_context();
    let mut command_a = AsyncCommand::new("/bin/sleep");
    command_a.arg("30");
    let child_a = AsyncManagedChildProcess::spawn_with_spec(
        &context_a,
        command_a,
        ManagedChildProcessSpec::new(RuntimeProcessKind::Tool, "quota-a"),
    )
    .await
    .unwrap();
    let pid_a = child_a.pid();
    assert!(os_process_exists(pid_a));

    // A finishes as cancelled while a separate cleanup task still owns its child.
    let (cancel_tx, cancel_rx) = oneshot::channel();
    let (release_tx, release_rx) = oneshot::channel();
    let (cleaned_tx, cleaned_rx) = oneshot::channel();
    let task_a = tokio::spawn(async move {
        cancel_rx.await.unwrap();
        child_a.request_cleanup();
        tokio::spawn(async move {
            release_rx.await.unwrap();
            child_a.kill().await.unwrap();
            let _ = cleaned_tx.send(());
        });
    });
    cancel_tx.send(()).unwrap();
    task_a.await.unwrap();
    assert!(
        os_process_exists(pid_a),
        "A must still be alive during draining"
    );

    let (attempt_tx, attempt_rx) = oneshot::channel();
    let mut task_b = tokio::spawn(async move {
        let mut command = AsyncCommand::new("/bin/sleep");
        command.arg("30");
        attempt_tx.send(()).unwrap();
        AsyncManagedChildProcess::spawn_with_spec(
            &context_b,
            command,
            ManagedChildProcessSpec::new(RuntimeProcessKind::Tool, "quota-b"),
        )
        .await
        .unwrap()
    });
    attempt_rx.await.unwrap();
    assert!(
        timeout(Duration::from_millis(200), &mut task_b)
            .await
            .is_err(),
        "B started while A's OS process was still alive"
    );
    assert!(
        os_process_exists(pid_a),
        "the quota check must precede A's exit"
    );

    release_tx.send(()).unwrap();
    timeout(Duration::from_secs(5), cleaned_rx)
        .await
        .unwrap()
        .unwrap();
    assert!(
        !os_process_exists(pid_a),
        "A must have exited before B starts"
    );
    let child_b = timeout(Duration::from_secs(5), task_b)
        .await
        .unwrap()
        .unwrap();
    assert!(os_process_exists(child_b.pid()));
    child_b.kill().await.unwrap();
}

// Fault control: this fixture deliberately drops its permit on cancellation
// instead of transferring it to the cleanup task. Both children are real OS
// processes; the overlap is the same observation forbidden by the main test.
#[tokio::test]
async fn early_permit_release_negative_control_observes_os_overlap() {
    let (runtime, _events) = TokioAgentRuntime::new(4);
    let quota = std::sync::Arc::new(tokio::sync::Semaphore::new(1));
    let permit_a = quota.clone().acquire_owned().await.unwrap();
    let mut command_a = AsyncCommand::new("/bin/sleep");
    command_a.arg("30");
    let child_a = AsyncManagedChildProcess::spawn_with_spec(
        &runtime.context(),
        command_a,
        ManagedChildProcessSpec::new(RuntimeProcessKind::Tool, "bad-a"),
    )
    .await
    .unwrap();
    child_a.request_cleanup();
    drop(permit_a); // Inject the premature return while A is still alive.
    let permit_b = timeout(Duration::from_secs(5), quota.acquire_owned())
        .await
        .unwrap()
        .unwrap();
    let mut command_b = AsyncCommand::new("/bin/sleep");
    command_b.arg("30");
    let child_b = AsyncManagedChildProcess::spawn_with_spec(
        &runtime.context(),
        command_b,
        ManagedChildProcessSpec::new(RuntimeProcessKind::Tool, "bad-b"),
    )
    .await
    .unwrap();
    let observed_overlap = os_process_exists(child_a.pid()) && os_process_exists(child_b.pid());
    child_b.kill().await.unwrap();
    child_a.kill().await.unwrap();
    drop(permit_b);
    assert!(
        observed_overlap,
        "the OS oracle must detect the injected early release"
    );
}

#[tokio::test]
async fn aborted_wait_reaps_child_before_reusing_quota() {
    let (runtime, _events) = TokioAgentRuntime::from_build_request(
        TokioAgentRuntimeBuildRequest::runtime_root(4)
            .with_process_limit(NonZeroUsize::new(1).unwrap()),
    );
    let mut command = AsyncCommand::new("/bin/sleep");
    command.arg("30").stdout(std::process::Stdio::piped());
    let child = AsyncManagedChildProcess::spawn_with_spec(
        &runtime.context(),
        command,
        ManagedChildProcessSpec::new(RuntimeProcessKind::Tool, "abort-a"),
    )
    .await
    .unwrap();
    let pid = child.pid();
    let task = tokio::spawn(child.wait_with_output());
    tokio::task::yield_now().await;
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    let mut command_b = AsyncCommand::new("/bin/sleep");
    command_b.arg("30");
    let child_b = timeout(
        Duration::from_secs(5),
        AsyncManagedChildProcess::spawn_with_spec(
            &runtime.context(),
            command_b,
            ManagedChildProcessSpec::new(RuntimeProcessKind::Tool, "abort-b"),
        ),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(
        !os_process_exists(pid),
        "A must be reaped before B is admitted"
    );
    child_b.kill().await.unwrap();
}

#[tokio::test]
async fn cancellation_while_waiting_for_quota_does_not_spawn() {
    let (runtime, _events) = TokioAgentRuntime::from_build_request(
        TokioAgentRuntimeBuildRequest::runtime_root(4)
            .with_process_limit(NonZeroUsize::new(1).unwrap()),
    );
    let mut command_a = AsyncCommand::new("/bin/sleep");
    command_a.arg("30");
    let child_a = AsyncManagedChildProcess::spawn_with_spec(
        &runtime.context(),
        command_a,
        ManagedChildProcessSpec::new(RuntimeProcessKind::Tool, "waiting-a"),
    )
    .await
    .unwrap();
    let context_b = runtime.context().child_context();
    let cancellation = context_b.cancellation_token();
    let task_b = tokio::spawn(async move {
        let mut command = AsyncCommand::new("/bin/sleep");
        command.arg("30");
        AsyncManagedChildProcess::spawn_with_spec(
            &context_b,
            command,
            ManagedChildProcessSpec::new(RuntimeProcessKind::Tool, "waiting-b"),
        )
        .await
    });
    tokio::task::yield_now().await;
    cancellation.cancel();
    let error = timeout(Duration::from_secs(5), task_b)
        .await
        .unwrap()
        .unwrap()
        .unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::Interrupted);
    assert_eq!(runtime.process_registry().active_processes().len(), 1);
    child_a.kill().await.unwrap();
}

#[tokio::test]
async fn failed_spawn_returns_the_unused_permit() {
    let (runtime, _events) = TokioAgentRuntime::from_build_request(
        TokioAgentRuntimeBuildRequest::runtime_root(4)
            .with_process_limit(NonZeroUsize::new(1).unwrap()),
    );
    let result = AsyncManagedChildProcess::spawn_with_spec(
        &runtime.context(),
        AsyncCommand::new("/this-program-does-not-exist-marlin-quota-test"),
        ManagedChildProcessSpec::new(RuntimeProcessKind::Tool, "missing"),
    )
    .await;
    assert!(result.is_err());
    let mut command = AsyncCommand::new("/bin/sleep");
    command.arg("30");
    let child = timeout(
        Duration::from_secs(5),
        AsyncManagedChildProcess::spawn_with_spec(
            &runtime.context(),
            command,
            ManagedChildProcessSpec::new(RuntimeProcessKind::Tool, "after-failure"),
        ),
    )
    .await
    .unwrap()
    .unwrap();
    child.kill().await.unwrap();
}
