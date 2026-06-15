use std::sync::{Arc, Barrier};

use marlin_agent_runtime::{
    RuntimeTaskOutcome, RuntimeTaskShutdownStatus, RuntimeTaskTrackerPolicy, TokioAgentRuntime,
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
