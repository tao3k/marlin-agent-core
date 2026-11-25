// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

#![cfg(feature = "ui-grpc")]

use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

use marlin_agent_app_server::{
    AppServer,
    ui_grpc::{MyGoBridgeHost, SchemeOverlayProjection},
    v1::{
        ConversationMessage, MessageRole, StreamTurnRequest,
        app_server_client::AppServerClient as V1AppServerClient, turn_event,
    },
    wire::marlin::{
        app::v1::{GetCapabilitiesRequest, app_server_client::AppServerClient},
        common::v1::{DenialCode, TargetKind},
        device::v1::{
            IngestCaptureRequest, RegisterDeviceNodeRequest,
            device_bridge_service_client::DeviceBridgeServiceClient,
        },
        gate::v1::{
            GateKind, PendingGate, SubmitToolReviewDecisionRequest, ToolReviewDecision,
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
    ModelEndpoint, ModelGatewayCompletionChoice, ModelGatewayCompletionResponse,
    ModelGatewayFuture, ModelGatewayRequest, ModelGatewayResult, ModelGatewayTransport,
    assistant_gateway_message,
};
use marlin_agent_runtime::plugin::{
    PublisherId, ReleaseAttestation as RuntimeAttestation, ReleaseTrust, SignedPanelRelease,
};
use marlin_agent_stream::{
    ModelGatewayEventStream, ModelStreamChunk, ModelStreamEvent, ModelStreamingGateway,
};
use tokio::net::TcpListener;
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
