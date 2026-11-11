use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use super::{edge_gateway::RuntimeStreamGateway, journal_gateway::JournalGateway};
use marlin_agent_protocol::{
    ModelEndpoint, ModelGatewayError, ModelGatewayFuture, ModelGatewayRequest, ModelGatewayResult,
    user_gateway_message,
};
use marlin_agent_runtime::{RuntimeEdgeLayer, RuntimeEdgePolicy, RuntimeEdgePolicyError};
use marlin_agent_sessions::{
    ConversationJournal, ConversationTurnOutcome, ConversationUncertaintyReason,
};
use marlin_agent_storage::{AgentId, InMemoryAgentStorage, ProjectId, SessionId};
use marlin_agent_stream::{ModelGatewayEventStream, ModelStreamingGateway};
use tokio_stream::StreamExt;

struct PendingGateway(Arc<AtomicUsize>);
impl ModelStreamingGateway for PendingGateway {
    fn stream(
        &self,
        _: ModelGatewayRequest,
    ) -> ModelGatewayFuture<ModelGatewayResult<ModelGatewayEventStream>> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Box::pin(std::future::pending())
    }
}

fn request() -> ModelGatewayRequest {
    ModelGatewayRequest::new(
        ModelEndpoint::new("provider", "model"),
        vec![user_gateway_message("hello")],
    )
}

#[tokio::test]
async fn runtime_timeout_applies_to_open_and_receipt_describes_scope() {
    let calls = Arc::new(AtomicUsize::new(0));
    let gateway = RuntimeStreamGateway::new(
        PendingGateway(calls.clone()),
        RuntimeEdgePolicy::new().with_timeout_ms(20),
    )
    .unwrap();
    assert_eq!(gateway.receipt().timeout_ms(), Some(20));
    assert_eq!(gateway.receipt().layers(), &[RuntimeEdgeLayer::Timeout]);
    let error = match tokio::time::timeout(Duration::from_secs(2), gateway.stream(request()))
        .await
        .unwrap()
    {
        Ok(_) => panic!("pending open should time out"),
        Err(error) => error.to_string(),
    };
    assert!(error.contains("remote outcome is unknown"), "{error}");
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn runtime_policy_rejects_zero_timeout_before_dispatch() {
    let calls = Arc::new(AtomicUsize::new(0));
    assert!(matches!(
        RuntimeStreamGateway::new(
            PendingGateway(calls.clone()),
            RuntimeEdgePolicy::new().with_timeout_ms(0)
        ),
        Err(RuntimeEdgePolicyError::ZeroTimeout)
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

struct FailingGateway;
impl ModelStreamingGateway for FailingGateway {
    fn stream(
        &self,
        _: ModelGatewayRequest,
    ) -> ModelGatewayFuture<ModelGatewayResult<ModelGatewayEventStream>> {
        Box::pin(async { Err(ModelGatewayError::Completion("fixture denial".into())) })
    }
}

struct TransportFailureGateway;
impl ModelStreamingGateway for TransportFailureGateway {
    fn stream(
        &self,
        _: ModelGatewayRequest,
    ) -> ModelGatewayFuture<ModelGatewayResult<ModelGatewayEventStream>> {
        Box::pin(async { Err(ModelGatewayError::StreamOpenTransport) })
    }
}

#[tokio::test]
async fn transport_error_is_typed_and_remains_unresolved_after_replay() {
    let storage = InMemoryAgentStorage::new();
    let project = ProjectId::new("project:transport").unwrap();
    let session = SessionId::new("session:transport").unwrap();
    let agent = AgentId::new("agent:transport").unwrap();
    let journal = ConversationJournal::open(
        storage.clone(),
        project.clone(),
        session.clone(),
        agent.clone(),
        |_| {},
    )
    .await
    .unwrap();
    let edge = RuntimeStreamGateway::new(
        TransportFailureGateway,
        RuntimeEdgePolicy::new().with_timeout_ms(1000),
    )
    .unwrap();
    let gateway = JournalGateway::new(edge, journal);
    assert!(matches!(
        gateway.stream(request()).await,
        Err(ModelGatewayError::StreamOpenTransport)
    ));
    let mut turns = Vec::new();
    let reopened = ConversationJournal::open(storage, project, session, agent, |turn| {
        turns.push(turn.clone())
    })
    .await
    .unwrap();
    assert!(matches!(
        turns[0].outcome,
        Some(ConversationTurnOutcome::Uncertain {
            reason: ConversationUncertaintyReason::StreamOpenTransport
        })
    ));
    assert!(reopened.unfinished().is_some());
}

#[tokio::test]
async fn provider_error_is_preserved() {
    let gateway = RuntimeStreamGateway::new(
        FailingGateway,
        RuntimeEdgePolicy::new().with_timeout_ms(1000),
    )
    .unwrap();
    let result = gateway.stream(request()).await;
    assert!(
        matches!(result, Err(ModelGatewayError::Completion(message)) if message == "fixture denial")
    );
}

struct OpenFailureGateway {
    calls: Arc<AtomicUsize>,
    before_dispatch: bool,
}

impl ModelStreamingGateway for OpenFailureGateway {
    fn stream(
        &self,
        _: ModelGatewayRequest,
    ) -> ModelGatewayFuture<ModelGatewayResult<ModelGatewayEventStream>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let error = if self.before_dispatch {
            ModelGatewayError::RequestNotDispatched("local route denied".into())
        } else {
            ModelGatewayError::Completion("ambiguous open failure".into())
        };
        Box::pin(async move { Err(error) })
    }
}

#[tokio::test]
async fn ambiguous_open_error_stays_unresolved_after_restart() {
    let storage = InMemoryAgentStorage::new();
    let project = ProjectId::new("project:open-unknown").unwrap();
    let session = SessionId::new("session:open-unknown").unwrap();
    let agent = AgentId::new("agent:open-unknown").unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let journal = ConversationJournal::open(
        storage.clone(),
        project.clone(),
        session.clone(),
        agent.clone(),
        |_| {},
    )
    .await
    .unwrap();
    let gateway = JournalGateway::new(
        OpenFailureGateway {
            calls: calls.clone(),
            before_dispatch: false,
        },
        journal,
    );
    assert!(matches!(
        gateway.stream(request()).await,
        Err(ModelGatewayError::StreamOpenUnclassified)
    ));
    assert!(gateway.stream(request()).await.is_err());
    let mut turns = Vec::new();
    let reopened = ConversationJournal::open(storage, project, session, agent, |turn| {
        turns.push(turn.clone())
    })
    .await
    .unwrap();
    assert!(matches!(
        turns[0].outcome,
        Some(ConversationTurnOutcome::Uncertain {
            reason: ConversationUncertaintyReason::StreamOpenUnclassified
        })
    ));
    let resumed = JournalGateway::new(
        OpenFailureGateway {
            calls: calls.clone(),
            before_dispatch: false,
        },
        reopened,
    );
    assert!(resumed.stream(request()).await.is_err());
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn explicitly_undispatched_request_does_not_block_the_next_turn() {
    let storage = InMemoryAgentStorage::new();
    let project = ProjectId::new("project:open-denial").unwrap();
    let session = SessionId::new("session:open-denial").unwrap();
    let agent = AgentId::new("agent:open-denial").unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let journal = ConversationJournal::open(
        storage.clone(),
        project.clone(),
        session.clone(),
        agent.clone(),
        |_| {},
    )
    .await
    .unwrap();
    let gateway = JournalGateway::new(
        OpenFailureGateway {
            calls: calls.clone(),
            before_dispatch: true,
        },
        journal,
    );
    for _ in 0..2 {
        assert!(matches!(
            gateway.stream(request()).await,
            Err(ModelGatewayError::RequestNotDispatched(_))
        ));
    }
    let mut turns = Vec::new();
    let reopened = ConversationJournal::open(storage, project, session, agent, |turn| {
        turns.push(turn.clone())
    })
    .await
    .unwrap();
    assert!(reopened.unfinished().is_none());
    assert_eq!(turns.len(), 2);
    assert!(turns.iter().all(|turn| matches!(
        turn.outcome,
        Some(ConversationTurnOutcome::RejectedBeforeDispatch { .. })
    )));
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

struct OpenThenPendingGateway;
impl ModelStreamingGateway for OpenThenPendingGateway {
    fn stream(
        &self,
        _: ModelGatewayRequest,
    ) -> ModelGatewayFuture<ModelGatewayResult<ModelGatewayEventStream>> {
        Box::pin(async { Ok(Box::pin(tokio_stream::pending()) as ModelGatewayEventStream) })
    }
}

#[tokio::test]
async fn runtime_open_timeout_does_not_claim_to_limit_stream_duration() {
    let gateway = RuntimeStreamGateway::new(
        OpenThenPendingGateway,
        RuntimeEdgePolicy::new().with_timeout_ms(20),
    )
    .unwrap();
    let mut stream = gateway.stream(request()).await.unwrap();
    assert!(
        tokio::time::timeout(Duration::from_millis(80), stream.next())
            .await
            .is_err()
    );
}

#[tokio::test]
async fn open_timeout_is_recorded_as_unresolved_and_blocks_further_turns() {
    let storage = InMemoryAgentStorage::new();
    let project = ProjectId::new("project:edge").unwrap();
    let session = SessionId::new("session:edge").unwrap();
    let agent = AgentId::new("agent:edge").unwrap();
    let journal = ConversationJournal::open(
        storage.clone(),
        project.clone(),
        session.clone(),
        agent.clone(),
        |_| {},
    )
    .await
    .unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let edge = RuntimeStreamGateway::new(
        PendingGateway(calls.clone()),
        RuntimeEdgePolicy::new().with_timeout_ms(20),
    )
    .unwrap();
    let gateway = JournalGateway::new(edge, journal);
    assert!(matches!(
        gateway.stream(request()).await,
        Err(ModelGatewayError::StreamOpenTimeout)
    ));
    assert!(gateway.stream(request()).await.is_err());
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let mut turns = Vec::new();
    let mut reopened = ConversationJournal::open(storage, project, session, agent, |turn| {
        turns.push(turn.clone())
    })
    .await
    .unwrap();
    assert!(matches!(
        &turns[0].outcome,
        Some(ConversationTurnOutcome::Uncertain {
            reason: ConversationUncertaintyReason::StreamOpenTimeout
        })
    ));
    assert!(reopened.unfinished().is_some());
    assert!(
        reopened
            .start(
                marlin_agent_sessions::ConversationTurnStart {
                    endpoint: ModelEndpoint::new("provider", "model"),
                    user_text: "next".into()
                },
                1
            )
            .await
            .is_err()
    );
}
