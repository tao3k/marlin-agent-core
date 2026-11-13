// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! MAR-PROC-R05/R06/R07: replay strict admission and owned pipe leases.
use marlin_agent_runtime::{
    AsyncManagedChildProcess, ManagedChildProcessSpec, RuntimeProcessContainmentRequirement,
    TokioAgentRuntime, TokioAgentRuntimeBuildRequest, process::RuntimeProcessKind,
};
use serde::Deserialize;
use std::{
    fs,
    num::NonZeroUsize,
    path::Path,
    process::{Command, Stdio},
    time::Duration,
};
use tokio::{process::Command as AsyncCommand, time::timeout};

#[derive(Deserialize)]
struct Trace {
    states: Vec<State>,
}
#[derive(Deserialize)]
struct State {
    group: String,
    task: String,
    pipe: String,
    owner: String,
    queued: bool,
    b: String,
    strict: String,
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
#[ignore = "requires generated Quint process-lease traces"]
async fn generated_lease_traces_match_process_and_pipe_ownership() {
    let directory =
        std::env::var_os("MARLIN_LEASE_TRACE_DIR").expect("MARLIN_LEASE_TRACE_DIR required");
    let mut files: Vec<_> = fs::read_dir(Path::new(&directory))
        .unwrap()
        .map(|p| p.unwrap().path())
        .collect();
    files.sort();
    assert_eq!(files.len(), 2);
    let mut observed_strict = false;
    for file in files {
        let trace: Trace = serde_json::from_slice(&fs::read(file).unwrap()).unwrap();
        let temp = tempfile::tempdir().unwrap();
        let marker = temp.path().join("must-not-spawn");
        let (runtime, _events) = TokioAgentRuntime::from_build_request(
            TokioAgentRuntimeBuildRequest::runtime_root(4)
                .with_process_limit(NonZeroUsize::new(1).unwrap()),
        );
        let mut a: Option<AsyncManagedChildProcess> = None;
        let mut pid = None;
        let mut pipe = None;
        let mut pipe_was_taken = false;
        let mut cancelled = false;
        let mut strict_rejected = false;
        let mut b_task = None;
        let mut b: Option<AsyncManagedChildProcess> = None;
        let mut previous: Option<&State> = None;
        for (step, expected) in trace.states.iter().enumerate() {
            if let Some(prev) = previous {
                if prev.strict == "ready" && expected.strict == "rejected" {
                    let mut command = AsyncCommand::new("/bin/sh");
                    command
                        .arg("-c")
                        .arg("touch \"$1\"")
                        .arg("fixture")
                        .arg(&marker);
                    let result = AsyncManagedChildProcess::spawn_with_spec(
                        &runtime.context(),
                        command,
                        ManagedChildProcessSpec::new(RuntimeProcessKind::Tool, "strict-replay")
                            .with_containment(
                                RuntimeProcessContainmentRequirement::EnforcedDescendantTree,
                            ),
                    )
                    .await;
                    assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::Unsupported);
                    assert!(!marker.exists());
                    strict_rejected = true;
                    observed_strict = true;
                }
                if prev.group == "pending" && expected.group == "running" {
                    let mut command = AsyncCommand::new("/bin/sleep");
                    command.arg("30").stdin(Stdio::piped());
                    let process = AsyncManagedChildProcess::spawn_with_spec(
                        &runtime.context(),
                        command,
                        ManagedChildProcessSpec::new(RuntimeProcessKind::Tool, "lease-a"),
                    )
                    .await
                    .unwrap();
                    pid = Some(process.pid());
                    a = Some(process);
                }
                if prev.pipe == "none" && expected.pipe == "held" {
                    pipe = a.as_mut().unwrap().take_stdin();
                    assert!(pipe.is_some());
                    pipe_was_taken = true;
                }
                if prev.task == "running" && expected.task == "cancelled" {
                    let process = a.take().unwrap();
                    // The task returns while the cleanup path retains the child.
                    a = Some(
                        tokio::spawn(async move {
                            process.request_cleanup();
                            process
                        })
                        .await
                        .unwrap(),
                    );
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
                            ManagedChildProcessSpec::new(RuntimeProcessKind::Tool, "lease-b"),
                        )
                        .await
                        .unwrap()
                    }));
                    rx.await.unwrap();
                }
                if prev.pipe == "held" && expected.pipe == "closed" {
                    drop(pipe.take());
                }
                if prev.group == "running" && expected.group == "exited" {
                    a.take().unwrap().kill().await.unwrap();
                }
                if prev.b == "pending" && expected.b == "running" {
                    b = Some(
                        timeout(Duration::from_secs(5), b_task.take().unwrap())
                            .await
                            .unwrap()
                            .unwrap(),
                    );
                }
                if prev.b == "running" && expected.b == "exited" {
                    b.take().unwrap().kill().await.unwrap();
                }
            }
            let group_alive = pid.is_some_and(alive);
            assert_eq!(
                group_alive,
                expected.group == "running",
                "step {step}: OS process"
            );
            let pipe_state = if pipe.is_some() {
                "held"
            } else if pipe_was_taken {
                "closed"
            } else {
                "none"
            };
            assert_eq!(pipe_state, expected.pipe, "step {step}: pipe lease");
            let task_state = if cancelled {
                "cancelled"
            } else if pid.is_some() {
                "running"
            } else {
                "pending"
            };
            assert_eq!(task_state, expected.task, "step {step}: task");
            let owner = if group_alive || pipe.is_some() {
                "a"
            } else if b.is_some() {
                "b"
            } else {
                "none"
            };
            assert_eq!(owner, expected.owner, "step {step}: observed owner");
            assert_eq!(b_task.is_some(), expected.queued, "step {step}: queue");
            assert_eq!(
                strict_rejected,
                expected.strict == "rejected",
                "step {step}: strict admission"
            );
            if let Some(waiting) = b_task.as_mut() {
                assert!(
                    timeout(Duration::from_millis(50), waiting).await.is_err(),
                    "B admitted while process or caller pipe is held"
                );
            }
            previous = Some(expected);
        }
    }
    assert!(
        observed_strict,
        "strict refusal must occur in the generated corpus"
    );
}
