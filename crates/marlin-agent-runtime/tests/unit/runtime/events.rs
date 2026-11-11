use marlin_agent_runtime::{RuntimeEvent, TokioAgentRuntime};
use tokio_stream::StreamExt;

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
