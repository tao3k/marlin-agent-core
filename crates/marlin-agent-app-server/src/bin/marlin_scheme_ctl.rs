// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Operator CLI (`marlin-scheme-ctl`) for inspecting and steering a live
//! `marlin-scheme-host` or `marlin-mygo-host` daemon over HTTP/2 gRPC.
//!
//! Usage:
//!   marlin-scheme-ctl --endpoint http://127.0.0.1:PORT [--token TOKEN] <command>
//!
//! Commands:
//!   tabs                                           Show live WorkbenchTabs from AppServer
//!   watch-tabs [--count N] [--timeout-ms MS]       Stream live WatchWorkbenchTabs updates
//!   audit                                          Show live non-authorizing Scheme FleetAuditCapsule
//!   watch-trajectory [--session ID] [--count N] [--timeout-ms MS]
//!                                                  Stream live WatchGraphTrajectory events
//!   gates                                          List pending one-shot ApprovalGateService gates
//!   approve <gate-id> [note]                       Submit one-shot GraphHumanDecision::Approve
//!   reject  <gate-id> [note]                       Submit one-shot GraphHumanDecision::Reject
//!   dispatch <action> [arg]                        Dispatch declarative `scheme:<action>` via UiWorkbenchService

use std::{
    env,
    io::{self, Write},
    time::Duration,
};

use marlin_agent_app_server::{
    MarlinSchemeHostClient, scheme_host_client::format_facets_sorted, v1::WorkbenchTabKindProto,
    wire::marlin::gate::v1::GraphHumanDecision,
};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    if let Err(err) = run().await {
        eprintln!("marlin-scheme-ctl error: {err}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let mut endpoint = env::var("MARLIN_BRIDGE_ENDPOINT")
        .ok()
        .filter(|s| !s.trim().is_empty());
    let mut token = env::var("MARLIN_BRIDGE_TOKEN")
        .ok()
        .filter(|s| !s.trim().is_empty());
    let mut rest = Vec::new();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--endpoint" | "-e" => {
                let val = args
                    .next()
                    .ok_or_else(|| "missing value for --endpoint".to_owned())?;
                endpoint = Some(val);
            }
            "--token" | "-t" => {
                let val = args
                    .next()
                    .ok_or_else(|| "missing value for --token".to_owned())?;
                token = Some(val);
            }
            "--help" | "-h" => {
                println!(
                    "Usage: marlin-scheme-ctl --endpoint <url> [--token <tok>] <tabs|watch-tabs|audit|watch-trajectory|gates|approve|reject|dispatch>"
                );
                return Ok(());
            }
            _ => rest.push(arg),
        }
    }

    let endpoint = endpoint.ok_or_else(|| {
        "missing --endpoint <http://host:port> (or MARLIN_BRIDGE_ENDPOINT)".to_owned()
    })?;
    let cmd = rest.first().map(String::as_str).ok_or_else(|| {
        "missing subcommand (tabs|watch-tabs|audit|watch-trajectory|gates|approve|reject|dispatch)"
            .to_owned()
    })?;

    let client = MarlinSchemeHostClient::connect(endpoint, token.as_deref()).await?;

    match cmd {
        "tabs" => {
            let resp = client
                .get_workbench_tabs()
                .await
                .map_err(|s| format!("get_workbench_tabs failed: {s}"))?;
            println!("TABS count={}", resp.tabs.len());
            for tab in resp.tabs {
                let kind = match WorkbenchTabKindProto::try_from(tab.kind) {
                    Ok(WorkbenchTabKindProto::PinnedIndex) => "PinnedIndex",
                    Ok(WorkbenchTabKindProto::ActiveTask) => "ActiveTask",
                    Ok(WorkbenchTabKindProto::ReadOnlyPeek) => "ReadOnlyPeek",
                    _ => "Unspecified",
                };
                println!(
                    "TAB kind={kind} id={} epoch={} status={}",
                    tab.id, tab.fencing_epoch, tab.status_label
                );
            }
        }
        "watch-tabs" => {
            let flags = parse_watch_flags(&rest[1..])?;
            let mut stream = client
                .watch_workbench_tabs()
                .await
                .map_err(|s| format!("watch_workbench_tabs failed: {s}"))?;
            let mut seen = 0usize;
            loop {
                let next_msg = match flags.timeout_ms {
                    Some(ms) => tokio::time::timeout(Duration::from_millis(ms), stream.message())
                        .await
                        .map_err(|_| format!("watch-tabs timed out after {ms}ms"))?,
                    None => stream.message().await,
                }
                .map_err(|s| format!("watch_workbench_tabs stream error: {s}"))?;
                let Some(resp) = next_msg else {
                    break;
                };
                seen += 1;
                let ids: Vec<&str> = resp.tabs.iter().map(|t| t.id.as_str()).collect();
                println!(
                    "WATCH_TABS seq={seen} count={} ids={}",
                    resp.tabs.len(),
                    ids.join(",")
                );
                let _ = io::stdout().flush();
                if flags.count.is_some_and(|limit| seen >= limit) {
                    break;
                }
            }
        }
        "watch-trajectory" => {
            let flags = parse_watch_flags(&rest[1..])?;
            let session_id = flags
                .session
                .unwrap_or_else(|| "marlin-scheme-ctl-watch".to_owned());
            let mut stream = client
                .watch_graph_trajectory(session_id)
                .await
                .map_err(|s| format!("watch_graph_trajectory failed: {s}"))?;
            let mut seen = 0usize;
            loop {
                let next_msg = match flags.timeout_ms {
                    Some(ms) => tokio::time::timeout(Duration::from_millis(ms), stream.message())
                        .await
                        .map_err(|_| format!("watch-trajectory timed out after {ms}ms"))?,
                    None => stream.message().await,
                }
                .map_err(|s| format!("watch_graph_trajectory stream error: {s}"))?;
                let Some(resp) = next_msg else {
                    break;
                };
                if let Some(ev) = resp.event {
                    seen += 1;
                    let meta = format_facets_sorted(&ev.metadata);
                    if meta.is_empty() {
                        println!(
                            "TRAJECTORY seq={seen} id={} node={} kind={} status={}",
                            ev.event_id, ev.node_id, ev.node_kind, ev.status
                        );
                    } else {
                        println!(
                            "TRAJECTORY seq={seen} id={} node={} kind={} status={} {meta}",
                            ev.event_id, ev.node_id, ev.node_kind, ev.status
                        );
                    }
                    let _ = io::stdout().flush();
                    if flags.count.is_some_and(|limit| seen >= limit) {
                        break;
                    }
                }
            }
        }
        "audit" => {
            let maybe = client
                .query_fleet_audit_capsule("default")
                .await
                .map_err(|s| format!("query_fleet_audit_capsule failed: {s}"))?;
            match maybe {
                Some(cap) => {
                    println!(
                        "AUDIT active={} archived={} peek={} defeaters={} inbox={} disclosure={} favored_collapsed={} authenticates={} authorizes={}",
                        cap.active_actor_count,
                        cap.archived_actor_count,
                        cap.peek_count,
                        cap.total_active_defeaters,
                        cap.total_human_inbox_items,
                        cap.disclosure_level,
                        cap.favored_path_collapsed,
                        cap.authenticates_evidence,
                        cap.authorizes_effect
                    );
                }
                None => {
                    println!("AUDIT none");
                }
            }
        }
        "gates" => {
            let gates = client
                .list_pending_gates()
                .await
                .map_err(|s| format!("list_pending_gates failed: {s}"))?;
            println!("GATES count={}", gates.len());
            for g in gates {
                println!("GATE id={} tool={}", g.gate_id, g.tool_or_node_name);
            }
        }
        "approve" | "reject" => {
            let gate_id = rest
                .get(1)
                .cloned()
                .ok_or_else(|| format!("usage: {cmd} <gate-id> [note]"))?;
            let note = if rest.len() > 2 {
                Some(rest[2..].join(" "))
            } else {
                None
            };
            let decision = if cmd == "approve" {
                GraphHumanDecision::Approve
            } else {
                GraphHumanDecision::Reject
            };
            let idem = format!("ctl-{cmd}-{gate_id}");
            let receipt = client
                .submit_graph_human_decision(&gate_id, decision, idem, note)
                .await
                .map_err(|s| format!("submit_graph_human_decision failed: {s}"))?;
            if receipt.claimed {
                println!("GATE_DECISION status=CLAIMED gate={}", receipt.gate_id);
            } else {
                let reason = receipt
                    .denial
                    .map(|d| d.message)
                    .unwrap_or_else(|| "denied".to_owned());
                println!(
                    "GATE_DECISION status=DENIED gate={} reason={reason}",
                    receipt.gate_id
                );
            }
        }
        "dispatch" => {
            let action = rest
                .get(1)
                .map(String::as_str)
                .ok_or_else(|| "usage: dispatch <action-sym> [arg]".to_owned())?;
            let arg = if rest.len() > 2 {
                Some(rest[2..].join(" "))
            } else {
                None
            };
            let resp = client
                .dispatch_scheme_command("marlin-scheme-ctl", action, arg.as_deref())
                .await
                .map_err(|s| format!("dispatch_scheme_command failed: {s}"))?;
            println!("DISPATCH admitted={} action={action}", resp.admitted);
        }
        other => {
            return Err(format!("unknown subcommand `{other}`"));
        }
    }
    Ok(())
}

struct WatchFlags {
    count: Option<usize>,
    timeout_ms: Option<u64>,
    session: Option<String>,
}

fn parse_watch_flags(args: &[String]) -> Result<WatchFlags, String> {
    let mut count = None;
    let mut timeout_ms = None;
    let mut session = None;
    let mut idx = 0;
    while idx < args.len() {
        match args[idx].as_str() {
            "--count" | "-n" => {
                idx += 1;
                let raw = args
                    .get(idx)
                    .ok_or_else(|| "missing value for --count".to_owned())?;
                let parsed = raw
                    .parse::<usize>()
                    .map_err(|e| format!("invalid --count `{raw}`: {e}"))?;
                if parsed == 0 {
                    return Err("--count must be >= 1".to_owned());
                }
                count = Some(parsed);
            }
            "--timeout-ms" => {
                idx += 1;
                let raw = args
                    .get(idx)
                    .ok_or_else(|| "missing value for --timeout-ms".to_owned())?;
                let parsed = raw
                    .parse::<u64>()
                    .map_err(|e| format!("invalid --timeout-ms `{raw}`: {e}"))?;
                timeout_ms = Some(parsed);
            }
            "--session" => {
                idx += 1;
                let raw = args
                    .get(idx)
                    .ok_or_else(|| "missing value for --session".to_owned())?;
                session = Some(raw.clone());
            }
            other => {
                return Err(format!("unknown watch option `{other}`"));
            }
        }
        idx += 1;
    }
    Ok(WatchFlags {
        count,
        timeout_ms,
        session,
    })
}
