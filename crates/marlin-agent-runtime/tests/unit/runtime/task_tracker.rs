use std::sync::{Arc, Barrier};

use marlin_agent_runtime::{
    ContextVisibility, RuntimeBlockingBridgePolicy, RuntimeBlockingBridgeStrategy,
    RuntimeFanoutJoinPolicy, RuntimeTaskOutcome, RuntimeTaskShutdownStatus,
    RuntimeTaskTrackerPolicy, SessionKind, SessionRuntimeSnapshot, TokioAgentRuntime,
    TokioAgentRuntimeBuildRequest, TokioRuntimeFlavor, TokioRuntimePolicy,
};

#[tokio::test]
async fn runtime_tracker_records_spawned_task_lifecycle() {
    let (runtime, _events) = TokioAgentRuntime::new(4);
    let (release_task, wait_for_release) = tokio::sync::oneshot::channel();

    let task = runtime.spawn(async move {
        wait_for_release
            .await
            .expect("release signal should arrive");
        7_u8
    });
    assert_eq!(runtime.task_tracker().active_task_count(), 1);

    release_task.send(()).expect("task release should send");
    assert_eq!(task.join().await.expect("task should finish"), 7);
    runtime.task_tracker().wait().await;
    assert_eq!(runtime.task_tracker().active_task_count(), 0);
}

#[tokio::test]
async fn runtime_tracker_records_blocking_task_lifecycle() {
    let (runtime, _events) = TokioAgentRuntime::new(4);
    let release_task = Arc::new(Barrier::new(2));
    let task_release = Arc::clone(&release_task);

    let task = runtime.spawn_blocking(move || {
        task_release.wait();
        11_u8
    });
    assert_eq!(runtime.task_tracker().active_task_count(), 1);

    release_task.wait();
    assert_eq!(task.join().await.expect("blocking task should finish"), 11);
    runtime.task_tracker().wait().await;
    assert_eq!(runtime.task_tracker().active_task_count(), 0);
}

#[tokio::test]
async fn runtime_shutdown_cancels_and_waits_for_tracked_tasks() {
    let (runtime, _events) = TokioAgentRuntime::new(4);
    let task = runtime.spawn_cancellable(async { std::future::pending::<()>().await });

    assert_eq!(runtime.task_tracker().active_task_count(), 1);
    let receipt = runtime
        .shutdown_tasks(&RuntimeTaskTrackerPolicy::cancel_on_shutdown(Some(1_000)))
        .await;

    assert_eq!(receipt.status(), &RuntimeTaskShutdownStatus::Completed);
    assert!(receipt.cancellation_requested());
    assert!(receipt.tracker_closed());
    assert_eq!(receipt.timeout_ms(), Some(1_000));
    assert_eq!(receipt.tracked_task_count_at_shutdown(), 1);
    assert_eq!(receipt.remaining_task_count(), 0);
    assert_eq!(
        task.join().await.expect("task should join"),
        RuntimeTaskOutcome::Cancelled
    );
}

#[tokio::test]
async fn runtime_shutdown_waits_without_cancelling_when_policy_requests_wait() {
    let (runtime, _events) = TokioAgentRuntime::new(4);
    let (release_task, wait_for_release) = tokio::sync::oneshot::channel();
    let task = runtime.spawn_cancellable(async move {
        wait_for_release
            .await
            .expect("release signal should arrive");
        13_u8
    });

    assert_eq!(runtime.task_tracker().active_task_count(), 1);
    let shutdown_runtime = runtime.clone();
    let shutdown = tokio::spawn(async move {
        let policy = RuntimeTaskTrackerPolicy::wait_on_shutdown(Some(1_000));
        shutdown_runtime.shutdown_tasks(&policy).await
    });

    for _ in 0..100 {
        if runtime.task_tracker().is_closed() {
            break;
        }
        tokio::task::yield_now().await;
    }
    assert!(runtime.task_tracker().is_closed());
    assert_eq!(runtime.task_tracker().active_task_count(), 1);
    release_task.send(()).expect("task release should send");

    let receipt = shutdown.await.expect("shutdown task should join");
    assert_eq!(receipt.status(), &RuntimeTaskShutdownStatus::Completed);
    assert!(!receipt.cancellation_requested());
    assert!(receipt.tracker_closed());
    assert_eq!(receipt.timeout_ms(), Some(1_000));
    assert_eq!(receipt.tracked_task_count_at_shutdown(), 1);
    assert_eq!(receipt.remaining_task_count(), 0);
    assert_eq!(
        task.join().await.expect("task should join"),
        RuntimeTaskOutcome::Completed(13)
    );
}

#[tokio::test]
async fn runtime_build_request_applies_session_task_tracker_policy() {
    let snapshot =
        SessionRuntimeSnapshot::new("runtime/requested", TokioRuntimePolicy::current_thread())
            .with_task_tracker(RuntimeTaskTrackerPolicy::wait_on_shutdown(Some(1_000)));
    let (runtime, _events) = TokioAgentRuntime::from_build_request(
        TokioAgentRuntimeBuildRequest::runtime_root(4).with_runtime_snapshot(snapshot),
    );
    let (release_task, wait_for_release) = tokio::sync::oneshot::channel();
    let task = runtime.spawn_cancellable(async move {
        wait_for_release
            .await
            .expect("release signal should arrive");
        17_u8
    });

    assert_eq!(
        runtime.runtime_snapshot().session_id().as_str(),
        "runtime.root"
    );
    assert_eq!(
        runtime.runtime_snapshot().tokio_policy().flavor(),
        &TokioRuntimeFlavor::CurrentThread
    );
    assert_eq!(runtime.task_tracker().active_task_count(), 1);
    let shutdown_runtime = runtime.clone();
    let shutdown = tokio::spawn(async move { shutdown_runtime.shutdown_session_tasks().await });

    for _ in 0..100 {
        if runtime.task_tracker().is_closed() {
            break;
        }
        tokio::task::yield_now().await;
    }
    assert!(runtime.task_tracker().is_closed());
    release_task.send(()).expect("task release should send");

    let receipt = shutdown.await.expect("shutdown task should join");
    assert_eq!(receipt.status(), &RuntimeTaskShutdownStatus::Completed);
    assert!(!receipt.cancellation_requested());
    assert_eq!(
        task.join().await.expect("task should join"),
        RuntimeTaskOutcome::Completed(17)
    );
}

#[test]
fn child_runtime_for_session_rebinds_runtime_snapshot_to_child_session() {
    let snapshot =
        SessionRuntimeSnapshot::new("runtime/requested", TokioRuntimePolicy::current_thread())
            .with_task_tracker(RuntimeTaskTrackerPolicy::wait_on_shutdown(Some(1_000)))
            .with_fanout_join(
                RuntimeFanoutJoinPolicy::bounded(2)
                    .with_cancel_on_first_error(false)
                    .with_retain_completion_order(true)
                    .with_shutdown_timeout_ms(Some(2_000)),
            )
            .with_blocking_bridge(RuntimeBlockingBridgePolicy::new(
                RuntimeBlockingBridgeStrategy::HelperThread,
                false,
            ));
    let (runtime, _events) = TokioAgentRuntime::from_build_request(
        TokioAgentRuntimeBuildRequest::runtime_root(4).with_runtime_snapshot(snapshot),
    );

    let (child, receipt) = runtime.child_runtime_for_session(
        SessionKind::SubAgent,
        "child/reviewer",
        ContextVisibility::default_runtime().with_max_history_items(Some(8)),
    );

    assert_eq!(
        runtime.runtime_snapshot().session_id().as_str(),
        "runtime.root"
    );
    assert_eq!(receipt.parent_session_id().as_str(), "runtime.root");
    assert_eq!(receipt.child_session_id().as_str(), "child/reviewer");
    assert_eq!(child.session().session_id().as_str(), "child/reviewer");
    assert_eq!(child.session().kind(), &SessionKind::SubAgent);
    assert_eq!(
        child.runtime_snapshot().session_id().as_str(),
        "child/reviewer"
    );
    assert_eq!(
        child.runtime_snapshot().tokio_policy().flavor(),
        &TokioRuntimeFlavor::CurrentThread
    );
    assert!(
        !child
            .runtime_snapshot()
            .task_tracker()
            .cancel_on_shutdown_enabled()
    );
    assert_eq!(
        child
            .runtime_snapshot()
            .task_tracker()
            .shutdown_timeout_ms(),
        Some(1_000)
    );
    assert_eq!(
        child.runtime_snapshot().fanout_join().max_parallelism(),
        Some(2)
    );
    assert!(
        !child
            .runtime_snapshot()
            .fanout_join()
            .cancel_on_first_error()
    );
    assert!(
        child
            .runtime_snapshot()
            .fanout_join()
            .retain_completion_order()
    );
    assert_eq!(
        child.runtime_snapshot().fanout_join().shutdown_timeout_ms(),
        Some(2_000)
    );
    assert_eq!(
        child.runtime_snapshot().blocking_bridge().strategy(),
        &RuntimeBlockingBridgeStrategy::HelperThread
    );
    assert!(
        !child
            .runtime_snapshot()
            .blocking_bridge()
            .blocking_non_cancellable()
    );
}
