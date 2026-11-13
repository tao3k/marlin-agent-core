// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use marlin_agent_runtime::{
    MAX_RUNTIME_EVENT_HEAP_BYTES, RuntimeEvent, RuntimeEventSendError, RuntimeEventSink,
    RuntimeEventSizeRejection, TokioAgentRuntime,
};
use tokio_stream::StreamExt;

#[tokio::test]
async fn oversized_runtime_events_never_enter_primary_or_capture_queues() {
    let (sink, mut primary) = RuntimeEventSink::channel(2);
    let (sink, mut capture) = sink.with_capture(2);
    let event = RuntimeEvent::new("runtime.test", "x".repeat(MAX_RUNTIME_EVENT_HEAP_BYTES));
    let error = sink.emit(event).await.unwrap_err();
    assert!(matches!(
        error,
        RuntimeEventSendError::Oversized(RuntimeEventSizeRejection {
            event,
            actual_bytes,
            limit_bytes: MAX_RUNTIME_EVENT_HEAP_BYTES,
        }) if event.topic == "runtime.test" && actual_bytes > MAX_RUNTIME_EVENT_HEAP_BYTES
    ));
    assert!(primary.try_next().is_none());
    assert!(capture.try_next().is_none());

    let mut retained = String::with_capacity(MAX_RUNTIME_EVENT_HEAP_BYTES + 1);
    retained.push('x');
    assert!(matches!(
        sink.emit(RuntimeEvent::new("runtime.test", retained)).await,
        Err(RuntimeEventSendError::Oversized(_))
    ));
    assert!(primary.try_next().is_none());
    assert!(capture.try_next().is_none());

    sink.emit(RuntimeEvent::new("runtime.test", "small"))
        .await
        .unwrap();
    assert_eq!(primary.recv().await.unwrap().message, "small");
    assert_eq!(capture.recv().await.unwrap().message, "small");
}

#[tokio::test]
async fn runtime_event_capture_tees_child_events_to_parent_and_capture_streams() {
    let (runtime, mut parent_events) = TokioAgentRuntime::new(4);
    let (capture_runtime, mut captured_events) = runtime.child_runtime_with_event_capture(4);

    capture_runtime
        .context()
        .emit(RuntimeEvent::new("runtime.test", "observed"))
        .await
        .expect("captured event sink should remain open");

    let parent_event = parent_events
        .next()
        .await
        .expect("parent stream should receive event");
    let captured_event = captured_events
        .next()
        .await
        .expect("capture stream should receive event");

    assert_eq!(parent_event, RuntimeEvent::new("runtime.test", "observed"));
    assert_eq!(captured_event, parent_event);
}

#[tokio::test]
async fn runtime_event_capture_survives_closed_parent_stream() {
    let (runtime, mut parent_events) = TokioAgentRuntime::new(4);
    let (capture_runtime, mut captured_events) = runtime.child_runtime_with_event_capture(4);

    parent_events.close();
    let emit_result = capture_runtime
        .context()
        .emit(RuntimeEvent::new(
            "runtime.test",
            "captured-after-parent-close",
        ))
        .await;

    assert!(
        emit_result.is_err(),
        "primary stream closure should remain observable to the emitter"
    );
    let captured_event = captured_events
        .next()
        .await
        .expect("capture stream should still receive event");

    assert_eq!(
        captured_event,
        RuntimeEvent::new("runtime.test", "captured-after-parent-close")
    );
}

#[tokio::test]
async fn captured_fanout_waits_for_all_live_subscribers_before_delivery() {
    let (runtime, mut parent_events) = TokioAgentRuntime::new(1);
    let (capture_runtime, mut captured_events) = runtime.child_runtime_with_event_capture(1);
    capture_runtime
        .context()
        .emit(RuntimeEvent::new("first", "one"))
        .await
        .unwrap();
    assert_eq!(parent_events.recv().await.unwrap().topic, "first");

    let context = capture_runtime.context();
    let mut pending =
        tokio::spawn(async move { context.emit(RuntimeEvent::new("second", "two")).await });
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(20), &mut pending)
            .await
            .is_err()
    );
    assert!(parent_events.try_next().is_none());

    assert_eq!(captured_events.recv().await.unwrap().topic, "first");
    tokio::time::timeout(std::time::Duration::from_secs(1), pending)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(parent_events.recv().await.unwrap().topic, "second");
    assert_eq!(captured_events.recv().await.unwrap().topic, "second");
}

#[tokio::test]
async fn cancelling_a_blocked_fanout_releases_capacity_without_partial_delivery() {
    let (runtime, mut parent_events) = TokioAgentRuntime::new(1);
    let (capture_runtime, mut captured_events) = runtime.child_runtime_with_event_capture(1);
    let context = capture_runtime.context();
    context
        .emit(RuntimeEvent::new("first", "one"))
        .await
        .unwrap();
    parent_events.recv().await.unwrap();

    let blocked_context = context.clone();
    let mut pending = tokio::spawn(async move {
        blocked_context
            .emit(RuntimeEvent::new("cancelled", "must not appear"))
            .await
    });
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(20), &mut pending)
            .await
            .is_err()
    );
    pending.abort();
    assert!(pending.await.unwrap_err().is_cancelled());
    assert!(parent_events.try_next().is_none());
    assert_eq!(captured_events.recv().await.unwrap().topic, "first");

    context
        .emit(RuntimeEvent::new("next", "delivered"))
        .await
        .unwrap();
    assert_eq!(parent_events.recv().await.unwrap().topic, "next");
    assert_eq!(captured_events.recv().await.unwrap().topic, "next");
}

#[tokio::test]
async fn multiple_captures_wait_for_the_slowest_active_subscriber() {
    let (runtime, mut parent_events) = TokioAgentRuntime::new(1);
    let (first_runtime, mut first_events) = runtime.child_runtime_with_event_capture(1);
    let (second_runtime, mut second_events) = first_runtime.child_runtime_with_event_capture(1);
    let context = second_runtime.context();
    context
        .emit(RuntimeEvent::new("first", "one"))
        .await
        .unwrap();
    parent_events.recv().await.unwrap();
    first_events.recv().await.unwrap();

    let mut pending =
        tokio::spawn(async move { context.emit(RuntimeEvent::new("second", "two")).await });
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(20), &mut pending)
            .await
            .is_err()
    );
    assert!(parent_events.try_next().is_none());
    assert!(first_events.try_next().is_none());
    second_events.recv().await.unwrap();

    pending.await.unwrap().unwrap();
    assert_eq!(parent_events.recv().await.unwrap().topic, "second");
    assert_eq!(first_events.recv().await.unwrap().topic, "second");
    assert_eq!(second_events.recv().await.unwrap().topic, "second");
}

#[tokio::test]
async fn receive_and_stream_share_order_and_close_state() {
    let (runtime, mut events) = TokioAgentRuntime::new(2);
    runtime
        .context()
        .emit(RuntimeEvent::new("first", "one"))
        .await
        .unwrap();
    runtime
        .context()
        .emit(RuntimeEvent::new("second", "two"))
        .await
        .unwrap();
    assert_eq!(events.recv().await.unwrap().topic, "first");
    assert_eq!(events.next().await.unwrap().topic, "second");
    events.close();
    assert!(events.recv().await.is_none());
}
