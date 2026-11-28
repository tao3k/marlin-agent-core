// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use std::{collections::BTreeMap, process::Stdio, sync::Arc, time::Duration};

#[cfg(feature = "turso-host")]
use marlin_agent_app_server::v1::{
    CancelSessionTurnRequest, GetSessionTurnCancellationRequest,
    get_session_turn_cancellation_response::Evidence,
};
use marlin_agent_app_server::{
    AppServer, TaskActorControlPlane,
    ui_grpc::MyGoBridgeHost,
    v1::{
        DispatchTaskActorRequest, GetSessionSummaryRequest, GetSessionTurnStatusRequest,
        GetWorkbenchTabsRequest, GraphDispatchOperationProto, GraphSubstrateTargetProto,
        OpenSessionRequest, RegisterTaskActorRequest as V1RegisterTaskActorRequest,
        SessionOpenMode, SessionRecoveryState, SessionTurnOutcome, StartSessionTurnRequest,
        TaskActorScopeModeProto, WatchSessionTurnRequest, WatchWorkbenchTabsRequest,
        app_server_client::AppServerClient as V1AppServerClient, session_turn_event,
    },
    wire::marlin::{
        common::v1::{DenialCode, TargetKind},
        gate::v1::{
            SubmitToolReviewDecisionRequest, ToolReviewDecision,
            approval_gate_service_client::ApprovalGateServiceClient,
        },
        ui::v1::{
            OpenWorkbenchRequest, ui_workbench_service_client::UiWorkbenchServiceClient,
        },
    },
};
use marlin_agent_runtime::{TaskActorId, plugin::ReleaseTrust};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    net::TcpListener,
    process::Command,
};
use tonic::{Code, Status};

use super::support::{
    auth_req, run_and_watch_session_turn, spawn_mygo_host_process, stop_mygo_host_process,
};

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

    // 6. Real-time streaming `watch-tabs --count 2`: start CLI stream process, register a
    //    live TaskActor on marlin-mygo-host, and confirm the CLI receives both tab snapshots.
    let mut watch_child = Command::new(ctl_bin)
        .env("MARLIN_BRIDGE_ENDPOINT", &endpoint)
        .env("MARLIN_BRIDGE_TOKEN", "test-token")
        .args(["watch-tabs", "--count", "2", "--timeout-ms", "8000"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .expect("spawn marlin-scheme-ctl watch-tabs");
    let watch_stdout = watch_child.stdout.take().expect("watch-tabs stdout");
    let mut watch_lines = BufReader::new(watch_stdout).lines();

    let first_frame = tokio::time::timeout(Duration::from_secs(5), watch_lines.next_line())
        .await
        .expect("watch-tabs first frame timeout")
        .expect("watch-tabs first frame io")
        .expect("watch-tabs exited before first frame");
    assert!(
        first_frame.contains("WATCH_TABS seq=1") && first_frame.contains("index"),
        "unexpected first watch-tabs frame: {first_frame}"
    );

    let mut v1 = V1AppServerClient::connect(endpoint).await.unwrap();
    v1.register_task_actor(auth_req(V1RegisterTaskActorRequest {
        task_id: "task-ctl-stream-1".to_owned(),
        title: "CLI Watch Stream Actor".to_owned(),
        scope_mode: TaskActorScopeModeProto::Project as i32,
    }))
    .await
    .unwrap();

    let second_frame = tokio::time::timeout(Duration::from_secs(5), watch_lines.next_line())
        .await
        .expect("watch-tabs second frame timeout")
        .expect("watch-tabs second frame io")
        .expect("watch-tabs exited before second frame");
    assert!(
        second_frame.contains("WATCH_TABS seq=2") && second_frame.contains("task-ctl-stream-1"),
        "unexpected second watch-tabs frame: {second_frame}"
    );
    let watch_exit = tokio::time::timeout(Duration::from_secs(5), watch_child.wait())
        .await
        .expect("watch-tabs exit timeout")
        .expect("wait watch-tabs");
    assert!(watch_exit.success(), "watch-tabs exited with {watch_exit}");

    drop(v1);
    stop_mygo_host_process(child).await;
}
