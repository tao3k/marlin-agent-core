// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! MAR-PROC-R01/R02: replay Quint ITF states through managed OS processes.

use marlin_agent_runtime::{
    AsyncManagedChildProcess, ManagedChildProcessSpec, TokioAgentRuntime,
    TokioAgentRuntimeBuildRequest, process::RuntimeProcessKind,
};
use serde::Deserialize;
use std::{fs, num::NonZeroUsize, path::Path, process::Command, time::Duration};
use tokio::{process::Command as AsyncCommand, sync::oneshot, task::JoinHandle, time::timeout};

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct State {
    owner: String,
    a: String,
    b: String,
    task_a: String,
    queued_b: bool,
}
#[derive(Deserialize)]
struct Trace {
    states: Vec<State>,
}

fn alive(pid: u32) -> bool {
    Command::new("/bin/kill")
        .args(["-0", &pid.to_string()])
        .output()
        .unwrap()
        .status
        .success()
}

async fn start(
    context: marlin_agent_runtime::RuntimeContext,
    name: &str,
) -> AsyncManagedChildProcess {
    let mut command = AsyncCommand::new("/bin/sleep");
    command.arg("30");
    AsyncManagedChildProcess::spawn_with_spec(
        &context,
        command,
        ManagedChildProcessSpec::new(RuntimeProcessKind::Tool, name),
    )
    .await
    .unwrap()
}

#[tokio::test]
#[ignore = "requires freshly generated Quint ITF traces"]
async fn generated_quint_traces_match_os_processes() {
    let directory =
        std::env::var_os("MARLIN_QUOTA_TRACE_DIR").expect("MARLIN_QUOTA_TRACE_DIR is required");
    let mut paths: Vec<_> = fs::read_dir(Path::new(&directory))
        .unwrap()
        .map(|p| p.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .collect();
    paths.sort();
    assert!(paths.len() >= 3, "expected generated Quint traces");
    let mut saw_queued_cancel = false;
    let mut saw_b_first = false;
    for path in &paths {
        let trace: Trace = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
        assert!(!trace.states.is_empty(), "empty trace: {}", path.display());
        let (runtime, _events) = TokioAgentRuntime::from_build_request(
            TokioAgentRuntimeBuildRequest::runtime_root(4)
                .with_process_limit(NonZeroUsize::new(1).unwrap()),
        );
        let mut a_pid = None;
        let mut b_pid = None;
        let mut cancel_a = None;
        let mut task_a: Option<JoinHandle<AsyncManagedChildProcess>> = None;
        let mut draining_a: Option<AsyncManagedChildProcess> = None;
        let mut b_child: Option<AsyncManagedChildProcess> = None;
        let mut waiting_b: Option<JoinHandle<AsyncManagedChildProcess>> = None;
        let mut previous: Option<&State> = None;
        for (index, expected) in trace.states.iter().enumerate() {
            if let Some(prev) = previous {
                if prev.a == "pending" && expected.a == "running" {
                    let child = start(runtime.context().child_context(), "trace-a").await;
                    a_pid = Some(child.pid());
                    let (tx, rx) = oneshot::channel();
                    cancel_a = Some(tx);
                    task_a = Some(tokio::spawn(async move {
                        rx.await.unwrap();
                        child.request_cleanup();
                        child
                    }));
                }
                if prev.a == "running" && expected.a == "draining" {
                    cancel_a.take().unwrap().send(()).unwrap();
                    draining_a = Some(task_a.take().unwrap().await.unwrap());
                }
                if !prev.queued_b && expected.queued_b {
                    let (tx, rx) = oneshot::channel();
                    let context = runtime.context().child_context();
                    waiting_b = Some(tokio::spawn(async move {
                        tx.send(()).unwrap();
                        start(context, "trace-b").await
                    }));
                    rx.await.unwrap();
                }
                if prev.a == "draining" && expected.a == "exited" {
                    draining_a.take().unwrap().kill().await.unwrap();
                }
                if prev.b == "pending" && expected.b == "running" {
                    let child = if let Some(task) = waiting_b.take() {
                        timeout(Duration::from_secs(5), task)
                            .await
                            .unwrap()
                            .unwrap()
                    } else {
                        start(runtime.context().child_context(), "trace-b").await
                    };
                    b_pid = Some(child.pid());
                    b_child = Some(child);
                }
                if prev.b == "running" && expected.b == "exited" {
                    b_child.take().unwrap().kill().await.unwrap();
                }
            } else {
                assert_eq!(
                    (
                        expected.a.as_str(),
                        expected.b.as_str(),
                        expected.owner.as_str(),
                        expected.task_a.as_str(),
                        expected.queued_b
                    ),
                    ("pending", "pending", "none", "pending", false)
                );
            }
            let a_live = a_pid.is_some_and(alive);
            let b_live = b_pid.is_some_and(alive);
            assert_eq!(
                a_live,
                expected.a == "running" || expected.a == "draining",
                "{} step {index}: A OS state",
                path.display()
            );
            assert_eq!(
                b_live,
                expected.b == "running",
                "{} step {index}: B OS state",
                path.display()
            );
            let owner = if a_live {
                "a"
            } else if b_live {
                "b"
            } else {
                "none"
            };
            assert!(
                !(a_live && b_live),
                "{} step {index}: OS process overlap",
                path.display()
            );
            assert_eq!(
                owner,
                expected.owner,
                "{} step {index}: observed owner",
                path.display()
            );
            let actual_task = if task_a.is_some() {
                "running"
            } else if draining_a.is_some() || a_pid.is_some() {
                "cancelled"
            } else {
                "pending"
            };
            assert_eq!(
                actual_task,
                expected.task_a,
                "{} step {index}: task result",
                path.display()
            );
            assert_eq!(
                waiting_b.is_some(),
                expected.queued_b,
                "{} step {index}: queued admission",
                path.display()
            );
            if let Some(task) = waiting_b.as_mut() {
                assert!(
                    timeout(Duration::from_millis(50), task).await.is_err(),
                    "{} step {index}: B admitted while A is alive",
                    path.display()
                );
                assert!(a_pid.is_some_and(alive), "A exited during admission probe");
            }
            saw_queued_cancel |= expected.queued_b && expected.task_a == "cancelled";
            saw_b_first |= expected.b == "running" && expected.a == "pending";
            previous = Some(expected);
        }
        if let Some(tx) = cancel_a.take() {
            tx.send(()).unwrap();
            draining_a = Some(task_a.take().unwrap().await.unwrap());
        }
        if let Some(child) = draining_a {
            child.kill().await.unwrap();
        }
        if let Some(task) = waiting_b {
            task.abort();
            let _ = task.await;
        }
        if let Some(child) = b_child {
            child.kill().await.unwrap();
        }
    }
    assert!(
        saw_queued_cancel && saw_b_first,
        "generated trace corpus lacks required interleavings"
    );
    eprintln!("replayed {} Quint traces with OS observations", paths.len());
}
