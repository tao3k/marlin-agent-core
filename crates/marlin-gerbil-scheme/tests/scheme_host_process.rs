// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Real OS-process double-boot end-to-end integration test for `marlin-scheme-host`.
//!
//! Spawns the real compiled `marlin-scheme-host` binary (main thread owning live embedded
//! Gambit/Gerbil Scheme VM Oracle, background multi-threaded Tokio hosting HTTP/2 `AppServer`
//! and `MyGoBridgeHost` over real disk `TursoAgentStorage` WAL), exercises all 5 `TaskActor`
//! RPCs, live `WatchWorkbenchTabs` streaming, and one-shot `ApprovalGateService`, sends graceful
//! `SIGTERM`, then reboots a brand-new OS process on the exact same Turso WAL database to
//! prove durable recovery cross-checked by a fresh embedded Gerbil Scheme Oracle.

use std::{
    path::Path,
    process::Stdio,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use marlin_agent_app_server::{
    GrpcCode, MarlinSchemeHostClient,
    v1::{
        BindGovernedContextRequest, DetourModeProto, DisclosureDepthProto,
        DispatchTaskActorRequest, ExecutionReceiptStatusProto, GetWorkbenchTabsRequest,
        GraphDispatchOperationProto, GraphSubstrateTargetProto, ReconcileTaskReceiptRequest,
        RegisterTaskActorRequest, TaskActorScopeModeProto, TaskActorStatusProto,
        WatchWorkbenchTabsRequest, WorkbenchTabKindProto,
        app_server_client::AppServerClient as V1AppServerClient,
    },
    wire::marlin::{
        common::v1::{DenialCode, TargetKind},
        gate::v1::{
            GateKind, GraphHumanDecision, SubmitGraphHumanDecisionRequest,
            approval_gate_service_client::ApprovalGateServiceClient,
        },
        projection::v1::{
            QueryProjectGraphRequest, SecretVisibility, WatchGraphTrajectoryRequest,
            projection_query_service_client::ProjectionQueryServiceClient,
        },
        ui::v1::{
            DispatchUiEventRequest, OpenWorkbenchRequest, dispatch_ui_event_request,
            ui_workbench_service_client::UiWorkbenchServiceClient,
        },
    },
};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
    process::{Child, Command},
};

async fn spawn_scheme_host_with_fleet(
    gate_id: &str,
    turso_path: &Path,
    fleet_checkpoint_key: Option<&str>,
) -> (
    Child,
    String,
    Option<String>,
    tokio::io::Lines<BufReader<tokio::process::ChildStdout>>,
) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_marlin-scheme-host"));
    cmd.env("MARLIN_BRIDGE_ADDR", "127.0.0.1:0")
        .env("MARLIN_BRIDGE_TURSO_PATH", turso_path)
        .env("MARLIN_BRIDGE_GATE_ID", gate_id)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .kill_on_drop(true);
    if let Some(key) = fleet_checkpoint_key {
        cmd.env("MARLIN_SCHEME_FLEET_CHECKPOINT_KEY", key);
        cmd.env(
            "MARLIN_SCHEME_LLM_LOOPBACK_RESPONSE",
            "deterministic-scheme-host-s26-payload",
        );
        cmd.env("MARLIN_SCHEME_SANDBOX_ALLOWLIST", "/bin/echo");
        if let Some(parent_dir) = turso_path.parent() {
            cmd.env("MARLIN_SCHEME_SANDBOX_ROOT", parent_dir);
        }
    }
    let mut child = cmd.spawn().expect("spawn marlin-scheme-host binary");
    let stdout = child.stdout.take().expect("piped stdout");
    let mut lines = BufReader::new(stdout).lines();
    let mut fleet_boot_line = None;
    let addr = loop {
        let line = tokio::time::timeout(Duration::from_secs(180), lines.next_line())
            .await
            .expect("timed out waiting for marlin-scheme-host READY line")
            .expect("read stdout line")
            .expect("marlin-scheme-host exited before printing READY");
        if line.starts_with("FLEET_BOOT ") {
            fleet_boot_line = Some(line);
            continue;
        }
        if let Some(rest) = line.strip_prefix("READY ") {
            break rest.trim().to_owned();
        }
        panic!("unexpected stdout line before READY: {line}");
    };
    (child, format!("http://{addr}"), fleet_boot_line, lines)
}

async fn spawn_scheme_host(gate_id: &str, turso_path: &Path) -> (Child, String) {
    let (child, ep, _boot, _lines) = spawn_scheme_host_with_fleet(gate_id, turso_path, None).await;
    (child, ep)
}

async fn stop_scheme_host(mut child: Child) {
    #[cfg(unix)]
    {
        if let Some(pid) = child.id() {
            let status = std::process::Command::new("kill")
                .args(["-TERM", &pid.to_string()])
                .status()
                .expect("send SIGTERM to marlin-scheme-host");
            assert!(status.success(), "kill -TERM failed");
        }
        let exit = tokio::time::timeout(Duration::from_secs(15), child.wait())
            .await
            .expect("timed out waiting for graceful marlin-scheme-host exit")
            .expect("wait on marlin-scheme-host");
        assert!(
            exit.success(),
            "marlin-scheme-host must exit cleanly on SIGTERM, got {exit}"
        );
    }
    #[cfg(not(unix))]
    {
        let _ = child.kill().await;
        let _ = child.wait().await;
    }
}

#[tokio::test]
async fn marlin_scheme_host_binary_live_scheme_oracle_and_turso_wal_survive_os_restart() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("scheme-host-s24.db");

    // -------------------------------------------------------------------------
    // OS Process Boot #1: Live Embedded Scheme VM Oracle + HTTP/2 + Turso WAL
    // -------------------------------------------------------------------------
    let (child1, ep1) = spawn_scheme_host("gate-scheme-proc-1", &db_path).await;

    let mut v1 = V1AppServerClient::connect(ep1.clone()).await.unwrap();
    let mut watch_client = V1AppServerClient::connect(ep1.clone()).await.unwrap();
    let mut tab_stream = watch_client
        .watch_workbench_tabs(WatchWorkbenchTabsRequest {})
        .await
        .unwrap()
        .into_inner();
    let init_tabs = tokio::time::timeout(Duration::from_secs(5), tab_stream.message())
        .await
        .expect("initial watch timeout")
        .unwrap()
        .expect("initial tabs");
    assert_eq!(init_tabs.tabs.len(), 1);
    assert_eq!(
        init_tabs.tabs[0].kind,
        WorkbenchTabKindProto::PinnedIndex as i32
    );

    // 1. Register TaskActor & verify live WatchWorkbenchTabs update.
    let reg = v1
        .register_task_actor(RegisterTaskActorRequest {
            task_id: "task-scheme-proc-1".to_owned(),
            title: "Live Scheme Host Actor".to_owned(),
            scope_mode: TaskActorScopeModeProto::Project as i32,
        })
        .await
        .unwrap()
        .into_inner()
        .cursor
        .unwrap();
    assert_eq!(reg.fencing_epoch, 1);

    // 2. Live Scheme Oracle admits valid ApplySteering & rejects steering without join_node_id.
    let disp_ok = v1
        .dispatch_task_actor(DispatchTaskActorRequest {
            request_id: "req-proc-steer-ok".to_owned(),
            task_id: "task-scheme-proc-1".to_owned(),
            operation: GraphDispatchOperationProto::ApplySteering as i32,
            fencing_epoch: 1,
            scope_mode: TaskActorScopeModeProto::Project as i32,
            join_node_id: Some("join-proc-s24".to_owned()),
            substrate_target: GraphSubstrateTargetProto::SqliteTurso as i32,
        })
        .await
        .unwrap()
        .into_inner();
    assert_eq!(disp_ok.fencing_epoch, 1);

    let disp_bad = v1
        .dispatch_task_actor(DispatchTaskActorRequest {
            request_id: "req-proc-steer-bad".to_owned(),
            task_id: "task-scheme-proc-1".to_owned(),
            operation: GraphDispatchOperationProto::ApplySteering as i32,
            fencing_epoch: 1,
            scope_mode: TaskActorScopeModeProto::Project as i32,
            join_node_id: None,
            substrate_target: GraphSubstrateTargetProto::SqliteTurso as i32,
        })
        .await
        .unwrap_err();
    assert!(disp_bad.message().contains("join"));

    // 3. Reconcile execution receipt via live Scheme Oracle (epoch 1 -> 2, WaitingSteering).
    let rec = v1
        .reconcile_task_receipt(ReconcileTaskReceiptRequest {
            request_id: "req-proc-steer-ok".to_owned(),
            task_id: "task-scheme-proc-1".to_owned(),
            fencing_epoch: 1,
            status: ExecutionReceiptStatusProto::Ok as i32,
        })
        .await
        .unwrap()
        .into_inner();
    assert!(rec.accepted);
    assert_eq!(rec.next_epoch, 2);
    assert_eq!(
        rec.next_status,
        TaskActorStatusProto::WaitingSteering as i32
    );

    // Verify WatchWorkbenchTabs streamed epoch=2 live over HTTP/2.
    let mut saw_epoch2 = false;
    for _ in 0..5 {
        if let Some(ev) = tokio::time::timeout(Duration::from_secs(5), tab_stream.message())
            .await
            .expect("watch epoch2 timeout")
            .unwrap()
            && ev
                .tabs
                .iter()
                .any(|t| t.id == "task-scheme-proc-1" && t.fencing_epoch == 2)
        {
            saw_epoch2 = true;
            break;
        }
    }
    assert!(saw_epoch2, "WatchWorkbenchTabs must stream epoch=2 update");
    drop(tab_stream);

    // 4. BindGovernedContext checked by live Scheme Oracle (valid admitted, auto_archive=true denied).
    let ctx_ok = v1
        .bind_governed_context(BindGovernedContextRequest {
            task_id: "task-scheme-proc-1".to_owned(),
            context_cut: 1,
            kv_prefix_digest: "sha256:scheme-host-s24".to_owned(),
            disclosure_depth: DisclosureDepthProto::L3 as i32,
            has_active_defeater: true,
            quarantined_memory_count: 1,
            sandbox_admitted: true,
            cedar_handoff_valid: true,
            detour_mode: DetourModeProto::BlockerActive as i32,
            auto_archive_enabled: false,
        })
        .await
        .unwrap()
        .into_inner();
    assert!(!ctx_ok.auto_archive_enabled);
    assert!(
        v1.bind_governed_context(BindGovernedContextRequest {
            task_id: "task-scheme-proc-1".to_owned(),
            context_cut: 1,
            kv_prefix_digest: "sha256:scheme-host-s24".to_owned(),
            disclosure_depth: DisclosureDepthProto::L3 as i32,
            has_active_defeater: true,
            quarantined_memory_count: 1,
            sandbox_admitted: true,
            cedar_handoff_valid: true,
            detour_mode: DetourModeProto::BlockerActive as i32,
            auto_archive_enabled: true,
        })
        .await
        .is_err()
    );

    // 5. GetWorkbenchTabs + co-hosted UiWorkbenchService + one-shot GraphHumanGate on same port.
    let tabs1 = v1
        .get_workbench_tabs(GetWorkbenchTabsRequest {})
        .await
        .unwrap()
        .into_inner();
    assert_eq!(tabs1.tabs.len(), 2);
    assert_eq!(tabs1.tabs[1].id, "task-scheme-proc-1");
    assert_eq!(tabs1.tabs[1].fencing_epoch, 2);
    assert_eq!(tabs1.tabs[1].status_label, "WaitingSteering");

    let mut ui = UiWorkbenchServiceClient::connect(ep1.clone())
        .await
        .unwrap();
    let wb = ui
        .open_workbench(OpenWorkbenchRequest {
            session_id: "s24-proc-ui".to_owned(),
            client_target_kind: TargetKind::DesktopMacos as i32,
            preferred_profile: None,
        })
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

    let mut gate = ApprovalGateServiceClient::connect(ep1).await.unwrap();
    let claim1 = gate
        .submit_graph_human_decision(SubmitGraphHumanDecisionRequest {
            turn: None,
            gate_id: "gate-scheme-proc-1".to_owned(),
            decision: GraphHumanDecision::Approve as i32,
            idempotency_key: "idem-s24-1".to_owned(),
            note: None,
        })
        .await
        .unwrap()
        .into_inner()
        .receipt
        .unwrap();
    assert!(claim1.claimed);
    assert_eq!(claim1.kind, GateKind::GraphHumanGate as i32);

    let replay = gate
        .submit_graph_human_decision(SubmitGraphHumanDecisionRequest {
            turn: None,
            gate_id: "gate-scheme-proc-1".to_owned(),
            decision: GraphHumanDecision::Approve as i32,
            idempotency_key: "idem-s24-2".to_owned(),
            note: None,
        })
        .await
        .unwrap()
        .into_inner()
        .receipt
        .unwrap();
    assert!(!replay.claimed);
    assert_eq!(replay.denial.unwrap().code, DenialCode::StaleOrdinal as i32);

    drop(v1);
    drop(watch_client);
    drop(ui);
    drop(gate);
    stop_scheme_host(child1).await;

    // -------------------------------------------------------------------------
    // OS Process Boot #2: Brand-new OS Process & Fresh Gambit VM on SAME Turso WAL
    // -------------------------------------------------------------------------
    let (child2, ep2) = spawn_scheme_host("gate-scheme-proc-2", &db_path).await;
    let mut v2 = V1AppServerClient::connect(ep2).await.unwrap();
    let tabs2 = v2
        .get_workbench_tabs(GetWorkbenchTabsRequest {})
        .await
        .unwrap()
        .into_inner();
    assert_eq!(tabs2, tabs1);

    drop(v2);
    stop_scheme_host(child2).await;
}

#[tokio::test]
async fn marlin_scheme_host_live_fleet_driver_turso_cas_and_human_gate_survive_os_restart() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("scheme-host-fleet-s26.db");
    let cp_key = "cas:fleet:scheme-host-s26";

    // -------------------------------------------------------------------------
    // OS Process Boot #1: Cold Scheme Fleet pump -> Turso CAS checkpoint ->
    // live HTTP/2 ApprovalGateService Approve -> Scheme task-human-approve
    // (defeaters: 1 -> 0) -> re-checkpoint to Turso CAS -> SIGTERM
    // -------------------------------------------------------------------------
    let (child1, ep1, boot1, mut lines1) =
        spawn_scheme_host_with_fleet("gate-fleet-s26-1", &db_path, Some(cp_key)).await;
    let boot1 = boot1.expect("expected FLEET_BOOT line on cold boot");
    assert!(
        boot1.contains("mode=cold")
            && boot1.contains("active=1")
            && boot1.contains("archived=1")
            && boot1.contains("defeaters=1"),
        "unexpected cold FLEET_BOOT line: {boot1}"
    );

    let mut watch_client = V1AppServerClient::connect(ep1.clone()).await.unwrap();
    let mut tab_stream = watch_client
        .watch_workbench_tabs(WatchWorkbenchTabsRequest {})
        .await
        .unwrap()
        .into_inner();
    let init_tabs = tokio::time::timeout(Duration::from_secs(5), tab_stream.message())
        .await
        .expect("initial fleet watch timeout")
        .unwrap()
        .expect("initial fleet tabs");
    assert_eq!(init_tabs.tabs.len(), 3);
    assert_eq!(
        init_tabs.tabs[0].kind,
        WorkbenchTabKindProto::PinnedIndex as i32
    );
    assert_eq!(init_tabs.tabs[1].id, "fleet-host-parent");
    assert_eq!(init_tabs.tabs[1].fencing_epoch, 1);
    assert_eq!(init_tabs.tabs[2].id, "fleet-host-sib");
    assert_eq!(
        init_tabs.tabs[2].kind,
        WorkbenchTabKindProto::ReadOnlyPeek as i32
    );

    let mut gate = ApprovalGateServiceClient::connect(ep1).await.unwrap();
    let receipt = gate
        .submit_graph_human_decision(SubmitGraphHumanDecisionRequest {
            turn: None,
            gate_id: "gate-fleet-s26-1".to_owned(),
            decision: GraphHumanDecision::Approve as i32,
            idempotency_key: "idem-fleet-s26-1".to_owned(),
            note: Some("def-turso-01".to_owned()),
        })
        .await
        .unwrap()
        .into_inner()
        .receipt
        .unwrap();
    assert!(receipt.claimed);

    // Wait for the live Gambit VM thread to finish dispatching `task-human-approve` and
    // storing the post-gate Scheme Fleet checkpoint into disk Turso CAS.
    let applied = tokio::time::timeout(Duration::from_secs(10), lines1.next_line())
        .await
        .expect("timed out waiting for FLEET_GATE_APPLIED")
        .expect("read FLEET_GATE_APPLIED")
        .expect("stdout closed before FLEET_GATE_APPLIED");
    assert!(
        applied.contains("FLEET_GATE_APPLIED")
            && applied.contains("gate=gate-fleet-s26-1")
            && applied.contains("active=1")
            && applied.contains("archived=1")
            && applied.contains("defeaters=0"),
        "unexpected FLEET_GATE_APPLIED line: {applied}"
    );

    // Live HTTP/2 WatchWorkbenchTabs stream must receive the post-gate epoch=2 push
    // synchronized directly from the Scheme Fleet approval transition!
    let mut saw_post_gate_epoch2 = false;
    for _ in 0..5 {
        if let Some(ev) = tokio::time::timeout(Duration::from_secs(5), tab_stream.message())
            .await
            .expect("watch post-gate epoch2 timeout")
            .unwrap()
            && ev
                .tabs
                .iter()
                .any(|t| t.id == "fleet-host-parent" && t.fencing_epoch == 2)
        {
            saw_post_gate_epoch2 = true;
            break;
        }
    }
    assert!(
        saw_post_gate_epoch2,
        "WatchWorkbenchTabs must stream live fleet-host-parent epoch=2 update after human gate approval"
    );

    drop(tab_stream);
    drop(watch_client);
    drop(gate);
    stop_scheme_host(child1).await;

    // -------------------------------------------------------------------------
    // OS Process Boot #2: Brand-new OS Process + Fresh Gambit VM restores the
    // EXACT post-gate Scheme Fleet (`defeaters=0`) AND TaskActorControlPlane (`epoch=2`)
    // directly from disk Turso WAL/CAS!
    // -------------------------------------------------------------------------
    let (child2, ep2, boot2, _lines2) =
        spawn_scheme_host_with_fleet("gate-fleet-s26-2", &db_path, Some(cp_key)).await;
    let boot2 = boot2.expect("expected FLEET_BOOT line on warm reboot");
    assert!(
        boot2.contains("mode=restored")
            && boot2.contains("active=1")
            && boot2.contains("archived=1")
            && boot2.contains("defeaters=0"),
        "restored OS boot must preserve post-gate Scheme Fleet defeaters=0, got: {boot2}"
    );

    let mut v2 = V1AppServerClient::connect(ep2).await.unwrap();
    let tabs2 = v2
        .get_workbench_tabs(GetWorkbenchTabsRequest {})
        .await
        .unwrap()
        .into_inner();
    assert_eq!(tabs2.tabs.len(), 3);
    assert_eq!(tabs2.tabs[1].id, "fleet-host-parent");
    assert_eq!(tabs2.tabs[1].fencing_epoch, 2);
    assert_eq!(tabs2.tabs[2].id, "fleet-host-sib");
    assert_eq!(
        tabs2.tabs[2].kind,
        WorkbenchTabKindProto::ReadOnlyPeek as i32
    );

    drop(v2);
    stop_scheme_host(child2).await;
}

async fn spawn_mock_openai_http_server(
    expected_api_key: String,
    expected_model: String,
    reply_content: String,
) -> (String, Arc<AtomicUsize>, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind mock openai http server");
    let addr = listener.local_addr().expect("mock openai local addr");
    let calls = Arc::new(AtomicUsize::new(0));
    let calls_srv = Arc::clone(&calls);
    let handle = tokio::spawn(async move {
        while let Ok((mut sock, _)) = listener.accept().await {
            let calls_conn = Arc::clone(&calls_srv);
            let exp_key = expected_api_key.clone();
            let exp_model = expected_model.clone();
            let content = reply_content.clone();
            tokio::spawn(async move {
                let mut buf = Vec::new();
                let mut tmp = [0u8; 4096];
                let header_end = loop {
                    let n = sock.read(&mut tmp).await.unwrap_or(0);
                    if n == 0 {
                        return;
                    }
                    buf.extend_from_slice(&tmp[..n]);
                    if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                        break pos;
                    }
                };
                let headers = String::from_utf8_lossy(&buf[..header_end]).into_owned();
                let content_len: usize = headers
                    .lines()
                    .find_map(|l| {
                        let lower = l.to_ascii_lowercase();
                        lower
                            .strip_prefix("content-length:")
                            .and_then(|v| v.trim().parse().ok())
                    })
                    .unwrap_or(0);
                while buf.len() < header_end + 4 + content_len {
                    let n = sock.read(&mut tmp).await.unwrap_or(0);
                    if n == 0 {
                        break;
                    }
                    buf.extend_from_slice(&tmp[..n]);
                }
                assert!(
                    headers.lines().any(
                        |l| l.eq_ignore_ascii_case(&format!("Authorization: Bearer {exp_key}"))
                    ),
                    "missing expected Bearer auth header in HTTP LLM request: {headers}"
                );
                let body_slice = &buf[header_end + 4..header_end + 4 + content_len];
                let req_json: serde_json::Value =
                    serde_json::from_slice(body_slice).expect("valid openai json body");
                assert_eq!(req_json["model"].as_str(), Some(exp_model.as_str()));
                assert!(
                    req_json["messages"]
                        .as_array()
                        .is_some_and(|arr| !arr.is_empty()),
                    "openai request must carry non-empty messages"
                );
                calls_conn.fetch_add(1, Ordering::SeqCst);

                let resp_body = serde_json::json!({
                    "id": "chatcmpl-marlin-s28",
                    "object": "chat.completion",
                    "model": exp_model,
                    "choices": [{
                        "index": 0,
                        "message": {
                            "role": "assistant",
                            "content": content,
                        },
                        "finish_reason": "stop",
                    }]
                })
                .to_string();
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    resp_body.len(),
                    resp_body
                );
                let _ = sock.write_all(resp.as_bytes()).await;
                let _ = sock.shutdown().await;
            });
        }
    });
    (format!("http://{addr}"), calls, handle)
}

async fn spawn_scheme_host_with_http_llm(
    gate_id: &str,
    turso_path: &Path,
    fleet_checkpoint_key: &str,
    llm_base_url: &str,
    llm_model: &str,
    llm_api_key: &str,
) -> (
    Child,
    String,
    String,
    tokio::io::Lines<BufReader<tokio::process::ChildStdout>>,
) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_marlin-scheme-host"));
    cmd.env("MARLIN_BRIDGE_ADDR", "127.0.0.1:0")
        .env("MARLIN_BRIDGE_TURSO_PATH", turso_path)
        .env("MARLIN_BRIDGE_GATE_ID", gate_id)
        .env("MARLIN_SCHEME_FLEET_CHECKPOINT_KEY", fleet_checkpoint_key)
        .env("MARLIN_SCHEME_LLM_BASE_URL", llm_base_url)
        .env("MARLIN_SCHEME_LLM_MODEL", llm_model)
        .env("MARLIN_SCHEME_LLM_API_KEY", llm_api_key)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .kill_on_drop(true);
    let mut child = cmd.spawn().expect("spawn marlin-scheme-host with http llm");
    let stdout = child.stdout.take().expect("piped stdout");
    let mut lines = BufReader::new(stdout).lines();
    let mut fleet_boot = None;
    let addr = loop {
        let line = tokio::time::timeout(Duration::from_secs(180), lines.next_line())
            .await
            .expect("timed out waiting for READY")
            .expect("read stdout")
            .expect("exited before READY");
        if line.starts_with("FLEET_BOOT ") {
            fleet_boot = Some(line);
            continue;
        }
        if let Some(rest) = line.strip_prefix("READY ") {
            break rest.trim().to_owned();
        }
        panic!("unexpected stdout line before READY: {line}");
    };
    (
        child,
        format!("http://{addr}"),
        fleet_boot.expect("expected FLEET_BOOT"),
        lines,
    )
}

#[tokio::test]
async fn marlin_scheme_host_real_http_openai_llm_and_dispatch_ui_event_survive_os_restart() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("scheme-host-http-llm-s28.db");
    let cp_key = "cas:fleet:scheme-host-http-s28";

    let (llm_url, llm_calls, llm_srv) = spawn_mock_openai_http_server(
        "sk-marlin-s28-test".to_owned(),
        "marlin-s28-test-model".to_owned(),
        "http-openai-s28-verified-output".to_owned(),
    )
    .await;

    // -------------------------------------------------------------------------
    // OS Process Boot #1: Real HTTP/1.1 OpenAI POST /v1/chat/completions ->
    // Turso CAS checkpoint -> DispatchUiEvent(RecordCommand) -> live HTTP/2
    // WatchWorkbenchTabs push (reopen sibling) -> SIGTERM
    // -------------------------------------------------------------------------
    let (child1, ep1, boot1, mut lines1) = spawn_scheme_host_with_http_llm(
        "gate-http-s28-1",
        &db_path,
        cp_key,
        &llm_url,
        "marlin-s28-test-model",
        "sk-marlin-s28-test",
    )
    .await;
    assert!(
        boot1.contains("mode=cold")
            && boot1.contains("active=1")
            && boot1.contains("archived=1")
            && boot1.contains("defeaters=1"),
        "unexpected cold boot line: {boot1}"
    );
    assert_eq!(
        llm_calls.load(Ordering::SeqCst),
        1,
        "cold boot must execute exactly 1 real HTTP OpenAI completion on CasMiss"
    );

    let mut watch_client = V1AppServerClient::connect(ep1.clone()).await.unwrap();
    let mut tab_stream = watch_client
        .watch_workbench_tabs(WatchWorkbenchTabsRequest {})
        .await
        .unwrap()
        .into_inner();
    let init_tabs = tokio::time::timeout(Duration::from_secs(5), tab_stream.message())
        .await
        .expect("initial watch timeout")
        .unwrap()
        .expect("initial tabs");
    assert_eq!(init_tabs.tabs.len(), 3);
    assert_eq!(
        init_tabs.tabs[2].kind,
        WorkbenchTabKindProto::ReadOnlyPeek as i32
    );

    let mut proj = ProjectionQueryServiceClient::connect(ep1.clone())
        .await
        .unwrap();
    // Security negative control: UI clients cannot elevate SecretVisibility::Allowed!
    assert!(
        proj.query_project_graph(QueryProjectGraphRequest {
            project_id: "default".to_owned(),
            session_id: None,
            surfaces: Vec::new(),
            secret_visibility: SecretVisibility::Allowed as i32,
            query: String::new(),
            page: None,
        })
        .await
        .is_err()
    );
    // Live read-only non-authorizing FleetAuditCapsule projection on cold boot!
    let cold_graph = proj
        .query_project_graph(QueryProjectGraphRequest {
            project_id: "default".to_owned(),
            session_id: None,
            surfaces: Vec::new(),
            secret_visibility: SecretVisibility::Denied as i32,
            query: String::new(),
            page: None,
        })
        .await
        .unwrap()
        .into_inner();
    assert_eq!(cold_graph.records.len(), 1);
    let cold_rec = &cold_graph.records[0];
    assert_eq!(cold_rec.kind, "scheme.fleet.audit_capsule");
    assert_eq!(
        cold_rec
            .facets
            .get("active_actor_count")
            .map(String::as_str),
        Some("1")
    );
    assert_eq!(
        cold_rec
            .facets
            .get("archived_actor_count")
            .map(String::as_str),
        Some("1")
    );
    assert_eq!(
        cold_rec
            .facets
            .get("total_active_defeaters")
            .map(String::as_str),
        Some("1")
    );
    assert_eq!(
        cold_rec
            .facets
            .get("favored_path_collapsed")
            .map(String::as_str),
        Some("false")
    );
    assert_eq!(
        cold_rec
            .facets
            .get("authenticates_evidence")
            .map(String::as_str),
        Some("false")
    );
    assert_eq!(
        cold_rec.facets.get("authorizes_effect").map(String::as_str),
        Some("false")
    );

    let mut traj_client = ProjectionQueryServiceClient::connect(ep1.clone())
        .await
        .unwrap();
    let mut traj_stream = traj_client
        .watch_graph_trajectory(WatchGraphTrajectoryRequest {
            session_id: "s29-traj".to_owned(),
            run_id: None,
            after_event_id: None,
        })
        .await
        .unwrap()
        .into_inner();
    let boot_traj = tokio::time::timeout(Duration::from_secs(5), traj_stream.message())
        .await
        .expect("initial trajectory timeout")
        .unwrap()
        .expect("initial boot trajectory event")
        .event
        .unwrap();
    assert_eq!(boot_traj.node_id, "fleet_boot_cold");

    let mut ui = UiWorkbenchServiceClient::connect(ep1.clone())
        .await
        .unwrap();

    // 1. Security negative control: UI command MUST NOT bypass one-shot ApprovalGateService!
    let _ = ui
        .dispatch_ui_event(DispatchUiEventRequest {
            session_id: "s28-ui".to_owned(),
            expected_generation: 0,
            event: Some(dispatch_ui_event_request::Event::RecordCommand(
                "scheme:task-human-approve def-turso-01".to_owned(),
            )),
        })
        .await
        .unwrap();

    // 2. Dispatch declarative `task-drill-l3` via gRPC DispatchUiEvent -> Scheme Fleet.
    let _ = ui
        .dispatch_ui_event(DispatchUiEventRequest {
            session_id: "s28-ui".to_owned(),
            expected_generation: 0,
            event: Some(dispatch_ui_event_request::Event::RecordCommand(
                "scheme:task-drill-l3".to_owned(),
            )),
        })
        .await
        .unwrap();
    let l3_line = tokio::time::timeout(Duration::from_secs(10), lines1.next_line())
        .await
        .expect("drill-l3 timeout")
        .expect("read drill-l3")
        .expect("stdout closed before drill-l3");
    assert!(
        l3_line.contains("FLEET_UI_ACTION action=task-drill-l3")
            && l3_line.contains("disclosure=L3")
            && l3_line.contains("defeaters=1"),
        "task-human-approve via UI must have been blocked (defeaters still 1); got: {l3_line}"
    );

    // 3. Dispatch `task-propose-branch` + `task-reopen fleet-host-sib` via gRPC DispatchUiEvent.
    let _ = ui
        .dispatch_ui_event(DispatchUiEventRequest {
            session_id: "s28-ui".to_owned(),
            expected_generation: 0,
            event: Some(dispatch_ui_event_request::Event::RecordCommand(
                "scheme:task-propose-branch B-s28:Verify HTTP LLM evidence".to_owned(),
            )),
        })
        .await
        .unwrap();
    let branch_line = tokio::time::timeout(Duration::from_secs(10), lines1.next_line())
        .await
        .expect("propose-branch timeout")
        .expect("read propose-branch")
        .expect("stdout closed before propose-branch");
    assert!(
        branch_line.contains("FLEET_UI_ACTION action=task-propose-branch")
            && branch_line.contains("active=1")
            && branch_line.contains("defeaters=1"),
        "unexpected propose-branch line: {branch_line}"
    );

    let _ = ui
        .dispatch_ui_event(DispatchUiEventRequest {
            session_id: "s28-ui".to_owned(),
            expected_generation: 0,
            event: Some(dispatch_ui_event_request::Event::RecordCommand(
                "scheme:task-reopen fleet-host-sib".to_owned(),
            )),
        })
        .await
        .unwrap();
    let reopen_line = tokio::time::timeout(Duration::from_secs(10), lines1.next_line())
        .await
        .expect("reopen timeout")
        .expect("read reopen")
        .expect("stdout closed before reopen");
    assert!(
        reopen_line.contains("FLEET_UI_ACTION action=task-reopen")
            && reopen_line.contains("active=2")
            && reopen_line.contains("archived=0")
            && reopen_line.contains("defeaters=1"),
        "unexpected reopen line: {reopen_line}"
    );

    // WatchWorkbenchTabs must immediately push `fleet-host-sib` promoted from ReadOnlyPeek -> ActiveTask!
    let mut saw_reopened_active = false;
    for _ in 0..8 {
        if let Some(ev) = tokio::time::timeout(Duration::from_secs(5), tab_stream.message())
            .await
            .expect("watch reopen timeout")
            .unwrap()
            && ev.tabs.iter().any(|t| {
                t.id == "fleet-host-sib" && t.kind == WorkbenchTabKindProto::ActiveTask as i32
            })
        {
            saw_reopened_active = true;
            break;
        }
    }
    assert!(
        saw_reopened_active,
        "WatchWorkbenchTabs must live-push fleet-host-sib as ActiveTask after task-reopen"
    );

    // Live WatchGraphTrajectory must stream the `task-reopen` Scheme Fleet transition!
    let mut saw_reopen_traj = false;
    for _ in 0..8 {
        if let Some(msg) = tokio::time::timeout(Duration::from_secs(5), traj_stream.message())
            .await
            .expect("trajectory reopen timeout")
            .unwrap()
            && let Some(ev) = msg.event
            && ev.node_id == "fleet_ui_action:task-reopen"
            && ev.metadata.get("active_actor_count").map(String::as_str) == Some("2")
            && ev.metadata.get("authorizes_effect").map(String::as_str) == Some("false")
        {
            saw_reopen_traj = true;
            break;
        }
    }
    assert!(
        saw_reopen_traj,
        "WatchGraphTrajectory must stream live fleet_ui_action:task-reopen event"
    );

    // QueryProjectGraph must immediately reflect the reopened sibling (active=2, archived=0, L3)!
    let live_graph = proj
        .query_project_graph(QueryProjectGraphRequest {
            project_id: "default".to_owned(),
            session_id: None,
            surfaces: Vec::new(),
            secret_visibility: SecretVisibility::Denied as i32,
            query: String::new(),
            page: None,
        })
        .await
        .unwrap()
        .into_inner();
    let live_rec = &live_graph.records[0];
    assert_eq!(
        live_rec
            .facets
            .get("active_actor_count")
            .map(String::as_str),
        Some("2")
    );
    assert_eq!(
        live_rec
            .facets
            .get("archived_actor_count")
            .map(String::as_str),
        Some("0")
    );
    assert_eq!(
        live_rec.facets.get("disclosure_level").map(String::as_str),
        Some("L3")
    );
    assert_eq!(
        live_rec.facets.get("authorizes_effect").map(String::as_str),
        Some("false")
    );

    drop(traj_stream);
    drop(traj_client);
    drop(proj);
    drop(tab_stream);
    drop(watch_client);
    drop(ui);
    stop_scheme_host(child1).await;

    // -------------------------------------------------------------------------
    // OS Process Boot #2: Brand-new OS Process restores `active=2 archived=0`
    // from disk Turso CAS with ZERO extra HTTP LLM calls and exposes the restored
    // FleetAuditCapsule via live HTTP/2 ProjectionQueryService!
    // -------------------------------------------------------------------------
    let (child2, ep2, boot2, _lines2) = spawn_scheme_host_with_http_llm(
        "gate-http-s28-2",
        &db_path,
        cp_key,
        &llm_url,
        "marlin-s28-test-model",
        "sk-marlin-s28-test",
    )
    .await;
    assert!(
        boot2.contains("mode=restored")
            && boot2.contains("active=2")
            && boot2.contains("archived=0")
            && boot2.contains("defeaters=1"),
        "restored boot must preserve reopened sibling active=2 archived=0; got: {boot2}"
    );
    assert_eq!(
        llm_calls.load(Ordering::SeqCst),
        1,
        "warm CAS restore must invoke ZERO additional HTTP LLM completions"
    );

    let mut v2 = V1AppServerClient::connect(ep2.clone()).await.unwrap();
    let tabs2 = v2
        .get_workbench_tabs(GetWorkbenchTabsRequest {})
        .await
        .unwrap()
        .into_inner();
    assert_eq!(tabs2.tabs.len(), 3);
    assert!(tabs2.tabs.iter().any(|t| {
        t.id == "fleet-host-parent" && t.kind == WorkbenchTabKindProto::ActiveTask as i32
    }));
    assert!(tabs2.tabs.iter().any(|t| {
        t.id == "fleet-host-sib" && t.kind == WorkbenchTabKindProto::ActiveTask as i32
    }));

    let mut proj2 = ProjectionQueryServiceClient::connect(ep2).await.unwrap();
    let restored_graph = proj2
        .query_project_graph(QueryProjectGraphRequest {
            project_id: "default".to_owned(),
            session_id: None,
            surfaces: Vec::new(),
            secret_visibility: SecretVisibility::Denied as i32,
            query: String::new(),
            page: None,
        })
        .await
        .unwrap()
        .into_inner();
    let r2 = &restored_graph.records[0];
    assert_eq!(r2.kind, "scheme.fleet.audit_capsule");
    assert_eq!(
        r2.facets.get("active_actor_count").map(String::as_str),
        Some("2")
    );
    assert_eq!(
        r2.facets.get("archived_actor_count").map(String::as_str),
        Some("0")
    );
    assert_eq!(
        r2.facets.get("authorizes_effect").map(String::as_str),
        Some("false")
    );

    drop(proj2);
    drop(v2);
    stop_scheme_host(child2).await;
    llm_srv.abort();
}

#[tokio::test]
async fn marlin_scheme_host_client_enforces_bearer_auth_and_drives_full_lifecycle_across_restart() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("scheme-host-client-s30.db");
    let cp_key = "cas:fleet:scheme-host-client-s30";
    let bearer_token = "secret-s30-operator-token";

    let (llm_url, llm_calls, llm_srv) = spawn_mock_openai_http_server(
        "sk-marlin-s30".to_owned(),
        "marlin-s30-model".to_owned(),
        "s30-client-llm-ok".to_owned(),
    )
    .await;

    let spawn_authed = |gate_id: &str| {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_marlin-scheme-host"));
        cmd.env("MARLIN_BRIDGE_ADDR", "127.0.0.1:0")
            .env("MARLIN_BRIDGE_TOKEN", bearer_token)
            .env("MARLIN_BRIDGE_TURSO_PATH", &db_path)
            .env("MARLIN_BRIDGE_GATE_ID", gate_id)
            .env("MARLIN_SCHEME_FLEET_CHECKPOINT_KEY", cp_key)
            .env("MARLIN_SCHEME_LLM_BASE_URL", &llm_url)
            .env("MARLIN_SCHEME_LLM_MODEL", "marlin-s30-model")
            .env("MARLIN_SCHEME_LLM_API_KEY", "sk-marlin-s30")
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .kill_on_drop(true);
        cmd
    };

    // Boot #1 with Bearer auth enforced.
    let mut child1 = spawn_authed("gate-s30-1").spawn().expect("spawn boot1");
    let mut lines1 = BufReader::new(child1.stdout.take().unwrap()).lines();
    let mut boot1 = None;
    let addr1 = loop {
        let line = tokio::time::timeout(Duration::from_secs(180), lines1.next_line())
            .await
            .expect("boot1 READY timeout")
            .unwrap()
            .expect("boot1 exited early");
        if line.starts_with("FLEET_BOOT ") {
            boot1 = Some(line);
            continue;
        }
        if let Some(rest) = line.strip_prefix("READY ") {
            break rest.trim().to_owned();
        }
    };
    assert!(boot1.unwrap().contains("mode=cold"));
    let ep1 = format!("http://{addr1}");

    // 1. Unauthenticated client MUST be rejected fail-closed across all services!
    let unauth = MarlinSchemeHostClient::connect(ep1.clone(), None)
        .await
        .expect("connect unauth");
    assert_eq!(
        unauth.get_workbench_tabs().await.unwrap_err().code(),
        GrpcCode::Unauthenticated
    );
    assert_eq!(
        unauth
            .query_fleet_audit_capsule("default")
            .await
            .unwrap_err()
            .code(),
        GrpcCode::Unauthenticated
    );
    assert_eq!(
        unauth.list_pending_gates().await.unwrap_err().code(),
        GrpcCode::Unauthenticated
    );

    // 2. Authenticated `MarlinSchemeHostClient` drives tabs, audit, UI dispatch, and one-shot gate.
    let client1 = MarlinSchemeHostClient::connect(ep1, Some(bearer_token))
        .await
        .expect("connect authed");
    let cap0 = client1
        .query_fleet_audit_capsule("default")
        .await
        .unwrap()
        .expect("cold capsule");
    assert_eq!(cap0.active_actor_count, 1);
    assert_eq!(cap0.archived_actor_count, 1);
    assert_eq!(cap0.total_active_defeaters, 1);
    assert!(!cap0.authorizes_effect);

    let gates = client1.list_pending_gates().await.unwrap();
    assert_eq!(gates.len(), 1);
    assert_eq!(gates[0].gate_id, "gate-s30-1");

    let mut traj = client1.watch_graph_trajectory("s30-traj").await.unwrap();
    let first_ev = tokio::time::timeout(Duration::from_secs(5), traj.message())
        .await
        .unwrap()
        .unwrap()
        .unwrap()
        .event
        .unwrap();
    assert_eq!(first_ev.node_id, "fleet_boot_cold");

    client1
        .dispatch_scheme_command("s30-ctl", "task-reopen", Some("fleet-host-sib"))
        .await
        .unwrap();
    let _ = tokio::time::timeout(Duration::from_secs(10), lines1.next_line())
        .await
        .unwrap()
        .unwrap();

    let claim = client1
        .submit_graph_human_decision(
            "gate-s30-1",
            GraphHumanDecision::Approve,
            "idem-s30-approve",
            Some("def-turso-01".to_owned()),
        )
        .await
        .unwrap();
    assert!(claim.claimed);
    let _ = tokio::time::timeout(Duration::from_secs(10), lines1.next_line())
        .await
        .unwrap()
        .unwrap();

    // Wait for `fleet_human_gate:gate-s30-1` trajectory event confirming control-plane sync.
    let mut saw_gate_traj = false;
    for _ in 0..8 {
        if let Some(msg) = tokio::time::timeout(Duration::from_secs(5), traj.message())
            .await
            .unwrap()
            .unwrap()
            && let Some(ev) = msg.event
            && ev.node_id == "fleet_human_gate:gate-s30-1"
            && ev
                .metadata
                .get("total_active_defeaters")
                .map(String::as_str)
                == Some("0")
        {
            saw_gate_traj = true;
            break;
        }
    }
    assert!(saw_gate_traj, "expected fleet_human_gate trajectory event");

    // One-shot replay must be denied.
    let replay = client1
        .submit_graph_human_decision(
            "gate-s30-1",
            GraphHumanDecision::Approve,
            "idem-s30-replay",
            None,
        )
        .await
        .unwrap();
    assert!(!replay.claimed);

    drop(traj);
    drop(client1);
    drop(unauth);
    stop_scheme_host(child1).await;

    // Boot #2 on SAME Turso WAL with Bearer auth: verify restored capsule (active=2, defeaters=0)
    // and ZERO extra HTTP LLM calls!
    let mut child2 = spawn_authed("gate-s30-2").spawn().expect("spawn boot2");
    let mut lines2 = BufReader::new(child2.stdout.take().unwrap()).lines();
    let mut boot2 = None;
    let addr2 = loop {
        let line = tokio::time::timeout(Duration::from_secs(180), lines2.next_line())
            .await
            .expect("boot2 READY timeout")
            .unwrap()
            .expect("boot2 exited early");
        if line.starts_with("FLEET_BOOT ") {
            boot2 = Some(line);
            continue;
        }
        if let Some(rest) = line.strip_prefix("READY ") {
            break rest.trim().to_owned();
        }
    };
    assert!(boot2.unwrap().contains("mode=restored"));

    let client2 = MarlinSchemeHostClient::connect(format!("http://{addr2}"), Some(bearer_token))
        .await
        .expect("connect authed boot2");
    let cap2 = client2
        .query_fleet_audit_capsule("default")
        .await
        .unwrap()
        .expect("restored capsule");
    assert_eq!(cap2.active_actor_count, 2);
    assert_eq!(cap2.archived_actor_count, 0);
    assert_eq!(cap2.total_active_defeaters, 0);
    assert!(!cap2.favored_path_collapsed);
    assert!(!cap2.authenticates_evidence);
    assert!(!cap2.authorizes_effect);
    assert_eq!(llm_calls.load(Ordering::SeqCst), 1);

    drop(client2);
    stop_scheme_host(child2).await;
    llm_srv.abort();
}

#[tokio::test]
async fn marlin_scheme_host_executes_allowlisted_sandbox_run_rejects_disallowed_command_and_handles_gate_reject_across_restart()
 {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("scheme-host-s31-sandbox.db");
    let sandbox_root = dir.path().join("sandbox-root");
    std::fs::create_dir_all(&sandbox_root).expect("create sandbox root");
    let cp_key = "fleet:scheme-host:s31:sandbox";
    let bearer_token = "marlin-s31-sandbox-secret";

    let (llm_url, llm_calls, llm_srv) = spawn_mock_openai_http_server(
        "sk-marlin-s31-sandbox".to_owned(),
        "marlin-s31-model".to_owned(),
        "sandbox-boot-llm-ok".to_owned(),
    )
    .await;

    let spawn_host = |gate_id: &str| {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_marlin-scheme-host"));
        cmd.env("MARLIN_BRIDGE_ADDR", "127.0.0.1:0")
            .env("MARLIN_BRIDGE_TOKEN", bearer_token)
            .env("MARLIN_BRIDGE_TURSO_PATH", &db_path)
            .env("MARLIN_BRIDGE_GATE_ID", gate_id)
            .env("MARLIN_SCHEME_FLEET_CHECKPOINT_KEY", cp_key)
            .env("MARLIN_SCHEME_LLM_BASE_URL", &llm_url)
            .env("MARLIN_SCHEME_LLM_MODEL", "marlin-s31-model")
            .env("MARLIN_SCHEME_LLM_API_KEY", "sk-marlin-s31-sandbox")
            .env("MARLIN_SCHEME_SANDBOX_ALLOWLIST", "/bin/echo")
            .env("MARLIN_SCHEME_SANDBOX_ROOT", &sandbox_root)
            .env_remove("MARLIN_SCHEME_LLM_LOOPBACK_RESPONSE")
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .kill_on_drop(true);
        cmd
    };

    // Boot #1: cold boot with AllowlistSandboxRunner (/bin/echo only) + Bearer auth
    let mut child1 = spawn_host("gate-s31-reject-1")
        .spawn()
        .expect("spawn boot1");
    let mut lines1 = BufReader::new(child1.stdout.take().unwrap()).lines();
    let mut boot1 = None;
    let addr1 = loop {
        let line = tokio::time::timeout(Duration::from_secs(180), lines1.next_line())
            .await
            .expect("boot1 READY timeout")
            .unwrap()
            .expect("boot1 exited early");
        if line.starts_with("FLEET_BOOT ") {
            boot1 = Some(line);
            continue;
        }
        if let Some(rest) = line.strip_prefix("READY ") {
            break rest.trim().to_owned();
        }
    };
    assert!(boot1.unwrap().contains("mode=cold"));
    let ep1 = format!("http://{addr1}");

    let client1 = MarlinSchemeHostClient::connect(ep1, Some(bearer_token))
        .await
        .expect("connect authed");
    let cap0 = client1
        .query_fleet_audit_capsule("default")
        .await
        .unwrap()
        .expect("cold capsule");
    assert_eq!(cap0.active_actor_count, 1);
    assert_eq!(cap0.archived_actor_count, 1);
    assert_eq!(cap0.total_active_defeaters, 1);
    assert_eq!(cap0.total_human_inbox_items, 1);

    let mut traj = client1.watch_graph_trajectory("s31-traj").await.unwrap();
    let first_ev = tokio::time::timeout(Duration::from_secs(5), traj.message())
        .await
        .unwrap()
        .unwrap()
        .unwrap()
        .event
        .unwrap();
    assert_eq!(first_ev.node_id, "fleet_boot_cold");

    // 1. Real allowlisted `SandboxRun` (`/bin/echo marlin-s31-ok`): executes OS child process,
    //    ingests `SandboxExited(0)` in Scheme, settles obligation & auto-archives sibling!
    client1
        .dispatch_scheme_command(
            "s31-ctl",
            "task-sandbox-verify",
            Some("sib-sbx-ok:/bin/echo:marlin-s31-ok"),
        )
        .await
        .unwrap();
    let ok_line = tokio::time::timeout(Duration::from_secs(15), lines1.next_line())
        .await
        .expect("sandbox ok stdout timeout")
        .unwrap()
        .expect("host exited after sandbox ok");
    assert!(
        ok_line.contains("FLEET_UI_ACTION action=task-sandbox-verify")
            && ok_line.contains("active=1")
            && ok_line.contains("archived=2"),
        "allowlisted SandboxRun must settle & auto-archive sibling; got: {ok_line}"
    );
    // Synchronize on the live trajectory event published after `set_graph_projection_records`.
    let ok_ev = tokio::time::timeout(Duration::from_secs(5), traj.message())
        .await
        .expect("sandbox ok trajectory timeout")
        .unwrap()
        .unwrap()
        .event
        .unwrap();
    assert_eq!(ok_ev.node_id, "fleet_ui_action:task-sandbox-verify");
    assert_eq!(
        ok_ev
            .metadata
            .get("archived_actor_count")
            .map(String::as_str),
        Some("2")
    );
    let cap_ok = client1
        .query_fleet_audit_capsule("default")
        .await
        .unwrap()
        .expect("post-sandbox-ok capsule");
    assert_eq!(cap_ok.active_actor_count, 1);
    assert_eq!(cap_ok.archived_actor_count, 2);
    assert_eq!(cap_ok.total_human_inbox_items, 1);
    assert_eq!(
        llm_calls.load(Ordering::SeqCst),
        1,
        "allowlisted SandboxRun must execute via OS child process with ZERO extra LLM calls"
    );

    // 2. Disallowed `SandboxRun` (`/bin/sh forbidden`): AllowlistSandboxRunner denies fail-closed
    //    without spawning `/bin/sh`; Scheme oscillation detector halts actor in `blocker-active`
    //    and escalates to `human-inbox` (active=2, archived=2, inbox=2).
    client1
        .dispatch_scheme_command(
            "s31-ctl",
            "task-sandbox-verify",
            Some("sib-sbx-deny:/bin/sh:forbidden"),
        )
        .await
        .unwrap();
    let deny_line = tokio::time::timeout(Duration::from_secs(15), lines1.next_line())
        .await
        .expect("sandbox deny stdout timeout")
        .unwrap()
        .expect("host exited after sandbox deny");
    assert!(
        deny_line.contains("FLEET_UI_ACTION action=task-sandbox-verify")
            && deny_line.contains("active=2")
            && deny_line.contains("archived=2")
            && deny_line.contains("inbox=2"),
        "disallowed SandboxRun must fail-closed & escalate to human-inbox; got: {deny_line}"
    );
    let deny_ev = tokio::time::timeout(Duration::from_secs(5), traj.message())
        .await
        .expect("sandbox deny trajectory timeout")
        .unwrap()
        .unwrap()
        .event
        .unwrap();
    assert_eq!(deny_ev.node_id, "fleet_ui_action:task-sandbox-verify");
    assert_eq!(
        deny_ev
            .metadata
            .get("total_human_inbox_items")
            .map(String::as_str),
        Some("2")
    );
    let cap_deny = client1
        .query_fleet_audit_capsule("default")
        .await
        .unwrap()
        .expect("post-sandbox-deny capsule");
    assert_eq!(cap_deny.active_actor_count, 2);
    assert_eq!(cap_deny.archived_actor_count, 2);
    assert_eq!(cap_deny.total_human_inbox_items, 2);
    assert_eq!(cap_deny.total_active_defeaters, 1);

    // 3. One-shot `GraphHumanDecision::Reject` claims the gate, emits rejected trajectory event,
    //    denies replay, and NEVER resolves defeaters or authorizes effects!
    let reject_receipt = client1
        .submit_graph_human_decision(
            "gate-s31-reject-1",
            GraphHumanDecision::Reject,
            "idem-s31-reject",
            Some("operator rejected".to_owned()),
        )
        .await
        .unwrap();
    assert!(reject_receipt.claimed);
    let rej_line = tokio::time::timeout(Duration::from_secs(10), lines1.next_line())
        .await
        .expect("gate reject stdout timeout")
        .unwrap()
        .expect("host exited after gate reject");
    assert!(
        rej_line.contains("FLEET_GATE_REJECTED gate=gate-s31-reject-1")
            && rej_line.contains("defeaters=1"),
        "expected FLEET_GATE_REJECTED with defeaters=1; got: {rej_line}"
    );

    let mut saw_reject_traj = false;
    for _ in 0..8 {
        if let Some(msg) = tokio::time::timeout(Duration::from_secs(5), traj.message())
            .await
            .unwrap()
            .unwrap()
            && let Some(ev) = msg.event
            && ev.node_id == "fleet_human_gate_rejected:gate-s31-reject-1"
            && ev.node_kind == "scheme_human_gate_rejected"
            && ev
                .metadata
                .get("total_active_defeaters")
                .map(String::as_str)
                == Some("1")
        {
            saw_reject_traj = true;
            break;
        }
    }
    assert!(
        saw_reject_traj,
        "expected fleet_human_gate_rejected trajectory event with defeaters=1"
    );

    // Replay after Reject MUST still be denied one-shot.
    let replay = client1
        .submit_graph_human_decision(
            "gate-s31-reject-1",
            GraphHumanDecision::Approve,
            "idem-s31-replay-after-reject",
            Some("def-turso-01".to_owned()),
        )
        .await
        .unwrap();
    assert!(!replay.claimed);

    drop(traj);
    drop(client1);
    stop_scheme_host(child1).await;

    // Boot #2 on EXACT SAME Turso WAL: restored capsule preserves sandbox archived sibling,
    // denied sandbox blocker escalation, and unresolved defeater with ZERO extra LLM calls.
    let mut child2 = spawn_host("gate-s31-reject-2")
        .spawn()
        .expect("spawn boot2");
    let mut lines2 = BufReader::new(child2.stdout.take().unwrap()).lines();
    let mut boot2 = None;
    let addr2 = loop {
        let line = tokio::time::timeout(Duration::from_secs(180), lines2.next_line())
            .await
            .expect("boot2 READY timeout")
            .unwrap()
            .expect("boot2 exited early");
        if line.starts_with("FLEET_BOOT ") {
            boot2 = Some(line);
            continue;
        }
        if let Some(rest) = line.strip_prefix("READY ") {
            break rest.trim().to_owned();
        }
    };
    assert!(boot2.unwrap().contains("mode=restored"));

    let client2 = MarlinSchemeHostClient::connect(format!("http://{addr2}"), Some(bearer_token))
        .await
        .expect("connect authed boot2");
    let cap2 = client2
        .query_fleet_audit_capsule("default")
        .await
        .unwrap()
        .expect("restored capsule");
    assert_eq!(cap2.active_actor_count, 2);
    assert_eq!(cap2.archived_actor_count, 2);
    assert_eq!(cap2.total_active_defeaters, 1);
    assert_eq!(cap2.total_human_inbox_items, 2);
    assert!(!cap2.favored_path_collapsed);
    assert!(!cap2.authenticates_evidence);
    assert!(!cap2.authorizes_effect);
    assert_eq!(llm_calls.load(Ordering::SeqCst), 1);

    drop(client2);
    stop_scheme_host(child2).await;
    llm_srv.abort();
}
