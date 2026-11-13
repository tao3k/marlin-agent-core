// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use marlin_agent_runtime::{
    AsyncManagedChildProcess, ManagedChildProcessSpec, RuntimeProcessContainmentRequirement,
    TokioAgentRuntime, TokioAgentRuntimeBuildRequest, process::RuntimeProcessKind,
};
use std::{
    num::NonZeroUsize,
    process::{Command, Stdio},
    time::Duration,
};
use tokio::{
    process::Command as AsyncCommand,
    time::{sleep, timeout},
};

fn runtime() -> TokioAgentRuntime {
    TokioAgentRuntime::from_build_request(
        TokioAgentRuntimeBuildRequest::runtime_root(4)
            .with_process_limit(NonZeroUsize::new(1).unwrap()),
    )
    .0
}
fn exists(pid: u32) -> bool {
    Command::new("/bin/kill")
        .args(["-0", &pid.to_string()])
        .output()
        .unwrap()
        .status
        .success()
}
struct Cleanup(u32);
impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = Command::new("/bin/kill")
            .args(["-9", &self.0.to_string()])
            .output();
    }
}

#[tokio::test]
async fn caller_held_stdin_retains_quota_after_process_exit() {
    let runtime = runtime();
    let mut command = AsyncCommand::new("/bin/sleep");
    command.arg("30").stdin(Stdio::piped());
    let mut a = AsyncManagedChildProcess::spawn_with_spec(
        &runtime.context(),
        command,
        ManagedChildProcessSpec::new(RuntimeProcessKind::Tool, "pipe-lease-a"),
    )
    .await
    .unwrap();
    let pid = a.pid();
    let stdin = a.take_stdin().unwrap();
    a.kill().await.unwrap();
    assert!(!exists(pid));
    let context = runtime.context().child_context();
    let (tx, rx) = tokio::sync::oneshot::channel();
    let mut b = tokio::spawn(async move {
        let mut command = AsyncCommand::new("/bin/sleep");
        command.arg("30");
        tx.send(()).unwrap();
        AsyncManagedChildProcess::spawn_with_spec(
            &context,
            command,
            ManagedChildProcessSpec::new(RuntimeProcessKind::Tool, "pipe-lease-b"),
        )
        .await
        .unwrap()
    });
    rx.await.unwrap();
    assert!(
        timeout(Duration::from_millis(200), &mut b).await.is_err(),
        "B admitted with a caller-held stdin pipe"
    );
    drop(stdin);
    let b = timeout(Duration::from_secs(5), b).await.unwrap().unwrap();
    b.kill().await.unwrap();
}

#[tokio::test]
async fn cancellation_reaper_retains_quota_until_external_pipe_drops() {
    let runtime = runtime();
    let mut command = AsyncCommand::new("/bin/sleep");
    command
        .arg("30")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut a = AsyncManagedChildProcess::spawn_with_spec(
        &runtime.context(),
        command,
        ManagedChildProcessSpec::new(RuntimeProcessKind::Tool, "cancel-pipe-a"),
    )
    .await
    .unwrap();
    let pid = a.pid();
    let stdin = a.take_stdin().unwrap();
    let waiter = tokio::spawn(a.wait_with_output());
    tokio::task::yield_now().await;
    waiter.abort();
    assert!(waiter.await.unwrap_err().is_cancelled());
    timeout(Duration::from_secs(5), async {
        while exists(pid) {
            sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    let context = runtime.context().child_context();
    let mut b = tokio::spawn(async move {
        let mut command = AsyncCommand::new("/bin/sleep");
        command.arg("30");
        AsyncManagedChildProcess::spawn_with_spec(
            &context,
            command,
            ManagedChildProcessSpec::new(RuntimeProcessKind::Tool, "cancel-pipe-b"),
        )
        .await
        .unwrap()
    });
    assert!(
        timeout(Duration::from_millis(200), &mut b).await.is_err(),
        "B admitted before caller-held pipe was dropped"
    );
    drop(stdin);
    let b = timeout(Duration::from_secs(5), b).await.unwrap().unwrap();
    b.kill().await.unwrap();
}

#[tokio::test]
async fn enforced_tree_requirement_rejects_before_spawning() {
    let temp = tempfile::tempdir().unwrap();
    let marker = temp.path().join("spawned");
    let runtime = runtime();
    let mut command = AsyncCommand::new("/bin/sh");
    command
        .arg("-c")
        .arg("touch \"$1\"")
        .arg("fixture")
        .arg(&marker);
    let result = AsyncManagedChildProcess::spawn_with_spec(
        &runtime.context(),
        command,
        ManagedChildProcessSpec::new(RuntimeProcessKind::Tool, "strict-tree")
            .with_containment(RuntimeProcessContainmentRequirement::EnforcedDescendantTree),
    )
    .await;
    assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::Unsupported);
    assert!(!marker.exists());
    assert!(runtime.process_registry().active_processes().is_empty());
}

#[tokio::test]
async fn default_process_group_does_not_contain_setsid_descendant() {
    let temp = tempfile::tempdir().unwrap();
    let marker = temp.path().join("escaped-pid");
    let runtime = runtime();
    let mut command = AsyncCommand::new("/bin/sh");
    command.arg("-c").arg("python3 -c 'import os,sys,time; os.setsid(); open(sys.argv[1],\"w\").write(str(os.getpid())); time.sleep(30)' \"$1\" </dev/null >/dev/null 2>&1 & wait").arg("fixture").arg(&marker);
    let child = AsyncManagedChildProcess::spawn_with_spec(
        &runtime.context(),
        command,
        ManagedChildProcessSpec::new(RuntimeProcessKind::Tool, "breakaway"),
    )
    .await
    .unwrap();
    let escaped: u32 = timeout(Duration::from_secs(5), async {
        loop {
            if let Ok(value) = std::fs::read_to_string(&marker)
                && let Ok(pid) = value.parse()
            {
                break pid;
            }
            sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("escaped descendant did not start");
    let _cleanup = Cleanup(escaped);
    timeout(Duration::from_secs(5), child.kill())
        .await
        .unwrap()
        .unwrap();
    assert!(
        exists(escaped),
        "negative control must demonstrate the process-group escape"
    );
    let mut command = AsyncCommand::new("/bin/sleep");
    command.arg("30");
    let b = timeout(
        Duration::from_secs(5),
        AsyncManagedChildProcess::spawn_with_spec(
            &runtime.context(),
            command,
            ManagedChildProcessSpec::new(RuntimeProcessKind::Tool, "after-breakaway"),
        ),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(
        exists(escaped),
        "B was admitted while an escaped descendant remained alive"
    );
    b.kill().await.unwrap();
}
