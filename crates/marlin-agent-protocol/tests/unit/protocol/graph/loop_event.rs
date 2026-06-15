use marlin_agent_protocol::{
    GraphLoopContinuationAction, GraphLoopContinuationDecision, GraphLoopContinuationReceipt,
    GraphLoopEvent, GraphLoopEventEnvelope, GraphLoopExecutionStatus, GraphLoopInputDrainPolicy,
    GraphLoopInputLane, GraphLoopInputQueueReceipt, GraphLoopIterationId, GraphLoopMessageRole,
    GraphLoopQueuedInput, GraphToolBatchDecision, GraphToolBatchExecutionMode,
    GraphToolBatchExecutionReceipt, GraphToolCallReceipt, GraphToolCallStatus,
};

#[test]
fn graph_loop_event_envelope_serializes_typed_lifecycle_event() {
    let event = GraphLoopEventEnvelope::new(
        "run-1",
        "event-1",
        123,
        GraphLoopEvent::MessageUpdate {
            role: GraphLoopMessageRole::Assistant,
            content_digest: Some("sha256:partial".to_string()),
        },
    )
    .with_iteration_id(2)
    .with_node_id("provider-stream")
    .with_trace_id("trace-1");

    let value = serde_json::to_value(&event).expect("event serializes");
    assert_eq!(value["run_id"], "run-1");
    assert_eq!(value["event_id"], "event-1");
    assert_eq!(value["iteration_id"], 2);
    assert_eq!(value["node_id"], "provider-stream");
    assert_eq!(value["trace_id"], "trace-1");
    assert_eq!(value["timestamp_ms"], 123);
    assert_eq!(value["event"]["type"], "message_update");
    assert_eq!(value["event"]["role"], "Assistant");
    assert_eq!(value["event"]["content_digest"], "sha256:partial");
}

#[test]
fn continuation_decision_records_deny_receipt_without_progress() {
    let receipt = GraphLoopContinuationReceipt::new(
        "run-1",
        GraphLoopIterationId::new(3),
        GraphLoopContinuationAction::Deny {
            reason: "assistant-tail".to_string(),
        },
    )
    .with_diagnostic("last_message_role=assistant");
    let decision = GraphLoopContinuationDecision::new(receipt);

    assert!(!decision.allows_progress());

    let value = serde_json::to_value(&decision).expect("decision serializes");
    assert_eq!(value["receipt"]["run_id"], "run-1");
    assert_eq!(value["receipt"]["iteration_id"], 3);
    assert_eq!(value["receipt"]["action"]["action"], "deny");
    assert_eq!(value["receipt"]["action"]["reason"], "assistant-tail");
    assert_eq!(
        value["receipt"]["diagnostics"][0],
        "last_message_role=assistant"
    );
}

#[test]
fn input_queue_receipt_distinguishes_steering_and_follow_up_policy() {
    let receipt = GraphLoopInputQueueReceipt::new(
        GraphLoopInputLane::Steering,
        GraphLoopInputDrainPolicy::DrainOne,
        2,
        vec![GraphLoopQueuedInput {
            lane: GraphLoopInputLane::Steering,
            input_id: "input-1".to_string(),
            role: GraphLoopMessageRole::User,
            content_digest: Some("sha256:steer".to_string()),
        }],
    );

    assert_eq!(receipt.queued_count_before, 2);
    assert_eq!(receipt.drained_inputs.len(), 1);
    assert_eq!(receipt.drained_inputs[0].input_id, "input-1");
}

#[test]
fn tool_batch_terminates_only_when_every_tool_requests_termination() {
    let first = GraphToolCallReceipt::new(
        "node-exec-1",
        "tool-call-1",
        "finish",
        GraphToolCallStatus::Completed,
    )
    .with_terminate();
    let second = GraphToolCallReceipt::new(
        "node-exec-2",
        "tool-call-2",
        "status",
        GraphToolCallStatus::Completed,
    );

    let mixed = GraphToolBatchExecutionReceipt::new(
        GraphToolBatchExecutionMode::Parallel,
        vec![first.clone(), second],
    );
    assert_eq!(mixed.decision, GraphToolBatchDecision::Continue);

    let terminating =
        GraphToolBatchExecutionReceipt::new(GraphToolBatchExecutionMode::Sequential, vec![first]);
    assert_eq!(terminating.decision, GraphToolBatchDecision::Terminate);
}

#[test]
fn agent_end_event_carries_terminal_execution_status() {
    let event = GraphLoopEvent::AgentEnd {
        status: GraphLoopExecutionStatus::Cancelled,
    };

    let value = serde_json::to_value(event).expect("event serializes");
    assert_eq!(value["type"], "agent_end");
    assert_eq!(value["status"], "Cancelled");
}
