// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use marlin_agent_app_server::ui_grpc::MyGoBridgeHost;
use tonic::metadata::{MetadataMap, MetadataValue};

fn meta_with_auth(value: &str) -> MetadataMap {
    let mut meta = MetadataMap::new();
    meta.insert(
        "authorization",
        MetadataValue::try_from(value).expect("valid ascii auth header"),
    );
    meta
}

#[test]
fn sm24_adv_l4_bearer_auth_rejects_blank_expected_and_enforces_exact_match() {
    // 1. Blank or whitespace-only expected_header MUST fail closed even when attacker sends matching empty/spaces header.
    let empty_meta = meta_with_auth("");
    assert!(
        MyGoBridgeHost::check_bearer_header(&empty_meta, Some("")).is_err(),
        "ADV-L4 AUTH BYPASS PROVEN: Some(\"\") accepted empty authorization header"
    );
    let spaces_meta = meta_with_auth("   ");
    assert!(
        MyGoBridgeHost::check_bearer_header(&spaces_meta, Some("   ")).is_err(),
        "ADV-L4 AUTH BYPASS PROVEN: Some(\"   \") accepted whitespace-only authorization header"
    );
    let missing_meta = MetadataMap::new();
    assert!(MyGoBridgeHost::check_bearer_header(&missing_meta, Some("")).is_err());

    // 2. Non-blank secret rejects prefix/truncated/mismatched tokens and admits exact match.
    let expected = "Bearer sm24-hostile-bridge-token-9f8e7d6c";
    assert!(
        MyGoBridgeHost::check_bearer_header(
            &meta_with_auth("Bearer sm24-hostile-bridge-token-9f8e7d6d"),
            Some(expected)
        )
        .is_err()
    );
    assert!(
        MyGoBridgeHost::check_bearer_header(
            &meta_with_auth("Bearer sm24-hostile-bridge-token"),
            Some(expected)
        )
        .is_err()
    );
    assert!(
        MyGoBridgeHost::check_bearer_header(&meta_with_auth(expected), Some(expected)).is_ok()
    );

    // 3. Explicit loopback dev mode (`None`) remains admitted.
    assert!(MyGoBridgeHost::check_bearer_header(&missing_meta, None).is_ok());
}

#[tokio::test]
async fn sm26_adv_l4_empty_bearer_prefix_and_estop_control_plane_confinement() {
    use marlin_agent_app_server::wire::marlin::{
        app::v1::{
            StartSessionTurnRequest, StreamTurnRequest, app_server_server::AppServer as AppServerRpc,
        },
        common::v1::TargetKind,
        device::v1::{
            RegisterDeviceNodeRequest, device_bridge_service_server::DeviceBridgeService,
        },
        plugin::v1::{
            SubmitScopedPanelActionRequest, plugin_service_server::PluginService,
        },
        ui::v1::{
            DispatchUiEventRequest, dispatch_ui_event_request,
            ui_workbench_service_server::UiWorkbenchService,
        },
    };
    use marlin_agent_kernel::{
        ExecutionCircuitBreaker, GovernanceAdmissionGate, ToolBudgetUnits,
    };
    use std::sync::Arc;
    use tonic::Request;

    // 1. ADV-L4 Empty-Bearer prefix exploit: `"Bearer "` and `"Bearer   "` MUST fail closed!
    let empty_bearer = meta_with_auth("Bearer ");
    assert!(
        MyGoBridgeHost::check_bearer_header(&empty_bearer, Some("Bearer ")).is_err(),
        "ADV-L4 EMPTY-BEARER BYPASS PROVEN: Some(\"Bearer \") admitted an empty Bearer credential!"
    );
    let spaces_bearer = meta_with_auth("Bearer   ");
    assert!(
        MyGoBridgeHost::check_bearer_header(&spaces_bearer, Some("Bearer   ")).is_err(),
        "ADV-L4 EMPTY-BEARER BYPASS PROVEN: Some(\"Bearer   \") admitted whitespace-only Bearer credential!"
    );
    assert!(
        MyGoBridgeHost::check_bearer_header(&meta_with_auth("Bearer"), Some("Bearer")).is_err(),
        "ADV-L4 EMPTY-BEARER BYPASS PROVEN: bare \"Bearer\" without credential was admitted!"
    );
    // Canonical entrypoint resolver refuses non-loopback without token & refuses explicit blank tokens everywhere.
    assert!(
        MyGoBridgeHost::resolve_bridge_auth_header("0.0.0.0:50051", Err(std::env::VarError::NotPresent)).is_err(),
        "SM26: non-loopback bind without MARLIN_BRIDGE_TOKEN must fail closed"
    );
    assert!(
        MyGoBridgeHost::resolve_bridge_auth_header("127.0.0.1:0", Ok(String::new())).is_err(),
        "SM26: explicit empty MARLIN_BRIDGE_TOKEN must fail closed"
    );
    assert!(
        MyGoBridgeHost::resolve_bridge_auth_header("127.0.0.1:0", Ok("   ".to_owned())).is_err(),
        "SM26: explicit whitespace MARLIN_BRIDGE_TOKEN must fail closed"
    );
    assert_eq!(
        MyGoBridgeHost::resolve_bridge_auth_header("127.0.0.1:0", Err(std::env::VarError::NotPresent))
            .expect("loopback dev mode"),
        None
    );
    assert_eq!(
        MyGoBridgeHost::resolve_bridge_auth_header("0.0.0.0:50051", Ok("tok-prod-99".to_owned()))
            .expect("non-loopback with token")
            .as_deref(),
        Some("Bearer tok-prod-99")
    );

    // 2. ADV-L4 Control-Plane RPC E-Stop bypass: trip Operator E-Stop on shared gate.
    let breaker = Arc::new(ExecutionCircuitBreaker::new(ToolBudgetUnits::new(64)));
    let gate = Arc::new(GovernanceAdmissionGate::new(breaker));
    gate.trip_operator_estop("operator", "SM26-ADV-L4 operator E-Stop")
        .expect("trip emergency stop");
    let host = MyGoBridgeHost::new_default_trust(|_| Ok(())).with_governance_gate(gate);

    // Mutating UI event MUST be denied with PermissionDenied while E-Stop is active.
    let ui_res = UiWorkbenchService::dispatch_ui_event(
        &host,
        Request::new(DispatchUiEventRequest {
            session_id: "sess-sm26".to_owned(),
            expected_generation: 0,
            event: Some(dispatch_ui_event_request::Event::RecordCommand(
                "model.select hostile".to_owned(),
            )),
        }),
    )
    .await;
    assert!(
        matches!(&ui_res, Err(st) if st.code() == tonic::Code::PermissionDenied),
        "ADV-L4 CONTROL-PLANE E-STOP BYPASS PROVEN: dispatch_ui_event succeeded under active E-Stop: {ui_res:?}"
    );

    // Untrusted WebView plugin scoped action MUST be denied with PermissionDenied under E-Stop.
    let action_res = PluginService::submit_scoped_panel_action(
        &host,
        Request::new(SubmitScopedPanelActionRequest {
            project_id: "default".to_owned(),
            session_id: "sess-sm26".to_owned(),
            plugin_id: "tao3k.panel".to_owned(),
            contribution_id: "main".to_owned(),
            activation_generation: 1,
            action_name: "run_tool".to_owned(),
            args_json: br#"{}"#.to_vec(),
            idempotency_key: "idem-sm26-1".to_owned(),
        }),
    )
    .await;
    assert!(
        matches!(&action_res, Err(st) if st.code() == tonic::Code::PermissionDenied),
        "ADV-L4 CONTROL-PLANE E-STOP BYPASS PROVEN: submit_scoped_panel_action bypassed active E-Stop: {action_res:?}"
    );

    // Device registration MUST be denied under E-Stop.
    let dev_res = DeviceBridgeService::register_device_node(
        &host,
        Request::new(RegisterDeviceNodeRequest {
            device_id: "dev-sm26".to_owned(),
            display_name: "SM26 Hostile Node".to_owned(),
            target_kind: TargetKind::DesktopMacos as i32,
            os_version: "macOS-15".to_owned(),
            app_version: "0.1.0".to_owned(),
            capabilities: Vec::new(),
        }),
    )
    .await;
    assert!(
        matches!(&dev_res, Err(st) if st.code() == tonic::Code::PermissionDenied),
        "ADV-L4 CONTROL-PLANE E-STOP BYPASS PROVEN: register_device_node bypassed active E-Stop: {dev_res:?}"
    );

    // Session turn & streaming turn MUST be denied under E-Stop.
    let turn_res = AppServerRpc::start_session_turn(
        &host,
        Request::new(StartSessionTurnRequest {
            session_id: "sess-sm26".to_owned(),
            after_turn: 0,
            user_text: "run hostile turn under E-Stop".to_owned(),
            submission_id: "sub-sm26-1".to_owned(),
            attachments: Vec::new(),
        }),
    )
    .await;
    assert!(
        matches!(&turn_res, Err(st) if st.code() == tonic::Code::PermissionDenied),
        "ADV-L4 CONTROL-PLANE E-STOP BYPASS PROVEN: start_session_turn bypassed active E-Stop: {turn_res:?}"
    );
    let stream_res = AppServerRpc::stream_turn(
        &host,
        Request::new(StreamTurnRequest {
            messages: Vec::new(),
        }),
    )
    .await;
    assert!(
        matches!(&stream_res, Err(st) if st.code() == tonic::Code::PermissionDenied),
        "ADV-L4 CONTROL-PLANE E-STOP BYPASS PROVEN: stream_turn bypassed active E-Stop!"
    );
}
