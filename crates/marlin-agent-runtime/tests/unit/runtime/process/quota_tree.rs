// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use marlin_agent_runtime::{
    AsyncManagedChildProcess, ManagedChildProcessSpec, RuntimeAdmissionError, TokioAgentRuntime,
    TokioAgentRuntimeBuildRequest, process::RuntimeProcessKind,
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

fn os_alive(pid: u32) -> bool {
    Command::new("/bin/kill")
        .args(["-0", &pid.to_string()])
        .output()
        .unwrap()
        .status
        .success()
}

async fn exercise_descendant_holds_output_pipe(capture_output: bool) {
    let temp = tempfile::tempdir().unwrap();
    let marker = temp.path().join("descendant.pid");
    let (runtime, _events) = TokioAgentRuntime::from_build_request(
        TokioAgentRuntimeBuildRequest::runtime_root(4)
            .with_process_limit(NonZeroUsize::new(1).unwrap()),
    );
    let mut command = AsyncCommand::new("/bin/sh");
    command
        .arg("-c")
        .arg("printf pipe-open; sleep 30 & echo $! > \"$1\"; exit 0")
        .arg("fixture")
        .arg(&marker)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let child = AsyncManagedChildProcess::spawn_with_spec(
        &runtime.context(),
        command,
        ManagedChildProcessSpec::new(RuntimeProcessKind::Tool, "tree-a"),
    )
    .await
    .unwrap();
    let leader = child.pid();
    let descendant: u32 = timeout(Duration::from_secs(5), async {
        loop {
            if let Ok(text) = std::fs::read_to_string(&marker)
                && let Ok(pid) = text.trim().parse()
            {
                break pid;
            }
            sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert!(os_alive(descendant));
    let waiter = tokio::spawn(async move {
        if capture_output {
            child.wait_with_output().await.map(|out| Some(out.stdout))
        } else {
            child.wait().await.map(|_| None)
        }
    });
    timeout(Duration::from_secs(5), async {
        while os_alive(leader) {
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
            ManagedChildProcessSpec::new(RuntimeProcessKind::Tool, "tree-b"),
        )
        .await
        .unwrap()
    });
    let early = timeout(Duration::from_millis(200), &mut b).await;
    let blocked = early.is_err();
    assert!(
        os_alive(descendant),
        "descendant should still hold the inherited pipe"
    );
    let _ = Command::new("/bin/kill")
        .args(["-9", &descendant.to_string()])
        .status();
    let output = timeout(Duration::from_secs(5), waiter)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    if capture_output {
        assert_eq!(output.unwrap(), b"pipe-open");
    }
    let child_b = match early {
        Ok(result) => result.unwrap(),
        Err(_) => timeout(Duration::from_secs(5), b).await.unwrap().unwrap(),
    };
    child_b.kill().await.unwrap();
    assert!(blocked, "quota was returned while the descendant was alive");
}

#[tokio::test]
async fn quota_survives_leader_exit_while_descendant_holds_output_pipe() {
    exercise_descendant_holds_output_pipe(false).await;
}

#[tokio::test]
async fn output_wait_reaches_eof_before_quota_reuse() {
    exercise_descendant_holds_output_pipe(true).await;
}

#[tokio::test]
async fn operation_stays_reserved_until_group_exits_without_quota() {
    let directory = tempfile::tempdir().unwrap();
    let (runtime, _) = TokioAgentRuntime::new(4);
    let context = runtime.context();
    let spec = || ManagedChildProcessSpec::new(RuntimeProcessKind::Tool, "same-operation");
    let marker = directory.path().join("descendant.pid");
    let mut command = AsyncCommand::new("/bin/sh");
    command.args([
        "-c",
        r#"(while [ -d "$1" ] && [ ! -e "$1/release" ]; do /bin/sleep 0.03; done) &
           echo $! > "$1/descendant.pid""#,
        "fixture",
    ]);
    command.arg(directory.path());
    let child = AsyncManagedChildProcess::spawn_with_spec(&context, command, spec())
        .await
        .unwrap();
    let leader = child.pid();
    let waiter = tokio::spawn(child.wait());
    let descendant = timeout(Duration::from_secs(5), async {
        loop {
            if !os_alive(leader)
                && let Ok(text) = std::fs::read_to_string(&marker)
                && let Ok(pid) = text.trim().parse::<u32>()
            {
                return pid;
            }
            sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();

    let attempt = AsyncManagedChildProcess::spawn_with_spec(
        &context,
        AsyncCommand::new("/usr/bin/true"),
        spec(),
    )
    .await;
    assert!(os_alive(descendant));
    assert_eq!(runtime.task_tracker().active_resource_count(), 1);
    assert_eq!(
        attempt
            .unwrap_err()
            .get_ref()
            .and_then(|error| error.downcast_ref::<RuntimeAdmissionError>()),
        Some(&RuntimeAdmissionError::OperationActive),
    );

    std::fs::write(directory.path().join("release"), []).unwrap();
    assert!(
        timeout(Duration::from_secs(5), waiter)
            .await
            .unwrap()
            .unwrap()
            .unwrap()
            .success()
    );
    assert!(!os_alive(descendant));
    assert_eq!(runtime.task_tracker().active_resource_count(), 0);
    let replacement = AsyncManagedChildProcess::spawn_with_spec(
        &context,
        AsyncCommand::new("/usr/bin/true"),
        spec(),
    )
    .await
    .unwrap();
    assert!(replacement.wait().await.unwrap().success());
}

#[tokio::test]
async fn killing_managed_group_waits_for_descendant_before_reuse() {
    let temp = tempfile::tempdir().unwrap();
    let marker = temp.path().join("descendant.pid");
    let (runtime, _events) = TokioAgentRuntime::from_build_request(
        TokioAgentRuntimeBuildRequest::runtime_root(4)
            .with_process_limit(NonZeroUsize::new(1).unwrap()),
    );
    let mut command = AsyncCommand::new("/bin/sh");
    command
        .arg("-c")
        .arg("sleep 30 & echo $! > \"$1\"; wait")
        .arg("fixture")
        .arg(&marker)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let child = AsyncManagedChildProcess::spawn_with_spec(
        &runtime.context(),
        command,
        ManagedChildProcessSpec::new(RuntimeProcessKind::Tool, "group-kill-a"),
    )
    .await
    .unwrap();
    let leader = child.pid();
    let descendant: u32 = timeout(Duration::from_secs(5), async {
        loop {
            if let Ok(text) = std::fs::read_to_string(&marker)
                && let Ok(pid) = text.trim().parse()
            {
                break pid;
            }
            sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert!(os_alive(leader) && os_alive(descendant));
    timeout(Duration::from_secs(5), child.kill())
        .await
        .unwrap()
        .unwrap();
    assert!(
        !os_alive(leader) && !os_alive(descendant),
        "group members must exit before permit reuse"
    );
    let mut command = AsyncCommand::new("/bin/sleep");
    command.arg("30");
    let next = timeout(
        Duration::from_secs(5),
        AsyncManagedChildProcess::spawn_with_spec(
            &runtime.context(),
            command,
            ManagedChildProcessSpec::new(RuntimeProcessKind::Tool, "group-kill-b"),
        ),
    )
    .await
    .unwrap()
    .unwrap();
    next.kill().await.unwrap();
}

#[tokio::test]
async fn process_group_cleanup_closes_child_side_of_stdin_pipe() {
    use tokio::io::AsyncWriteExt;
    let (runtime, _events) = TokioAgentRuntime::from_build_request(
        TokioAgentRuntimeBuildRequest::runtime_root(4)
            .with_process_limit(NonZeroUsize::new(1).unwrap()),
    );
    let mut command = AsyncCommand::new("/bin/sleep");
    command.arg("30").stdin(Stdio::piped());
    let mut child = AsyncManagedChildProcess::spawn_with_spec(
        &runtime.context(),
        command,
        ManagedChildProcessSpec::new(RuntimeProcessKind::Tool, "pipe-cleanup"),
    )
    .await
    .unwrap();
    let mut writer = child.take_stdin().unwrap();
    child.kill().await.unwrap();
    let error = timeout(Duration::from_secs(5), async {
        loop {
            let result = match writer.write_all(b"probe").await {
                Ok(()) => writer.flush().await,
                Err(error) => Err(error),
            };
            if let Err(error) = result {
                break error;
            }
            sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("pipe reader must close");
    assert_eq!(
        error.kind(),
        std::io::ErrorKind::BrokenPipe,
        "OS must report no remaining reader"
    );
}
