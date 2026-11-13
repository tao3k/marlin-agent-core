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
        CancelSessionTurnRequest, GetSessionSummaryRequest, SessionRecoveryState,
        SessionTurnOutcome, SessionTurnRef, StartSessionTurnRequest, WatchSessionTurnRequest,
        app_server_client::AppServerClient, session_turn_event::Event,
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
    sync::{Semaphore, oneshot},
};
use tokio_stream::wrappers::TcpListenerStream;
use tonic::{Code, Request, Status, transport::Server};

struct Gateway {
    calls: AtomicUsize,
    release: Arc<Semaphore>,
    dispatch_log: Option<PathBuf>,
}

#[derive(Clone)]
struct PausedStartStore<S> {
    storage: S,
    committed: Arc<Semaphore>,
    release: Arc<Semaphore>,
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
        self.storage.append_session_event(record)
    }

    fn append_session_events_atomically(
        &self,
        records: Vec<SessionEventRecord>,
    ) -> StorageFuture<'_, SessionEventBatchWriteReceipt> {
        Box::pin(async move {
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
        self.storage.list_session_events_page(request)
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
        let store = PausedStartStore {
            storage: storage.clone(),
            committed: Arc::new(Semaphore::new(0)),
            release: Arc::new(Semaphore::new(0)),
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
        let start = StartSessionTurnRequest { session_id: "gui-session".into(), user_text: "请开始".into(), after_turn: 0, submission_id: "gui-send-1".into() };
        let summary = || GetSessionSummaryRequest { session_id: start.session_id.clone() };
        assert_eq!(client.get_session_summary(summary()).await.unwrap_err().code(), Code::Unauthenticated);
        let empty = client.get_session_summary(authorized(summary())).await.unwrap().into_inner();
        assert_eq!(empty.recovery_state(), SessionRecoveryState::Empty);
        assert!(empty.latest_turn.is_none() && empty.latest_event_id.is_none());
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
        assert_eq!(turn.ordinal, 1);
        assert_eq!(gateway.calls.load(Ordering::Relaxed), 0);
        let mut changed = start.clone();
        changed.user_text.push('！');
        assert_eq!(client.start_session_turn(authorized(changed)).await.unwrap_err().code(), Code::AlreadyExists);
        let mut changed = start.clone();
        changed.after_turn = 1;
        assert_eq!(client.start_session_turn(authorized(changed)).await.unwrap_err().code(), Code::AlreadyExists);
        store.release.add_permits(1);
        let watch = |after_event_id| authorized(WatchSessionTurnRequest { turn: Some(turn.clone()), after_event_id });
        let mut stream = client.watch_session_turn(watch(None)).await.unwrap().into_inner();
        assert!(matches!(stream.message().await.unwrap().unwrap().event, Some(Event::Started(_))));
        let first = stream.message().await.unwrap().unwrap();
        assert!(matches!(first.event, Some(Event::TextCheckpoint(delta)) if delta.text == "第一段"));
        let active = client.get_session_summary(authorized(summary())).await.unwrap().into_inner();
        assert_eq!(active.recovery_state(), SessionRecoveryState::Active);
        assert_eq!(active.latest_turn, Some(turn.clone()));
        assert_eq!(active.latest_event_id.as_deref(), Some(first.event_id.as_str()));
        drop(stream);
        drop(client);

        // Wait for the next checkpoint to be in the journal with no GUI attached.
        release.add_permits(1);
        loop {
            if storage.count_session_events(&project, &SessionId::new("gui-session").unwrap()).await.unwrap().get() == 4 { break; }
            tokio::task::yield_now().await;
        }
        let mut client = AppServerClient::connect(address).await.unwrap();
        assert_eq!(client.watch_session_turn(watch(Some("invented-cursor".into()))).await.unwrap_err().code(), Code::InvalidArgument);
        assert_eq!(client.start_session_turn(authorized(start.clone())).await.unwrap().into_inner().turn, Some(turn.clone()));
        let mut different_submission = start.clone();
        different_submission.submission_id = "gui-send-2".into();
        assert_eq!(client.start_session_turn(authorized(different_submission)).await.unwrap_err().code(), Code::FailedPrecondition);
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
        let unresolved = client.get_session_summary(authorized(summary())).await.unwrap().into_inner();
        assert_eq!(unresolved.recovery_state(), SessionRecoveryState::Unresolved);
        assert_eq!(unresolved.latest_turn, Some(turn.clone()));
        assert_eq!(unresolved.latest_event_id.as_deref(), Some(last.event_id.as_str()));
        let mut suffix = client.watch_session_turn(watch(Some(last.event_id))).await.unwrap().into_inner();
        assert!(suffix.message().await.unwrap().is_none());
        let repeat = client.cancel_session_turn(cancel()).await.unwrap().into_inner();
        assert!(!repeat.runtime.unwrap().cancellation_requested);
        let retry = client.start_session_turn(authorized(start)).await.unwrap().into_inner();
        assert_eq!(retry.turn, Some(turn));
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
    let store = PausedStartStore {
        storage,
        committed: Arc::new(Semaphore::new(0)),
        release: Arc::new(Semaphore::new(0)),
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
    assert_eq!(phase, "reopened");
    assert_eq!(control.read_u8().await.unwrap(), b'Q');
    stop.send(()).unwrap();
    server.await.unwrap().unwrap();
    assert_eq!(gateway.calls.load(Ordering::Relaxed), 0);
    let session = SessionId::new("gui-session").unwrap();
    // Original start, submission claim, and first text checkpoint.
    assert_eq!(
        store
            .storage
            .count_session_events(&project, &session)
            .await
            .unwrap()
            .get(),
        3
    );
    println!("REOPENED_NO_DISPATCH");
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
            user_text: "请开始".into(),
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
            matches!(&checkpoint.event, Some(Event::TextCheckpoint(text)) if text.text == "第一段")
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
        let recovered = client
            .start_session_turn(authorized(request.clone()))
            .await
            .unwrap()
            .into_inner();
        assert_eq!(recovered.turn, Some(expected.clone()));
        let mut changed = request;
        changed.user_text.push('！');
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
