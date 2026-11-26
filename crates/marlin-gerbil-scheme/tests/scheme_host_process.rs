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
        let line = tokio::time::timeout(Duration::from_secs(90), lines.next_line())
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
