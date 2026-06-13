use std::time::Duration;

use marlin_agent_protocol::{
    LoopEvidenceKind, ModelEndpoint, ModelGateway, ModelGatewayError, ModelGatewayRequest,
    ModelGatewayTransport, user_gateway_message,
};
use marlin_agent_test_support::{
    ScriptedChunkGate, ScriptedModelGateway, ScriptedModelStream, ScriptedModelStreamEvent,
    scripted_stream_gate_evidence,
};

#[tokio::test]
async fn scripted_model_stream_collects_text_delta_receipt() {
    let receipt = ScriptedModelStream::single_text_delta("hello")
        .collect()
        .await;

    assert_eq!(receipt.events.len(), 3);
    assert_eq!(receipt.chunk_count, 1);
    assert!(receipt.completed);
    assert!(!receipt.failed);
    assert!(matches!(
        receipt.events[1],
        ScriptedModelStreamEvent::Chunk(_)
    ));
}

#[tokio::test]
async fn scripted_model_stream_waits_for_chunk_gate() {
    let gate = ScriptedChunkGate::closed();
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
async fn scripted_model_stream_gate_projects_runtime_evidence() {
    let gate = ScriptedChunkGate::closed();
    let collection = tokio::spawn(
        ScriptedModelStream::single_text_delta("hello")
            .with_chunk_gate(gate.clone())
            .collect(),
    );

    gate.release_next();
    let receipt = collection
        .await
        .expect("scripted stream task should complete");
    let evidence = scripted_stream_gate_evidence("review-stream", &receipt, &gate);
    let detail = evidence.detail.as_deref().expect("stream gate detail");

    assert!(evidence.present);
    assert_eq!(evidence.kind, LoopEvidenceKind::Runtime);
    assert_eq!(evidence.subject, "scripted-stream-gate:review-stream");
    assert!(detail.contains("chunk_count=1"));
    assert!(detail.contains("gate_sequences=[1]"));
    assert!(detail.contains("admitted_chunks=1"));
    assert!(detail.contains("completed=true"));
    assert!(detail.contains("failed=false"));
    assert!(detail.contains("live_llm=false"));
}

#[tokio::test]
async fn scripted_model_gateway_records_request_receipt() {
    let gateway = ScriptedModelGateway::completion_failure("scripted failure");
    let request = ModelGatewayRequest::new(
        ModelEndpoint::new("anthropic", "claude-opus-4-8"),
        vec![user_gateway_message("hello")],
    )
    .with_transport(ModelGatewayTransport::Sse);

    let result = gateway.complete(request).await;

    assert!(matches!(
        result,
        Err(ModelGatewayError::Completion(message)) if message == "scripted failure"
    ));
    assert_eq!(gateway.queued_outcomes(), 0);

    let requests = gateway.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].litellm_model_id, "anthropic/claude-opus-4-8");
    assert_eq!(requests[0].message_count, 1);
    assert!(!requests[0].has_options);
    assert_eq!(requests[0].transport, ModelGatewayTransport::Sse);
}

#[tokio::test]
async fn scripted_model_gateway_reports_missing_outcome_without_network() {
    let gateway = ScriptedModelGateway::empty();
    let request = ModelGatewayRequest::new(
        ModelEndpoint::new("anthropic", "claude-opus-4-8"),
        vec![user_gateway_message("hello")],
    );

    let result = gateway.complete(request).await;

    assert!(matches!(
        result,
        Err(ModelGatewayError::Completion(message))
            if message == "scripted model gateway has no queued completion outcome"
    ));
    assert_eq!(gateway.requests().len(), 1);
}
