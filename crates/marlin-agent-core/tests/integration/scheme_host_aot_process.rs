// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Real OS-process double-boot E2E test for the statically Linked-AOT
//! `marlin-scheme-host-aot` daemon binary.
//!
//! Proves:
//! 1. Sub-second / single-digit-second startup with ZERO runtime `.o1` `dlopen`
//!    and ZERO external `scheme/.gerbil` directory (`MARLIN_SCHEME_DIR=/nonexistent`).
//! 2. Bearer auth fail-closed enforcement across all 4 gRPC services via `MarlinSchemeHostClient`.
//! 3. Real allowlisted `/bin/echo` `SandboxRun` (zero extra LLM calls) + fail-closed
//!    `/bin/sh` denial escalating to `human-inbox` via Scheme oscillation detection.
//! 4. One-shot `ApprovalGateService::submit_graph_human_decision(Approve)` + replay denial.
//! 5. Graceful `SIGTERM` + OS Process Boot #2 restoring durable Turso WAL + CAS state
//!    with ZERO extra HTTP LLM calls.

use std::{
    process::Stdio,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

use marlin_agent_app_server::{
    GrpcCode, MarlinSchemeHostClient, wire::marlin::gate::v1::GraphHumanDecision,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
    process::{Child, Command},
};

async fn spawn_local_openai_http_server(
    expected_api_key: String,
    expected_model: String,
    reply_content: String,
) -> (String, Arc<AtomicUsize>, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind local openai http server");
    let addr = listener.local_addr().expect("local addr");
    let calls = Arc::new(AtomicUsize::new(0));
    let calls_srv = calls.clone();

    let handle = tokio::spawn(async move {
        while let Ok((mut stream, _)) = listener.accept().await {
            let expected_key = expected_api_key.clone();
            let expected_mod = expected_model.clone();
            let reply = reply_content.clone();
            let calls_inner = calls_srv.clone();
            tokio::spawn(async move {
                let mut buf = Vec::new();
                let mut tmp = [0u8; 4096];
                let mut header_end = None;
                let mut content_len = 0usize;
                loop {
                    let n = match stream.read(&mut tmp).await {
                        Ok(0) | Err(_) => return,
                        Ok(n) => n,
                    };
                    buf.extend_from_slice(&tmp[..n]);
                    if header_end.is_none()
                        && let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n")
                    {
                        header_end = Some(pos + 4);
                        let headers = String::from_utf8_lossy(&buf[..pos]);
                        for line in headers.lines() {
                            if let Some((k, v)) = line.split_once(':')
                                && k.trim().eq_ignore_ascii_case("content-length")
                            {
                                content_len = v.trim().parse::<usize>().unwrap_or(0);
                            }
                        }
                    }
                    if let Some(end) = header_end
                        && buf.len() >= end + content_len
                    {
                        break;
                    }
                }
                let end = header_end.unwrap_or(buf.len());
                let headers = String::from_utf8_lossy(&buf[..end]);
                let body = String::from_utf8_lossy(&buf[end..]);
                let auth_ok = headers.lines().any(|l| {
                    l.eq_ignore_ascii_case(&format!("authorization: Bearer {expected_key}"))
                });
                let model_ok = body.contains(&format!("\"model\":\"{expected_mod}\""))
                    || body.contains(&format!("\"model\": \"{expected_mod}\""));
                calls_inner.fetch_add(1, Ordering::SeqCst);
                let (status, resp_body) = if auth_ok && model_ok {
                    (
                        "200 OK",
                        format!(
                            "{{\"id\":\"aot-chat-1\",\"choices\":[{{\"message\":{{\"role\":\"assistant\",\"content\":\"{reply}\"}}}}]}}"
                        ),
                    )
                } else {
                    (
                        "401 Unauthorized",
                        "{\"error\":\"unauthorized\"}".to_owned(),
                    )
                };
                let raw = format!(
                    "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{resp_body}",
                    resp_body.len()
                );
                let _ = stream.write_all(raw.as_bytes()).await;
            });
        }
    });

    (format!("http://{addr}"), calls, handle)
}

async fn stop_host(mut child: Child) {
    #[cfg(unix)]
    {
        if let Some(pid) = child.id() {
            let status = std::process::Command::new("kill")
                .args(["-TERM", &pid.to_string()])
                .status()
                .expect("send SIGTERM to marlin-scheme-host-aot");
            assert!(status.success(), "kill -TERM failed");
        }
        let exit = tokio::time::timeout(Duration::from_secs(15), child.wait())
            .await
            .expect("timed out waiting for graceful marlin-scheme-host-aot exit")
            .expect("wait on marlin-scheme-host-aot");
        assert!(
            exit.success(),
            "marlin-scheme-host-aot should exit cleanly on SIGTERM, got {exit}"
        );
    }
    #[cfg(not(unix))]
    {
        let _ = child.kill().await;
        let _ = child.wait().await;
    }
}

#[tokio::test]
async fn linked_aot_scheme_host_boots_fast_without_scheme_tree_and_survives_os_restart() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("scheme-host-aot-s32.db");
    let sandbox_root = dir.path().join("sandbox-root");
    std::fs::create_dir_all(&sandbox_root).expect("create sandbox root");
    let cp_key = "fleet:scheme-host-aot:s32";
    let bearer_token = "marlin-aot-s32-secret";

    let (llm_url, llm_calls, llm_srv) = spawn_local_openai_http_server(
        "sk-marlin-aot-s32".to_owned(),
        "marlin-aot-model".to_owned(),
        "linked-aot-llm-ok".to_owned(),
    )
    .await;

    let spawn_aot = |gate_id: &str| {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_marlin-scheme-host-aot"));
        // Deliberately point MARLIN_SCHEME_DIR to a nonexistent path to prove Linked-AOT
        // never loads external `.o1` modules or requires a `scheme/.gerbil` tree at runtime!
        cmd.env("MARLIN_SCHEME_DIR", "/nonexistent-aot-must-not-read")
            .env("MARLIN_BRIDGE_ADDR", "127.0.0.1:0")
            .env("MARLIN_BRIDGE_TOKEN", bearer_token)
            .env("MARLIN_BRIDGE_TURSO_PATH", &db_path)
            .env("MARLIN_BRIDGE_GATE_ID", gate_id)
            .env("MARLIN_SCHEME_FLEET_CHECKPOINT_KEY", cp_key)
            .env("MARLIN_SCHEME_LLM_BASE_URL", &llm_url)
            .env("MARLIN_SCHEME_LLM_MODEL", "marlin-aot-model")
            .env("MARLIN_SCHEME_LLM_API_KEY", "sk-marlin-aot-s32")
            .env("MARLIN_SCHEME_SANDBOX_ALLOWLIST", "/bin/echo")
            .env("MARLIN_SCHEME_SANDBOX_ROOT", &sandbox_root)
            .env_remove("MARLIN_SCHEME_LLM_LOOPBACK_RESPONSE")
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .kill_on_drop(true);
        cmd
    };

    // -------------------------------------------------------------------------
    // OS Process Boot #1: Linked-AOT cold boot (ZERO dynamic `.o1` dlopen!)
    // -------------------------------------------------------------------------
    let t0 = Instant::now();
    let mut child1 = spawn_aot("gate-aot-s32-1")
        .spawn()
        .expect("spawn aot boot1");
    let mut lines1 = BufReader::new(child1.stdout.take().unwrap()).lines();
    let mut boot1 = None;
    let addr1 = loop {
        let line = tokio::time::timeout(Duration::from_secs(15), lines1.next_line())
            .await
            .expect("linked-aot boot1 MUST become READY within 15s (no dlopen)")
            .unwrap()
            .expect("aot boot1 exited early");
        if line.starts_with("FLEET_BOOT ") {
            boot1 = Some(line);
            continue;
        }
        if let Some(rest) = line.strip_prefix("READY ") {
            break rest.trim().to_owned();
        }
    };
    let boot1_ms = t0.elapsed().as_millis();
    eprintln!("LINKED_AOT_BOOT1_MS={boot1_ms}");
    assert!(boot1.unwrap().contains("mode=cold"));
    assert_eq!(llm_calls.load(Ordering::SeqCst), 1);
    let ep1 = format!("http://{addr1}");

    // 1. Unauthenticated client rejected fail-closed.
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

    // 2. Authenticated Linked-AOT full lifecycle: SandboxRun OK + SandboxRun Deny + Gate Approve!
    let client1 = MarlinSchemeHostClient::connect(ep1, Some(bearer_token))
        .await
        .expect("connect authed");
    let mut traj = client1.watch_graph_trajectory("aot-traj").await.unwrap();
    let ev0 = tokio::time::timeout(Duration::from_secs(5), traj.message())
        .await
        .unwrap()
        .unwrap()
        .unwrap()
        .event
        .unwrap();
    assert_eq!(ev0.node_id, "fleet_boot_cold");

    // 2a. Allowlisted `/bin/echo` SandboxRun settles & auto-archives with ZERO extra LLM calls.
    client1
        .dispatch_scheme_command(
            "aot-ctl",
            "task-sandbox-verify",
            Some("sib-aot-ok:/bin/echo:aot-sandbox-verified"),
        )
        .await
        .unwrap();
    let ok_line = tokio::time::timeout(Duration::from_secs(10), lines1.next_line())
        .await
        .expect("aot sandbox ok stdout timeout")
        .unwrap()
        .expect("aot host exited");
    assert!(
        ok_line.contains("FLEET_UI_ACTION action=task-sandbox-verify")
            && ok_line.contains("active=1")
            && ok_line.contains("archived=2"),
        "unexpected aot sandbox ok line: {ok_line}"
    );
    let ev_ok = tokio::time::timeout(Duration::from_secs(5), traj.message())
        .await
        .unwrap()
        .unwrap()
        .unwrap()
        .event
        .unwrap();
    assert_eq!(ev_ok.node_id, "fleet_ui_action:task-sandbox-verify");
    assert_eq!(llm_calls.load(Ordering::SeqCst), 1);

    // 2b. Disallowed `/bin/sh` fails closed & escalates to human-inbox (active=2, archived=2, inbox=2).
    client1
        .dispatch_scheme_command(
            "aot-ctl",
            "task-sandbox-verify",
            Some("sib-aot-deny:/bin/sh:forbidden"),
        )
        .await
        .unwrap();
    let deny_line = tokio::time::timeout(Duration::from_secs(10), lines1.next_line())
        .await
        .expect("aot sandbox deny stdout timeout")
        .unwrap()
        .expect("aot host exited");
    assert!(
        deny_line.contains("FLEET_UI_ACTION action=task-sandbox-verify")
            && deny_line.contains("active=2")
            && deny_line.contains("archived=2")
            && deny_line.contains("inbox=2"),
        "unexpected aot sandbox deny line: {deny_line}"
    );
    let _ = tokio::time::timeout(Duration::from_secs(5), traj.message())
        .await
        .unwrap()
        .unwrap();

    // 2b-neg. Security invariant: `scheme:task-human-approve` over `DispatchUiEvent` MUST NEVER
    //         bypass `ApprovalGateService` — defeaters must remain 1 until one-shot gate approval.
    client1
        .dispatch_scheme_command("aot-ctl", "task-human-approve", Some("def-turso-01"))
        .await
        .unwrap();
    let pre_gate_cap = client1
        .query_fleet_audit_capsule("default")
        .await
        .unwrap()
        .expect("pre-gate capsule");
    assert_eq!(
        pre_gate_cap.total_active_defeaters, 1,
        "DispatchUiEvent(scheme:task-human-approve) must never bypass ApprovalGateService"
    );

    // 2c. One-shot `GraphHumanDecision::Approve` resolves `def-turso-01` (`defeaters=0`).
    let claim = client1
        .submit_graph_human_decision(
            "gate-aot-s32-1",
            GraphHumanDecision::Approve,
            "idem-aot-approve",
            Some("def-turso-01".to_owned()),
        )
        .await
        .unwrap();
    assert!(claim.claimed);
    let gate_line = tokio::time::timeout(Duration::from_secs(10), lines1.next_line())
        .await
        .expect("aot gate stdout timeout")
        .unwrap()
        .expect("aot host exited");
    assert!(
        gate_line.contains("FLEET_GATE_APPLIED gate=gate-aot-s32-1")
            && gate_line.contains("defeaters=0"),
        "unexpected aot gate line: {gate_line}"
    );
    let replay = client1
        .submit_graph_human_decision(
            "gate-aot-s32-1",
            GraphHumanDecision::Approve,
            "idem-aot-replay",
            None,
        )
        .await
        .unwrap();
    assert!(!replay.claimed);

    drop(traj);
    drop(client1);
    drop(unauth);
    stop_host(child1).await;

    // -------------------------------------------------------------------------
    // OS Process Boot #2: Warm Linked-AOT restore from Turso WAL (ZERO LLM calls!)
    // -------------------------------------------------------------------------
    let t1 = Instant::now();
    let mut child2 = spawn_aot("gate-aot-s32-2")
        .spawn()
        .expect("spawn aot boot2");
    let mut lines2 = BufReader::new(child2.stdout.take().unwrap()).lines();
    let mut boot2 = None;
    let addr2 = loop {
        let line = tokio::time::timeout(Duration::from_secs(15), lines2.next_line())
            .await
            .expect("linked-aot boot2 MUST become READY within 15s (no dlopen)")
            .unwrap()
            .expect("aot boot2 exited early");
        if line.starts_with("FLEET_BOOT ") {
            boot2 = Some(line);
            continue;
        }
        if let Some(rest) = line.strip_prefix("READY ") {
            break rest.trim().to_owned();
        }
    };
    let boot2_ms = t1.elapsed().as_millis();
    let boot2_budget_ms: u128 = std::env::var("MARLIN_AOT_BOOT2_BUDGET_MS")
        .ok()
        .and_then(|v| v.trim().parse::<u128>().ok())
        .filter(|ms| *ms > 0)
        .unwrap_or(2_500);
    eprintln!("LINKED_AOT_BOOT2_MS={boot2_ms} LINKED_AOT_BOOT2_BUDGET_MS={boot2_budget_ms}");
    assert!(
        boot2_ms <= boot2_budget_ms,
        "Linked-AOT warm restart regressed: boot2_ms={boot2_ms}ms exceeded budget {boot2_budget_ms}ms (check AOT closure bloat or accidental dynamic dlopen)"
    );
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
    assert_eq!(cap2.total_active_defeaters, 0);
    assert_eq!(cap2.total_human_inbox_items, 2);
    assert!(!cap2.favored_path_collapsed);
    assert!(!cap2.authenticates_evidence);
    assert!(!cap2.authorizes_effect);
    assert_eq!(llm_calls.load(Ordering::SeqCst), 1);

    // 3. Exercise the real compiled `marlin-scheme-ctl` CLI against the live Linked-AOT host.
    //    When `MARLIN_SCHEME_CTL_BIN` or `CI` is set, missing the binary MUST fail closed
    //    rather than silently skipping CLI E2E coverage.
    let explicit_ctl = std::env::var_os("MARLIN_SCHEME_CTL_BIN").map(std::path::PathBuf::from);
    let require_ctl = explicit_ctl.is_some()
        || std::env::var("CI")
            .ok()
            .is_some_and(|v| matches!(v.trim(), "1" | "true" | "TRUE" | "yes"));
    let ctl_candidate = explicit_ctl
        .or_else(|| {
            std::path::Path::new(env!("CARGO_BIN_EXE_marlin-scheme-host-aot"))
                .parent()
                .map(|dir| dir.join("marlin-scheme-ctl"))
        })
        .filter(|p| p.is_file());
    if require_ctl {
        assert!(
            ctl_candidate.is_some(),
            "strict Linked-AOT CTL E2E enabled (CI or MARLIN_SCHEME_CTL_BIN set), but marlin-scheme-ctl binary was not found"
        );
    }
    if let Some(ctl_bin) = ctl_candidate {
        let ep2 = format!("http://{addr2}");
        let unauth_ctl = Command::new(&ctl_bin)
            .env_remove("MARLIN_BRIDGE_ENDPOINT")
            .env_remove("MARLIN_BRIDGE_TOKEN")
            .args(["--endpoint", &ep2, "audit"])
            .output()
            .await
            .expect("run unauth marlin-scheme-ctl against aot host");
        assert!(
            !unauth_ctl.status.success(),
            "unauth marlin-scheme-ctl must be rejected by aot host"
        );

        let audit_ctl = Command::new(&ctl_bin)
            .env("MARLIN_BRIDGE_ENDPOINT", &ep2)
            .env("MARLIN_BRIDGE_TOKEN", bearer_token)
            .arg("audit")
            .output()
            .await
            .expect("run marlin-scheme-ctl audit against aot host");
        assert!(
            audit_ctl.status.success(),
            "ctl audit failed: {}",
            String::from_utf8_lossy(&audit_ctl.stderr)
        );
        let audit_out = String::from_utf8_lossy(&audit_ctl.stdout);
        assert!(
            audit_out.contains("AUDIT active=2 archived=2")
                && audit_out.contains("defeaters=0 inbox=2")
                && audit_out.contains("authorizes=false"),
            "unexpected ctl audit output from aot host: {audit_out}"
        );

        let traj_ctl = Command::new(&ctl_bin)
            .env("MARLIN_BRIDGE_ENDPOINT", &ep2)
            .env("MARLIN_BRIDGE_TOKEN", bearer_token)
            .args(["watch-trajectory", "--count", "1", "--timeout-ms", "5000"])
            .output()
            .await
            .expect("run marlin-scheme-ctl watch-trajectory against aot host");
        assert!(
            traj_ctl.status.success(),
            "ctl watch-trajectory failed: {}",
            String::from_utf8_lossy(&traj_ctl.stderr)
        );
        let traj_out = String::from_utf8_lossy(&traj_ctl.stdout);
        assert!(
            traj_out.contains("TRAJECTORY seq=1") && traj_out.contains("fleet_boot_restored"),
            "unexpected ctl watch-trajectory output from aot host: {traj_out}"
        );

        let gates_ctl = Command::new(&ctl_bin)
            .env("MARLIN_BRIDGE_ENDPOINT", &ep2)
            .env("MARLIN_BRIDGE_TOKEN", bearer_token)
            .arg("gates")
            .output()
            .await
            .expect("run marlin-scheme-ctl gates against aot host");
        assert!(gates_ctl.status.success());
        let gates_out = String::from_utf8_lossy(&gates_ctl.stdout);
        assert!(
            gates_out.contains("GATES count=1") && gates_out.contains("GATE id=gate-aot-s32-2"),
            "unexpected ctl gates output: {gates_out}"
        );

        let rej_ctl = Command::new(&ctl_bin)
            .env("MARLIN_BRIDGE_ENDPOINT", &ep2)
            .env("MARLIN_BRIDGE_TOKEN", bearer_token)
            .args(["reject", "gate-aot-s32-2", "aot-cli-reject"])
            .output()
            .await
            .expect("run marlin-scheme-ctl reject against aot host");
        assert!(rej_ctl.status.success());
        let rej_out = String::from_utf8_lossy(&rej_ctl.stdout);
        assert!(
            rej_out.contains("GATE_DECISION status=CLAIMED gate=gate-aot-s32-2"),
            "unexpected ctl reject output: {rej_out}"
        );
        let rej_host_line = tokio::time::timeout(Duration::from_secs(10), lines2.next_line())
            .await
            .expect("aot host gate-rejected stdout timeout")
            .unwrap()
            .expect("aot host exited early");
        assert!(
            rej_host_line.contains("FLEET_GATE_REJECTED gate=gate-aot-s32-2"),
            "expected FLEET_GATE_REJECTED line, got: {rej_host_line}"
        );

        let tabs_ctl = Command::new(&ctl_bin)
            .env("MARLIN_BRIDGE_ENDPOINT", &ep2)
            .env("MARLIN_BRIDGE_TOKEN", bearer_token)
            .arg("tabs")
            .output()
            .await
            .expect("run marlin-scheme-ctl tabs against aot host");
        assert!(
            tabs_ctl.status.success(),
            "ctl tabs failed: {}",
            String::from_utf8_lossy(&tabs_ctl.stderr)
        );
        let tabs_out = String::from_utf8_lossy(&tabs_ctl.stdout);
        assert!(
            tabs_out.contains("TAB kind=PinnedIndex id=index")
                && tabs_out.contains("TAB kind=ActiveTask id=fleet-host-parent")
                && tabs_out.contains("TAB kind=ActiveTask id=sib-aot-deny"),
            "unexpected ctl tabs output from aot host: {tabs_out}"
        );

        let watch_tabs_ctl = Command::new(&ctl_bin)
            .env("MARLIN_BRIDGE_ENDPOINT", &ep2)
            .env("MARLIN_BRIDGE_TOKEN", bearer_token)
            .args(["watch-tabs", "--count", "1", "--timeout-ms", "5000"])
            .output()
            .await
            .expect("run marlin-scheme-ctl watch-tabs against aot host");
        assert!(
            watch_tabs_ctl.status.success(),
            "ctl watch-tabs failed: {}",
            String::from_utf8_lossy(&watch_tabs_ctl.stderr)
        );
        let watch_tabs_out = String::from_utf8_lossy(&watch_tabs_ctl.stdout);
        assert!(
            watch_tabs_out.contains("WATCH_TABS seq=1")
                && watch_tabs_out.contains("fleet-host-parent")
                && watch_tabs_out.contains("sib-aot-deny"),
            "unexpected ctl watch-tabs output from aot host: {watch_tabs_out}"
        );

        for (action, arg) in [
            ("task-detour-blocker", "blk-aot-cli"),
            ("task-detour-exit-return", ""),
            ("task-propose-branch", "B-aot-cli:Verify Turso lineage"),
        ] {
            let mut cmd = Command::new(&ctl_bin);
            cmd.env("MARLIN_BRIDGE_ENDPOINT", &ep2)
                .env("MARLIN_BRIDGE_TOKEN", bearer_token)
                .args(["dispatch", action]);
            if !arg.is_empty() {
                cmd.arg(arg);
            }
            let out = cmd
                .output()
                .await
                .unwrap_or_else(|e| panic!("run marlin-scheme-ctl dispatch {action}: {e}"));
            assert!(
                out.status.success(),
                "ctl dispatch {action} failed: {}",
                String::from_utf8_lossy(&out.stderr)
            );
            let stdout_str = String::from_utf8_lossy(&out.stdout);
            assert!(
                stdout_str.contains(&format!("DISPATCH admitted=true action={action}")),
                "unexpected ctl dispatch output for {action}: {stdout_str}"
            );
            let host_line = tokio::time::timeout(Duration::from_secs(10), lines2.next_line())
                .await
                .expect("aot host ui action stdout timeout")
                .unwrap()
                .expect("aot host exited early");
            assert!(
                host_line.contains(&format!("FLEET_UI_ACTION action={action}")),
                "expected FLEET_UI_ACTION for {action}, got: {host_line}"
            );
        }
        eprintln!("LINKED_AOT_CTL_E2E=VERIFIED");
    }

    drop(client2);
    stop_host(child2).await;
    llm_srv.abort();
}
