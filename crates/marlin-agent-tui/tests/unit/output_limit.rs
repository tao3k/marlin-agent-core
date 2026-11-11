use super::output_limit::OutputLimit;
use marlin_agent_protocol::{
    ModelGatewayCompletionChoice, ModelGatewayCompletionResponse, ModelGatewayError,
    ModelStreamInterruption, assistant_gateway_message,
};
use marlin_agent_stream::{ModelStreamChunk, ModelStreamEvent, ModelThinkingChunk};

fn chunk(sequence: u64, text: &str) -> ModelStreamEvent {
    ModelStreamEvent::Chunk(ModelStreamChunk::new(sequence, text))
}

#[test]
fn output_budget_combines_thinking_and_answer_without_double_counting() {
    let thinking = ModelStreamEvent::Thinking(ModelThinkingChunk::new(1, "abc"));
    let mut streamed = OutputLimit::new(5);
    assert!(streamed.check(&thinking).is_ok());
    assert!(streamed.check(&chunk(1, "de")).is_ok());
    assert!(streamed.check(&completed("de")).is_ok());
    let mut terminal_only = OutputLimit::new(5);
    assert!(terminal_only.check(&thinking).is_ok());
    assert!(terminal_only.check(&chunk(1, "")).is_ok());
    assert!(terminal_only.check(&chunk(1, "def")).is_err());
    assert!(terminal_only.check(&completed("def")).is_err());
}

#[test]
fn completion_counts_reserved_metadata_and_all_choices_before_queueing() {
    let mut id = String::with_capacity(super::output_limit::MAX_COMPLETION_HEAP_BYTES + 1);
    id.push_str("id");
    let response = ModelGatewayCompletionResponse::new(
        id,
        "model",
        vec![ModelGatewayCompletionChoice::new(
            0,
            assistant_gateway_message("ok"),
            None,
        )],
    );
    let actual = response.heap_capacity();
    let mut limit = OutputLimit::new(16);
    assert!(
        matches!(limit.check(&ModelStreamEvent::Completed { response }),
        Err(ModelGatewayError::StreamInterrupted(ModelStreamInterruption::CompletionHeapLimit {
            limit_bytes: super::output_limit::MAX_COMPLETION_HEAP_BYTES, actual_bytes
        })) if actual_bytes == actual)
    );
}

#[test]
fn completion_counts_the_combined_capacity_of_multiple_choices() {
    let make_choice = |index| {
        let mut metadata = String::with_capacity(17 * 1024 * 1024);
        metadata.push_str("stop");
        ModelGatewayCompletionChoice::new(index, assistant_gateway_message("ok"), Some(metadata))
    };
    let response =
        ModelGatewayCompletionResponse::new("id", "model", vec![make_choice(0), make_choice(1)]);
    assert!(matches!(
        OutputLimit::new(16).check(&ModelStreamEvent::Completed { response }),
        Err(ModelGatewayError::StreamInterrupted(
            ModelStreamInterruption::CompletionHeapLimit { .. }
        ))
    ));
}

fn completed(text: &str) -> ModelStreamEvent {
    ModelStreamEvent::Completed {
        response: ModelGatewayCompletionResponse::new(
            "id",
            "model",
            vec![ModelGatewayCompletionChoice::new(
                0,
                assistant_gateway_message(text),
                None,
            )],
        ),
    }
}

#[test]
fn output_limit_counts_utf8_bytes_and_rejects_whole_chunks() {
    let mut limit = OutputLimit::new(5);
    assert!(limit.check(&chunk(1, "中")).is_ok());
    assert!(limit.check(&chunk(2, "🐳")).is_err());
    assert!(limit.check(&chunk(2, "ab")).is_ok());
    assert!(limit.check(&chunk(3, "x")).is_err());
    assert!(limit.check(&completed("中ab")).is_ok());
    assert!(limit.check(&completed("中abc")).is_err());
}

#[test]
fn output_limit_checks_completion_without_streamed_chunks() {
    let mut limit = OutputLimit::new(2);
    assert!(limit.check(&completed("ab")).is_ok());
    assert!(limit.check(&completed("abc")).is_err());
}
