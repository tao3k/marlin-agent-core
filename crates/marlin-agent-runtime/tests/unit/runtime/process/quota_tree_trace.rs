// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! MAR-PROC-R03/R04: replay process-tree ITF with OS PIDs and output EOF.

use marlin_agent_runtime::{
    AsyncManagedChildProcess, ManagedChildProcessSpec, TokioAgentRuntime,
    TokioAgentRuntimeBuildRequest, process::RuntimeProcessKind,
};
use serde::Deserialize;
use std::{
    fs,
    num::NonZeroUsize,
    path::Path,
    process::{Command, Stdio},
    time::Duration,
};
use tokio::{
    io::AsyncWriteExt,
    process::Command as AsyncCommand,
    time::{sleep, timeout},
};

#[derive(Deserialize)]
struct Trace {
    states: Vec<State>,
}
#[derive(Deserialize)]
struct State {
    leader: String,
    descendant: String,
    pipe: String,
    task: String,
    owner: String,
    queued: bool,
    b: String,
}
fn alive(pid: u32) -> bool {
    Command::new("/bin/kill")
        .args(["-0", &pid.to_string()])
        .output()
        .unwrap()
        .status
        .success()
}

#[tokio::test]
#[ignore = "requires generated Quint process-tree traces"]
async fn generated_tree_traces_match_os_and_pipe_eof() {
    let path = std::env::var_os("MARLIN_TREE_TRACE_DIR").expect("MARLIN_TREE_TRACE_DIR required");
    let mut files: Vec<_> = fs::read_dir(Path::new(&path))
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    files.sort();
    assert_eq!(files.len(), 2);
    for file in files {
        let trace: Trace = serde_json::from_slice(&fs::read(file).unwrap()).unwrap();
        let temp = tempfile::tempdir().unwrap();
        let marker = temp.path().join("pid");
        let (runtime, _events) = TokioAgentRuntime::from_build_request(
            TokioAgentRuntimeBuildRequest::runtime_root(4)
                .with_process_limit(NonZeroUsize::new(1).unwrap()),
        );
        let mut leader = None;
        let mut descendant = None;
        let mut stdin = None;
        let mut child = None;
        let mut waiter = None;
        let mut b_task = None;
        let mut b_child: Option<AsyncManagedChildProcess> = None;
        let mut pipe_open = false;
        let mut cancelled = false;
        let mut previous: Option<&State> = None;
        for (step, expected) in trace.states.iter().enumerate() {
            if let Some(prev) = previous {
                if prev.leader == "pending" && expected.leader == "running" {
                    let mut command = AsyncCommand::new("/bin/sh");
                    command
                        .arg("-c")
                        .arg("printf marker; sleep 30 & echo $! > \"$1\"; read line")
                        .arg("fixture")
                        .arg(&marker)
                        .stdin(Stdio::piped())
                        .stdout(Stdio::piped())
                        .stderr(Stdio::piped());
                    let mut process = AsyncManagedChildProcess::spawn_with_spec(
                        &runtime.context(),
                        command,
                        ManagedChildProcessSpec::new(RuntimeProcessKind::Tool, "trace-tree-a"),
                    )
                    .await
                    .unwrap();
                    leader = Some(process.pid());
                    stdin = process.take_stdin();
                    child = Some(process);
                    descendant = Some(
                        timeout(Duration::from_secs(5), async {
                            loop {
                                if let Ok(value) = fs::read_to_string(&marker)
                                    && let Ok(pid) = value.trim().parse::<u32>()
                                {
                                    break pid;
                                }
                                sleep(Duration::from_millis(10)).await;
                            }
                        })
                        .await
                        .unwrap(),
                    );
                    pipe_open = true;
                }
                if prev.task == "running" && expected.task == "cancelled" {
                    let process = child.take().unwrap();
                    // The task result is cancelled while the cleanup owns the process.
                    let cleanup = tokio::spawn(async move {
                        process.request_cleanup();
                        process.wait_with_output().await
                    });
                    waiter = Some(cleanup);
                    cancelled = true;
                }
                if !prev.queued && expected.queued {
                    let context = runtime.context().child_context();
                    let (tx, rx) = tokio::sync::oneshot::channel();
                    b_task = Some(tokio::spawn(async move {
                        let mut command = AsyncCommand::new("/bin/sleep");
                        command.arg("30");
                        tx.send(()).unwrap();
                        AsyncManagedChildProcess::spawn_with_spec(
                            &context,
                            command,
                            ManagedChildProcessSpec::new(RuntimeProcessKind::Tool, "trace-tree-b"),
                        )
                        .await
                        .unwrap()
                    }));
                    rx.await.unwrap();
                }
                if prev.leader == "running" && expected.leader == "exited" {
                    let mut writer = stdin.take().unwrap();
                    writer.write_all(b"done\n").await.unwrap();
                    drop(writer);
                    timeout(Duration::from_secs(5), async {
                        while alive(leader.unwrap()) {
                            sleep(Duration::from_millis(10)).await;
                        }
                    })
                    .await
                    .unwrap();
                }
                if prev.descendant == "running" && expected.descendant == "exited" {
                    assert!(
                        Command::new("/bin/kill")
                            .args(["-9", &descendant.unwrap().to_string()])
                            .status()
                            .unwrap()
                            .success()
                    );
                    let output = timeout(Duration::from_secs(5), waiter.take().unwrap())
                        .await
                        .unwrap()
                        .unwrap()
                        .unwrap();
                    assert_eq!(
                        output.stdout, b"marker",
                        "pipe did not reach EOF with expected bytes"
                    );
                    pipe_open = false;
                }
                if prev.b == "pending" && expected.b == "running" {
                    b_child = Some(
                        timeout(Duration::from_secs(5), b_task.take().unwrap())
                            .await
                            .unwrap()
                            .unwrap(),
                    );
                }
                if prev.b == "running" && expected.b == "exited" {
                    b_child.take().unwrap().kill().await.unwrap();
                }
            }
            let leader_live = leader.is_some_and(alive);
            let descendant_live = descendant.is_some_and(alive);
            assert_eq!(
                leader_live,
                expected.leader == "running",
                "step {step} leader"
            );
            assert_eq!(
                descendant_live,
                expected.descendant == "running",
                "step {step} descendant"
            );
            assert_eq!(pipe_open, expected.pipe == "open", "step {step} pipe");
            assert_eq!(cancelled, expected.task == "cancelled", "step {step} task");
            let owner = if leader_live || descendant_live || pipe_open {
                "a"
            } else if b_child.is_some() {
                "b"
            } else {
                "none"
            };
            assert_eq!(owner, expected.owner, "step {step} owner");
            assert_eq!(b_task.is_some(), expected.queued, "step {step} queued");
            if let Some(task) = b_task.as_mut() {
                assert!(
                    timeout(Duration::from_millis(50), task).await.is_err(),
                    "B admitted with live descendant or pipe"
                );
            }
            previous = Some(expected);
        }
    }
}
