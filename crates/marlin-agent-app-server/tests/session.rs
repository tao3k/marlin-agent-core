// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

#![cfg(feature = "grpc")]

use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use marlin_agent_app_server::{
    AppServer, SessionBinding,
    v1::{
        CancelSessionTurnRequest, SessionTurnOutcome, StartSessionTurnRequest,
        WatchSessionTurnRequest, app_server_client::AppServerClient, session_turn_event::Event,
    },
};
use marlin_agent_protocol::{
    ModelEndpoint, ModelGatewayFuture, ModelGatewayRequest, ModelGatewayResult,
    ModelGatewayTransport, ModelStreamInterruption,
};
use marlin_agent_runtime::{TokioAgentRuntime, TokioAgentRuntimeBuildRequest};
use marlin_agent_sessions::{
    AgentSessionContext, ContextVisibility, ConversationJournal, ConversationTurnOutcome,
    ConversationUncertaintyReason,
};
use marlin_agent_storage::{AgentId, InMemoryAgentStorage, ProjectId, SessionId};
use marlin_agent_stream::{
    ModelGatewayEventStream, ModelStreamChunk, ModelStreamEvent, ModelStreamingGateway,
};
use tokio::{
    net::TcpListener,
    sync::{Semaphore, oneshot},
};
use tokio_stream::wrappers::TcpListenerStream;
use tonic::{Code, Request, Status, transport::Server};

struct Gateway {
    calls: AtomicUsize,
    release: Arc<Semaphore>,
}

impl ModelStreamingGateway for Gateway {
    fn stream(
        &self,
        request: ModelGatewayRequest,
    ) -> ModelGatewayFuture<ModelGatewayResult<ModelGatewayEventStream>> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        let release = self.release.clone();
        Box::pin(async move {
            assert_eq!(request.messages().last().unwrap().content, "请开始");
            let stream = async_stream::try_stream! {
                yield ModelStreamEvent::Started { transport: ModelGatewayTransport::Sse };
                yield ModelStreamEvent::Chunk(ModelStreamChunk::new(1, "第一段"));
                release.acquire().await.unwrap().forget();
                yield ModelStreamEvent::Chunk(ModelStreamChunk::new(2, "第二段"));
                std::future::pending::<()>().await;
            };
            Ok(Box::pin(stream) as ModelGatewayEventStream)
        })
    }
}

fn authorized<T>(message: T) -> Request<T> {
    let mut request = Request::new(message);
    request
        .metadata_mut()
        .insert("authorization", "Bearer fixture".parse().unwrap());
    request
}

#[tokio::test]
async fn gui_reconnects_with_the_journal_cursor_and_cancels_its_runtime_task() {
    tokio::time::timeout(Duration::from_secs(10), async {
        let storage = InMemoryAgentStorage::default();
        let project = ProjectId::new("project").unwrap();
        let agent = AgentId::new("assistant").unwrap();
        let (runtime, _events) = TokioAgentRuntime::from_build_request(
            TokioAgentRuntimeBuildRequest::runtime_root(16).with_session(
                AgentSessionContext::root("gui-session", ContextVisibility::default_runtime()),
            ),
        );
        let binding = SessionBinding::new(storage.clone(), project.clone(), agent.clone(), runtime).unwrap();
        let release = Arc::new(Semaphore::new(0));
        let gateway = Arc::new(Gateway { calls: AtomicUsize::new(0), release: release.clone() });
        let service = AppServer::new(ModelEndpoint::new("openai", "gpt-fixture"), gateway.clone(), |metadata| {
            if metadata.get("authorization").is_some_and(|value| value == "Bearer fixture") {
                Ok(())
            } else { Err(Status::unauthenticated("authorization required")) }
        }).unwrap().with_session(binding).into_service();
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).await.unwrap();
        let address = format!("http://{}", listener.local_addr().unwrap());
        let (stop, shutdown) = oneshot::channel();
        let server = tokio::spawn(Server::builder().add_service(service).serve_with_incoming_shutdown(
            TcpListenerStream::new(listener), async { let _ = shutdown.await; },
        ));
        let mut client = AppServerClient::connect(address.clone()).await.unwrap();
        let start = StartSessionTurnRequest { session_id: "gui-session".into(), user_text: "请开始".into(), after_turn: 0 };
        assert_eq!(client.start_session_turn(start.clone()).await.unwrap_err().code(), Code::Unauthenticated);
        let turn = client.start_session_turn(authorized(start.clone())).await.unwrap().into_inner().turn.unwrap();
        assert_eq!(turn.ordinal, 1);
        let watch = |after_event_id| authorized(WatchSessionTurnRequest { turn: Some(turn.clone()), after_event_id });
        let mut stream = client.watch_session_turn(watch(None)).await.unwrap().into_inner();
        assert!(matches!(stream.message().await.unwrap().unwrap().event, Some(Event::Started(_))));
        let first = stream.message().await.unwrap().unwrap();
        assert!(matches!(first.event, Some(Event::TextCheckpoint(delta)) if delta.text == "第一段"));
        drop(stream);
        drop(client);

        // Wait for the next checkpoint to be in the journal with no GUI attached.
        release.add_permits(1);
        loop {
            use marlin_agent_storage::SessionEventStore;
            if storage.count_session_events(&project, &SessionId::new("gui-session").unwrap()).await.unwrap().get() == 3 { break; }
            tokio::task::yield_now().await;
        }
        let mut client = AppServerClient::connect(address).await.unwrap();
        assert_eq!(client.watch_session_turn(watch(Some("invented-cursor".into()))).await.unwrap_err().code(), Code::InvalidArgument);
        assert_eq!(client.start_session_turn(authorized(start)).await.unwrap_err().code(), Code::FailedPrecondition);
        let mut resumed = client.watch_session_turn(watch(Some(first.event_id.clone()))).await.unwrap().into_inner();
        let next = resumed.message().await.unwrap().unwrap();
        assert!(next.event_id > first.event_id);
        assert!(matches!(next.event, Some(Event::TextCheckpoint(delta)) if delta.text == "第二段"));
        let mut foreign = turn.clone();
        foreign.session_id = "other-session".into();
        assert_eq!(client.cancel_session_turn(authorized(CancelSessionTurnRequest { turn: Some(foreign) })).await.unwrap_err().code(), Code::NotFound);
        let cancel = || authorized(CancelSessionTurnRequest { turn: Some(turn.clone()) });
        let receipt = client.cancel_session_turn(cancel()).await.unwrap().into_inner();
        assert_eq!(receipt.turn, Some(turn.clone()));
        let runtime = receipt.runtime.unwrap();
        assert!(runtime.completed && runtime.tracker_closed && runtime.cancellation_requested);
        assert_eq!(runtime.remaining_tasks, 0);
        assert_eq!(receipt.outcome, i32::from(SessionTurnOutcome::RemoteUncertain));
        let last = resumed.message().await.unwrap().unwrap();
        assert!(matches!(last.event, Some(Event::Outcome(outcome)) if outcome == i32::from(SessionTurnOutcome::RemoteUncertain)));
        assert!(resumed.message().await.unwrap().is_none());
        let mut suffix = client.watch_session_turn(watch(Some(last.event_id))).await.unwrap().into_inner();
        assert!(suffix.message().await.unwrap().is_none());
        let repeat = client.cancel_session_turn(cancel()).await.unwrap().into_inner();
        assert!(!repeat.runtime.unwrap().cancellation_requested);
        assert_eq!(gateway.calls.load(Ordering::Relaxed), 1);

        let journal = ConversationJournal::open(storage, project, SessionId::new("gui-session").unwrap(), agent, |_| {}).await.unwrap();
        let observed = journal.unfinished().unwrap();
        assert_eq!(observed.partial_text, "第一段第二段");
        assert!(matches!(observed.outcome, Some(ConversationTurnOutcome::Uncertain {
            reason: ConversationUncertaintyReason::StreamInterrupted { reason: ModelStreamInterruption::CancelledLocally },
        })));
        stop.send(()).unwrap();
        server.await.unwrap().unwrap();
    }).await.unwrap();
}
