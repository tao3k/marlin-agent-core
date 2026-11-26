// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Operator CLI (`marlin-scheme-ctl`) for inspecting and steering a live
//! `marlin-scheme-host` or `marlin-mygo-host` daemon over HTTP/2 gRPC.
//!
//! Usage:
//!   marlin-scheme-ctl [--json] --endpoint http://127.0.0.1:PORT [--token TOKEN] <command>
//!
//! Commands:
//!   wait-ready [--timeout-ms MS] [--interval-ms MS]
//!                                                  Poll until host is reachable and authenticated (fails fast on auth denial)
//!   status                                         Fetch unified live tabs + audit + governance + gates snapshot
//!   governance                                     Show live Scheme Governance Authority snapshot (breaker/ASCENT/Cedar/proofs)
//!   gov <subaction> [args...]                      Apply live incremental governance fact mutation, Bi-Temporal derivation DAG registration, or compiled Cedar probe (eval-clean|eval-incident|delegate|revoke|taint-edge|sanitize|context-flow|declassify|retract|register-derivation|check-cedar)
//!   estop [actor:reason]                           Trigger live operator E-Stop via UiWorkbenchService (`governance-estop`)
//!   records [--project ID] [--kind PREFIX]         Query read-only ProjectionQueryService records (SecretVisibility::Denied)
//!   open-workbench [session-id]                    Open live UiWorkbenchService session snapshot
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
    time::{Duration, Instant},
};

use marlin_agent_app_server::{
    GrpcCode, MarlinSchemeHostClient,
    scheme_host_client::{
        GovernanceFactAction, format_facets_sorted, graph_record_to_json, json_escape,
        workbench_tab_kind_label,
    },
    wire::marlin::{common::v1::TargetKind, gate::v1::GraphHumanDecision},
};

#[derive(Debug, Eq, PartialEq)]
struct CliOptions {
    endpoint: Option<String>,
    token: Option<String>,
    json: bool,
    help: bool,
    rest: Vec<String>,
}

fn parse_cli_args<I>(
    mut args: I,
    default_endpoint: Option<String>,
    default_token: Option<String>,
) -> Result<CliOptions, String>
where
    I: Iterator<Item = String>,
{
    let mut endpoint = default_endpoint;
    let mut token = default_token;
    let mut json = false;
    let mut help = false;
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
            "--json" => {
                json = true;
            }
            "--help" | "-h" => {
                help = true;
                return Ok(CliOptions {
                    endpoint,
                    token,
                    json,
                    help,
                    rest,
                });
            }
            _ => rest.push(arg),
        }
    }
    Ok(CliOptions {
        endpoint,
        token,
        json,
        help,
        rest,
    })
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    if let Err(err) = run().await {
        eprintln!("marlin-scheme-ctl error: {err}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), String> {
    let default_endpoint = env::var("MARLIN_BRIDGE_ENDPOINT")
        .ok()
        .filter(|s| !s.trim().is_empty());
    let default_token = env::var("MARLIN_BRIDGE_TOKEN")
        .ok()
        .filter(|s| !s.trim().is_empty());
    let opts = parse_cli_args(env::args().skip(1), default_endpoint, default_token)?;

    if opts.help {
        println!(
            "Usage: marlin-scheme-ctl [--json] --endpoint <url> [--token <tok>] <wait-ready|status|governance|gov|estop|records|open-workbench|tabs|watch-tabs|audit|watch-trajectory|gates|approve|reject|dispatch>"
        );
        return Ok(());
    }

    let endpoint = opts.endpoint.ok_or_else(|| {
        "missing --endpoint <http://host:port> (or MARLIN_BRIDGE_ENDPOINT)".to_owned()
    })?;
    let rest = opts.rest;
    let cmd = rest.first().map(String::as_str).ok_or_else(|| {
        "missing subcommand (wait-ready|status|governance|gov|estop|records|open-workbench|tabs|watch-tabs|audit|watch-trajectory|gates|approve|reject|dispatch)"
            .to_owned()
    })?;

    if cmd == "wait-ready" {
        let flags = parse_wait_ready_flags(&rest[1..])?;
        let started = Instant::now();
        let deadline = started + Duration::from_millis(flags.timeout_ms);
        let interval = Duration::from_millis(flags.interval_ms);
        loop {
            match MarlinSchemeHostClient::connect(endpoint.clone(), opts.token.as_deref()).await {
                Ok(client) => match client.get_workbench_tabs().await {
                    Ok(resp) => {
                        let elapsed_ms = started.elapsed().as_millis();
                        if opts.json {
                            println!(
                                "{{\"ready\":true,\"tab_count\":{},\"elapsed_ms\":{elapsed_ms}}}",
                                resp.tabs.len()
                            );
                        } else {
                            println!("READY tabs={} elapsed_ms={elapsed_ms}", resp.tabs.len());
                        }
                        return Ok(());
                    }
                    Err(status)
                        if matches!(
                            status.code(),
                            GrpcCode::Unauthenticated | GrpcCode::PermissionDenied
                        ) =>
                    {
                        return Err(format!("wait-ready auth denied: {status}"));
                    }
                    Err(status) => {
                        if Instant::now() >= deadline {
                            return Err(format!(
                                "wait-ready timed out after {}ms (last status: {status})",
                                flags.timeout_ms
                            ));
                        }
                    }
                },
                Err(err) => {
                    if Instant::now() >= deadline {
                        return Err(format!(
                            "wait-ready timed out after {}ms (last connect error: {err})",
                            flags.timeout_ms
                        ));
                    }
                }
            }
            tokio::time::sleep(interval).await;
        }
    }

    let client = MarlinSchemeHostClient::connect(endpoint, opts.token.as_deref()).await?;

    match cmd {
        "records" => {
            let flags = parse_records_flags(&rest[1..])?;
            let project_id = flags.project.unwrap_or_else(|| "default".to_owned());
            let mut records = client
                .query_project_graph_records(project_id)
                .await
                .map_err(|s| format!("query_project_graph_records failed: {s}"))?;
            if let Some(prefix) = flags.kind_prefix.as_deref() {
                records.retain(|r| r.kind.starts_with(prefix));
            }
            if opts.json {
                let items: Vec<String> = records.iter().map(graph_record_to_json).collect();
                println!("{{\"records\":[{}]}}", items.join(","));
            } else {
                println!("RECORDS count={}", records.len());
                for rec in records {
                    let facets = format_facets_sorted(&rec.facets);
                    if facets.is_empty() {
                        println!(
                            "RECORD id={} kind={} title={}",
                            rec.record_id, rec.kind, rec.title
                        );
                    } else {
                        println!(
                            "RECORD id={} kind={} title={} {facets}",
                            rec.record_id, rec.kind, rec.title
                        );
                    }
                }
            }
        }
        "governance" => {
            let maybe = client
                .query_governance_authority_snapshot("default")
                .await
                .map_err(|s| format!("query_governance_authority_snapshot failed: {s}"))?;
            if opts.json {
                println!(
                    "{}",
                    maybe
                        .as_ref()
                        .map_or_else(|| "null".to_owned(), |g| g.to_json())
                );
            } else {
                match maybe {
                    Some(gov) => {
                        println!(
                            "GOVERNANCE admitted={} tripped={} trip_epoch={} rev_epoch={} policy_rev={} budget={}/{} rearm_gate={}",
                            gov.admitted,
                            gov.breaker_tripped,
                            gov.breaker_trip_epoch,
                            gov.revocation_epoch,
                            gov.policy_revision,
                            gov.breaker_budget_used,
                            gov.breaker_budget_limit,
                            if gov.rearm_gate_id.is_empty() {
                                "none"
                            } else {
                                gov.rearm_gate_id.as_str()
                            }
                        );
                        if gov.fenced_actors_json != "[]" {
                            println!("FENCED {}", gov.fenced_actors_json);
                        }
                        if gov.retracted_conclusions_json != "[]" {
                            println!("RETRACTED {}", gov.retracted_conclusions_json);
                        }
                        if let Some(probe) = &gov.last_cedar_probe_json {
                            println!("CEDAR_PROBE {probe}");
                        }
                    }
                    None => println!("GOVERNANCE none"),
                }
            }
        }
        "gov" => {
            let action = parse_gov_fact_action(&rest[1..])?;
            let (wire_sym, wire_arg) = action.to_wire_command();
            let resp = client
                .dispatch_governance_fact("marlin-scheme-ctl", &action)
                .await
                .map_err(|s| format!("dispatch_governance_fact({wire_sym}) failed: {s}"))?;
            if opts.json {
                println!(
                    "{{\"admitted\":{},\"action\":{},\"arg\":{}}}",
                    resp.admitted,
                    json_escape(wire_sym),
                    wire_arg
                        .as_deref()
                        .map_or_else(|| "null".to_owned(), json_escape)
                );
            } else {
                println!(
                    "GOV admitted={} action={wire_sym} arg={}",
                    resp.admitted,
                    wire_arg.as_deref().unwrap_or("")
                );
            }
        }
        "estop" => {
            let arg = if rest.len() > 1 {
                Some(rest[1..].join(" "))
            } else {
                Some("operator:manual-estop".to_owned())
            };
            let resp = client
                .dispatch_scheme_command("marlin-scheme-ctl", "governance-estop", arg.as_deref())
                .await
                .map_err(|s| format!("dispatch governance-estop failed: {s}"))?;
            if opts.json {
                println!(
                    "{{\"admitted\":{},\"action\":\"governance-estop\",\"arg\":{}}}",
                    resp.admitted,
                    arg.as_deref()
                        .map_or_else(|| "null".to_owned(), json_escape)
                );
            } else {
                println!(
                    "ESTOP admitted={} arg={}",
                    resp.admitted,
                    arg.as_deref().unwrap_or("")
                );
            }
        }
        "status" => {
            let snap = client
                .fetch_status_snapshot("default")
                .await
                .map_err(|s| format!("fetch_status_snapshot failed: {s}"))?;
            if opts.json {
                println!("{}", snap.to_json());
            } else {
                println!(
                    "STATUS tabs={} gates={}",
                    snap.tabs.tabs.len(),
                    snap.gates.len()
                );
                for tab in &snap.tabs.tabs {
                    println!(
                        "TAB kind={} id={} epoch={} status={}",
                        workbench_tab_kind_label(tab.kind),
                        tab.id,
                        tab.fencing_epoch,
                        tab.status_label
                    );
                }
                match &snap.audit {
                    Some(cap) => println!(
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
                    ),
                    None => println!("AUDIT none"),
                }
                if let Some(gov) = &snap.governance {
                    println!(
                        "GOVERNANCE admitted={} tripped={} trip_epoch={} rev_epoch={} policy_rev={} budget={}/{} rearm_gate={}",
                        gov.admitted,
                        gov.breaker_tripped,
                        gov.breaker_trip_epoch,
                        gov.revocation_epoch,
                        gov.policy_revision,
                        gov.breaker_budget_used,
                        gov.breaker_budget_limit,
                        if gov.rearm_gate_id.is_empty() {
                            "none"
                        } else {
                            gov.rearm_gate_id.as_str()
                        }
                    );
                }
                for g in &snap.gates {
                    println!("GATE id={} tool={}", g.gate_id, g.tool_or_node_name);
                }
            }
        }
        "open-workbench" => {
            let session_id = rest
                .get(1)
                .cloned()
                .unwrap_or_else(|| "marlin-scheme-ctl".to_owned());
            let wb = client
                .open_workbench(session_id, TargetKind::DesktopMacos, None)
                .await
                .map_err(|s| format!("open_workbench failed: {s}"))?;
            if opts.json {
                let slots_json: Vec<String> = wb
                    .slots
                    .iter()
                    .map(|s| {
                        format!(
                            "{{\"slot_id\":{},\"fallback_renderer\":{},\"allow_untrusted_plugin\":{}}}",
                            json_escape(&s.slot_id),
                            json_escape(&s.fallback_renderer),
                            s.allow_untrusted_plugin
                        )
                    })
                    .collect();
                println!(
                    "{{\"generation\":{},\"profile_name\":{},\"model_label\":{},\"session_label\":{},\"slots\":[{}]}}",
                    wb.generation,
                    json_escape(&wb.profile_name),
                    json_escape(&wb.model_label),
                    json_escape(&wb.session_label),
                    slots_json.join(",")
                );
            } else {
                println!(
                    "WORKBENCH gen={} profile={} session={} slots={}",
                    wb.generation,
                    wb.profile_name,
                    wb.session_label,
                    wb.slots.len()
                );
                for slot in wb.slots {
                    println!(
                        "SLOT id={} fallback={} allow_untrusted={}",
                        slot.slot_id, slot.fallback_renderer, slot.allow_untrusted_plugin
                    );
                }
            }
        }
        "tabs" => {
            let resp = client
                .get_workbench_tabs()
                .await
                .map_err(|s| format!("get_workbench_tabs failed: {s}"))?;
            if opts.json {
                let tabs_json: Vec<String> = resp
                    .tabs
                    .iter()
                    .map(|t| {
                        format!(
                            "{{\"kind\":{},\"id\":{},\"fencing_epoch\":{},\"status_label\":{}}}",
                            json_escape(workbench_tab_kind_label(t.kind)),
                            json_escape(&t.id),
                            t.fencing_epoch,
                            json_escape(&t.status_label)
                        )
                    })
                    .collect();
                println!("{{\"tabs\":[{}]}}", tabs_json.join(","));
            } else {
                println!("TABS count={}", resp.tabs.len());
                for tab in resp.tabs {
                    println!(
                        "TAB kind={} id={} epoch={} status={}",
                        workbench_tab_kind_label(tab.kind),
                        tab.id,
                        tab.fencing_epoch,
                        tab.status_label
                    );
                }
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
                if opts.json {
                    let tabs_json: Vec<String> = resp
                        .tabs
                        .iter()
                        .map(|t| {
                            format!(
                                "{{\"kind\":{},\"id\":{},\"fencing_epoch\":{},\"status_label\":{}}}",
                                json_escape(workbench_tab_kind_label(t.kind)),
                                json_escape(&t.id),
                                t.fencing_epoch,
                                json_escape(&t.status_label)
                            )
                        })
                        .collect();
                    println!("{{\"seq\":{seen},\"tabs\":[{}]}}", tabs_json.join(","));
                } else {
                    let ids: Vec<&str> = resp.tabs.iter().map(|t| t.id.as_str()).collect();
                    println!(
                        "WATCH_TABS seq={seen} count={} ids={}",
                        resp.tabs.len(),
                        ids.join(",")
                    );
                }
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
                    if opts.json {
                        let mut pairs: Vec<(&String, &String)> = ev.metadata.iter().collect();
                        pairs.sort_by(|a, b| a.0.cmp(b.0));
                        let meta_json: Vec<String> = pairs
                            .into_iter()
                            .map(|(k, v)| format!("{}:{}", json_escape(k), json_escape(v)))
                            .collect();
                        println!(
                            "{{\"seq\":{seen},\"event_id\":{},\"node_id\":{},\"node_kind\":{},\"status\":{},\"metadata\":{{{}}}}}",
                            json_escape(&ev.event_id),
                            json_escape(&ev.node_id),
                            json_escape(&ev.node_kind),
                            json_escape(&ev.status),
                            meta_json.join(",")
                        );
                    } else {
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
            if opts.json {
                println!(
                    "{}",
                    maybe
                        .as_ref()
                        .map_or_else(|| "null".to_owned(), |c| c.to_json())
                );
            } else {
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
        }
        "gates" => {
            let gates = client
                .list_pending_gates()
                .await
                .map_err(|s| format!("list_pending_gates failed: {s}"))?;
            if opts.json {
                let items: Vec<String> = gates
                    .iter()
                    .map(|g| {
                        format!(
                            "{{\"gate_id\":{},\"tool_or_node_name\":{}}}",
                            json_escape(&g.gate_id),
                            json_escape(&g.tool_or_node_name)
                        )
                    })
                    .collect();
                println!("{{\"gates\":[{}]}}", items.join(","));
            } else {
                println!("GATES count={}", gates.len());
                for g in gates {
                    println!("GATE id={} tool={}", g.gate_id, g.tool_or_node_name);
                }
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
            if opts.json {
                let denial = receipt
                    .denial
                    .as_ref()
                    .map_or_else(|| "null".to_owned(), |d| json_escape(&d.message));
                println!(
                    "{{\"claimed\":{},\"gate_id\":{},\"denial\":{denial}}}",
                    receipt.claimed,
                    json_escape(&receipt.gate_id)
                );
            } else if receipt.claimed {
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
            if opts.json {
                println!(
                    "{{\"admitted\":{},\"action\":{}}}",
                    resp.admitted,
                    json_escape(action)
                );
            } else {
                println!("DISPATCH admitted={} action={action}", resp.admitted);
            }
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

struct WaitReadyFlags {
    timeout_ms: u64,
    interval_ms: u64,
}

struct RecordsFlags {
    project: Option<String>,
    kind_prefix: Option<String>,
}

fn parse_wait_ready_flags(args: &[String]) -> Result<WaitReadyFlags, String> {
    let mut timeout_ms = 5_000u64;
    let mut interval_ms = 100u64;
    let mut idx = 0;
    while idx < args.len() {
        match args[idx].as_str() {
            "--timeout-ms" => {
                idx += 1;
                let raw = args
                    .get(idx)
                    .ok_or_else(|| "missing value for --timeout-ms".to_owned())?;
                let parsed = raw
                    .parse::<u64>()
                    .map_err(|e| format!("invalid --timeout-ms `{raw}`: {e}"))?;
                if parsed == 0 {
                    return Err("--timeout-ms must be >= 1".to_owned());
                }
                timeout_ms = parsed;
            }
            "--interval-ms" => {
                idx += 1;
                let raw = args
                    .get(idx)
                    .ok_or_else(|| "missing value for --interval-ms".to_owned())?;
                let parsed = raw
                    .parse::<u64>()
                    .map_err(|e| format!("invalid --interval-ms `{raw}`: {e}"))?;
                if parsed == 0 {
                    return Err("--interval-ms must be >= 1".to_owned());
                }
                interval_ms = parsed;
            }
            other => {
                return Err(format!("unknown wait-ready option `{other}`"));
            }
        }
        idx += 1;
    }
    Ok(WaitReadyFlags {
        timeout_ms,
        interval_ms,
    })
}

fn parse_csv_arg(raw: &str) -> Vec<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed == "-" || trimmed.eq_ignore_ascii_case("none") {
        return Vec::new();
    }
    trimmed
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect()
}

fn parse_gov_fact_action(args: &[String]) -> Result<GovernanceFactAction, String> {
    let sub = args.first().map(String::as_str).ok_or_else(|| {
        "usage: gov <eval-clean|eval-incident|delegate|revoke|taint-edge|sanitize|context-flow|declassify|retract|register-derivation|check-cedar> [args...]"
            .to_owned()
    })?;
    match sub {
        "eval-clean" if args.len() == 1 => Ok(GovernanceFactAction::EvalClean),
        "eval-incident" if args.len() == 1 => Ok(GovernanceFactAction::EvalIncident),
        "delegate" if args.len() == 3 => Ok(GovernanceFactAction::Delegate {
            parent: args[1].clone(),
            child: args[2].clone(),
        }),
        "revoke" if args.len() == 2 => Ok(GovernanceFactAction::Revoke {
            actor: args[1].clone(),
        }),
        "taint-edge" if args.len() == 3 || args.len() == 4 => {
            let role = args.get(3).cloned();
            if let Some(r) = role.as_deref()
                && !matches!(r, "source" | "sink" | "both")
            {
                return Err(format!(
                    "invalid taint-edge role `{r}` (expected source|sink|both)"
                ));
            }
            Ok(GovernanceFactAction::TaintEdge {
                src: args[1].clone(),
                dst: args[2].clone(),
                role,
            })
        }
        "sanitize" if args.len() == 2 => Ok(GovernanceFactAction::Sanitize {
            node: args[1].clone(),
        }),
        "context-flow" if args.len() == 5 => Ok(GovernanceFactAction::ContextFlow {
            entry: args[1].clone(),
            src_scope: args[2].clone(),
            target: args[3].clone(),
            dst_scope: args[4].clone(),
        }),
        "declassify" if args.len() == 2 => Ok(GovernanceFactAction::Declassify {
            entry: args[1].clone(),
        }),
        "retract" if args.len() == 2 => Ok(GovernanceFactAction::Retract {
            subject: args[1].clone(),
        }),
        "register-derivation" if args.len() == 4 || args.len() == 5 => {
            let subject_premises = parse_csv_arg(&args[3]);
            if subject_premises.is_empty() {
                return Err(
                    "register-derivation requires at least one non-empty subject premise"
                        .to_owned(),
                );
            }
            let parent_derivations = args
                .get(4)
                .map(|raw| parse_csv_arg(raw))
                .unwrap_or_default();
            Ok(GovernanceFactAction::RegisterDerivation {
                derivation_id: args[1].clone(),
                policy_id: args[2].clone(),
                subject_premises,
                parent_derivations,
            })
        }
        "check-cedar" if args.len() == 4 || args.len() == 5 => {
            let step_up_fresh = match args.get(4).map(String::as_str) {
                None | Some("false") => false,
                Some("true") => true,
                Some(other) => {
                    return Err(format!(
                        "invalid check-cedar step_up_fresh `{other}` (expected true|false)"
                    ));
                }
            };
            Ok(GovernanceFactAction::CheckCedar {
                actor: args[1].clone(),
                action: args[2].clone(),
                resource: args[3].clone(),
                step_up_fresh,
            })
        }
        other => Err(format!(
            "invalid gov subaction `{other}` or wrong arity (expected eval-clean|eval-incident|delegate|revoke|taint-edge|sanitize|context-flow|declassify|retract|register-derivation|check-cedar)"
        )),
    }
}

fn parse_records_flags(args: &[String]) -> Result<RecordsFlags, String> {
    let mut project = None;
    let mut kind_prefix = None;
    let mut idx = 0;
    while idx < args.len() {
        match args[idx].as_str() {
            "--project" => {
                idx += 1;
                let raw = args
                    .get(idx)
                    .ok_or_else(|| "missing value for --project".to_owned())?;
                project = Some(raw.clone());
            }
            "--kind" => {
                idx += 1;
                let raw = args
                    .get(idx)
                    .ok_or_else(|| "missing value for --kind".to_owned())?;
                kind_prefix = Some(raw.clone());
            }
            other => {
                return Err(format!("unknown records option `{other}`"));
            }
        }
        idx += 1;
    }
    Ok(RecordsFlags {
        project,
        kind_prefix,
    })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_cli_args_supports_json_endpoint_and_token_flags() {
        let args = [
            "--json",
            "-e",
            "http://127.0.0.1:9001",
            "--token",
            "sec-1",
            "status",
        ]
        .into_iter()
        .map(str::to_owned);
        let parsed = parse_cli_args(args, None, None).expect("parse cli args");
        assert!(parsed.json);
        assert!(!parsed.help);
        assert_eq!(parsed.endpoint.as_deref(), Some("http://127.0.0.1:9001"));
        assert_eq!(parsed.token.as_deref(), Some("sec-1"));
        assert_eq!(parsed.rest, vec!["status".to_owned()]);
    }

    #[test]
    fn parse_cli_args_rejects_missing_flag_values() {
        assert!(parse_cli_args(["--endpoint".to_owned()].into_iter(), None, None).is_err());
        assert!(parse_cli_args(["--token".to_owned()].into_iter(), None, None).is_err());
    }

    #[test]
    fn parse_watch_flags_validates_bounds_and_unknown_options() {
        let ok = parse_watch_flags(&[
            "--count".to_owned(),
            "2".to_owned(),
            "--timeout-ms".to_owned(),
            "3000".to_owned(),
            "--session".to_owned(),
            "s-1".to_owned(),
        ])
        .expect("valid watch flags");
        assert_eq!(ok.count, Some(2));
        assert_eq!(ok.timeout_ms, Some(3000));
        assert_eq!(ok.session.as_deref(), Some("s-1"));

        assert!(parse_watch_flags(&["--count".to_owned(), "0".to_owned()]).is_err());
        assert!(parse_watch_flags(&["--bogus".to_owned()]).is_err());
    }

    #[test]
    fn parse_wait_ready_and_records_flags_validate_inputs() {
        let wr = parse_wait_ready_flags(&[
            "--timeout-ms".to_owned(),
            "2500".to_owned(),
            "--interval-ms".to_owned(),
            "50".to_owned(),
        ])
        .expect("valid wait-ready flags");
        assert_eq!(wr.timeout_ms, 2500);
        assert_eq!(wr.interval_ms, 50);
        assert!(parse_wait_ready_flags(&["--timeout-ms".to_owned(), "0".to_owned()]).is_err());
        assert!(parse_wait_ready_flags(&["--interval-ms".to_owned(), "0".to_owned()]).is_err());
        assert!(parse_wait_ready_flags(&["--nope".to_owned()]).is_err());

        let rec = parse_records_flags(&[
            "--project".to_owned(),
            "alpha".to_owned(),
            "--kind".to_owned(),
            "scheme.fleet.".to_owned(),
        ])
        .expect("valid records flags");
        assert_eq!(rec.project.as_deref(), Some("alpha"));
        assert_eq!(rec.kind_prefix.as_deref(), Some("scheme.fleet."));
        assert!(parse_records_flags(&["--kind".to_owned()]).is_err());
        assert!(parse_records_flags(&["--bad".to_owned()]).is_err());
    }

    #[test]
    fn parse_gov_fact_action_validates_all_variants_and_rejects_rearm() {
        let s = |vals: &[&str]| vals.iter().map(|v| (*v).to_owned()).collect::<Vec<_>>();
        assert_eq!(
            parse_gov_fact_action(&s(&["eval-clean"])).unwrap(),
            GovernanceFactAction::EvalClean
        );
        assert_eq!(
            parse_gov_fact_action(&s(&["delegate", "ops-root", "agent-lead"])).unwrap(),
            GovernanceFactAction::Delegate {
                parent: "ops-root".to_owned(),
                child: "agent-lead".to_owned(),
            }
        );
        assert_eq!(
            parse_gov_fact_action(&s(&["taint-edge", "in", "pipe", "source"])).unwrap(),
            GovernanceFactAction::TaintEdge {
                src: "in".to_owned(),
                dst: "pipe".to_owned(),
                role: Some("source".to_owned()),
            }
        );
        assert_eq!(
            parse_gov_fact_action(&s(&["sanitize", "pipe"])).unwrap(),
            GovernanceFactAction::Sanitize {
                node: "pipe".to_owned(),
            }
        );
        assert_eq!(
            parse_gov_fact_action(&s(&["check-cedar", "worker", "run_tool", "workspace"])).unwrap(),
            GovernanceFactAction::CheckCedar {
                actor: "worker".to_owned(),
                action: "run_tool".to_owned(),
                resource: "workspace".to_owned(),
                step_up_fresh: false,
            }
        );
        assert_eq!(
            parse_gov_fact_action(&s(&[
                "check-cedar",
                "alice",
                "rearm_breaker",
                "workspace",
                "true",
            ]))
            .unwrap(),
            GovernanceFactAction::CheckCedar {
                actor: "alice".to_owned(),
                action: "rearm_breaker".to_owned(),
                resource: "workspace".to_owned(),
                step_up_fresh: true,
            }
        );
        assert_eq!(
            parse_gov_fact_action(&s(&[
                "register-derivation",
                "D-TOKEN",
                "pol.cert",
                "token-cert-v1",
                "none",
            ]))
            .unwrap(),
            GovernanceFactAction::RegisterDerivation {
                derivation_id: "D-TOKEN".to_owned(),
                policy_id: "pol.cert".to_owned(),
                subject_premises: vec!["token-cert-v1".to_owned()],
                parent_derivations: Vec::new(),
            }
        );
        assert_eq!(
            parse_gov_fact_action(&s(&[
                "register-derivation",
                "D-PLAN",
                "pol.plan",
                "workspace-ok,scope-ok",
                "D-TOKEN",
            ]))
            .unwrap(),
            GovernanceFactAction::RegisterDerivation {
                derivation_id: "D-PLAN".to_owned(),
                policy_id: "pol.plan".to_owned(),
                subject_premises: vec!["workspace-ok".to_owned(), "scope-ok".to_owned()],
                parent_derivations: vec!["D-TOKEN".to_owned()],
            }
        );
        assert!(parse_gov_fact_action(&s(&["rearm", "alice"])).is_err());
        assert!(parse_gov_fact_action(&s(&["taint-edge", "a", "b", "bogus"])).is_err());
        assert!(parse_gov_fact_action(&s(&["delegate", "only-one"])).is_err());
        assert!(parse_gov_fact_action(&s(&["check-cedar", "alice", "run_tool"])).is_err());
        assert!(
            parse_gov_fact_action(&s(&["check-cedar", "alice", "run_tool", "ws", "maybe"]))
                .is_err()
        );
        assert!(
            parse_gov_fact_action(&s(&["register-derivation", "D-BAD", "pol.x", "none"])).is_err()
        );
        assert!(parse_gov_fact_action(&s(&["register-derivation", "D-BAD", "pol.x"])).is_err());
    }
}
