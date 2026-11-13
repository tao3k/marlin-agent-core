// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

#![cfg(feature = "grpc")]

use std::{
    fs::OpenOptions,
    io::Write,
    path::PathBuf,
    process::Stdio,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use marlin_agent_app_server::{
    AppServer, SessionBinding,
    v1::{
        CancelSessionTurnRequest, GetSessionSummaryRequest, GetSessionTurnCancellationRequest,
        GetSessionTurnStatusRequest, SessionRecoveryState, SessionTurnOutcome, SessionTurnRef,
        StartSessionTurnRequest, WatchSessionTurnRequest, app_server_client::AppServerClient,
        get_session_turn_cancellation_response::Evidence, session_turn_event::Event,
    },
};
use marlin_agent_protocol::{
    ModelEndpoint, ModelGatewayFuture, ModelGatewayRequest, ModelGatewayResult,
    ModelGatewayTransport, ModelStreamInterruption,
};
use marlin_agent_runtime::{TokioAgentRuntime, TokioAgentRuntimeBuildRequest};
use marlin_agent_sessions::{
    AgentSessionContext, ContextVisibility, ConversationJournal, ConversationTerminalObservation,
    ConversationTurnOutcome, ConversationTurnStart, ConversationUncertaintyReason,
};
use marlin_agent_storage::{
    AgentId, InMemoryAgentStorage, ProjectId, SessionEventBatchWriteReceipt, SessionEventCount,
    SessionEventCursor, SessionEventPageRequest, SessionEventRecord, SessionEventStore, SessionId,
    SessionPageRequest, StorageFuture, StoragePage, TursoAgentStorage, TursoAgentStorageConfig,
    TursoBatchTransactionMode, TursoOptimizationProfile,
};
use marlin_agent_stream::{
    ModelGatewayEventStream, ModelStreamChunk, ModelStreamEvent, ModelStreamingGateway,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader, Lines},
    net::TcpListener,
    process::{Child, ChildStdout, Command},
    sync::{Barrier, Semaphore, oneshot},
};
use tokio_stream::wrappers::TcpListenerStream;
use tonic::{
    Code, Request, Status,
    transport::{Endpoint, Server},
};

const SUBMISSION_PAYLOAD: &str = "fixture-input-1";
const FIRST_CHECKPOINT: &str = "fixture-chunk-1";
const SECOND_CHECKPOINT: &str = "fixture-chunk-2";

struct Gateway {
    calls: AtomicUsize,
    release: Arc<Semaphore>,
    dispatch_log: Option<PathBuf>,
}

#[derive(Clone)]
struct PausedStartStore<S> {
    storage: S,
    pause_replay: Option<(Arc<Semaphore>, Arc<Semaphore>)>,
    before_start_commit: Option<Arc<Barrier>>,
    committed: Arc<Semaphore>,
    release: Arc<Semaphore>,
    pause_cancellation: Option<CancellationWritePause>,
    fail_uncertain_write: bool,
}

#[derive(Clone)]
struct CancellationWritePause {
    reached: Arc<Semaphore>,
    release: Arc<Semaphore>,
    after_commit: bool,
}

impl CancellationWritePause {
    async fn wait(&self) {
        self.reached.add_permits(1);
        self.release.acquire().await.unwrap().forget();
    }
}

impl<S: SessionEventStore> SessionEventStore for PausedStartStore<S> {
    fn latest_session_event_header<'a>(
        &'a self,
        project: &'a ProjectId,
        session: &'a SessionId,
        from_turn: &'a marlin_agent_storage::TurnId,
        before_turn: &'a marlin_agent_storage::TurnId,
    ) -> StorageFuture<'a, Option<marlin_agent_storage::SessionEventHeader>> {
        self.storage
            .latest_session_event_header(project, session, from_turn, before_turn)
    }

    fn count_session_events<'a>(
        &'a self,
        project: &'a ProjectId,
        session: &'a SessionId,
    ) -> StorageFuture<'a, SessionEventCount> {
        self.storage.count_session_events(project, session)
    }

    fn list_sessions_page(
        &self,
        request: SessionPageRequest,
    ) -> StorageFuture<'_, StoragePage<SessionId, SessionId>> {
        self.storage.list_sessions_page(request)
    }

    fn append_session_event(&self, record: SessionEventRecord) -> StorageFuture<'_, ()> {
        Box::pin(async move {
            if self.fail_uncertain_write
                && record.event_kind == "marlin.conversation.turn.uncertain.v1"
            {
                return Err(marlin_agent_storage::StorageError::Backend {
                    message: "fixture refuses the terminal write".into(),
                });
            }
            let pause = self
                .pause_cancellation
                .as_ref()
                .filter(|_| record.event_kind == "marlin.conversation.cancellation.v1");
            if let Some(pause) = pause.filter(|pause| !pause.after_commit) {
                pause.wait().await;
            }
            self.storage.append_session_event(record).await?;
            if let Some(pause) = pause.filter(|pause| pause.after_commit) {
                pause.wait().await;
            }
            Ok(())
        })
    }

    fn append_session_events_atomically(
        &self,
        records: Vec<SessionEventRecord>,
    ) -> StorageFuture<'_, SessionEventBatchWriteReceipt> {
        Box::pin(async move {
            if let Some(barrier) = &self.before_start_commit {
                barrier.wait().await;
            }
            let receipt = self
                .storage
                .append_session_events_atomically(records)
                .await?;
            self.committed.add_permits(1);
            self.release.acquire().await.unwrap().forget();
            Ok(receipt)
        })
    }

    fn list_session_events_page(
        &self,
        request: SessionEventPageRequest,
    ) -> StorageFuture<'_, StoragePage<SessionEventRecord, SessionEventCursor>> {
        Box::pin(async move {
            if let Some((reading, release)) = &self.pause_replay
                && request.turn_body_budget().is_none()
            {
                reading.add_permits(1);
                release.acquire().await.unwrap().forget();
            }
            self.storage.list_session_events_page(request).await
        })
    }
}

impl ModelStreamingGateway for Gateway {
    fn stream(
        &self,
        request: ModelGatewayRequest,
    ) -> ModelGatewayFuture<ModelGatewayResult<ModelGatewayEventStream>> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        if let Some(path) = &self.dispatch_log {
            let mut log = OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .unwrap();
            writeln!(log, "{}", std::process::id()).unwrap();
            log.sync_all().unwrap();
        }
        let release = self.release.clone();
        Box::pin(async move {
            assert_eq!(
                request.messages().last().unwrap().content,
                SUBMISSION_PAYLOAD
            );
            let stream = async_stream::try_stream! {
                yield ModelStreamEvent::Started { transport: ModelGatewayTransport::Sse };
                yield ModelStreamEvent::Chunk(ModelStreamChunk::new(1, FIRST_CHECKPOINT));
                release.acquire().await.unwrap().forget();
                yield ModelStreamEvent::Chunk(ModelStreamChunk::new(2, SECOND_CHECKPOINT));
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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn turso_http2_concurrent_admission_uses_one_runtime_owner() {
    tokio::time::timeout(Duration::from_secs(20), async {
        let directory = tempfile::tempdir().unwrap();
        let project = ProjectId::new("project").unwrap();
        let agent = AgentId::new("assistant").unwrap();
        let session = SessionId::new("gui-session").unwrap();
        let endpoint = ModelEndpoint::new("openai", "gpt-fixture");
        let storage = TursoAgentStorage::open_local(TursoAgentStorageConfig {
            path: directory.path().join("race.turso"),
            optimization_profile: TursoOptimizationProfile::AsyncIoOnlyCompatibility,
            batch_transaction_mode: TursoBatchTransactionMode::Immediate,
        })
        .await
        .unwrap();
        let store = PausedStartStore {
            storage,
            pause_replay: None,
            before_start_commit: Some(Arc::new(Barrier::new(2))),
            committed: Arc::new(Semaphore::new(0)),
            release: Arc::new(Semaphore::new(1)),
            pause_cancellation: None,
            fail_uncertain_write: false,
        };
        let gateway = Arc::new(Gateway {
            calls: AtomicUsize::new(0),
            release: Arc::new(Semaphore::new(0)),
            dispatch_log: None,
        });
        let mut listeners = Vec::new();
        for _ in 0..2 {
            let (runtime, _) = TokioAgentRuntime::from_build_request(
                TokioAgentRuntimeBuildRequest::runtime_root(16).with_session(
                    AgentSessionContext::root(
                        session.as_str(),
                        ContextVisibility::default_runtime(),
                    ),
                ),
            );
            let binding =
                SessionBinding::new(store.clone(), project.clone(), agent.clone(), runtime)
                    .unwrap();
            let service = AppServer::new(endpoint.clone(), gateway.clone(), |_| Ok(()))
                .unwrap()
                .with_session(binding)
                .into_service();
            let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
                .await
                .unwrap();
            let address = format!("http://{}", listener.local_addr().unwrap());
            let (stop, shutdown) = oneshot::channel();
            let server = tokio::spawn(
                Server::builder()
                    .add_service(service)
                    .serve_with_incoming_shutdown(TcpListenerStream::new(listener), async {
                        let _ = shutdown.await;
                    }),
            );
            listeners.push((
                AppServerClient::connect(address).await.unwrap(),
                stop,
                server,
            ));
        }
        let [
            (mut left, stop_left, left_server),
            (mut right, stop_right, right_server),
        ] = listeners.try_into().ok().unwrap();
        let start = StartSessionTurnRequest {
            session_id: session.as_str().into(),
            user_text: SUBMISSION_PAYLOAD.into(),
            after_turn: 0,
            submission_id: "race-send-1".into(),
        };
        // Both listeners reach Turso's transaction with their own Runtime
        // scope and the same turn ordinal before either write can proceed.
        let (first, second) = tokio::join!(
            left.start_session_turn(start.clone()),
            right.start_session_turn(start.clone()),
        );
        let turn = first.unwrap().into_inner().turn.unwrap();
        assert_eq!(second.unwrap().into_inner().turn, Some(turn.clone()));
        assert_eq!(turn.ordinal, 1);
        assert_eq!(store.committed.available_permits(), 1);
        let status = GetSessionTurnStatusRequest {
            turn: Some(turn.clone()),
        };
        let left_active = left
            .get_session_turn_status(status.clone())
            .await
            .unwrap()
            .into_inner()
            .recovery_state();
        let right_active = right
            .get_session_turn_status(status)
            .await
            .unwrap()
            .into_inner()
            .recovery_state();
        let (owner, observer) = match (left_active, right_active) {
            (SessionRecoveryState::Active, SessionRecoveryState::Unresolved) => {
                (&mut left, &mut right)
            }
            (SessionRecoveryState::Unresolved, SessionRecoveryState::Active) => {
                (&mut right, &mut left)
            }
            states => panic!("expected exactly one live turn owner, got {states:?}"),
        };
        let mut changed = start;
        changed.user_text.push_str("-modified");
        assert_eq!(
            observer
                .start_session_turn(changed)
                .await
                .unwrap_err()
                .code(),
            Code::AlreadyExists
        );

        let watch = WatchSessionTurnRequest {
            turn: Some(turn.clone()),
            after_event_id: None,
        };
        let mut stream = owner.watch_session_turn(watch).await.unwrap().into_inner();
        assert!(matches!(
            stream.message().await.unwrap().unwrap().event,
            Some(Event::Started(_))
        ));
        assert!(matches!(
            stream.message().await.unwrap().unwrap().event,
            Some(Event::TextCheckpoint(_))
        ));
        drop(stream);
        let cancel = CancelSessionTurnRequest {
            turn: Some(turn.clone()),
        };
        assert_eq!(
            observer
                .cancel_session_turn(cancel.clone())
                .await
                .unwrap_err()
                .code(),
            Code::NotFound
        );
        let cancelled = owner
            .cancel_session_turn(cancel.clone())
            .await
            .unwrap()
            .into_inner();
        assert_eq!(cancelled.outcome(), SessionTurnOutcome::RemoteUncertain);
        assert!(
            cancelled
                .runtime
                .as_ref()
                .is_some_and(|receipt| receipt.completed)
        );
        assert_eq!(
            observer
                .cancel_session_turn(cancel)
                .await
                .unwrap()
                .into_inner(),
            cancelled
        );
        assert_eq!(gateway.calls.load(Ordering::Relaxed), 1);
        assert_eq!(
            store
                .count_session_events(&project, &session)
                .await
                .unwrap()
                .get(),
            5
        );

        stop_left.send(()).unwrap();
        stop_right.send(()).unwrap();
        left_server.await.unwrap().unwrap();
        right_server.await.unwrap().unwrap();
    })
    .await
    .expect("Turso HTTP/2 concurrent admission exceeded its deadline");
}

#[tokio::test]
async fn gui_reconnects_with_the_journal_cursor_and_cancels_its_runtime_task() {
    tokio::time::timeout(Duration::from_secs(10), async {
        let storage = InMemoryAgentStorage::default();
        let cancellation_writing = Arc::new(Semaphore::new(0));
        let release_cancellation = Arc::new(Semaphore::new(0));
        let replay_reading = Arc::new(Semaphore::new(0));
        let release_replay = Arc::new(Semaphore::new(0));
        let store = PausedStartStore {
            storage: storage.clone(),
            pause_replay: Some((replay_reading.clone(), release_replay.clone())),
            before_start_commit: None,
            committed: Arc::new(Semaphore::new(0)),
            release: Arc::new(Semaphore::new(0)),
            pause_cancellation: Some(CancellationWritePause {
                reached: cancellation_writing.clone(),
                release: release_cancellation.clone(),
                after_commit: false,
            }),
            fail_uncertain_write: false,
        };
        let project = ProjectId::new("project").unwrap();
        let agent = AgentId::new("assistant").unwrap();
        let (runtime, _events) = TokioAgentRuntime::from_build_request(
            TokioAgentRuntimeBuildRequest::runtime_root(16).with_session(
                AgentSessionContext::root("gui-session", ContextVisibility::default_runtime()),
            ),
        );
        let binding = SessionBinding::new(store.clone(), project.clone(), agent.clone(), runtime).unwrap();
        let release = Arc::new(Semaphore::new(0));
        let gateway = Arc::new(Gateway { calls: AtomicUsize::new(0), release: release.clone(), dispatch_log: None });
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
        let session_id = "gui-session";
        let summary = || GetSessionSummaryRequest { session_id: session_id.into() };
        assert_eq!(client.get_session_summary(summary()).await.unwrap_err().code(), Code::Unauthenticated);
        let empty = client.get_session_summary(authorized(summary())).await.unwrap().into_inner();
        assert_eq!(empty.recovery_state(), SessionRecoveryState::Empty);
        assert!(empty.latest_turn.is_none() && empty.latest_event_id.is_none());
        let mut history = ConversationJournal::open(
            storage.clone(), project.clone(), SessionId::new(session_id).unwrap(),
            agent.clone(), |_| {},
        ).await.unwrap();
        let earlier = history.start(ConversationTurnStart {
            endpoint: ModelEndpoint::new("openai", "gpt-fixture"), user_text: "prior-input".into(),
        }, 0).await.unwrap();
        history.finish(earlier, ConversationTerminalObservation::RejectedBeforeDispatch {
            diagnostic: "fixture rejection".into(),
        }, 1).await.unwrap();
        let start = StartSessionTurnRequest { session_id: session_id.into(), user_text: SUBMISSION_PAYLOAD.into(), after_turn: earlier.get(), submission_id: "gui-send-1".into() };
        let mut expired = authorized(start.clone());
        expired.set_timeout(Duration::from_secs(1));
        let mut expiring_client = client.clone();
        let expiring = tokio::spawn(async move { expiring_client.start_session_turn(expired).await });
        replay_reading.acquire().await.unwrap().forget();
        let code = expiring.await.unwrap().unwrap_err().code();
        assert!(matches!(code, Code::DeadlineExceeded | Code::Cancelled), "{code:?}");
        // Recovery is responsive while the fixture keeps the journal scan blocked.
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                match client.get_session_summary(authorized(summary())).await {
                    Ok(reply) => {
                        assert_eq!(reply.into_inner().latest_turn.unwrap().ordinal, earlier.get());
                        break;
                    }
                    Err(error) if error.code() == Code::Aborted => {
                        tokio::time::sleep(Duration::from_millis(10)).await;
                    }
                    Err(error) => panic!("unexpected recovery status: {error}"),
                }
            }
        }).await.expect("expired admission kept the Session locked");
        assert_eq!(gateway.calls.load(Ordering::Relaxed), 0);
        release_replay.add_permits(2);
        let prior_ref = SessionTurnRef { session_id: start.session_id.clone(), ordinal: earlier.get() };
        let prior_summary = client.get_session_summary(authorized(summary())).await.unwrap().into_inner();
        assert_eq!(prior_summary.recovery_state(), SessionRecoveryState::Finished);
        assert_eq!(prior_summary.latest_turn, Some(prior_ref.clone()));
        let status = |turn| GetSessionTurnStatusRequest { turn: Some(turn) };
        assert_eq!(client.get_session_turn_status(status(prior_ref.clone())).await.unwrap_err().code(), Code::Unauthenticated);
        assert_eq!(client.get_session_turn_status(authorized(status(SessionTurnRef {
            ordinal: u64::MAX, ..prior_ref.clone()
        }))).await.unwrap_err().code(), Code::NotFound);
        assert_eq!(client.get_session_turn_status(authorized(status(SessionTurnRef {
            session_id: "outside".into(), ..prior_ref.clone()
        }))).await.unwrap_err().code(), Code::NotFound);
        assert_eq!(client.get_session_summary(authorized(GetSessionSummaryRequest { session_id: "outside".into() })).await.unwrap_err().code(), Code::NotFound);
        assert_eq!(client.start_session_turn(start.clone()).await.unwrap_err().code(), Code::Unauthenticated);
        let mut missing_id = start.clone();
        missing_id.submission_id.clear();
        assert_eq!(client.start_session_turn(authorized(missing_id)).await.unwrap_err().code(), Code::InvalidArgument);
        let initial_request = authorized(start.clone());
        let lost_response = tokio::spawn(async move { client.start_session_turn(initial_request).await });
        store.committed.acquire().await.unwrap().forget();
        lost_response.abort();
        assert!(lost_response.await.unwrap_err().is_cancelled());
        let mut client = AppServerClient::connect(address.clone()).await.unwrap();
        assert_eq!(client.get_session_summary(authorized(summary())).await.unwrap_err().code(), Code::Aborted);
        let turn = client.start_session_turn(authorized(start.clone())).await.unwrap().into_inner().turn.unwrap();
        assert_eq!(turn.ordinal, earlier.get() + 1);
        assert_eq!(gateway.calls.load(Ordering::Relaxed), 0);
        let mut changed = start.clone();
        changed.user_text.push_str("-modified");
        assert_eq!(client.start_session_turn(authorized(changed)).await.unwrap_err().code(), Code::AlreadyExists);
        let mut changed = start.clone();
        changed.after_turn = 0;
        assert_eq!(client.start_session_turn(authorized(changed)).await.unwrap_err().code(), Code::AlreadyExists);
        let watch = |after_event_id| authorized(WatchSessionTurnRequest { turn: Some(turn.clone()), after_event_id });
        // Admission still holds the owner lock after committing the start.
        let mut request = watch(None);
        request.set_timeout(Duration::from_secs(1));
        let mut blocked_watch = client.watch_session_turn(request).await.unwrap().into_inner();
        assert_eq!(blocked_watch.message().await.unwrap_err().code(), Code::DeadlineExceeded);
        store.release.add_permits(1);
        let mut stream = client.watch_session_turn(watch(None)).await.unwrap().into_inner();
        assert!(matches!(stream.message().await.unwrap().unwrap().event, Some(Event::Started(_))));
        let first = stream.message().await.unwrap().unwrap();
        assert!(matches!(first.event, Some(Event::TextCheckpoint(delta)) if delta.text == FIRST_CHECKPOINT));
        let active = client.get_session_summary(authorized(summary())).await.unwrap().into_inner();
        assert_eq!(active.recovery_state(), SessionRecoveryState::Active);
        assert_eq!(active.latest_turn, Some(turn.clone()));
        assert_eq!(active.latest_event_id.as_deref(), Some(first.event_id.as_str()));
        let current = client.get_session_turn_status(authorized(status(turn.clone()))).await.unwrap().into_inner();
        assert_eq!(current.recovery_state(), SessionRecoveryState::Active);
        assert_eq!(current.turn, Some(turn.clone()));
        assert_eq!(current.latest_event_id, first.event_id);
        assert_eq!(current.outcome, None);
        let prior = client.get_session_turn_status(authorized(status(prior_ref.clone()))).await.unwrap().into_inner();
        assert_eq!(prior.recovery_state(), SessionRecoveryState::Finished);
        assert_eq!(prior.turn, Some(prior_ref));
        assert_eq!(Some(prior.latest_event_id), prior_summary.latest_event_id);
        assert_eq!(prior.outcome, Some(SessionTurnOutcome::RejectedBeforeDispatch.into()));
        drop(stream);
        drop(client);

        // Wait for the next checkpoint to be in the journal with no GUI attached.
        release.add_permits(1);
        loop {
            if storage.count_session_events(&project, &SessionId::new("gui-session").unwrap()).await.unwrap().get() == 6 { break; }
            tokio::task::yield_now().await;
        }
        let mut client = AppServerClient::connect(address.clone()).await.unwrap();
        assert_eq!(client.watch_session_turn(watch(Some("invented-cursor".into()))).await.unwrap_err().code(), Code::InvalidArgument);
        assert_eq!(client.start_session_turn(authorized(start.clone())).await.unwrap().into_inner().turn, Some(turn.clone()));
        let mut different_submission = start.clone();
        different_submission.submission_id = "gui-send-2".into();
        assert_eq!(client.start_session_turn(authorized(different_submission)).await.unwrap_err().code(), Code::FailedPrecondition);
        let mut resumed = client.watch_session_turn(watch(Some(first.event_id.clone()))).await.unwrap().into_inner();
        let next = resumed.message().await.unwrap().unwrap();
        assert!(next.event_id > first.event_id);
        assert!(matches!(next.event, Some(Event::TextCheckpoint(delta)) if delta.text == SECOND_CHECKPOINT));
        let mut foreign = turn.clone();
        foreign.session_id = "other-session".into();
        assert_eq!(client.cancel_session_turn(authorized(CancelSessionTurnRequest { turn: Some(foreign) })).await.unwrap_err().code(), Code::NotFound);
        let cancel = || authorized(CancelSessionTurnRequest { turn: Some(turn.clone()) });
        let read_cancellation = || authorized(GetSessionTurnCancellationRequest { turn: Some(turn.clone()) });
        let missing = client.get_session_turn_cancellation(read_cancellation()).await.unwrap().into_inner();
        assert!(matches!(missing.evidence, Some(Evidence::Unavailable(_))));
        assert_eq!(missing.outcome(), SessionTurnOutcome::Unspecified);
        let mut request = cancel();
        request.set_timeout(Duration::from_secs(1));
        let mut stop_client = client.clone();
        let timed_cancel = tokio::spawn(async move { stop_client.cancel_session_turn(request).await });
        cancellation_writing.acquire().await.unwrap().forget();
        let retry_request = cancel();
        let mut retry_client = client.clone();
        let retry_cancel = tokio::spawn(async move { retry_client.cancel_session_turn(retry_request).await });
        let code = timed_cancel.await.unwrap().unwrap_err().code();
        assert!(matches!(code, Code::DeadlineExceeded | Code::Cancelled), "{code:?}");
        release_cancellation.add_permits(1);
        // Read after the timeout while the concurrent retry waits on the same write.
        drop(client);
        let mut client = AppServerClient::connect(address.clone()).await.unwrap();
        let runtime = loop {
            let reply = client.get_session_turn_cancellation(read_cancellation()).await.unwrap().into_inner();
            if let Some(Evidence::Recorded(runtime)) = reply.evidence {
                assert_eq!(reply.turn, Some(turn.clone()));
                assert_eq!(reply.outcome, i32::from(SessionTurnOutcome::RemoteUncertain));
                break runtime;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        };
        assert!(runtime.completed && runtime.tracker_closed && runtime.cancellation_requested);
        assert_eq!(runtime.remaining_tasks, 0);
        assert_eq!(retry_cancel.await.unwrap().unwrap().into_inner().runtime, Some(runtime.clone()));
        let last = resumed.message().await.unwrap().unwrap();
        assert!(matches!(last.event, Some(Event::Outcome(outcome)) if outcome == i32::from(SessionTurnOutcome::RemoteUncertain)));
        assert!(resumed.message().await.unwrap().is_none());
        let unresolved = client.get_session_summary(authorized(summary())).await.unwrap().into_inner();
        assert_eq!(unresolved.recovery_state(), SessionRecoveryState::Unresolved);
        assert_eq!(unresolved.latest_turn, Some(turn.clone()));
        assert_eq!(unresolved.latest_event_id.as_deref(), Some(last.event_id.as_str()));
        let retired = client.get_session_turn_status(authorized(status(turn.clone()))).await.unwrap().into_inner();
        assert_eq!(retired.recovery_state(), SessionRecoveryState::Unresolved);
        assert_eq!(retired.latest_event_id, last.event_id);
        assert_eq!(retired.outcome, Some(SessionTurnOutcome::RemoteUncertain.into()));
        let mut suffix = client.watch_session_turn(watch(Some(last.event_id))).await.unwrap().into_inner();
        assert!(suffix.message().await.unwrap().is_none());
        let repeat = client.cancel_session_turn(cancel()).await.unwrap().into_inner();
        assert_eq!(repeat.runtime, Some(runtime.clone()));
        assert_eq!(cancellation_writing.available_permits(), 0);
        let recorded = client.get_session_turn_cancellation(read_cancellation()).await.unwrap().into_inner();
        assert_eq!(recorded.outcome(), SessionTurnOutcome::RemoteUncertain);
        assert_eq!(recorded.evidence, Some(Evidence::Recorded(runtime)));
        let retry = client.start_session_turn(authorized(start)).await.unwrap().into_inner();
        assert_eq!(retry.turn, Some(turn));
        assert_eq!(gateway.calls.load(Ordering::Relaxed), 1);

        let journal = ConversationJournal::open(storage, project, SessionId::new("gui-session").unwrap(), agent, |_| {}).await.unwrap();
        let observed = journal.unfinished().unwrap();
        assert_eq!(observed.partial_text, [FIRST_CHECKPOINT, SECOND_CHECKPOINT].concat());
        assert!(matches!(observed.outcome, Some(ConversationTurnOutcome::Uncertain {
            reason: ConversationUncertaintyReason::StreamInterrupted { reason: ModelStreamInterruption::CancelledLocally },
        })));
        stop.send(()).unwrap();
        server.await.unwrap().unwrap();
    }).await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn http2_session_watch_releases_admission_under_backpressure() {
    tokio::time::timeout(Duration::from_secs(20), async {
        let project = ProjectId::new("project").unwrap();
        let agent = AgentId::new("assistant").unwrap();
        let root = tempfile::tempdir().unwrap();
        let store = TursoAgentStorage::open_local(TursoAgentStorageConfig {
            path: root.path().join("watch.turso"),
            optimization_profile: TursoOptimizationProfile::AsyncIoOnlyCompatibility,
            batch_transaction_mode: TursoBatchTransactionMode::Immediate,
        }).await.unwrap();
        let endpoint = ModelEndpoint::new("openai", "gpt-fixture");
        let history_session = SessionId::new("history").unwrap();
        let live_session = SessionId::new("live").unwrap();
        let pressure_session = SessionId::new("pressure").unwrap();
        let mut journal = ConversationJournal::open(
            store.clone(), project.clone(), history_session.clone(), agent.clone(), |_| {},
        ).await.unwrap();
        let history_turn = journal.start(ConversationTurnStart {
            endpoint: endpoint.clone(), user_text: SUBMISSION_PAYLOAD.into(),
        }, 0).await.unwrap();
        // These journal checkpoints fit the wire size but expand in stored JSON.
        // Eleven such bodies exceed 8 MiB before the 16-record count limit.
        let padding = "\0".repeat(128 * 1024 - 2);
        let checkpoint = |sequence| format!("{sequence:02}{padding}");
        for sequence in 1..=20 {
            journal.checkpoint(history_turn, &checkpoint(sequence), sequence).await.unwrap();
        }
        journal.mark_uncertain(history_turn, ConversationUncertaintyReason::StreamInterrupted {
            reason: ModelStreamInterruption::CancelledLocally,
        }, 21).await.unwrap();
        store.append_session_event(SessionEventRecord {
            project_id: project.clone(), session_id: history_session.clone(), agent_id: agent.clone(),
            turn_id: marlin_agent_storage::TurnId::new("conversation:00000000000000000002").unwrap(),
            event_id: marlin_agent_storage::EventId::new("conversation:00000000000000000002:00-start").unwrap(),
            event_kind: "marlin.conversation.turn.started.v1".into(), causality_parent_event_id: None,
            body: vec![0; 8 * 1024 * 1024 + 1], created_at_unix_ms: 22,
        }).await.unwrap();
        let mut pressure = ConversationJournal::open(
            store.clone(), project.clone(), pressure_session.clone(), agent.clone(), |_| {},
        ).await.unwrap();
        let pressure_turn = pressure.start(ConversationTurnStart {
            endpoint: endpoint.clone(), user_text: SUBMISSION_PAYLOAD.into(),
        }, 0).await.unwrap();
        let pressure_chunk = "a".repeat(16 * 1024);
        for sequence in 1..=5 {
            pressure.checkpoint(pressure_turn, &pressure_chunk, sequence).await.unwrap();
        }
        pressure.mark_uncertain(pressure_turn, ConversationUncertaintyReason::StreamInterrupted {
            reason: ModelStreamInterruption::CancelledLocally,
        }, 6).await.unwrap();
        let binding = |id: &SessionId| {
            let (runtime, _) = TokioAgentRuntime::from_build_request(
                TokioAgentRuntimeBuildRequest::runtime_root(16).with_session(AgentSessionContext::root(
                    id.as_str(), ContextVisibility::default_runtime(),
                )),
            );
            SessionBinding::new(store.clone(), project.clone(), agent.clone(), runtime).unwrap()
        };
        let history = binding(&history_session);
        let live = binding(&live_session);
        let pressure = binding(&pressure_session);
        let gateway = Arc::new(Gateway {
            calls: AtomicUsize::new(0), release: Arc::new(Semaphore::new(0)), dispatch_log: None,
        });
        let service = AppServer::new(endpoint, gateway.clone(), |_| Ok(())).unwrap()
            .with_session_lookup(move |_, id, _| {
                let binding = match id.as_str() {
                    "history" => Ok(history.clone()),
                    "live" => Ok(live.clone()),
                    "pressure" => Ok(pressure.clone()),
                    _ => Err(Status::not_found("unknown fixture Session")),
                };
                async move { binding }
            }).into_service();
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).await.unwrap();
        let address = format!("http://{}", listener.local_addr().unwrap());
        let (stop, shutdown) = oneshot::channel();
        let server = tokio::spawn(Server::builder().add_service(service).serve_with_incoming_shutdown(
            TcpListenerStream::new(listener), async { let _ = shutdown.await; },
        ));
        let channel = Endpoint::from_shared(address.clone()).unwrap()
            .http2_adaptive_window(false).initial_stream_window_size(0).connect().await.unwrap();
        let mut slow = AppServerClient::new(channel);
        let mut control = AppServerClient::connect(address).await.unwrap();
        let history_ref = SessionTurnRef {
            session_id: history_session.as_str().into(), ordinal: history_turn.get(),
        };
        let watch = || WatchSessionTurnRequest { turn: Some(history_ref.clone()), after_event_id: None };
        let blocked_watch = || WatchSessionTurnRequest {
            turn: Some(SessionTurnRef { session_id: pressure_session.as_str().into(), ordinal: pressure_turn.get() }),
            after_event_id: None,
        };
        let live_turn = control.start_session_turn(StartSessionTurnRequest {
            session_id: live_session.as_str().into(), user_text: SUBMISSION_PAYLOAD.into(),
            after_turn: 0, submission_id: "watch-control-1".into(),
        }).await.unwrap().into_inner().turn.unwrap();
        while store.count_session_events(&project, &live_session).await.unwrap().get() < 3 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        let mut held = Vec::new();
        for _ in 0..16 {
            held.push(slow.watch_session_turn(blocked_watch()).await.unwrap().into_inner());
        }
        assert_eq!(slow.watch_session_turn(blocked_watch()).await.unwrap_err().code(), Code::ResourceExhausted);

        // Control has a separate connection and does not wait for a watch slot.
        let summary = control.get_session_summary(GetSessionSummaryRequest {
            session_id: live_session.as_str().into(),
        }).await.unwrap().into_inner();
        assert_eq!(summary.latest_turn, Some(live_turn.clone()));
        assert_eq!(summary.recovery_state(), SessionRecoveryState::Active);
        let receipt = control.cancel_session_turn(CancelSessionTurnRequest { turn: Some(live_turn) })
            .await.unwrap().into_inner().runtime.unwrap();
        assert!(receipt.completed && receipt.cancellation_requested);
        assert_eq!(slow.watch_session_turn(blocked_watch()).await.unwrap_err().code(), Code::ResourceExhausted);

        drop(held.pop().unwrap());
        let expired_stream = loop {
            let mut expiring = Request::new(blocked_watch());
            expiring.set_timeout(Duration::from_secs(3));
            match slow.watch_session_turn(expiring).await {
                Ok(response) => break response.into_inner(),
                Err(error) if error.code() == Code::ResourceExhausted => {
                    tokio::time::sleep(Duration::from_millis(20)).await;
                }
                Err(error) => panic!("unexpected watch admission status: {error}"),
            }
        };
        assert_eq!(slow.watch_session_turn(blocked_watch()).await.unwrap_err().code(), Code::ResourceExhausted);

        // The timed stream stays alive on the client with zero HTTP/2 credit.
        let replacement = loop {
            match slow.watch_session_turn(blocked_watch()).await {
                Ok(response) => break response.into_inner(),
                Err(error) if error.code() == Code::ResourceExhausted => {
                    tokio::time::sleep(Duration::from_millis(20)).await;
                }
                Err(error) => panic!("unexpected watch admission status: {error}"),
            }
        };
        drop(expired_stream);
        drop(held);
        drop(replacement);
        drop(slow);

        let mut replay = loop {
            match control.watch_session_turn(watch()).await {
                Ok(response) => break response.into_inner(),
                Err(error) if error.code() == Code::ResourceExhausted => {
                    tokio::time::sleep(Duration::from_millis(20)).await;
                }
                Err(error) => panic!("unexpected replay status: {error}"),
            }
        };
        let started = replay.message().await.unwrap().unwrap();
        assert!(matches!(started.event, Some(Event::Started(_))));
        let first = replay.message().await.unwrap().unwrap();
        assert!(matches!(first.event, Some(Event::TextCheckpoint(value)) if value.text == checkpoint(1)));
        drop(replay);
        let mut cursor = first.event_id;
        let mut suffix = control.watch_session_turn(WatchSessionTurnRequest {
            after_event_id: Some(cursor.clone()), ..watch()
        }).await.unwrap().into_inner();
        for sequence in 2..=20 {
            let event = suffix.message().await.unwrap().unwrap();
            assert!(event.event_id > cursor);
            assert!(matches!(event.event, Some(Event::TextCheckpoint(value)) if value.text == checkpoint(sequence)));
            cursor = event.event_id;
        }
        let outcome = suffix.message().await.unwrap().unwrap();
        assert!(outcome.event_id > cursor);
        assert!(matches!(outcome.event, Some(Event::Outcome(value)) if value == i32::from(SessionTurnOutcome::RemoteUncertain)));
        assert!(suffix.message().await.unwrap().is_none());
        let mut rejected = control.watch_session_turn(WatchSessionTurnRequest {
            turn: Some(SessionTurnRef { ordinal: 2, ..history_ref }), after_event_id: None,
        }).await.unwrap().into_inner();
        assert_eq!(rejected.message().await.unwrap_err().code(), Code::ResourceExhausted);
        let refused = control.start_session_turn(StartSessionTurnRequest {
            session_id: history_session.as_str().into(), user_text: SUBMISSION_PAYLOAD.into(),
            after_turn: 2, submission_id: "oversized-history".into(),
        }).await.unwrap_err();
        assert_eq!(refused.code(), Code::ResourceExhausted);
        assert_eq!(store.count_session_events(&project, &history_session).await.unwrap().get(), 23);
        assert_eq!(gateway.calls.load(Ordering::Relaxed), 1);
        stop.send(()).unwrap();
        server.await.unwrap().unwrap();
    }).await.expect("HTTP/2 watch pressure fixture exceeded its deadline");
}

const FIXTURE_ROOT: &str = "MARLIN_APP_SERVER_RESTART_FIXTURE_ROOT";
const FIXTURE_PHASE: &str = "MARLIN_APP_SERVER_RESTART_FIXTURE_PHASE";

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "subprocess fixture for the Turso restart test"]
async fn turso_app_server_process() {
    let root = PathBuf::from(std::env::var_os(FIXTURE_ROOT).expect("fixture directory"));
    let phase = std::env::var(FIXTURE_PHASE).unwrap();
    let storage = TursoAgentStorage::open_local(TursoAgentStorageConfig {
        path: root.join("session.turso"),
        optimization_profile: TursoOptimizationProfile::AsyncIoOnlyCompatibility,
        batch_transaction_mode: TursoBatchTransactionMode::Immediate,
    })
    .await
    .unwrap();
    let project = ProjectId::new("project").unwrap();
    let agent = AgentId::new("assistant").unwrap();
    let (runtime, _events) = TokioAgentRuntime::from_build_request(
        TokioAgentRuntimeBuildRequest::runtime_root(16).with_session(AgentSessionContext::root(
            "gui-session",
            ContextVisibility::default_runtime(),
        )),
    );
    let cancellation_committed = Arc::new(Semaphore::new(0));
    let store = PausedStartStore {
        storage,
        pause_replay: None,
        before_start_commit: None,
        committed: Arc::new(Semaphore::new(0)),
        release: Arc::new(Semaphore::new(0)),
        pause_cancellation: (phase == "cancel-response-lost").then(|| CancellationWritePause {
            reached: cancellation_committed.clone(),
            release: Arc::new(Semaphore::new(0)),
            after_commit: true,
        }),
        fail_uncertain_write: phase == "cancel-write-error",
    };
    let binding = SessionBinding::new(store.clone(), project.clone(), agent, runtime).unwrap();
    let gateway = Arc::new(Gateway {
        calls: AtomicUsize::new(0),
        release: Arc::new(Semaphore::new(0)),
        dispatch_log: Some(root.join("dispatches")),
    });
    let service = AppServer::new(
        ModelEndpoint::new("openai", "gpt-fixture"),
        gateway.clone(),
        |metadata| {
            if metadata
                .get("authorization")
                .is_some_and(|value| value == "Bearer fixture")
            {
                Ok(())
            } else {
                Err(Status::unauthenticated("authorization required"))
            }
        },
    )
    .unwrap()
    .with_session(binding)
    .into_service();
    let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
        .await
        .unwrap();
    let address = listener.local_addr().unwrap();
    let (stop, shutdown) = oneshot::channel();
    let server = tokio::spawn(
        Server::builder()
            .add_service(service)
            .serve_with_incoming_shutdown(TcpListenerStream::new(listener), async {
                let _ = shutdown.await;
            }),
    );
    println!("\nREADY http://{address}");

    let mut control = tokio::io::stdin();
    if phase == "initial" {
        store.committed.acquire().await.unwrap().forget();
        println!("START_COMMITTED");
        assert_eq!(control.read_u8().await.unwrap(), b'R');
        store.release.add_permits(1);
        // The parent kills this process after observing a durable checkpoint.
        std::future::pending::<()>().await;
    }
    if phase == "cancel-response-lost" {
        store.release.add_permits(1);
        cancellation_committed.acquire().await.unwrap().forget();
        println!("CANCELLATION_COMMITTED");
        std::future::pending::<()>().await;
    }
    if matches!(phase.as_str(), "cancelled" | "cancel-write-error") {
        store.release.add_permits(1);
        std::future::pending::<()>().await;
    }
    let event_count = match phase.as_str() {
        "reopened" => 3,
        "reopened-cancelled" => 5,
        "reopened-cancel-write-error" => 4,
        _ => panic!("unexpected fixture phase {phase}"),
    };
    assert_eq!(control.read_u8().await.unwrap(), b'Q');
    stop.send(()).unwrap();
    server.await.unwrap().unwrap();
    assert_eq!(gateway.calls.load(Ordering::Relaxed), 0);
    let session = SessionId::new("gui-session").unwrap();
    assert_eq!(
        store
            .storage
            .count_session_events(&project, &session)
            .await
            .unwrap()
            .get(),
        event_count
    );
    println!("REOPENED_NO_DISPATCH");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn turso_http2_recovers_cancellation_evidence_after_process_restart() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let directory = tempfile::tempdir().unwrap();
        let (mut initial, mut output) =
            start_turso_process(directory.path(), "cancel-response-lost");
        let initial_pid = initial.id().unwrap();
        let mut client = AppServerClient::connect(fixture_marker(&mut output, "READY ").await)
            .await
            .unwrap();
        let turn = client
            .start_session_turn(authorized(StartSessionTurnRequest {
                session_id: "gui-session".into(),
                user_text: SUBMISSION_PAYLOAD.into(),
                after_turn: 0,
                submission_id: "cancelled-send-1".into(),
            }))
            .await
            .unwrap()
            .into_inner()
            .turn
            .unwrap();
        let mut events = client
            .watch_session_turn(authorized(WatchSessionTurnRequest {
                turn: Some(turn.clone()),
                after_event_id: None,
            }))
            .await
            .unwrap()
            .into_inner();
        assert!(matches!(
            events.message().await.unwrap().unwrap().event,
            Some(Event::Started(_))
        ));
        let checkpoint = events.message().await.unwrap().unwrap();
        assert!(matches!(
            &checkpoint.event,
            Some(Event::TextCheckpoint(text)) if text.text == FIRST_CHECKPOINT
        ));
        let request = authorized(CancelSessionTurnRequest {
            turn: Some(turn.clone()),
        });
        let mut cancelling_client = client.clone();
        let lost_response =
            tokio::spawn(async move { cancelling_client.cancel_session_turn(request).await });
        fixture_marker(&mut output, "CANCELLATION_COMMITTED").await;
        assert!(!lost_response.is_finished());
        lost_response.abort();
        assert!(lost_response.await.unwrap_err().is_cancelled());
        let committed = client
            .get_session_turn_cancellation(authorized(GetSessionTurnCancellationRequest {
                turn: Some(turn.clone()),
            }))
            .await
            .unwrap()
            .into_inner();
        assert_eq!(committed.turn, Some(turn.clone()));
        assert_eq!(committed.outcome(), SessionTurnOutcome::RemoteUncertain);
        let Some(Evidence::Recorded(receipt)) = committed.evidence else {
            panic!("cancellation observation must be durable before killing the host");
        };
        assert!(receipt.completed && receipt.cancellation_requested);
        drop(events);
        drop(client);
        initial.kill().await.unwrap();
        assert!(!initial.wait().await.unwrap().success());

        let (mut reopened, mut output) =
            start_turso_process(directory.path(), "reopened-cancelled");
        assert_ne!(reopened.id().unwrap(), initial_pid);
        let mut client = AppServerClient::connect(fixture_marker(&mut output, "READY ").await)
            .await
            .unwrap();
        let recovered = client
            .get_session_turn_cancellation(authorized(GetSessionTurnCancellationRequest {
                turn: Some(turn.clone()),
            }))
            .await
            .unwrap()
            .into_inner();
        assert_eq!(recovered.outcome(), SessionTurnOutcome::RemoteUncertain);
        assert_eq!(recovered.turn, Some(turn.clone()));
        assert_eq!(
            recovered.evidence,
            Some(Evidence::Recorded(receipt.clone()))
        );
        let retried = client
            .cancel_session_turn(authorized(CancelSessionTurnRequest {
                turn: Some(turn.clone()),
            }))
            .await
            .unwrap()
            .into_inner();
        assert_eq!(retried.outcome(), SessionTurnOutcome::RemoteUncertain);
        assert_eq!(retried.runtime, Some(receipt));
        let status = client
            .get_session_turn_status(authorized(GetSessionTurnStatusRequest {
                turn: Some(turn.clone()),
            }))
            .await
            .unwrap()
            .into_inner();
        assert_eq!(status.recovery_state(), SessionRecoveryState::Unresolved);
        assert_eq!(
            status.outcome,
            Some(SessionTurnOutcome::RemoteUncertain.into())
        );
        let mut suffix = client
            .watch_session_turn(authorized(WatchSessionTurnRequest {
                turn: Some(turn),
                after_event_id: Some(checkpoint.event_id),
            }))
            .await
            .unwrap()
            .into_inner();
        let terminal = suffix.message().await.unwrap().unwrap();
        assert_eq!(terminal.event_id, status.latest_event_id);
        assert_eq!(
            terminal.event,
            Some(Event::Outcome(SessionTurnOutcome::RemoteUncertain.into()))
        );
        assert!(suffix.message().await.unwrap().is_none());
        drop(suffix);
        drop(client);
        reopened
            .stdin
            .as_mut()
            .unwrap()
            .write_all(b"Q")
            .await
            .unwrap();
        fixture_marker(&mut output, "REOPENED_NO_DISPATCH").await;
        assert!(reopened.wait().await.unwrap().success());
        assert_eq!(
            std::fs::read_to_string(directory.path().join("dispatches")).unwrap(),
            format!("{initial_pid}\n")
        );
    })
    .await
    .expect("Turso HTTP/2 cancellation recovery exceeded its deadline");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn turso_http2_recovers_retirement_when_terminal_commit_fails() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let directory = tempfile::tempdir().unwrap();
        let (mut initial, mut output) = start_turso_process(directory.path(), "cancel-write-error");
        let initial_pid = initial.id().unwrap();
        let mut client = AppServerClient::connect(fixture_marker(&mut output, "READY ").await)
            .await
            .unwrap();
        let submission = StartSessionTurnRequest {
            session_id: "gui-session".into(),
            user_text: SUBMISSION_PAYLOAD.into(),
            after_turn: 0,
            submission_id: "cancel-write-error-1".into(),
        };
        let turn = client
            .start_session_turn(authorized(submission.clone()))
            .await
            .unwrap()
            .into_inner()
            .turn
            .unwrap();
        let mut events = client
            .watch_session_turn(authorized(WatchSessionTurnRequest {
                turn: Some(turn.clone()),
                after_event_id: None,
            }))
            .await
            .unwrap()
            .into_inner();
        assert!(matches!(
            events.message().await.unwrap().unwrap().event,
            Some(Event::Started(_))
        ));
        assert!(matches!(
            events.message().await.unwrap().unwrap().event,
            Some(Event::TextCheckpoint(_))
        ));
        drop(events);

        let cancel = || {
            authorized(CancelSessionTurnRequest {
                turn: Some(turn.clone()),
            })
        };
        let read = || {
            authorized(GetSessionTurnCancellationRequest {
                turn: Some(turn.clone()),
            })
        };
        let error = client.cancel_session_turn(cancel()).await.unwrap_err();
        assert_eq!(error.code(), Code::Unavailable);
        assert_eq!(error.message(), "Session event storage unavailable");
        let recorded = client
            .get_session_turn_cancellation(read())
            .await
            .unwrap()
            .into_inner();
        assert_eq!(recorded.outcome(), SessionTurnOutcome::Unspecified);
        let Some(Evidence::Recorded(receipt)) = recorded.evidence else {
            panic!("expected a persisted Runtime retirement receipt");
        };
        assert!(receipt.completed && receipt.cancellation_requested);
        assert_eq!(receipt.remaining_tasks, 0);

        drop(client);
        initial.kill().await.unwrap();
        assert!(!initial.wait().await.unwrap().success());
        let (mut reopened, mut output) =
            start_turso_process(directory.path(), "reopened-cancel-write-error");
        assert_ne!(reopened.id().unwrap(), initial_pid);
        let mut client = AppServerClient::connect(fixture_marker(&mut output, "READY ").await)
            .await
            .unwrap();
        let recovered = client
            .get_session_turn_cancellation(read())
            .await
            .unwrap()
            .into_inner();
        assert_eq!(recovered.outcome(), SessionTurnOutcome::Unspecified);
        assert_eq!(
            recovered.evidence,
            Some(Evidence::Recorded(receipt.clone()))
        );
        let retry = client
            .cancel_session_turn(cancel())
            .await
            .unwrap()
            .into_inner();
        assert_eq!(retry.runtime, Some(receipt));
        assert_eq!(retry.outcome(), SessionTurnOutcome::Unspecified);
        let status = client
            .get_session_turn_status(authorized(GetSessionTurnStatusRequest {
                turn: Some(turn.clone()),
            }))
            .await
            .unwrap()
            .into_inner();
        assert_eq!(status.recovery_state(), SessionRecoveryState::Unresolved);
        assert_eq!(status.outcome, None);
        let start = client
            .start_session_turn(authorized(submission))
            .await
            .unwrap()
            .into_inner();
        assert_eq!(start.turn, Some(turn));

        drop(client);
        reopened
            .stdin
            .as_mut()
            .unwrap()
            .write_all(b"Q")
            .await
            .unwrap();
        fixture_marker(&mut output, "REOPENED_NO_DISPATCH").await;
        assert!(reopened.wait().await.unwrap().success());
        assert_eq!(
            std::fs::read_to_string(directory.path().join("dispatches")).unwrap(),
            format!("{initial_pid}\n")
        );
    })
    .await
    .expect("Turso HTTP/2 failed-terminal recovery exceeded its deadline");
}

async fn fixture_marker(lines: &mut Lines<BufReader<ChildStdout>>, prefix: &str) -> String {
    while let Some(line) = lines.next_line().await.unwrap() {
        if let Some(value) = line.strip_prefix(prefix) {
            return value.to_owned();
        }
    }
    panic!("server process exited before {prefix}");
}

fn start_turso_process(
    root: &std::path::Path,
    phase: &str,
) -> (Child, Lines<BufReader<ChildStdout>>) {
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--ignored",
            "--exact",
            "--nocapture",
            "turso_app_server_process",
        ])
        .env(FIXTURE_ROOT, root)
        .env(FIXTURE_PHASE, phase)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let lines = BufReader::new(child.stdout.take().unwrap()).lines();
    (child, lines)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn turso_http2_recovers_lost_submission_after_process_restart() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let directory = tempfile::tempdir().unwrap();
        let (mut initial, mut output) = start_turso_process(directory.path(), "initial");
        let initial_pid = initial.id().unwrap();
        let address = fixture_marker(&mut output, "READY ").await;
        let mut client = AppServerClient::connect(address.clone()).await.unwrap();
        let request = StartSessionTurnRequest {
            session_id: "gui-session".into(),
            user_text: SUBMISSION_PAYLOAD.into(),
            after_turn: 0,
            submission_id: "restart-send-1".into(),
        };
        let submitted = authorized(request.clone());
        let response = tokio::spawn(async move { client.start_session_turn(submitted).await });
        fixture_marker(&mut output, "START_COMMITTED").await;
        response.abort();
        assert!(response.await.unwrap_err().is_cancelled());
        initial
            .stdin
            .as_mut()
            .unwrap()
            .write_all(b"R")
            .await
            .unwrap();

        // The fixture starts with an empty Session. Observe the first dispatch
        // and committed checkpoint without ever receiving the start response.
        let expected = SessionTurnRef {
            session_id: request.session_id.clone(),
            ordinal: 1,
        };
        let watch = || {
            authorized(WatchSessionTurnRequest {
                turn: Some(expected.clone()),
                after_event_id: None,
            })
        };
        let mut client = AppServerClient::connect(address).await.unwrap();
        let mut events = client
            .watch_session_turn(watch())
            .await
            .unwrap()
            .into_inner();
        assert!(matches!(
            events.message().await.unwrap().unwrap().event,
            Some(Event::Started(_))
        ));
        let checkpoint = events.message().await.unwrap().unwrap();
        assert!(
            matches!(&checkpoint.event, Some(Event::TextCheckpoint(text)) if text.text == FIRST_CHECKPOINT)
        );
        drop(events);
        drop(client);
        initial.kill().await.unwrap();
        assert!(!initial.wait().await.unwrap().success());

        let (mut reopened, mut output) = start_turso_process(directory.path(), "reopened");
        assert_ne!(reopened.id().unwrap(), initial_pid);
        let mut client = AppServerClient::connect(fixture_marker(&mut output, "READY ").await)
            .await
            .unwrap();
        let summary = client
            .get_session_summary(authorized(GetSessionSummaryRequest {
                session_id: request.session_id.clone(),
            }))
            .await
            .unwrap()
            .into_inner();
        assert_eq!(summary.recovery_state(), SessionRecoveryState::Unresolved);
        assert_eq!(summary.latest_turn, Some(expected.clone()));
        assert_eq!(
            summary.latest_event_id.as_deref(),
            Some(checkpoint.event_id.as_str())
        );
        let status = client
            .get_session_turn_status(authorized(GetSessionTurnStatusRequest {
                turn: Some(expected.clone()),
            }))
            .await
            .unwrap()
            .into_inner();
        assert_eq!(status.recovery_state(), SessionRecoveryState::Unresolved);
        assert_eq!(status.latest_event_id, checkpoint.event_id);
        assert_eq!(status.outcome, None);
        let readback = client
            .get_session_turn_cancellation(authorized(GetSessionTurnCancellationRequest {
                turn: Some(expected.clone()),
            }))
            .await
            .unwrap()
            .into_inner();
        assert!(matches!(readback.evidence, Some(Evidence::Unavailable(_))));
        assert_eq!(readback.outcome(), SessionTurnOutcome::Unspecified);
        let recovered = client
            .start_session_turn(authorized(request.clone()))
            .await
            .unwrap()
            .into_inner();
        assert_eq!(recovered.turn, Some(expected.clone()));
        let mut changed = request;
        changed.user_text.push_str("-modified");
        assert_eq!(
            client
                .start_session_turn(authorized(changed))
                .await
                .unwrap_err()
                .code(),
            Code::AlreadyExists
        );

        let mut events = client
            .watch_session_turn(watch())
            .await
            .unwrap()
            .into_inner();
        assert!(matches!(
            events.message().await.unwrap().unwrap().event,
            Some(Event::Started(_))
        ));
        assert_eq!(events.message().await.unwrap(), Some(checkpoint));
        assert_eq!(
            events.message().await.unwrap_err().code(),
            Code::FailedPrecondition
        );
        drop(events);
        drop(client);
        reopened
            .stdin
            .as_mut()
            .unwrap()
            .write_all(b"Q")
            .await
            .unwrap();
        fixture_marker(&mut output, "REOPENED_NO_DISPATCH").await;
        assert!(reopened.wait().await.unwrap().success());
        assert_eq!(
            std::fs::read_to_string(directory.path().join("dispatches")).unwrap(),
            format!("{initial_pid}\n")
        );
    })
    .await
    .expect("Turso HTTP/2 restart fixture exceeded its deadline");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "subprocess fixture for a provisioned App Server"]
async fn provisioned_app_server_process() {
    use marlin_agent_app_server::SessionOpen;
    use std::collections::HashMap;
    use tokio::sync::Mutex;

    let root = PathBuf::from(std::env::var_os(FIXTURE_ROOT).expect("fixture directory"));
    let storage = TursoAgentStorage::open_local(TursoAgentStorageConfig {
        path: root.join("session.turso"),
        optimization_profile: TursoOptimizationProfile::AsyncIoOnlyCompatibility,
        batch_transaction_mode: TursoBatchTransactionMode::Immediate,
    })
    .await
    .unwrap();
    let bindings = Arc::new(Mutex::new(HashMap::new()));
    let gateway = Arc::new(Gateway {
        calls: AtomicUsize::new(0),
        release: Arc::new(Semaphore::new(0)),
        dispatch_log: Some(root.join("dispatches")),
    });
    let service = AppServer::new(
        ModelEndpoint::new("openai", "gpt-fixture"),
        gateway,
        |metadata| fixture_principal(metadata).map(|_| ()),
    )
    .unwrap()
    .with_session_lookup(move |metadata, id, mode| {
        let bindings = bindings.clone();
        let storage = storage.clone();
        async move {
            let principal = fixture_principal(&metadata)?;
            let project = ProjectId::new("project").unwrap();
            let agent = AgentId::new("assistant").unwrap();
            let mut bindings = bindings.lock().await;
            if mode == SessionOpen::Existing {
                let admission = storage
                    .list_session_events_page(SessionEventPageRequest::new(
                        project.clone(),
                        id.clone(),
                        marlin_agent_storage::StoragePageLimit::new(1).unwrap(),
                    ))
                    .await
                    .map_err(|_| Status::internal("fixture Session admission lookup"))?;
                let allowed = admission.items.first().is_some_and(|event| {
                    event.turn_id.as_str() == "app-session"
                        && event.event_id.as_str() == "admitted"
                        && event.event_kind == "fixture.session.admitted"
                        && event.agent_id == agent
                        && event.body == principal.as_bytes()
                });
                if !allowed {
                    return Err(Status::not_found("Session is outside the fixture scope"));
                }
                if let Some(binding) = bindings.get(&id) {
                    return Ok(SessionBinding::clone(binding));
                }
            }
            if bindings.len() >= 4 {
                return Err(Status::resource_exhausted("fixture Session limit reached"));
            }
            if mode == SessionOpen::Create {
                // The fixture binds the authenticated principal before returning
                // the handle, including Sessions with no conversation turns yet.
                storage
                    .append_session_event(SessionEventRecord {
                        project_id: project.clone(),
                        session_id: id.clone(),
                        agent_id: agent.clone(),
                        turn_id: marlin_agent_storage::TurnId::new("app-session").unwrap(),
                        event_id: marlin_agent_storage::EventId::new("admitted").unwrap(),
                        event_kind: "fixture.session.admitted".into(),
                        causality_parent_event_id: None,
                        body: principal.as_bytes().to_vec(),
                        created_at_unix_ms: 0,
                    })
                    .await
                    .map_err(|error| match error {
                        marlin_agent_storage::StorageError::DuplicateSessionEvent { .. } => {
                            Status::already_exists("fixture Session identity is in use")
                        }
                        _ => Status::internal("fixture Session admission write"),
                    })?;
            }
            let (runtime, _events) = TokioAgentRuntime::from_build_request(
                TokioAgentRuntimeBuildRequest::runtime_root(16).with_session(
                    AgentSessionContext::root(id.as_str(), ContextVisibility::default_runtime()),
                ),
            );
            let binding = SessionBinding::new(storage, project, agent, runtime)
                .map_err(|_| Status::internal("fixture Session binding"))?;
            bindings.insert(id, binding.clone());
            Ok(binding)
        }
    })
    .into_service();

    let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
        .await
        .unwrap();
    println!("\nREADY http://{}", listener.local_addr().unwrap());
    Server::builder()
        .add_service(service)
        .serve_with_incoming(TcpListenerStream::new(listener))
        .await
        .unwrap();
}

fn fixture_principal(metadata: &tonic::metadata::MetadataMap) -> Result<&'static str, Status> {
    match metadata
        .get("authorization")
        .and_then(|value| value.to_str().ok())
    {
        Some("Bearer alice") => Ok("alice"),
        Some("Bearer bob") => Ok("bob"),
        _ => Err(Status::unauthenticated("fixture principal required")),
    }
}
