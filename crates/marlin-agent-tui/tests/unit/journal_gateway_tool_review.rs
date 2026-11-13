// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Review, claim, execution and replay across the journal gateway.

use super::{ids, open, request};
use crate::harness::journal_gateway::JournalGateway;
use marlin_agent_protocol::{
    ModelGatewayCompletionChoice, ModelGatewayCompletionOptions, ModelGatewayCompletionResponse,
    ModelGatewayError, ModelGatewayFuture, ModelGatewayRequest, ModelGatewayResult,
    ModelStreamInterruption, ModelToolCall, ModelToolCallId, ModelToolName,
    assistant_gateway_message,
};
use marlin_agent_runtime::{
    ModelToolCatalog, ModelToolDeclaration, ModelToolOffer, ModelToolOutcome, RuntimeContext,
    RuntimeFuture, ShellToolDeclarationRequest, TokioAgentRuntime, ToolRuntime,
};
use marlin_agent_sessions::{
    ConversationJournal, ConversationToolReviewDecision, ConversationToolReviewRequest,
    ConversationToolReviewStatus, ConversationTurnOutcome, ConversationUncertaintyReason,
};
use marlin_agent_storage::InMemoryAgentStorage;
use marlin_agent_stream::{ModelGatewayEventStream, ModelStreamEvent, ModelStreamingGateway};
use std::sync::Arc;
use tokio_stream::StreamExt;

fn tool_request() -> ModelGatewayRequest {
    request().with_options(ModelGatewayCompletionOptions {
        tools: Some(vec![tool_declaration().definition()]),
        ..ModelGatewayCompletionOptions::default()
    })
}
fn tool_declaration() -> ModelToolDeclaration {
    ModelToolDeclaration::shell(ShellToolDeclarationRequest {
        name: ModelToolName::new("read_marker").unwrap(),
        description: "Read a marker".into(),
        argument: "command".into(),
        timeout_seconds: 5,
        output_limit: 1024,
    })
    .unwrap()
}
struct Handler;

impl ToolRuntime for Handler {
    type Invocation = ModelToolCall;
    type Output = ModelToolOutcome;

    fn run_tool(
        &self,
        _call: ModelToolCall,
        _context: RuntimeContext,
    ) -> RuntimeFuture<Self::Output> {
        Box::pin(async { ModelToolOutcome::Completed("marker".into()) })
    }
}

fn tool_offer() -> ModelToolOffer {
    let mut catalog = ModelToolCatalog::new();
    catalog
        .register(tool_declaration(), Arc::new(Handler))
        .unwrap();
    catalog.offer()
}
struct ToolFixture;
impl ModelStreamingGateway for ToolFixture {
    fn stream(
        &self,
        _: ModelGatewayRequest,
    ) -> ModelGatewayFuture<ModelGatewayResult<ModelGatewayEventStream>> {
        let mut message = assistant_gateway_message("inspect the workspace");
        message.tool_calls.push(ModelToolCall {
            id: ModelToolCallId::new("call-1").unwrap(),
            name: ModelToolName::new("read_marker").unwrap(),
            arguments: serde_json::json!({"command":"printf marker"})
                .as_object()
                .unwrap()
                .clone(),
        });
        let response = ModelGatewayCompletionResponse::new(
            "id",
            "model",
            vec![ModelGatewayCompletionChoice::new(0, message, None)],
        );
        Box::pin(async move {
            Ok(
                Box::pin(tokio_stream::iter([Ok(ModelStreamEvent::ToolRequested {
                    response,
                })])) as ModelGatewayEventStream,
            )
        })
    }
}

#[tokio::test]
async fn tool_request_is_persisted_before_delivery_and_blocks_a_second_turn() {
    let storage = InMemoryAgentStorage::new();
    let gateway =
        JournalGateway::new(ToolFixture, open(storage.clone()).await).with_tool_offer(tool_offer());
    let mut stream = gateway.stream(tool_request()).await.unwrap();
    assert!(matches!(
        stream.next().await,
        Some(Ok(ModelStreamEvent::ToolRequested { .. }))
    ));
    assert!(stream.next().await.is_none());
    assert!(matches!(
        gateway.stream(tool_request()).await,
        Err(ModelGatewayError::Completion(message)) if message.contains("pending")
    ));
}

#[tokio::test]
async fn unoffered_tool_call_never_becomes_a_reviewable_request() {
    let storage = InMemoryAgentStorage::new();
    let gateway = JournalGateway::new(ToolFixture, open(storage.clone()).await);
    let mut stream = gateway.stream(request()).await.unwrap();
    assert!(matches!(
        stream.next().await,
        Some(Err(ModelGatewayError::StreamInterrupted(
            ModelStreamInterruption::InvalidCompletion
        )))
    ));
    let (project, session, agent) = ids();
    let mut turns = Vec::new();
    let reopened = ConversationJournal::open(storage, project, session, agent, |turn| {
        turns.push(turn.clone());
    })
    .await
    .unwrap();
    assert!(reopened.pending_tool_request().is_none());
    assert!(matches!(
        turns[0].outcome,
        Some(ConversationTurnOutcome::Uncertain {
            reason: ConversationUncertaintyReason::StreamInterrupted {
                reason: ModelStreamInterruption::InvalidCompletion
            }
        })
    ));
}

#[tokio::test]
async fn tool_definition_without_runtime_handler_is_not_dispatched() {
    let storage = InMemoryAgentStorage::new();
    let gateway = JournalGateway::new(ToolFixture, open(storage.clone()).await);
    assert!(matches!(
        gateway.stream(tool_request()).await,
        Err(ModelGatewayError::RequestNotDispatched(_))
    ));
    let (project, session, agent) = ids();
    let mut replayed = 0;
    let reopened = ConversationJournal::open(storage, project, session, agent, |_| replayed += 1)
        .await
        .unwrap();
    assert_eq!(replayed, 0);
    assert!(reopened.pending_tool_request().is_none());
}

#[tokio::test]
async fn review_handle_commits_handler_result_after_one_durable_claim() {
    let storage = InMemoryAgentStorage::new();
    let gateway =
        JournalGateway::new(ToolFixture, open(storage.clone()).await).with_tool_offer(tool_offer());
    let review = gateway.tool_review_handle().unwrap();
    let mut stream = gateway.stream(tool_request()).await.unwrap();
    assert!(matches!(
        stream.next().await,
        Some(Ok(ModelStreamEvent::ToolRequested { .. }))
    ));
    assert!(stream.next().await.is_none());
    let key = review.try_pending().unwrap().key();
    let (owner, _) = TokioAgentRuntime::new(4);
    let outcome = review
        .decide(
            ConversationToolReviewRequest::new(
                key.clone(),
                ConversationToolReviewDecision::ApproveOnce,
            ),
            owner.context(),
        )
        .await
        .unwrap();
    let crate::harness::journal_gateway::JournalToolReviewOutcome::Results(receipt) = outcome
    else {
        panic!("approval must commit results")
    };
    assert_eq!(receipt.key(), &key);
    assert_eq!(receipt.results().len(), 1);
    assert!(
        review
            .decide(
                ConversationToolReviewRequest::new(
                    key,
                    ConversationToolReviewDecision::ApproveOnce
                ),
                owner.context(),
            )
            .await
            .is_err()
    );
    let (project, session, agent) = ids();
    let reopened = ConversationJournal::open(storage, project, session, agent, |_| {})
        .await
        .unwrap();
    assert_eq!(
        reopened.pending_tool_request().unwrap().status(),
        ConversationToolReviewStatus::ResultsCommitted
    );
}

#[tokio::test]
async fn review_handle_rejection_never_creates_a_claim_or_result() {
    let storage = InMemoryAgentStorage::new();
    let gateway =
        JournalGateway::new(ToolFixture, open(storage.clone()).await).with_tool_offer(tool_offer());
    let review = gateway.tool_review_handle().unwrap();
    let mut stream = gateway.stream(tool_request()).await.unwrap();
    assert!(matches!(
        stream.next().await,
        Some(Ok(ModelStreamEvent::ToolRequested { .. }))
    ));
    assert!(stream.next().await.is_none());
    let key = review.try_pending().unwrap().key();
    let (owner, _) = TokioAgentRuntime::new(4);
    let outcome = review
        .decide(
            ConversationToolReviewRequest::new(key, ConversationToolReviewDecision::Reject),
            owner.context(),
        )
        .await
        .unwrap();
    assert!(matches!(
        outcome,
        crate::harness::journal_gateway::JournalToolReviewOutcome::Rejected(_)
    ));
    let (project, session, agent) = ids();
    let reopened = ConversationJournal::open(storage, project, session, agent, |_| {})
        .await
        .unwrap();
    let pending = reopened.pending_tool_request().unwrap();
    assert_eq!(pending.status(), ConversationToolReviewStatus::Rejected);
    assert!(pending.results_receipt().is_none());
}
