// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
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
    AppServer,
    v1::{
        ConversationMessage, GetCapabilitiesRequest, MessageRole, StreamTurnRequest,
        app_server_client::AppServerClient, turn_event::Event,
    },
};
use marlin_agent_protocol::{
    ModelEndpoint, ModelGatewayCompletionChoice, ModelGatewayCompletionResponse,
    ModelGatewayFuture, ModelGatewayRequest, ModelGatewayResult, ModelGatewayTransport,
    assistant_gateway_message,
};
use marlin_agent_stream::{
    ModelGatewayEventStream, ModelStreamChunk, ModelStreamEvent, ModelStreamingGateway,
    ModelThinkingChunk,
};
use tokio::{
    net::TcpListener,
    sync::{mpsc, oneshot},
};
use tokio_stream::wrappers::TcpListenerStream;
use tonic::{Code, Request, Status, transport::Server};

struct Gateway {
    calls: AtomicUsize,
    dropped: mpsc::UnboundedSender<()>,
}

struct DropNotice(mpsc::UnboundedSender<()>);

impl Drop for DropNotice {
    fn drop(&mut self) {
        let _ = self.0.send(());
    }
}

impl ModelStreamingGateway for Gateway {
    fn stream(
        &self,
        request: ModelGatewayRequest,
    ) -> ModelGatewayFuture<ModelGatewayResult<ModelGatewayEventStream>> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        let notice = DropNotice(self.dropped.clone());
        Box::pin(async move {
            assert_eq!(
                request.endpoint(),
                &ModelEndpoint::new("openai", "gpt-fixture")
            );
            let mode = request.messages().last().unwrap().content.clone();
            let stream = async_stream::try_stream! {
                let _notice = notice;
                yield ModelStreamEvent::Started { transport: ModelGatewayTransport::Sse };
                if mode == "wait" {
                    std::future::pending::<()>().await;
                }
                if mode != "truncated" {
                    yield ModelStreamEvent::Thinking(ModelThinkingChunk::new(1, "host-only"));
                    yield ModelStreamEvent::Chunk(ModelStreamChunk::new(1, "你好"));
                    yield ModelStreamEvent::Completed {
                        response: ModelGatewayCompletionResponse::new("fixture", "gpt-fixture", vec![
                            ModelGatewayCompletionChoice::new(0, assistant_gateway_message("你好"), None),
                        ]),
                    };
                }
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

fn turn(text: &str) -> StreamTurnRequest {
    StreamTurnRequest {
        messages: vec![ConversationMessage {
            role: MessageRole::User.into(),
            text: text.into(),
        }],
    }
}

#[tokio::test]
async fn external_client_streams_turns_and_releases_abandoned_work() {
    let (dropped_tx, mut dropped) = mpsc::unbounded_channel();
    let gateway = Arc::new(Gateway {
        calls: AtomicUsize::new(0),
        dropped: dropped_tx,
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
    .into_service();
    let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
        .await
        .unwrap();
    let address = listener.local_addr().unwrap();
    let (stop_tx, stop_rx) = oneshot::channel();
    let server = tokio::spawn(
        Server::builder()
            .add_service(service)
            .serve_with_incoming_shutdown(TcpListenerStream::new(listener), async {
                let _ = stop_rx.await;
            }),
    );
    let mut client = AppServerClient::connect(format!("http://{address}"))
        .await
        .unwrap();

    assert_eq!(
        client.stream_turn(turn("hello")).await.unwrap_err().code(),
        Code::Unauthenticated
    );
    assert_eq!(
        client
            .stream_turn(authorized(turn("  ")))
            .await
            .unwrap_err()
            .code(),
        Code::InvalidArgument
    );
    let oversized = "x".repeat(256 * 1024);
    assert_eq!(
        client
            .stream_turn(authorized(turn(&oversized)))
            .await
            .unwrap_err()
            .code(),
        Code::OutOfRange
    );
    assert_eq!(gateway.calls.load(Ordering::Relaxed), 0);
    let capabilities = client
        .get_capabilities(authorized(GetCapabilitiesRequest {}))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(capabilities.max_request_bytes, 256 * 1024);

    let mut stream = client
        .stream_turn(authorized(turn("hello")))
        .await
        .unwrap()
        .into_inner();
    assert!(matches!(
        stream.message().await.unwrap().unwrap().event,
        Some(Event::Started(_))
    ));
    assert!(
        matches!(stream.message().await.unwrap().unwrap().event, Some(Event::TextDelta(delta)) if delta.sequence == 1 && delta.text == "你好")
    );
    assert!(
        matches!(stream.message().await.unwrap().unwrap().event, Some(Event::Completed(receipt)) if receipt.text_chunks == 1 && receipt.text_bytes == 6)
    );
    assert!(stream.message().await.unwrap().is_none());
    assert_eq!(dropped.recv().await, Some(()));

    let mut stream = client
        .stream_turn(authorized(turn("truncated")))
        .await
        .unwrap()
        .into_inner();
    assert!(stream.message().await.unwrap().is_some());
    assert_eq!(stream.message().await.unwrap_err().code(), Code::DataLoss);
    assert_eq!(dropped.recv().await, Some(()));

    let mut stream = client
        .stream_turn(authorized(turn("wait")))
        .await
        .unwrap()
        .into_inner();
    assert!(stream.message().await.unwrap().is_some());
    drop(stream);
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(3), dropped.recv())
            .await
            .unwrap(),
        Some(())
    );

    let mut request = authorized(turn("wait"));
    request.set_timeout(Duration::from_secs(1));
    let mut stream = client.stream_turn(request).await.unwrap().into_inner();
    assert!(stream.message().await.unwrap().is_some());
    assert_eq!(
        stream.message().await.unwrap_err().code(),
        Code::DeadlineExceeded
    );
    assert_eq!(dropped.recv().await, Some(()));
    let mut active = Vec::new();
    for _ in 0..16 {
        let mut stream = client
            .stream_turn(authorized(turn("wait")))
            .await
            .unwrap()
            .into_inner();
        assert!(stream.message().await.unwrap().is_some());
        active.push(stream);
    }
    let calls_before_rejection = gateway.calls.load(Ordering::Relaxed);
    assert_eq!(
        client
            .stream_turn(authorized(turn("wait")))
            .await
            .unwrap_err()
            .code(),
        Code::ResourceExhausted
    );
    assert_eq!(
        gateway.calls.load(Ordering::Relaxed),
        calls_before_rejection
    );
    drop(active);
    tokio::time::timeout(Duration::from_secs(3), async {
        for _ in 0..16 {
            assert_eq!(dropped.recv().await, Some(()));
        }
    })
    .await
    .unwrap();
    let reopened = client.stream_turn(authorized(turn("hello"))).await;
    assert!(reopened.is_ok());
    drop(reopened);
    stop_tx.send(()).unwrap();
    server.await.unwrap().unwrap();
}

#[tokio::test]
async fn deadline_releases_provider_when_response_is_not_polled() {
    use marlin_agent_app_server::v1::app_server_server::AppServer as AppServerRpc;
    use tokio_stream::StreamExt;

    let (dropped_tx, mut dropped) = mpsc::unbounded_channel();
    let gateway = Arc::new(Gateway {
        calls: AtomicUsize::new(0),
        dropped: dropped_tx,
    });
    let service = AppServer::new(ModelEndpoint::new("openai", "gpt-fixture"), gateway, |_| {
        Ok(())
    })
    .unwrap();
    let mut request = authorized(turn("wait"));
    request.set_timeout(Duration::from_millis(100));
    let mut response = AppServerRpc::stream_turn(&service, request)
        .await
        .unwrap()
        .into_inner();

    assert_eq!(
        tokio::time::timeout(Duration::from_secs(3), dropped.recv())
            .await
            .unwrap(),
        Some(())
    );
    assert!(matches!(
        response.next().await.unwrap().unwrap().event,
        Some(Event::Started(_))
    ));
    assert_eq!(
        response.next().await.unwrap().unwrap_err().code(),
        Code::DeadlineExceeded
    );
}

#[tokio::test]
async fn session_free_task_actor_control_plane_works_over_http2_grpc() {
    use marlin_agent_app_server::v1::{
        BindGovernedContextRequest, DetourModeProto, DisclosureDepthProto,
        DispatchTaskActorRequest, ExecutionReceiptStatusProto, GetWorkbenchTabsRequest,
        GraphDispatchOperationProto, GraphSubstrateTargetProto, ReconcileTaskReceiptRequest,
        RegisterTaskActorRequest, TaskActorScopeModeProto, WatchWorkbenchTabsRequest,
        WorkbenchTabKindProto,
    };
    use tokio_stream::StreamExt;

    let (dropped_tx, _dropped) = mpsc::unbounded_channel();
    let gateway = Arc::new(Gateway {
        calls: AtomicUsize::new(0),
        dropped: dropped_tx,
    });
    let service = AppServer::new(
        ModelEndpoint::new("openai", "gpt-fixture"),
        gateway,
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
    .unwrap();

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (stop_tx, stop_rx) = oneshot::channel::<()>();
    let server = tokio::spawn(async move {
        Server::builder()
            .add_service(service.into_service())
            .serve_with_incoming_shutdown(TcpListenerStream::new(listener), async {
                let _ = stop_rx.await;
            })
            .await
    });

    let mut client = AppServerClient::connect(format!("http://{address}"))
        .await
        .unwrap();

    // Unauthorized access fails closed (MAR-APP-R01).
    let unauth = client
        .get_workbench_tabs(Request::new(GetWorkbenchTabsRequest {}))
        .await
        .unwrap_err();
    assert_eq!(unauth.code(), Code::Unauthenticated);

    // Subscribe to live Workbench tabs stream before registering any actor.
    let mut tab_stream = client
        .watch_workbench_tabs(authorized(WatchWorkbenchTabsRequest {}))
        .await
        .unwrap()
        .into_inner();
    let initial_tabs = tab_stream.next().await.unwrap().unwrap();
    assert_eq!(initial_tabs.tabs.len(), 1);
    assert_eq!(
        initial_tabs.tabs[0].kind,
        i32::from(WorkbenchTabKindProto::PinnedIndex)
    );

    // Register Session-free TaskActor at epoch 1.
    let reg = client
        .register_task_actor(authorized(RegisterTaskActorRequest {
            task_id: "task-grpc-01".into(),
            title: "gRPC GraphEngine Task".into(),
            scope_mode: TaskActorScopeModeProto::Project.into(),
        }))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(reg.cursor.as_ref().unwrap().fencing_epoch, 1);

    // Stream delivers updated pinned-Index + active Task tab immediately.
    let updated_tabs = tab_stream.next().await.unwrap().unwrap();
    assert_eq!(updated_tabs.tabs.len(), 2);
    assert_eq!(updated_tabs.tabs[1].id, "task-grpc-01");

    // Bind governed ai-agentic-context capsule (L2, sandbox + cedar admitted).
    let bound = client
        .bind_governed_context(authorized(BindGovernedContextRequest {
            task_id: "task-grpc-01".into(),
            context_cut: 1,
            kv_prefix_digest: "sha256:grpc-prefix-01".into(),
            disclosure_depth: DisclosureDepthProto::L2.into(),
            has_active_defeater: false,
            quarantined_memory_count: 1,
            sandbox_admitted: true,
            cedar_handoff_valid: true,
            detour_mode: DetourModeProto::Mainline.into(),
            auto_archive_enabled: true,
        }))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(bound.context_cut, 1);

    // Dispatch graph frontier at epoch 1 -> succeeds.
    let dispatched = client
        .dispatch_task_actor(authorized(DispatchTaskActorRequest {
            request_id: "req-grpc-01".into(),
            task_id: "task-grpc-01".into(),
            fencing_epoch: 1,
            operation: GraphDispatchOperationProto::ExecuteFrontier.into(),
            substrate_target: GraphSubstrateTargetProto::SqliteTurso.into(),
            scope_mode: TaskActorScopeModeProto::Project.into(),
            join_node_id: None,
        }))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(dispatched.fencing_epoch, 1);

    // Reconcile epoch 1 receipt -> accepted, advances epoch to 2.
    let rec1 = client
        .reconcile_task_receipt(authorized(ReconcileTaskReceiptRequest {
            request_id: "req-grpc-01".into(),
            task_id: "task-grpc-01".into(),
            fencing_epoch: 1,
            status: ExecutionReceiptStatusProto::Ok.into(),
        }))
        .await
        .unwrap()
        .into_inner();
    assert!(rec1.accepted);
    assert_eq!(rec1.next_epoch, 2);

    // Late split-brain receipt at stale epoch 1 -> rejected without mutating epoch 2.
    let stale = client
        .reconcile_task_receipt(authorized(ReconcileTaskReceiptRequest {
            request_id: "req-grpc-01".into(),
            task_id: "task-grpc-01".into(),
            fencing_epoch: 1,
            status: ExecutionReceiptStatusProto::Ok.into(),
        }))
        .await
        .unwrap()
        .into_inner();
    assert!(!stale.accepted);
    assert_eq!(stale.next_epoch, 2);

    drop(tab_stream);
    stop_tx.send(()).unwrap();
    server.await.unwrap().unwrap();
}
