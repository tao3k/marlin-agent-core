// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

#![cfg(feature = "ui-grpc")]

use std::{
    collections::{BTreeMap, HashMap},
    path::Path,
    process::Stdio,
    sync::Arc,
    time::Duration,
};

#[cfg(feature = "turso-host")]
use marlin_agent_app_server::v1::{
    CancelSessionTurnRequest, GetSessionTurnCancellationRequest,
    get_session_turn_cancellation_response::Evidence,
};
use marlin_agent_app_server::{
    AppServer, CompositeAtomicEffectExecutor, RegisterTaskActorRequest, TaskActorControlPlane,
    ui_grpc::{MyGoBridgeHost, SchemeOverlayProjection},
    v1::{
        ConversationMessage, DispatchTaskActorRequest, GetSessionSummaryRequest,
        GetSessionTurnStatusRequest, GetWorkbenchTabsRequest, GraphDispatchOperationProto,
        GraphSubstrateTargetProto, MessageRole, OpenSessionRequest,
        RegisterTaskActorRequest as V1RegisterTaskActorRequest, SessionOpenMode,
        SessionRecoveryState, SessionTurnOutcome, StartSessionTurnRequest, StreamTurnRequest,
        TaskActorScopeModeProto, WatchSessionTurnRequest, WatchWorkbenchTabsRequest,
        app_server_client::AppServerClient as V1AppServerClient, session_turn_event, turn_event,
    },
    wire::marlin::{
        app::v1::{GetCapabilitiesRequest, app_server_client::AppServerClient},
        common::v1::{DenialCode, TargetKind},
        device::v1::{
            IngestCaptureRequest, RegisterDeviceNodeRequest,
            device_bridge_service_client::DeviceBridgeServiceClient,
        },
        gate::v1::{
            GateKind, GetGateReceiptRequest, GraphHumanDecision, PendingGate,
            SubmitGraphHumanDecisionRequest, SubmitToolReviewDecisionRequest, ToolReviewDecision,
            approval_gate_service_client::ApprovalGateServiceClient,
        },
        plugin::v1::{
            EnableWorkflowRequest, InstallSignedReleaseRequest, PanelRuntimeKind,
            ReleaseAttestation, SignedReleaseEnvelope, SubmitScopedPanelActionRequest,
            plugin_service_client::PluginServiceClient,
        },
        projection::v1::{
            QueryProjectGraphRequest, SecretVisibility,
            projection_query_service_client::ProjectionQueryServiceClient,
        },
        ui::v1::{
            DispatchUiEventRequest, OpenWorkbenchRequest, SlotCardinality, SlotDeclaration,
            dispatch_ui_event_request, ui_workbench_service_client::UiWorkbenchServiceClient,
        },
    },
};
use marlin_agent_protocol::{
    ModelEndpoint, ModelGateway, ModelGatewayCompletionChoice, ModelGatewayCompletionResponse,
    ModelGatewayFuture, ModelGatewayRequest, ModelGatewayResult, ModelGatewayTransport,
    assistant_gateway_message, user_gateway_message,
};
use marlin_agent_runtime::{
    AtomicEffect, AtomicEffectReceipt, CasGetEffectPayload, CasPutEffectPayload, DetourMode,
    DetourNodeId, DetourState, FencingEpoch, LlmCompleteEffectPayload, SandboxRunEffectPayload,
    TaskActorId, TaskActorScopeMode,
    plugin::{
        PublisherId, ReleaseAttestation as RuntimeAttestation, ReleaseTrust, SignedPanelRelease,
    },
};
use marlin_agent_storage::{
    AgentId, ArtifactPointerKey, ProjectId, SessionId, TursoAgentStorage, TursoAgentStorageConfig,
    TursoBatchTransactionMode, TursoOptimizationProfile,
};
use marlin_agent_stream::{
    ModelGatewayEventStream, ModelStreamChunk, ModelStreamEvent, ModelStreamingGateway,
};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    net::TcpListener,
    process::{Child, Command},
};
use tokio_stream::{StreamExt, wrappers::TcpListenerStream};
use tonic::{Code, Request, Status, transport::Server};

fn auth_req<T>(msg: T) -> Request<T> {
    let mut r = Request::new(msg);
    r.metadata_mut()
        .insert("authorization", "Bearer test-token".parse().unwrap());
    r
}

#[tokio::test]
async fn mygo_bridge_enforces_ui_slots_one_shot_gates_signed_panels_and_device_overlays() {
    // Deterministic Ed25519 keypair via ring-free helper: use fixed dummy trust test
    // plus verify unknown publisher rejection and live multi-service roundtrips.
    let trust = ReleaseTrust::new(BTreeMap::new());
    let host = MyGoBridgeHost::new(trust, |meta| match meta.get("authorization") {
        Some(v) if v == "Bearer test-token" => Ok(()),
        _ => Err(Status::unauthenticated("missing bearer token")),
    });

    host.enqueue_pending_gate(PendingGate {
        gate_id: "gate-ios-1".to_owned(),
        kind: GateKind::DeviceCapability as i32,
        turn: None,
        call_id: Some("call-1".to_owned()),
        tool_or_node_name: "ios.calendar.create".to_owned(),
        prompt: "Create calendar event?".to_owned(),
        details_heading: "iOS EventKit".to_owned(),
        choices: Vec::new(),
        card: None,
        created_at_unix_ms: 1,
        expires_at_unix_ms: None,
    });

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        Server::builder()
            .add_service(host.app_service())
            .add_service(host.ui_service())
            .add_service(host.gate_service())
            .add_service(host.plugin_service())
            .add_service(host.device_service())
            .add_service(host.projection_service())
            .serve_with_incoming(TcpListenerStream::new(listener))
            .await
            .unwrap();
    });

    let endpoint = format!("http://{addr}");

    // 0. AppServer.GetCapabilities advertises all 6 orthogonal services & host target kind.
    let mut app_client = AppServerClient::connect(endpoint.clone()).await.unwrap();
    let caps = app_client
        .get_capabilities(auth_req(GetCapabilitiesRequest {}))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(caps.supported_services.len(), 6);
    assert!(caps.supported_services.iter().any(|s| s == "marlin.ui.v1"));
    assert_ne!(caps.host_target_kind, TargetKind::Unspecified as i32);

    // 1. UiWorkbenchService: unauthorized rejected, authorized opens with restricted approval.gate
    let mut ui_client = UiWorkbenchServiceClient::connect(endpoint.clone())
        .await
        .unwrap();
    let unauth = ui_client
        .open_workbench(OpenWorkbenchRequest {
            session_id: "s1".to_owned(),
            client_target_kind: TargetKind::DesktopMacos as i32,
            preferred_profile: None,
        })
        .await
        .unwrap_err();
    assert_eq!(unauth.code(), Code::Unauthenticated);

    let wb = ui_client
        .open_workbench(auth_req(OpenWorkbenchRequest {
            session_id: "s1".to_owned(),
            client_target_kind: TargetKind::DesktopMacos as i32,
            preferred_profile: None,
        }))
        .await
        .unwrap()
        .into_inner()
        .workbench
        .unwrap();
    assert_eq!(wb.generation, 1);
    assert!(wb.slots.iter().any(|s| {
        s.slot_id == "approval.gate"
            && s.fallback_renderer == "host.gate"
            && !s.allow_untrusted_plugin
    }));

    // Stale generation dispatch must be denied.
    let stale = ui_client
        .dispatch_ui_event(auth_req(DispatchUiEventRequest {
            session_id: "s1".to_owned(),
            expected_generation: 99,
            event: Some(dispatch_ui_event_request::Event::SelectProfile(
                "ios-personal".to_owned(),
            )),
        }))
        .await
        .unwrap()
        .into_inner();
    assert!(!stale.admitted);
    assert_eq!(
        stale.denial.unwrap().code,
        DenialCode::StaleGeneration as i32
    );

    // 2. ApprovalGateService: one-shot claim succeeds once, replay denied.
    let mut gate_client = ApprovalGateServiceClient::connect(endpoint.clone())
        .await
        .unwrap();
    let first = gate_client
        .submit_tool_review_decision(auth_req(SubmitToolReviewDecisionRequest {
            turn: None,
            gate_id: "gate-ios-1".to_owned(),
            call_id: "call-1".to_owned(),
            decision: ToolReviewDecision::ApproveOnce as i32,
            idempotency_key: "idem-1".to_owned(),
            rationale: None,
        }))
        .await
        .unwrap()
        .into_inner()
        .receipt
        .unwrap();
    assert!(first.claimed);
    assert!(first.receipt.is_some());

    let replay = gate_client
        .submit_tool_review_decision(auth_req(SubmitToolReviewDecisionRequest {
            turn: None,
            gate_id: "gate-ios-1".to_owned(),
            call_id: "call-1".to_owned(),
            decision: ToolReviewDecision::ApproveOnce as i32,
            idempotency_key: "idem-2".to_owned(),
            rationale: None,
        }))
        .await
        .unwrap()
        .into_inner()
        .receipt
        .unwrap();
    assert!(!replay.claimed);
    assert_eq!(replay.denial.unwrap().code, DenialCode::StaleOrdinal as i32);

    // 3. PluginService: unknown publisher rejected; inactive action rejected.
    let mut plugin_client = PluginServiceClient::connect(endpoint.clone())
        .await
        .unwrap();
    let bad_install = plugin_client
        .install_signed_release(auth_req(InstallSignedReleaseRequest {
            release: Some(SignedReleaseEnvelope {
                manifest_toml: b"id = \"evil.x\"".to_vec(),
                attestation: Some(ReleaseAttestation {
                    publisher_id: "unknown.pub".to_owned(),
                    source_revision: vec![0; 20],
                    build_evidence_digest: vec![0; 32],
                }),
                ed25519_signature: vec![0; 64],
                panel_assets: Vec::new(),
            }),
        }))
        .await
        .unwrap()
        .into_inner();
    assert!(!bad_install.installed);
    assert_eq!(
        bad_install.denial.unwrap().code,
        DenialCode::Unauthorized as i32
    );
    let _ = SignedPanelRelease::signing_bytes(
        b"test",
        &RuntimeAttestation {
            publisher: PublisherId::try_from("tao3k.official".to_owned()).unwrap(),
            source_revision: [0; 20],
            build_evidence_digest: [0; 32],
        },
    );
    let inactive_act = plugin_client
        .submit_scoped_panel_action(auth_req(SubmitScopedPanelActionRequest {
            project_id: "p1".to_owned(),
            session_id: "s1".to_owned(),
            plugin_id: "tao3k.missing".to_owned(),
            contribution_id: "panel".to_owned(),
            activation_generation: 1,
            action_name: "org.move".to_owned(),
            args_json: Vec::new(),
            idempotency_key: "k1".to_owned(),
        }))
        .await
        .unwrap()
        .into_inner()
        .receipt
        .unwrap();
    assert!(!inactive_act.admitted);

    let bad_enable = plugin_client
        .enable_workflow(auth_req(EnableWorkflowRequest {
            project_id: "p1".to_owned(),
            plugin_id: "tao3k.missing".to_owned(),
            contribution_id: "panel".to_owned(),
            settings_toml: Vec::new(),
        }))
        .await
        .unwrap()
        .into_inner()
        .receipt
        .unwrap();
    assert!(!bad_enable.activated);

    // 4. DeviceBridgeService: iOS device registration selects "ios-personal" overlay + idempotent capture.
    let mut dev_client = DeviceBridgeServiceClient::connect(endpoint.clone())
        .await
        .unwrap();
    let reg = dev_client
        .register_device_node(auth_req(RegisterDeviceNodeRequest {
            device_id: "iphone-16".to_owned(),
            display_name: "My iPhone".to_owned(),
            target_kind: TargetKind::MobileIos as i32,
            os_version: "19.0".to_owned(),
            app_version: "0.1.0".to_owned(),
            capabilities: Vec::new(),
        }))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(reg.active_profile_overlay, "ios-personal");

    let cap1 = dev_client
        .ingest_capture(auth_req(IngestCaptureRequest {
            session_id: "s1".to_owned(),
            device_id: "iphone-16".to_owned(),
            source_channel: "ios.share-extension".to_owned(),
            text: Some("Screenshot from WeChat".to_owned()),
            on_device_ocr_text: None,
            attachments: Vec::new(),
            metadata: HashMap::new(),
            idempotency_key: "cap-idem-1".to_owned(),
        }))
        .await
        .unwrap()
        .into_inner();
    assert!(cap1.accepted);

    // 5. ProjectionQueryService: refuses secret visibility escalation.
    let mut proj_client = ProjectionQueryServiceClient::connect(endpoint)
        .await
        .unwrap();
    let secret_err = proj_client
        .query_project_graph(auth_req(QueryProjectGraphRequest {
            project_id: "p1".to_owned(),
            session_id: None,
            surfaces: Vec::new(),
            secret_visibility: SecretVisibility::Allowed as i32,
            query: String::new(),
            page: None,
        }))
        .await
        .unwrap_err();
    assert_eq!(secret_err.code(), Code::PermissionDenied);

    server.abort();
}

struct EchoGateway;

impl ModelStreamingGateway for EchoGateway {
    fn stream(
        &self,
        _request: ModelGatewayRequest,
    ) -> ModelGatewayFuture<ModelGatewayResult<ModelGatewayEventStream>> {
        Box::pin(async move {
            let stream = async_stream::try_stream! {
                yield ModelStreamEvent::Started { transport: ModelGatewayTransport::Sse };
                yield ModelStreamEvent::Chunk(ModelStreamChunk::new(1, "cohosted-ok"));
                yield ModelStreamEvent::Completed {
                    response: ModelGatewayCompletionResponse::new(
                        "cohost-fixture",
                        "gpt-fixture",
                        vec![ModelGatewayCompletionChoice::new(
                            0,
                            assistant_gateway_message("cohosted-ok"),
                            None,
                        )],
                    ),
                };
            };
            Ok(Box::pin(stream) as ModelGatewayEventStream)
        })
    }
}

impl ModelGateway for EchoGateway {
    fn complete(
        &self,
        request: ModelGatewayRequest,
    ) -> ModelGatewayFuture<ModelGatewayResult<ModelGatewayCompletionResponse>> {
        let prompt = request
            .messages()
            .last()
            .map(|m| m.content.clone())
            .unwrap_or_default();
        Box::pin(async move {
            Ok(ModelGatewayCompletionResponse::new(
                "cohost-atomic-fixture",
                "gpt-fixture",
                vec![ModelGatewayCompletionChoice::new(
                    0,
                    assistant_gateway_message(format!("atomic-echo:{prompt}")),
                    None,
                )],
            ))
        })
    }
}

#[tokio::test]
async fn real_app_server_cohosts_with_mygo_bridge_and_validates_scheme_overlays() {
    // 1. SchemeOverlayProjection::validate enforces mobile App Store and host.gate invariants.
    let bad_ios = SchemeOverlayProjection {
        target_kind: TargetKind::MobileIos,
        profile_name: "ios-bad".to_owned(),
        layout_kind: "mobile-stack".to_owned(),
        allowed_runtimes: vec![PanelRuntimeKind::WebviewUntrusted],
        device_protocols: Vec::new(),
        slots: vec![SlotDeclaration {
            slot_id: "approval.gate".to_owned(),
            cardinality: SlotCardinality::Single as i32,
            fallback_renderer: "host.gate".to_owned(),
            allow_untrusted_plugin: false,
        }],
    };
    assert!(bad_ios.validate().is_err());

    let bad_legacy_ns = SchemeOverlayProjection {
        target_kind: TargetKind::DesktopMacos,
        profile_name: "legacy-bad".to_owned(),
        layout_kind: "desktop-split".to_owned(),
        allowed_runtimes: vec![PanelRuntimeKind::Declarative],
        device_protocols: Vec::new(),
        slots: vec![SlotDeclaration {
            slot_id: "approval.gate".to_owned(),
            cardinality: SlotCardinality::Single as i32,
            fallback_renderer: "native.gate".to_owned(),
            allow_untrusted_plugin: false,
        }],
    };
    assert!(bad_legacy_ns.validate().is_err());

    // 2. Real AppServer<S> co-hosted with MyGoBridgeHost via AppServer::into_ui_router!
    let app_server = AppServer::new(
        ModelEndpoint::new("openai", "gpt-fixture"),
        Arc::new(EchoGateway),
        |meta| match meta.get("authorization") {
            Some(v) if v == "Bearer test-token" => Ok(()),
            _ => Err(Status::unauthenticated("missing bearer token")),
        },
    )
    .unwrap();
    let bridge = MyGoBridgeHost::new(ReleaseTrust::new(BTreeMap::new()), |meta| {
        match meta.get("authorization") {
            Some(v) if v == "Bearer test-token" => Ok(()),
            _ => Err(Status::unauthenticated("missing bearer token")),
        }
    });

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        app_server
            .into_ui_router(bridge)
            .serve_with_incoming(TcpListenerStream::new(listener))
            .await
            .unwrap();
    });

    let endpoint = format!("http://{addr}");

    // Real AppServer StreamTurn serves live model gateway chunks on the shared port.
    let mut v1_client = V1AppServerClient::connect(endpoint.clone()).await.unwrap();
    let mut stream = v1_client
        .stream_turn(auth_req(StreamTurnRequest {
            messages: vec![ConversationMessage {
                role: MessageRole::User as i32,
                text: "hello".to_owned(),
            }],
        }))
        .await
        .unwrap()
        .into_inner();
    let mut text = String::new();
    while let Some(ev) = stream.next().await {
        if let Some(turn_event::Event::TextDelta(delta)) = ev.unwrap().event {
            text.push_str(&delta.text);
        }
    }
    assert_eq!(text, "cohosted-ok");

    // On the EXACT SAME HTTP/2 port, MyGoBridgeHost UiWorkbenchService serves locked host.gate.
    let mut ui_client = UiWorkbenchServiceClient::connect(endpoint).await.unwrap();
    let wb = ui_client
        .open_workbench(auth_req(OpenWorkbenchRequest {
            session_id: "cohosted-s1".to_owned(),
            client_target_kind: TargetKind::DesktopMacos as i32,
            preferred_profile: None,
        }))
        .await
        .unwrap()
        .into_inner()
        .workbench
        .unwrap();
    assert!(wb.slots.iter().any(|s| {
        s.slot_id == "approval.gate"
            && s.fallback_renderer == "host.gate"
            && !s.allow_untrusted_plugin
    }));

    server.abort();
}

#[tokio::test]
async fn mygo_bridge_cohosted_grpc_graph_human_gate_and_turso_atomic_pump_survive_restart() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cfg = TursoAgentStorageConfig {
        path: dir.path().join("cohosted-s14.db"),
        optimization_profile: TursoOptimizationProfile::AsyncIoWithMvcc,
        batch_transaction_mode: TursoBatchTransactionMode::Concurrent,
    };
    let project = ProjectId::new("proj-s14").unwrap();
    let session = SessionId::new("sess-s14").unwrap();
    let agent = AgentId::new("agent-s14").unwrap();
    let pointer_key = ArtifactPointerKey::new("ptr:s14:actors").unwrap();
    let task_id = TaskActorId::new("task-s14-main").unwrap();
    let detour_node_id = DetourNodeId::new("detour-s14-gate").unwrap();
    let cas_key = "k:s14:fleet-checkpoint";
    let checkpoint_bytes = b"(fleet-wire-v1 (round . 14) (gate . \"gate-graph-s14\"))".to_vec();

    // -------------------------------------------------------------------------
    // Boot #1: Real disk Turso WAL + co-hosted AppServer/MyGoBridgeHost over HTTP/2
    // -------------------------------------------------------------------------
    let storage1 = TursoAgentStorage::open_local(cfg.clone())
        .await
        .expect("open turso #1");
    let task_actors1 = TaskActorControlPlane::rehydrate_or_new(&storage1, &project, &pointer_key)
        .await
        .expect("rehydrate empty control plane");
    let persist1 = task_actors1.spawn_storage_persistence_loop(
        storage1.clone(),
        project.clone(),
        pointer_key.clone(),
        agent.clone(),
    );
    task_actors1
        .register_actor(RegisterTaskActorRequest {
            task_id: task_id.clone(),
            title: "S14 Graph Human Gate Actor".to_owned(),
            scope_mode: TaskActorScopeMode::Project,
        })
        .expect("register actor");
    task_actors1
        .enter_detour(&task_id, DetourMode::BlockerActive, detour_node_id.clone())
        .expect("enter blocker detour");

    let atomic1 = Arc::new(CompositeAtomicEffectExecutor::new(
        storage1.clone(),
        EchoGateway,
        project.clone(),
        session.clone(),
        agent.clone(),
    ));
    let bridge1 = MyGoBridgeHost::new(ReleaseTrust::new(BTreeMap::new()), |meta| {
        match meta.get("authorization") {
            Some(v) if v == "Bearer test-token" => Ok(()),
            _ => Err(Status::unauthenticated("missing bearer token")),
        }
    })
    .with_task_actors(task_actors1.clone())
    .with_atomic_executor(atomic1);

    bridge1.enqueue_pending_gate(PendingGate {
        gate_id: "gate-graph-s14".to_owned(),
        kind: GateKind::GraphHumanGate as i32,
        turn: None,
        call_id: None,
        tool_or_node_name: "node-human-approve-s14".to_owned(),
        prompt: "Approve bounded graph branch?".to_owned(),
        details_heading: "GraphEngine Human Gate".to_owned(),
        choices: Vec::new(),
        card: None,
        created_at_unix_ms: 14,
        expires_at_unix_ms: None,
    });

    let app_server1 = AppServer::new(
        ModelEndpoint::new("openai", "gpt-fixture"),
        Arc::new(EchoGateway),
        |meta| match meta.get("authorization") {
            Some(v) if v == "Bearer test-token" => Ok(()),
            _ => Err(Status::unauthenticated("missing bearer token")),
        },
    )
    .unwrap()
    .with_task_actors(task_actors1.clone());

    let listener1 = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr1 = listener1.local_addr().unwrap();
    let bridge1_srv = bridge1.clone();
    let server1 = tokio::spawn(async move {
        app_server1
            .into_ui_router(bridge1_srv)
            .serve_with_incoming(TcpListenerStream::new(listener1))
            .await
            .unwrap();
    });
    let endpoint1 = format!("http://{addr1}");

    // 1a. Co-hosted UiWorkbenchService serves locked host.gate on the same HTTP/2 listener.
    let mut ui_client = UiWorkbenchServiceClient::connect(endpoint1.clone())
        .await
        .unwrap();
    let wb = ui_client
        .open_workbench(auth_req(OpenWorkbenchRequest {
            session_id: "s14-live".to_owned(),
            client_target_kind: TargetKind::DesktopMacos as i32,
            preferred_profile: None,
        }))
        .await
        .unwrap()
        .into_inner()
        .workbench
        .unwrap();
    assert!(wb.slots.iter().any(|s| {
        s.slot_id == "approval.gate"
            && s.fallback_renderer == "host.gate"
            && !s.allow_untrusted_plugin
    }));

    // 1b. ApprovalGateService::submit_graph_human_decision enforces auth + one-shot claim.
    let mut gate_client = ApprovalGateServiceClient::connect(endpoint1).await.unwrap();
    let unauth_err = gate_client
        .submit_graph_human_decision(SubmitGraphHumanDecisionRequest {
            turn: None,
            gate_id: "gate-graph-s14".to_owned(),
            decision: GraphHumanDecision::Approve as i32,
            idempotency_key: "idem-unauth".to_owned(),
            note: None,
        })
        .await
        .unwrap_err();
    assert_eq!(unauth_err.code(), Code::Unauthenticated);

    let first_claim = gate_client
        .submit_graph_human_decision(auth_req(SubmitGraphHumanDecisionRequest {
            turn: None,
            gate_id: "gate-graph-s14".to_owned(),
            decision: GraphHumanDecision::Approve as i32,
            idempotency_key: "idem-s14-1".to_owned(),
            note: Some("approved over HTTP/2".to_owned()),
        }))
        .await
        .unwrap()
        .into_inner()
        .receipt
        .unwrap();
    assert!(first_claim.claimed);
    assert_eq!(first_claim.kind, GateKind::GraphHumanGate as i32);
    assert!(first_claim.denial.is_none());
    let minted_receipt_id = first_claim
        .receipt
        .as_ref()
        .expect("minted receipt")
        .receipt_id
        .clone();
    assert!(!minted_receipt_id.is_empty());

    // Fetch durable receipt via GetGateReceipt over gRPC.
    let fetched = gate_client
        .get_gate_receipt(auth_req(GetGateReceiptRequest {
            gate_id: "gate-graph-s14".to_owned(),
        }))
        .await
        .unwrap()
        .into_inner()
        .receipt
        .unwrap();
    assert!(fetched.claimed);
    assert_eq!(
        fetched.receipt.as_ref().unwrap().receipt_id,
        minted_receipt_id
    );

    // Duplicate claim on the same gate MUST be rejected with DenialCode::StaleOrdinal.
    let dup_claim = gate_client
        .submit_graph_human_decision(auth_req(SubmitGraphHumanDecisionRequest {
            turn: None,
            gate_id: "gate-graph-s14".to_owned(),
            decision: GraphHumanDecision::Approve as i32,
            idempotency_key: "idem-s14-2".to_owned(),
            note: None,
        }))
        .await
        .unwrap()
        .into_inner()
        .receipt
        .unwrap();
    assert!(!dup_claim.claimed);
    assert!(dup_claim.receipt.is_none());
    assert_eq!(
        dup_claim.denial.unwrap().code,
        DenialCode::StaleOrdinal as i32
    );

    // Unknown gate ID MUST be rejected with DenialCode::NotFound.
    let missing_claim = gate_client
        .submit_graph_human_decision(auth_req(SubmitGraphHumanDecisionRequest {
            turn: None,
            gate_id: "gate-graph-nonexistent".to_owned(),
            decision: GraphHumanDecision::Reject as i32,
            idempotency_key: "idem-s14-3".to_owned(),
            note: None,
        }))
        .await
        .unwrap()
        .into_inner()
        .receipt
        .unwrap();
    assert!(!missing_claim.claimed);
    assert_eq!(
        missing_claim.denial.unwrap().code,
        DenialCode::NotFound as i32
    );

    // 1c. Concurrent 4-primitive atomic pump on the exact same live MyGoBridgeHost:
    //     CasPut -> Turso WAL, LlmComplete -> EchoGateway, SandboxRun -> fail-closed Deny.
    let rcpts1 = bridge1
        .run_atomic_effects(vec![
            AtomicEffect::CasPut(CasPutEffectPayload {
                effect_id: "e-s14-put".to_owned(),
                key: cas_key.to_owned(),
                bytes: checkpoint_bytes.clone(),
            }),
            AtomicEffect::LlmComplete(LlmCompleteEffectPayload {
                effect_id: "e-s14-llm".to_owned(),
                request: ModelGatewayRequest::new(
                    ModelEndpoint::new("openai", "gpt-fixture"),
                    vec![user_gateway_message("s14-prompt")],
                ),
            }),
            AtomicEffect::SandboxRun(SandboxRunEffectPayload {
                effect_id: "e-s14-sandbox-deny".to_owned(),
                program: "/bin/echo".to_owned(),
                args: vec!["should-not-run".to_owned()],
                cwd: None,
            }),
        ])
        .await
        .expect("mounted atomic executor runs batch");
    assert_eq!(rcpts1.len(), 3);
    assert!(matches!(
        &rcpts1[0],
        AtomicEffectReceipt::CasStored(id) if id == "e-s14-put"
    ));
    assert!(matches!(
        &rcpts1[1],
        AtomicEffectReceipt::LlmCompleted(p)
            if p.effect_id == "e-s14-llm" && p.content == "atomic-echo:s14-prompt"
    ));
    assert!(matches!(
        &rcpts1[2],
        AtomicEffectReceipt::EffectError(p)
            if p.effect_id == "e-s14-sandbox-deny" && p.message.contains("denied")
    ));

    // Flush TaskActorControlPlane write-through loop, shut down Boot #1 completely.
    persist1.flush_and_stop().await.expect("flush boot #1");
    server1.abort();
    let _ = server1.await;
    drop(bridge1);
    drop(task_actors1);
    drop(storage1);

    // -------------------------------------------------------------------------
    // Boot #2: Full cold restart on the same disk Turso WAL database
    // -------------------------------------------------------------------------
    let storage2 = TursoAgentStorage::open_local(cfg)
        .await
        .expect("open turso #2");
    let task_actors2 = TaskActorControlPlane::rehydrate_or_new(&storage2, &project, &pointer_key)
        .await
        .expect("rehydrate boot #2 control plane");
    let snap2 = task_actors2.snapshot();
    let restored_actor = snap2
        .actors
        .get(&task_id)
        .expect("actor survived turso restart");
    assert_eq!(restored_actor.cursor.task_id, task_id);
    assert_eq!(
        restored_actor.detour_state,
        DetourState::Active {
            mode: DetourMode::BlockerActive,
            origin_epoch: FencingEpoch::new(1).unwrap(),
            detour_node_id,
        }
    );

    let bridge2 = MyGoBridgeHost::new(ReleaseTrust::new(BTreeMap::new()), |meta| {
        match meta.get("authorization") {
            Some(v) if v == "Bearer test-token" => Ok(()),
            _ => Err(Status::unauthenticated("missing bearer token")),
        }
    })
    .with_task_actors(task_actors2)
    .with_atomic_executor(Arc::new(CompositeAtomicEffectExecutor::new(
        storage2,
        EchoGateway,
        project,
        session,
        agent,
    )));

    let rcpts2 = bridge2
        .run_atomic_effects(vec![AtomicEffect::CasGet(CasGetEffectPayload {
            effect_id: "e-s14-get".to_owned(),
            key: cas_key.to_owned(),
        })])
        .await
        .expect("boot #2 cas get");
    assert!(matches!(
        rcpts2.as_slice(),
        [AtomicEffectReceipt::CasHit(p)]
            if p.effect_id == "e-s14-get" && p.bytes == checkpoint_bytes
    ));
}

async fn spawn_mygo_host_process(gate_id: &str, turso_path: Option<&Path>) -> (Child, String) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_marlin-mygo-host"));
    cmd.env("MARLIN_BRIDGE_ADDR", "127.0.0.1:0")
        .env("MARLIN_BRIDGE_TOKEN", "test-token")
        .env("MARLIN_BRIDGE_REAL_SESSION", "1")
        .env("MARLIN_BRIDGE_CHUNK_DELAY_MS", "1")
        .env("MARLIN_BRIDGE_GATE_ID", gate_id)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .kill_on_drop(true);
    if let Some(path) = turso_path {
        cmd.env("MARLIN_BRIDGE_TURSO_PATH", path);
    } else {
        cmd.env_remove("MARLIN_BRIDGE_TURSO_PATH");
    }
    let mut child = cmd.spawn().expect("spawn marlin-mygo-host binary");
    let stdout = child.stdout.take().expect("piped stdout");
    let mut lines = BufReader::new(stdout).lines();
    let ready_line = tokio::time::timeout(Duration::from_secs(10), lines.next_line())
        .await
        .expect("timed out waiting for marlin-mygo-host READY line")
        .expect("read READY line")
        .expect("marlin-mygo-host exited before printing READY");
    let addr = ready_line
        .strip_prefix("READY ")
        .unwrap_or_else(|| panic!("unexpected READY line: {ready_line}"))
        .trim()
        .to_owned();
    (child, format!("http://{addr}"))
}

async fn stop_mygo_host_process(mut child: Child) {
    #[cfg(unix)]
    {
        if let Some(pid) = child.id() {
            let status = std::process::Command::new("kill")
                .args(["-TERM", &pid.to_string()])
                .status()
                .expect("send SIGTERM to marlin-mygo-host");
            assert!(status.success(), "kill -TERM failed");
        }
        let exit = tokio::time::timeout(Duration::from_secs(10), child.wait())
            .await
            .expect("timed out waiting for graceful marlin-mygo-host exit")
            .expect("wait on marlin-mygo-host");
        assert!(
            exit.success(),
            "marlin-mygo-host should exit cleanly on SIGTERM, got {exit}"
        );
    }
    #[cfg(not(unix))]
    {
        let _ = child.kill().await;
        let _ = child.wait().await;
    }
}

async fn run_and_watch_session_turn(
    client: &mut V1AppServerClient<tonic::transport::Channel>,
    session_id: &str,
    user_text: &str,
    submission_id: &str,
) -> (marlin_agent_app_server::v1::SessionTurnRef, String) {
    let turn = client
        .start_session_turn(auth_req(StartSessionTurnRequest {
            session_id: session_id.to_owned(),
            user_text: user_text.to_owned(),
            after_turn: 0,
            submission_id: submission_id.to_owned(),
        }))
        .await
        .unwrap()
        .into_inner()
        .turn
        .expect("started turn ref");

    let mut events = client
        .watch_session_turn(auth_req(WatchSessionTurnRequest {
            turn: Some(turn.clone()),
            after_event_id: None,
        }))
        .await
        .unwrap()
        .into_inner();

    let mut collected_text = String::new();
    let mut completed = false;
    while let Some(ev) = events.message().await.unwrap() {
        match ev.event {
            Some(session_turn_event::Event::TextCheckpoint(cp)) => {
                collected_text.push_str(&cp.text);
            }
            Some(session_turn_event::Event::Outcome(code)) => {
                assert_eq!(code, SessionTurnOutcome::Completed as i32);
                completed = true;
                break;
            }
            _ => {}
        }
    }
    assert!(completed, "expected Completed terminal outcome");
    (turn, collected_text)
}

#[tokio::test]
async fn marlin_mygo_host_binary_in_memory_real_session_serves_grpc_and_one_shot_gate() {
    let (child, endpoint) = spawn_mygo_host_process("gate-bin-mem-1", None).await;

    let mut ui_client = UiWorkbenchServiceClient::connect(endpoint.clone())
        .await
        .unwrap();
    let wb = ui_client
        .open_workbench(auth_req(OpenWorkbenchRequest {
            session_id: "s16-mem-ui".to_owned(),
            client_target_kind: TargetKind::DesktopMacos as i32,
            preferred_profile: None,
        }))
        .await
        .unwrap()
        .into_inner()
        .workbench
        .unwrap();
    assert!(wb.slots.iter().any(|s| {
        s.slot_id == "approval.gate"
            && s.fallback_renderer == "host.gate"
            && !s.allow_untrusted_plugin
    }));

    let mut v1_client = V1AppServerClient::connect(endpoint.clone()).await.unwrap();
    let opened = v1_client
        .open_session(auth_req(OpenSessionRequest {
            session_id: "s16-mem-sess".to_owned(),
            mode: SessionOpenMode::Create as i32,
        }))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(opened.session_id, "s16-mem-sess");

    let (turn, reply) =
        run_and_watch_session_turn(&mut v1_client, "s16-mem-sess", "ping-s16-mem", "sub-mem-1")
            .await;
    assert_eq!(
        reply,
        "[marlin-core] processed \"ping-s16-mem\" via durable ConversationJournal."
    );
    let summary = v1_client
        .get_session_summary(auth_req(GetSessionSummaryRequest {
            session_id: "s16-mem-sess".to_owned(),
        }))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(summary.recovery_state(), SessionRecoveryState::Finished);
    assert_eq!(summary.latest_turn, Some(turn.clone()));
    let status = v1_client
        .get_session_turn_status(auth_req(GetSessionTurnStatusRequest { turn: Some(turn) }))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(status.outcome, Some(SessionTurnOutcome::Completed as i32));

    let mut gate_client = ApprovalGateServiceClient::connect(endpoint).await.unwrap();
    let first = gate_client
        .submit_tool_review_decision(auth_req(SubmitToolReviewDecisionRequest {
            turn: None,
            gate_id: "gate-bin-mem-1".to_owned(),
            call_id: "call-e2e-1".to_owned(),
            decision: ToolReviewDecision::ApproveOnce as i32,
            idempotency_key: "idem-bin-mem-1".to_owned(),
            rationale: None,
        }))
        .await
        .unwrap()
        .into_inner()
        .receipt
        .unwrap();
    assert!(first.claimed);
    let replay = gate_client
        .submit_tool_review_decision(auth_req(SubmitToolReviewDecisionRequest {
            turn: None,
            gate_id: "gate-bin-mem-1".to_owned(),
            call_id: "call-e2e-1".to_owned(),
            decision: ToolReviewDecision::ApproveOnce as i32,
            idempotency_key: "idem-bin-mem-2".to_owned(),
            rationale: None,
        }))
        .await
        .unwrap()
        .into_inner()
        .receipt
        .unwrap();
    assert!(!replay.claimed);
    assert_eq!(replay.denial.unwrap().code, DenialCode::StaleOrdinal as i32);

    stop_mygo_host_process(child).await;
}

#[tokio::test]
async fn app_server_grpc_fails_closed_when_mounted_oracle_diverges_on_tabs_or_dispatch() {
    struct RejectingOracle;

    impl marlin_agent_runtime::GraphEngineOracle for RejectingOracle {
        fn project_dispatch(
            &self,
            _envelope: &marlin_agent_runtime::MarlinDispatchEnvelope,
        ) -> Result<
            Result<
                marlin_agent_runtime::AdmittedMarlinDispatch,
                marlin_agent_runtime::GraphDispatchRejection,
            >,
            String,
        > {
            Err("scheme oracle refused dispatch".to_owned())
        }

        fn reconcile_receipt(
            &self,
            _cursor: &marlin_agent_runtime::TaskActorCursor,
            _receipt: &marlin_agent_runtime::MarlinExecutionReceipt,
        ) -> Result<marlin_agent_runtime::ReceiptReconciliationDecision, String> {
            Err("scheme oracle refused receipt".to_owned())
        }

        fn project_workbench_tabs(
            &self,
            _focused_task_id: Option<&TaskActorId>,
            _active_tasks: &[marlin_agent_runtime::WorkbenchActiveTaskInput],
            _peek_task_ids: &[TaskActorId],
        ) -> Result<marlin_agent_runtime::WorkbenchTabProjection, String> {
            Err("scheme oracle refused workbench tabs".to_owned())
        }

        fn project_governed_context(
            &self,
            _capsule: &marlin_agent_runtime::GovernedContextCapsule,
        ) -> Result<
            Result<
                marlin_agent_runtime::AdmittedGovernedContext,
                marlin_agent_runtime::GovernedContextRejection,
            >,
            String,
        > {
            Err("scheme oracle refused governed context".to_owned())
        }
    }

    let plane = TaskActorControlPlane::new().with_oracle(Arc::new(RejectingOracle));
    let bridge = MyGoBridgeHost::new(ReleaseTrust::new(BTreeMap::new()), |meta| {
        match meta.get("authorization") {
            Some(v) if v == "Bearer test-token" => Ok(()),
            _ => Err(Status::unauthenticated("missing bearer token")),
        }
    })
    .with_task_actors(plane.clone());
    let app = AppServer::control_plane_only(|meta| match meta.get("authorization") {
        Some(v) if v == "Bearer test-token" => Ok(()),
        _ => Err(Status::unauthenticated("missing bearer token")),
    })
    .with_task_actors(plane);

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let _ = app.serve_with_ui_on(bridge, listener).await;
    });
    let endpoint = format!("http://{addr}");

    let mut client = V1AppServerClient::connect(endpoint).await.unwrap();
    client
        .register_task_actor(auth_req(V1RegisterTaskActorRequest {
            task_id: "task-oracle-deny".to_owned(),
            title: "Oracle Denied Actor".to_owned(),
            scope_mode: TaskActorScopeModeProto::Project as i32,
        }))
        .await
        .unwrap();

    let dispatch_err = client
        .dispatch_task_actor(auth_req(DispatchTaskActorRequest {
            request_id: "req-deny-1".to_owned(),
            task_id: "task-oracle-deny".to_owned(),
            operation: GraphDispatchOperationProto::ExecuteFrontier as i32,
            fencing_epoch: 1,
            scope_mode: TaskActorScopeModeProto::Project as i32,
            join_node_id: None,
            substrate_target: GraphSubstrateTargetProto::PureMemory as i32,
        }))
        .await
        .unwrap_err();
    assert_eq!(dispatch_err.code(), Code::Internal);
    assert!(dispatch_err.message().contains("scheme oracle refused"));

    let tabs_err = client
        .get_workbench_tabs(auth_req(GetWorkbenchTabsRequest {}))
        .await
        .unwrap_err();
    assert_eq!(tabs_err.code(), Code::Internal);
    assert!(tabs_err.message().contains("scheme oracle refused"));

    let watch_err = client
        .watch_workbench_tabs(auth_req(WatchWorkbenchTabsRequest {}))
        .await
        .unwrap_err();
    assert_eq!(watch_err.code(), Code::Internal);
    assert!(watch_err.message().contains("scheme oracle refused"));

    server.abort();
}

#[cfg(feature = "turso-host")]
#[tokio::test]
async fn marlin_mygo_host_binary_turso_wal_survives_real_os_process_restart() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("mygo-host-s16.db");

    // -------------------------------------------------------------------------
    // OS Process Boot #1: Create durable session + stream turn + claim one-shot gate
    // -------------------------------------------------------------------------
    let (child1, endpoint1) = spawn_mygo_host_process("gate-bin-turso-1", Some(&db_path)).await;
    let mut v1_client1 = V1AppServerClient::connect(endpoint1.clone()).await.unwrap();
    let opened1 = v1_client1
        .open_session(auth_req(OpenSessionRequest {
            session_id: "s16-proc-durable".to_owned(),
            mode: SessionOpenMode::Create as i32,
        }))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(opened1.session_id, "s16-proc-durable");

    let (turn1, reply1) = run_and_watch_session_turn(
        &mut v1_client1,
        "s16-proc-durable",
        "hello-os-restart",
        "sub-proc-1",
    )
    .await;
    assert_eq!(
        reply1,
        "[marlin-core] processed \"hello-os-restart\" via durable ConversationJournal."
    );

    let mut gate_client1 = ApprovalGateServiceClient::connect(endpoint1).await.unwrap();
    let claim1 = gate_client1
        .submit_tool_review_decision(auth_req(SubmitToolReviewDecisionRequest {
            turn: None,
            gate_id: "gate-bin-turso-1".to_owned(),
            call_id: "call-e2e-1".to_owned(),
            decision: ToolReviewDecision::ApproveOnce as i32,
            idempotency_key: "idem-bin-turso-1".to_owned(),
            rationale: None,
        }))
        .await
        .unwrap()
        .into_inner()
        .receipt
        .unwrap();
    assert!(claim1.claimed);

    // Graceful SIGTERM -> triggers persistence_handle.flush_and_stop() and clean exit 0.
    drop(v1_client1);
    drop(gate_client1);
    stop_mygo_host_process(child1).await;

    // -------------------------------------------------------------------------
    // OS Process Boot #2: Fresh binary process on the EXACT SAME Turso WAL path
    // -------------------------------------------------------------------------
    let (child2, endpoint2) = spawn_mygo_host_process("gate-bin-turso-2", Some(&db_path)).await;
    let mut v1_client2 = V1AppServerClient::connect(endpoint2).await.unwrap();

    // Duplicate Create must fail with AlreadyExists because admission survived restart.
    let dup_create_err = v1_client2
        .open_session(auth_req(OpenSessionRequest {
            session_id: "s16-proc-durable".to_owned(),
            mode: SessionOpenMode::Create as i32,
        }))
        .await
        .unwrap_err();
    assert_eq!(dup_create_err.code(), Code::AlreadyExists);

    // Existing mode rehydrates cleanly from Turso WAL across OS processes.
    let reopened = v1_client2
        .open_session(auth_req(OpenSessionRequest {
            session_id: "s16-proc-durable".to_owned(),
            mode: SessionOpenMode::Existing as i32,
        }))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(reopened.session_id, "s16-proc-durable");

    let summary = v1_client2
        .get_session_summary(auth_req(GetSessionSummaryRequest {
            session_id: "s16-proc-durable".to_owned(),
        }))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(summary.recovery_state(), SessionRecoveryState::Finished);
    assert_eq!(summary.latest_turn, Some(turn1.clone()));

    let status = v1_client2
        .get_session_turn_status(auth_req(GetSessionTurnStatusRequest { turn: Some(turn1) }))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(status.recovery_state(), SessionRecoveryState::Finished);
    assert_eq!(status.outcome, Some(SessionTurnOutcome::Completed as i32));

    drop(v1_client2);
    stop_mygo_host_process(child2).await;
}

#[cfg(feature = "turso-host")]
#[tokio::test]
async fn marlin_mygo_host_binary_turso_cancellation_and_task_actor_tabs_survive_os_restart() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("mygo-host-s17.db");

    // -------------------------------------------------------------------------
    // OS Process Boot #1: Register Session-free TaskActor + mid-stream cancel
    // -------------------------------------------------------------------------
    let (child1, endpoint1) = spawn_mygo_host_process("gate-bin-s17-1", Some(&db_path)).await;
    let mut client1 = V1AppServerClient::connect(endpoint1).await.unwrap();

    // 1. Register a Session-free TaskActor over live HTTP/2 gRPC.
    let reg = client1
        .register_task_actor(auth_req(V1RegisterTaskActorRequest {
            task_id: "task-os-s17".to_owned(),
            title: "OS Restart Actor".to_owned(),
            scope_mode: TaskActorScopeModeProto::Project as i32,
        }))
        .await
        .unwrap()
        .into_inner()
        .cursor
        .expect("registered task actor cursor");
    assert_eq!(reg.task_id, "task-os-s17");
    assert_eq!(reg.fencing_epoch, 1);

    let tabs1 = client1
        .get_workbench_tabs(auth_req(GetWorkbenchTabsRequest {}))
        .await
        .unwrap()
        .into_inner();
    assert!(tabs1.tabs.iter().any(|t| t.id == "index"));
    assert!(
        tabs1
            .tabs
            .iter()
            .any(|t| t.id == "task-os-s17" && t.fencing_epoch == 1)
    );

    // 2. Start a `[hold-after-chunk-1]` streaming turn, wait for chunk 1, then cancel!
    client1
        .open_session(auth_req(OpenSessionRequest {
            session_id: "s17-cancel-sess".to_owned(),
            mode: SessionOpenMode::Create as i32,
        }))
        .await
        .unwrap();
    let turn = client1
        .start_session_turn(auth_req(StartSessionTurnRequest {
            session_id: "s17-cancel-sess".to_owned(),
            user_text: "[hold-after-chunk-1] cancel across OS restart".to_owned(),
            after_turn: 0,
            submission_id: "sub-s17-cancel-1".to_owned(),
        }))
        .await
        .unwrap()
        .into_inner()
        .turn
        .expect("started held turn");

    let mut events = client1
        .watch_session_turn(auth_req(WatchSessionTurnRequest {
            turn: Some(turn.clone()),
            after_event_id: None,
        }))
        .await
        .unwrap()
        .into_inner();
    let mut saw_chunk1 = false;
    while let Some(ev) = tokio::time::timeout(Duration::from_secs(5), events.message())
        .await
        .expect("timed out waiting for chunk 1")
        .unwrap()
    {
        if let Some(session_turn_event::Event::TextCheckpoint(cp)) = ev.event {
            assert_eq!(cp.text, "[marlin-core] ");
            saw_chunk1 = true;
            break;
        }
    }
    assert!(saw_chunk1, "expected durable chunk 1 before cancelling");

    let cancelled = client1
        .cancel_session_turn(auth_req(CancelSessionTurnRequest {
            turn: Some(turn.clone()),
        }))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(cancelled.turn, Some(turn.clone()));
    assert_eq!(cancelled.outcome(), SessionTurnOutcome::RemoteUncertain);
    let receipt = cancelled.runtime.expect("runtime shutdown receipt");
    assert!(receipt.completed && receipt.cancellation_requested);

    let cancel_check1 = client1
        .get_session_turn_cancellation(auth_req(GetSessionTurnCancellationRequest {
            turn: Some(turn.clone()),
        }))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(cancel_check1.outcome(), SessionTurnOutcome::RemoteUncertain);
    assert_eq!(cancel_check1.evidence, Some(Evidence::Recorded(receipt)));

    drop(events);
    drop(client1);
    stop_mygo_host_process(child1).await;

    // -------------------------------------------------------------------------
    // OS Process Boot #2: Fresh binary process on the EXACT SAME Turso WAL path
    // -------------------------------------------------------------------------
    let (child2, endpoint2) = spawn_mygo_host_process("gate-bin-s17-2", Some(&db_path)).await;
    let mut client2 = V1AppServerClient::connect(endpoint2).await.unwrap();

    // 1. Session-free TaskActor tabs survived OS process restart via Turso WAL!
    let tabs2 = client2
        .get_workbench_tabs(auth_req(GetWorkbenchTabsRequest {}))
        .await
        .unwrap()
        .into_inner();
    assert!(tabs2.tabs.iter().any(|t| t.id == "index"));
    assert!(
        tabs2
            .tabs
            .iter()
            .any(|t| t.id == "task-os-s17" && t.fencing_epoch == 1)
    );

    // 2. Authoritative cancellation receipt survived OS process restart & retries idempotently!
    let cancel_check2 = client2
        .get_session_turn_cancellation(auth_req(GetSessionTurnCancellationRequest {
            turn: Some(turn.clone()),
        }))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(cancel_check2.outcome(), SessionTurnOutcome::RemoteUncertain);
    assert_eq!(cancel_check2.evidence, Some(Evidence::Recorded(receipt)));

    let retry_cancel = client2
        .cancel_session_turn(auth_req(CancelSessionTurnRequest {
            turn: Some(turn.clone()),
        }))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(retry_cancel.outcome(), SessionTurnOutcome::RemoteUncertain);
    assert_eq!(retry_cancel.runtime, Some(receipt));

    let status2 = client2
        .get_session_turn_status(auth_req(GetSessionTurnStatusRequest { turn: Some(turn) }))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(status2.recovery_state(), SessionRecoveryState::Unresolved);
    assert_eq!(
        status2.outcome,
        Some(SessionTurnOutcome::RemoteUncertain as i32)
    );

    drop(client2);
    stop_mygo_host_process(child2).await;
}

#[tokio::test]
async fn marlin_scheme_ctl_binary_round_trips_bearer_auth_tabs_gates_and_dispatch_against_mygo_host()
 {
    let (child, endpoint) = spawn_mygo_host_process("gate-ctl-e2e-1", None).await;
    let ctl_bin = env!("CARGO_BIN_EXE_marlin-scheme-ctl");

    // 1. Unauthenticated CLI invocation must fail closed (Bearer token required).
    let unauth = Command::new(ctl_bin)
        .env_remove("MARLIN_BRIDGE_ENDPOINT")
        .env_remove("MARLIN_BRIDGE_TOKEN")
        .args(["--endpoint", &endpoint, "tabs"])
        .output()
        .await
        .expect("run marlin-scheme-ctl without token");
    assert!(
        !unauth.status.success(),
        "marlin-scheme-ctl without token must exit non-zero"
    );
    let unauth_err = String::from_utf8_lossy(&unauth.stderr);
    assert!(
        unauth_err.contains("invalid bearer token")
            || unauth_err.contains("authentication credentials"),
        "expected unauthenticated bearer error on stderr, got: {unauth_err}"
    );

    // 2. Authenticated `tabs` via CLI flags returns PinnedIndex workbench tab.
    let tabs_out = Command::new(ctl_bin)
        .env_remove("MARLIN_BRIDGE_ENDPOINT")
        .env_remove("MARLIN_BRIDGE_TOKEN")
        .args(["--endpoint", &endpoint, "--token", "test-token", "tabs"])
        .output()
        .await
        .expect("run marlin-scheme-ctl tabs");
    assert!(
        tabs_out.status.success(),
        "tabs failed: {}",
        String::from_utf8_lossy(&tabs_out.stderr)
    );
    let tabs_stdout = String::from_utf8_lossy(&tabs_out.stdout);
    assert!(
        tabs_stdout.contains("TABS count=")
            && tabs_stdout.contains("TAB kind=PinnedIndex id=index"),
        "unexpected tabs output: {tabs_stdout}"
    );

    // 3. Authenticated `gates` via MARLIN_BRIDGE_* env vars lists the seeded gate.
    let gates_out = Command::new(ctl_bin)
        .env("MARLIN_BRIDGE_ENDPOINT", &endpoint)
        .env("MARLIN_BRIDGE_TOKEN", "test-token")
        .arg("gates")
        .output()
        .await
        .expect("run marlin-scheme-ctl gates");
    assert!(
        gates_out.status.success(),
        "gates failed: {}",
        String::from_utf8_lossy(&gates_out.stderr)
    );
    let gates_stdout = String::from_utf8_lossy(&gates_out.stdout);
    assert!(
        gates_stdout.contains("GATES count=1") && gates_stdout.contains("GATE id=gate-ctl-e2e-1"),
        "unexpected gates output: {gates_stdout}"
    );

    // 4. Authenticated `dispatch` submits declarative `scheme:<action>` command.
    let dispatch_out = Command::new(ctl_bin)
        .env_remove("MARLIN_BRIDGE_ENDPOINT")
        .env_remove("MARLIN_BRIDGE_TOKEN")
        .args([
            "--endpoint",
            &endpoint,
            "--token",
            "test-token",
            "dispatch",
            "task-drill-l3",
            "task-alpha",
        ])
        .output()
        .await
        .expect("run marlin-scheme-ctl dispatch");
    assert!(
        dispatch_out.status.success(),
        "dispatch failed: {}",
        String::from_utf8_lossy(&dispatch_out.stderr)
    );
    let dispatch_stdout = String::from_utf8_lossy(&dispatch_out.stdout);
    assert!(
        dispatch_stdout.contains("DISPATCH admitted=true action=task-drill-l3"),
        "unexpected dispatch output: {dispatch_stdout}"
    );

    // 5. Authenticated `audit` queries ProjectionQueryService without leaking secrets.
    let audit_out = Command::new(ctl_bin)
        .env_remove("MARLIN_BRIDGE_ENDPOINT")
        .env_remove("MARLIN_BRIDGE_TOKEN")
        .args(["--endpoint", &endpoint, "--token", "test-token", "audit"])
        .output()
        .await
        .expect("run marlin-scheme-ctl audit");
    assert!(
        audit_out.status.success(),
        "audit failed: {}",
        String::from_utf8_lossy(&audit_out.stderr)
    );
    let audit_stdout = String::from_utf8_lossy(&audit_out.stdout);
    assert!(
        audit_stdout.contains("AUDIT"),
        "unexpected audit output: {audit_stdout}"
    );

    stop_mygo_host_process(child).await;
}
