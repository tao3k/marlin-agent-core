use marlin_agent_protocol::{AgentEvent, AgentEventTopic};

#[test]
fn agent_event_topic_round_trips_through_events() {
    let topic = AgentEventTopic::new("kernel.execution");
    let event = AgentEvent::new(topic.clone(), "started");

    assert_eq!(topic.as_str(), "kernel.execution");
    assert_eq!(event.topic, "kernel.execution");
    assert_eq!(event.topic_id(), topic);
    assert_eq!(
        AgentEvent::new("kernel.node", "started").topic_id(),
        AgentEventTopic::new("kernel.node")
    );
}
