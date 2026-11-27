// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! SM19 adversarial test: cancelling `HookDispatcher::dispatch` while async
//! hook tasks are in flight MUST abort every spawned child task instead of
//! detaching orphan background tasks.

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use marlin_agent_hooks::{HookDispatcher, HookInvocation, HookRegistration, HookRegistry};
use marlin_agent_protocol::{
    HookEventName, HookExecutionMode, HookHandlerType, HookRunSummary,
};
use marlin_agent_runtime::{HookRuntime, RuntimeContext, RuntimeFuture, TokioAgentRuntime};

struct SlowCancellableHook {
    started: Arc<tokio::sync::Notify>,
    completed_after_cancel: Arc<AtomicBool>,
}

impl HookRuntime for SlowCancellableHook {
    type Request = HookInvocation;
    type Output = HookRunSummary;

    fn run_hook(
        &self,
        request: Self::Request,
        _context: RuntimeContext,
    ) -> RuntimeFuture<Self::Output> {
        let started = Arc::clone(&self.started);
        let completed = Arc::clone(&self.completed_after_cancel);
        Box::pin(async move {
            started.notify_one();
            tokio::time::sleep(Duration::from_millis(80)).await;
            completed.store(true, Ordering::SeqCst);
            HookRunSummary::running("slow-async", request.event_name, HookHandlerType::Command)
                .completed()
        })
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sm19_cancelling_hook_dispatch_aborts_in_flight_async_hooks() {
    let started = Arc::new(tokio::sync::Notify::new());
    let completed_after_cancel = Arc::new(AtomicBool::new(false));
    let hook_a = Arc::new(SlowCancellableHook {
        started: Arc::clone(&started),
        completed_after_cancel: Arc::clone(&completed_after_cancel),
    });
    let hook_b = Arc::new(SlowCancellableHook {
        started: Arc::new(tokio::sync::Notify::new()),
        completed_after_cancel: Arc::clone(&completed_after_cancel),
    });

    let mut registry = HookRegistry::new();
    registry.register(
        HookRegistration::new(
            "slow-async-hook-a",
            HookEventName::PreToolUse,
            HookHandlerType::Command,
            hook_a,
        )
        .with_execution_mode(HookExecutionMode::Async),
    );
    registry.register(
        HookRegistration::new(
            "slow-async-hook-b",
            HookEventName::PreToolUse,
            HookHandlerType::Command,
            hook_b,
        )
        .with_execution_mode(HookExecutionMode::Async),
    );
    let dispatcher = HookDispatcher::new(registry);
    let (runtime, _events) = TokioAgentRuntime::new(16);

    // Use `Box::pin` so `drop(dispatch_fut)` drops the future itself (not just
    // a stack-pinned `&mut` handle), exercising mid-join abort AND queued-task
    // abort simultaneously.
    let mut dispatch_fut = Box::pin(
        dispatcher.dispatch(&runtime, HookInvocation::new(HookEventName::PreToolUse)),
    );

    // Poll until the async hook tasks are actively running, then drop
    // `dispatch_fut` to simulate caller timeout / Ctrl-C cancellation.
    tokio::select! {
        _ = &mut dispatch_fut => panic!("hook should still be sleeping"),
        _ = started.notified() => {}
    }
    drop(dispatch_fut);

    // Wait longer than the hook's 80ms sleep; an abort-on-drop dispatcher MUST
    // have cancelled the task before it can set `completed_after_cancel`.
    tokio::time::sleep(Duration::from_millis(150)).await;
    assert!(
        !completed_after_cancel.load(Ordering::SeqCst),
        "orphaned async hook task kept running after HookDispatcher::dispatch was cancelled!"
    );
}
