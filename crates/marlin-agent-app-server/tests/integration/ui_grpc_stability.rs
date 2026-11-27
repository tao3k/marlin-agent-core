// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

#![cfg(feature = "ui-grpc")]

//! Live HTTP/2 gRPC adversarial stability tests for `MyGoBridgeHost` (`ui_grpc.rs`)
//! refining `dev/quint/stability/MarlinMyGoBridgeStabilityOverlay.qnt` (SM3c) and
//! `dev/quint/stability/MarlinMyGoLagResyncOverlay.qnt` (SM5c).

use std::{collections::BTreeMap, time::Duration};

use marlin_agent_app_server::{
    ui_grpc::MyGoBridgeHost,
    wire::marlin::{
        device::v1::{
            DownloadArtifactRequest, UploadArtifactRequest,
            device_bridge_service_client::DeviceBridgeServiceClient,
        },
        ui::v1::{
            DispatchUiEventRequest, WatchWorkbenchRequest, dispatch_ui_event_request,
            ui_workbench_service_client::UiWorkbenchServiceClient,
        },
    },
};
use marlin_agent_runtime::plugin::ReleaseTrust;
use tokio::net::TcpListener;
use tokio_stream::{StreamExt, wrappers::TcpListenerStream};
use tonic::{
    Code, Request, Status,
    transport::{Endpoint, Server},
};

fn auth_req<T>(msg: T) -> Request<T> {
    let mut r = Request::new(msg);
    r.metadata_mut()
        .insert("authorization", "Bearer stability-token".parse().unwrap());
    r
}

async fn spawn_live_bridge() -> (String, tokio::task::JoinHandle<()>) {
    let host = MyGoBridgeHost::new(ReleaseTrust::new(BTreeMap::new()), |meta| {
        match meta.get("authorization") {
            Some(v) if v == "Bearer stability-token" => Ok(()),
            _ => Err(Status::unauthenticated("missing bearer token")),
        }
    });
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
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
    (format!("http://{addr}"), handle)
}

/// Adversarial test refining `StreamLagResyncCoalescing.qnt` (`stepSilentLagDisconnect`):
/// a slow `WatchWorkbench` subscriber whose broadcast ring overflows (>64 events)
/// MUST receive a fresh resynced snapshot and keep streaming subsequent updates,
/// never silently terminating with EOF.
#[tokio::test]
async fn watch_workbench_resyncs_snapshot_on_broadcast_lag_instead_of_dropping_stream() {
    let (endpoint, server) = spawn_live_bridge().await;
    let mut ui_client = UiWorkbenchServiceClient::connect(endpoint.clone())
        .await
        .unwrap();
    // Connect watch_client with a tight HTTP/2 flow-control window so a stalled
    // reader applies real HTTP/2 backpressure to `watch_workbench` and overflows
    // the server's `broadcast::channel(64)` ring (real-world slow UI client).
    let slow_channel = Endpoint::from_shared(endpoint)
        .unwrap()
        .initial_stream_window_size(1024)
        .initial_connection_window_size(1024)
        .connect()
        .await
        .unwrap();
    let mut watch_client = UiWorkbenchServiceClient::new(slow_channel);

    // Open WatchWorkbench stream and consume initial snapshot (gen=1).
    let mut stream = watch_client
        .watch_workbench(auth_req(WatchWorkbenchRequest {
            session_id: "lag-session".to_owned(),
            after_generation: 0,
        }))
        .await
        .unwrap()
        .into_inner();

    let first = tokio::time::timeout(Duration::from_secs(2), stream.next())
        .await
        .expect("initial snapshot timed out")
        .expect("stream closed before initial snapshot")
        .unwrap();
    let base_gen = first.workbench.unwrap().generation;

    // Stall the subscriber while bursting 160 updates (>> broadcast::channel(64) ring
    // + 1024B HTTP/2 window) so the server receiver definitively hits RecvError::Lagged.
    const BURST: u64 = 160;
    for i in 0..BURST {
        let resp = ui_client
            .dispatch_ui_event(auth_req(DispatchUiEventRequest {
                session_id: "lag-session".to_owned(),
                expected_generation: 0,
                event: Some(dispatch_ui_event_request::Event::SelectFocus(format!(
                    "pane-{i}"
                ))),
            }))
            .await
            .unwrap()
            .into_inner();
        assert!(resp.admitted);
    }

    // Drain until we observe the latest post-burst generation; stream MUST NOT drop on Lagged!
    let target_gen = base_gen + BURST;
    let mut max_seen = base_gen;
    while max_seen < target_gen {
        let maybe_msg = tokio::time::timeout(Duration::from_secs(2), stream.next())
            .await
            .expect("watch_workbench stalled after broadcast lag");
        let msg = maybe_msg
            .expect("CHALLENGE FAILED (StreamLagResyncCoalescing SM5c): watch_workbench silently terminated on RecvError::Lagged!")
            .unwrap();
        max_seen = max_seen.max(msg.workbench.unwrap().generation);
    }
    assert_eq!(max_seen, target_gen);

    // Confirm stream remains live for a follow-up update after lag recovery.
    ui_client
        .dispatch_ui_event(auth_req(DispatchUiEventRequest {
            session_id: "lag-session".to_owned(),
            expected_generation: 0,
            event: Some(dispatch_ui_event_request::Event::SelectFocus(
                "pane-after-resync".to_owned(),
            )),
        }))
        .await
        .unwrap();
    let next_msg = tokio::time::timeout(Duration::from_secs(2), stream.next())
        .await
        .expect("post-resync update timed out")
        .expect("stream closed after lag resync")
        .unwrap();
    assert_eq!(next_msg.workbench.unwrap().generation, target_gen + 1);

    server.abort();
}

/// Adversarial test refining `BoundedProjectionEviction.qnt` (`stepUnboundedGrowth`):
/// uploading 80 artifacts MUST evict older entries within a bounded retention cap
/// (default <=64) instead of retaining all historical payloads unboundedly in RAM.
#[tokio::test]
async fn upload_artifact_evicts_oldest_entries_within_bounded_retention_cap() {
    let (endpoint, server) = spawn_live_bridge().await;
    let mut dev_client = DeviceBridgeServiceClient::connect(endpoint).await.unwrap();

    const UPLOADS: usize = 80;
    let mut ids = Vec::with_capacity(UPLOADS);
    for i in 0..UPLOADS {
        let payload = vec![(i % 251) as u8; 1024];
        let req_stream = tokio_stream::iter(vec![UploadArtifactRequest {
            offset: 0,
            mime_type: "application/octet-stream".to_owned(),
            data: payload,
            eof: true,
        }]);
        let resp = dev_client
            .upload_artifact(auth_req(req_stream))
            .await
            .unwrap()
            .into_inner();
        ids.push(resp.artifact.unwrap().artifact_id);
    }

    // Newest artifact MUST still be downloadable.
    let newest_id = ids.last().unwrap().clone();
    let newest = dev_client
        .download_artifact(auth_req(DownloadArtifactRequest {
            artifact_id: newest_id,
            offset: 0,
            max_bytes: 0,
        }))
        .await;
    assert!(newest.is_ok(), "newest artifact must remain resident");

    // Oldest artifact MUST have been evicted under bounded retention (<=64).
    let oldest_id = ids.first().unwrap().clone();
    let oldest = dev_client
        .download_artifact(auth_req(DownloadArtifactRequest {
            artifact_id: oldest_id.clone(),
            offset: 0,
            max_bytes: 0,
        }))
        .await;
    assert_eq!(
        oldest.expect_err("CHALLENGE FAILED (BoundedProjectionEviction SM3c): oldest artifact was never evicted after 80 uploads!").code(),
        Code::NotFound,
        "oldest artifact {oldest_id} should be evicted with NotFound"
    );

    server.abort();
}

/// SM28 (ADV-L4): Active Operator E-Stop MUST fail closed on `ApprovalGateService`
/// (`submit_tool_review_decision`, `submit_graph_human_decision`) AND `DeviceBridgeService::open_device_channel`,
/// preventing live `DeviceInvocationRequest` dispatch while E-Stop is active.
#[tokio::test]
async fn sm28_adv_l4_estop_blocks_approval_gate_and_device_channel() {
    use std::sync::Arc;
    use marlin_agent_app_server::wire::marlin::{
        device::v1::{
            DeviceHeartbeat, OpenDeviceChannelRequest, open_device_channel_request,
        },
        gate::v1::{
            GateKind, GraphHumanDecision, PendingGate, SubmitGraphHumanDecisionRequest,
            SubmitToolReviewDecisionRequest, ToolReviewDecision,
            approval_gate_service_client::ApprovalGateServiceClient,
        },
    };
    use marlin_agent_kernel::{
        ExecutionCircuitBreaker, GovernanceAdmissionGate, ToolBudgetUnits,
    };

    let breaker = Arc::new(ExecutionCircuitBreaker::new(ToolBudgetUnits::new(64)));
    let gate = Arc::new(GovernanceAdmissionGate::new(breaker));
    gate.trip_operator_estop("operator", "SM28-ADV-L4 operator E-Stop")
        .expect("trip emergency stop");

    let host = MyGoBridgeHost::new_default_trust(|meta| match meta.get("authorization") {
        Some(v) if v == "Bearer stability-token" => Ok(()),
        _ => Err(Status::unauthenticated("missing bearer token")),
    })
    .with_governance_gate(gate);

    host.enqueue_pending_gate(PendingGate {
        gate_id: "gate-dev-sm28".to_owned(),
        kind: GateKind::DeviceCapability as i32,
        turn: None,
        call_id: Some("call-sm28".to_owned()),
        tool_or_node_name: "device.notify".to_owned(),
        prompt: "Hostile device dispatch during E-Stop".to_owned(),
        details_heading: "SM28 Probe".to_owned(),
        choices: Vec::new(),
        card: None,
        created_at_unix_ms: 1,
        expires_at_unix_ms: None,
    });
    host.enqueue_pending_gate(PendingGate {
        gate_id: "gate-graph-sm28".to_owned(),
        kind: GateKind::GraphHumanGate as i32,
        turn: None,
        call_id: None,
        tool_or_node_name: "node-human".to_owned(),
        prompt: "Hostile graph human approval during E-Stop".to_owned(),
        details_heading: "SM28 Probe".to_owned(),
        choices: Vec::new(),
        card: None,
        created_at_unix_ms: 2,
        expires_at_unix_ms: None,
    });

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        Server::builder()
            .add_service(host.gate_service())
            .add_service(host.device_service())
            .serve_with_incoming(TcpListenerStream::new(listener))
            .await
            .unwrap();
    });
    let endpoint = format!("http://{addr}");

    let mut gate_client = ApprovalGateServiceClient::connect(endpoint.clone())
        .await
        .unwrap();
    let mut dev_client = DeviceBridgeServiceClient::connect(endpoint).await.unwrap();

    // 1. submit_tool_review_decision(ApproveOnce) MUST fail with PermissionDenied during active E-Stop.
    let tool_res = gate_client
        .submit_tool_review_decision(auth_req(SubmitToolReviewDecisionRequest {
            turn: None,
            gate_id: "gate-dev-sm28".to_owned(),
            call_id: "call-sm28".to_owned(),
            decision: ToolReviewDecision::ApproveOnce as i32,
            idempotency_key: "idem-sm28-tool".to_owned(),
            rationale: None,
        }))
        .await;
    assert_eq!(
        tool_res
            .expect_err("ADV-L4 SM28 E-STOP BYPASS: submit_tool_review_decision succeeded during active E-Stop!")
            .code(),
        Code::PermissionDenied
    );

    // 2. submit_graph_human_decision MUST fail with PermissionDenied during active E-Stop.
    let graph_res = gate_client
        .submit_graph_human_decision(auth_req(SubmitGraphHumanDecisionRequest {
            turn: None,
            gate_id: "gate-graph-sm28".to_owned(),
            decision: GraphHumanDecision::Approve as i32,
            idempotency_key: "idem-sm28-graph".to_owned(),
            note: None,
        }))
        .await;
    assert_eq!(
        graph_res
            .expect_err("ADV-L4 SM28 E-STOP BYPASS: submit_graph_human_decision succeeded during active E-Stop!")
            .code(),
        Code::PermissionDenied
    );

    // 3. open_device_channel MUST fail with PermissionDenied during active E-Stop.
    let chan_req = tokio_stream::iter(vec![OpenDeviceChannelRequest {
        frame: Some(open_device_channel_request::Frame::Heartbeat(
            DeviceHeartbeat { unix_ms: 1 },
        )),
    }]);
    let chan_res = dev_client.open_device_channel(auth_req(chan_req)).await;
    assert_eq!(
        chan_res
            .expect_err("ADV-L4 SM28 E-STOP BYPASS: open_device_channel succeeded during active E-Stop!")
            .code(),
        Code::PermissionDenied
    );

    server.abort();
}

/// SM28 (ADV-L4): Forged `CapabilityUpdate` frames carrying unregistered `lease_id`s
/// MUST fail closed without mutating `device_capabilities` or evicting registered devices,
/// while legitimate registered leases update their bound device cleanly.
#[tokio::test]
async fn sm28_adv_l4_forged_device_lease_update_cannot_evict_registered_devices() {
    use marlin_agent_app_server::wire::marlin::{
        common::v1::TargetKind,
        device::v1::{
            DeviceCapabilityDescriptor, DeviceCapabilityUpdate, OpenDeviceChannelRequest,
            RegisterDeviceNodeRequest, open_device_channel_request,
        },
    };

    let host = MyGoBridgeHost::new_default_trust(|meta| match meta.get("authorization") {
        Some(v) if v == "Bearer stability-token" => Ok(()),
        _ => Err(Status::unauthenticated("missing bearer token")),
    });
    let host_inspect = host.clone();

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        Server::builder()
            .add_service(host.device_service())
            .serve_with_incoming(TcpListenerStream::new(listener))
            .await
            .unwrap();
    });
    let endpoint = format!("http://{addr}");
    let mut dev_client = DeviceBridgeServiceClient::connect(endpoint).await.unwrap();

    // 1. Register a legitimate victim device.
    let reg = dev_client
        .register_device_node(auth_req(RegisterDeviceNodeRequest {
            device_id: "dev-victim-sm28".to_owned(),
            display_name: "Victim Node".to_owned(),
            target_kind: TargetKind::DesktopMacos as i32,
            os_version: "macOS-15".to_owned(),
            app_version: "0.1.0".to_owned(),
            capabilities: vec![DeviceCapabilityDescriptor {
                capability_id: "device.notify".to_owned(),
                protocol_family: "desktop.notify".to_owned(),
                ..Default::default()
            }],
        }))
        .await
        .unwrap()
        .into_inner();
    assert!(!reg.lease_id.is_empty());
    assert_eq!(
        host_inspect
            .registered_device_capabilities("dev-victim-sm28")
            .unwrap()
            .len(),
        1
    );

    // 2. Flood forged CapabilityUpdate frames using unregistered lease_ids (> MAX_BRIDGE_RECORDS = 256).
    let forged_frames: Vec<OpenDeviceChannelRequest> = (0..300)
        .map(|i| OpenDeviceChannelRequest {
            frame: Some(open_device_channel_request::Frame::CapabilityUpdate(
                DeviceCapabilityUpdate {
                    lease_id: format!("forged-lease-{i}"),
                    capabilities: Vec::new(),
                },
            )),
        })
        .collect();
    let mut forged_stream = dev_client
        .open_device_channel(auth_req(tokio_stream::iter(forged_frames)))
        .await
        .unwrap()
        .into_inner();
    let first_err = forged_stream
        .next()
        .await
        .expect("forged lease stream must emit PermissionDenied error")
        .expect_err("forged lease_id must fail closed");
    assert_eq!(first_err.code(), Code::PermissionDenied);

    // Victim device MUST still be registered and untouched!
    let still_caps = host_inspect
        .registered_device_capabilities("dev-victim-sm28")
        .expect("ADV-L4 SM28 FORGED-LEASE EVICTION DoS: victim device was evicted!");
    assert_eq!(still_caps.len(), 1);
    assert_eq!(still_caps[0].capability_id, "device.notify");
    assert!(
        host_inspect
            .registered_device_capabilities("forged-lease-0")
            .is_none()
    );

    // 3. Legitimate registered lease_id updates its bound device cleanly.
    let valid_update = tokio_stream::iter(vec![OpenDeviceChannelRequest {
        frame: Some(open_device_channel_request::Frame::CapabilityUpdate(
            DeviceCapabilityUpdate {
                lease_id: reg.lease_id,
                capabilities: vec![
                    DeviceCapabilityDescriptor {
                        capability_id: "device.notify".to_owned(),
                        protocol_family: "desktop.notify".to_owned(),
                        ..Default::default()
                    },
                    DeviceCapabilityDescriptor {
                        capability_id: "device.clipboard".to_owned(),
                        protocol_family: "desktop.clipboard".to_owned(),
                        ..Default::default()
                    },
                ],
            },
        )),
    }]);
    let mut valid_stream = dev_client
        .open_device_channel(auth_req(valid_update))
        .await
        .unwrap()
        .into_inner();
    while let Some(item) = valid_stream.next().await {
        item.expect("valid bound lease update must succeed");
    }
    let updated_caps = host_inspect
        .registered_device_capabilities("dev-victim-sm28")
        .expect("victim device remains registered");
    assert_eq!(updated_caps.len(), 2);

    server.abort();
}
