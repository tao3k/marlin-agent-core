use std::time::Duration;

use marlin_agent_protocol::ModelEndpoint;
use marlin_agent_stream::{
    ChunkGate, LiteLlmModelClientError, ModelStreamEvent, ModelStreamGateway, ModelStreamRequest,
    ModelStreamTransport, user_message,
};
use marlin_agent_test_support::{ScriptedModelGateway, ScriptedModelStream};

#[tokio::test]
async fn scripted_model_stream_collects_text_delta_receipt() {
    let receipt = ScriptedModelStream::single_text_delta("hello")
        .collect()
        .await;

    assert_eq!(receipt.events.len(), 3);
    assert_eq!(receipt.chunk_count, 1);
    assert!(receipt.completed);
    assert!(!receipt.failed);
    assert!(matches!(receipt.events[1], ModelStreamEvent::Chunk(_)));
}

#[tokio::test]
async fn scripted_model_stream_waits_for_chunk_gate() {
    let gate = ChunkGate::closed();
    let collection = tokio::spawn(
        ScriptedModelStream::single_text_delta("hello")
            .with_chunk_gate(gate.clone())
            .collect(),
    );

    tokio::time::sleep(Duration::from_millis(10)).await;
    assert!(
        !collection.is_finished(),
        "scripted stream should wait for chunk gate"
    );

    gate.release_next();
    let receipt = collection
        .await
        .expect("scripted stream task should complete");

    assert_eq!(receipt.chunk_count, 1);
    assert_eq!(receipt.gate_sequences, vec![1]);
    assert_eq!(gate.admitted_chunks(), 1);
}

#[tokio::test]
async fn scripted_model_gateway_records_request_receipt() {
    let gateway = ScriptedModelGateway::completion_failure("scripted failure");
    let request = ModelStreamRequest::new(
        ModelEndpoint::new("anthropic", "claude-opus-4-8"),
        vec![user_message("hello")],
    )
    .with_transport(ModelStreamTransport::Sse);

    let result = gateway.complete(request).await;

    assert!(matches!(
        result,
        Err(LiteLlmModelClientError::Completion(message)) if message == "scripted failure"
    ));
    assert_eq!(gateway.queued_outcomes(), 0);

    let requests = gateway.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].litellm_model_id, "anthropic/claude-opus-4-8");
    assert_eq!(requests[0].message_count, 1);
    assert!(!requests[0].has_options);
    assert_eq!(requests[0].transport, ModelStreamTransport::Sse);
}

#[tokio::test]
async fn scripted_model_gateway_reports_missing_outcome_without_network() {
    let gateway = ScriptedModelGateway::empty();
    let request = ModelStreamRequest::new(
        ModelEndpoint::new("anthropic", "claude-opus-4-8"),
        vec![user_message("hello")],
    );

    let result = gateway.complete(request).await;

    assert!(matches!(
        result,
        Err(LiteLlmModelClientError::Completion(message))
            if message == "scripted model gateway has no queued completion outcome"
    ));
    assert_eq!(gateway.requests().len(), 1);
}
