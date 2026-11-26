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

use std::{path::Path, process::Stdio, time::Duration};

use marlin_agent_app_server::{
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
        ui::v1::{OpenWorkbenchRequest, ui_workbench_service_client::UiWorkbenchServiceClient},
    },
};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::{Child, Command},
};

async fn spawn_scheme_host(gate_id: &str, turso_path: &Path) -> (Child, String) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_marlin-scheme-host"));
    cmd.env("MARLIN_BRIDGE_ADDR", "127.0.0.1:0")
        .env("MARLIN_BRIDGE_TURSO_PATH", turso_path)
        .env("MARLIN_BRIDGE_GATE_ID", gate_id)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .kill_on_drop(true);
    let mut child = cmd.spawn().expect("spawn marlin-scheme-host binary");
    let stdout = child.stdout.take().expect("piped stdout");
    let mut lines = BufReader::new(stdout).lines();
    let ready_line = tokio::time::timeout(Duration::from_secs(90), lines.next_line())
        .await
        .expect("timed out waiting for marlin-scheme-host READY line")
        .expect("read READY line")
        .expect("marlin-scheme-host exited before printing READY");
    let addr = ready_line
        .strip_prefix("READY ")
        .unwrap_or_else(|| panic!("unexpected READY line: {ready_line}"))
        .trim()
        .to_owned();
    (child, format!("http://{addr}"))
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
