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
