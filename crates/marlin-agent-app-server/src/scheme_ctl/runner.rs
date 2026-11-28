// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Operator CLI (`marlin-scheme-ctl`) implementation owned by
//! `marlin-agent-app-server`.


use std::{
    env,
    io::{self, Write},
    time::{Duration, Instant},
};

use crate::{
    GrpcCode, MarlinSchemeHostClient,
    scheme_host_client::{
        format_facets_sorted, graph_record_to_json, json_escape, workbench_tab_kind_label,
    },
    wire::marlin::{common::v1::TargetKind, gate::v1::GraphHumanDecision},
};

use super::args::{
    parse_cli_args, parse_fleet_task_action, parse_gov_fact_action, parse_probe_cedar_args,
    parse_records_flags, parse_wait_ready_flags, parse_watch_flags, print_governance_human,
};

/// Executes the `marlin-scheme-ctl` CLI against a live host endpoint.
pub async fn run_cli() -> Result<(), String> {
    let default_endpoint = env::var("MARLIN_BRIDGE_ENDPOINT")
        .ok()
        .filter(|s| !s.trim().is_empty());
    let default_token = env::var("MARLIN_BRIDGE_TOKEN")
        .ok()
        .filter(|s| !s.trim().is_empty());
    let opts = parse_cli_args(env::args().skip(1), default_endpoint, default_token)?;

    if opts.help {
        println!(
            "Usage: marlin-scheme-ctl [--json] --endpoint <url> [--token <tok>] <wait-ready|status|governance|gov|probe-cedar|task|estop|records|open-workbench|tabs|watch-tabs|audit|watch-trajectory|gates|approve|reject|dispatch>"
        );
        return Ok(());
    }

    let endpoint = opts.endpoint.ok_or_else(|| {
        "missing --endpoint <http://host:port> (or MARLIN_BRIDGE_ENDPOINT)".to_owned()
    })?;
    let rest = opts.rest;
    let cmd = rest.first().map(String::as_str).ok_or_else(|| {
        "missing subcommand (wait-ready|status|governance|gov|probe-cedar|task|estop|records|open-workbench|tabs|watch-tabs|audit|watch-trajectory|gates|approve|reject|dispatch)"
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
                    Some(gov) => print_governance_human(&gov),
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
        "probe-cedar" => {
            let probe_args = parse_probe_cedar_args(&rest[1..])?;
            let probe = client
                .probe_cedar_and_wait(
                    "marlin-scheme-ctl",
                    probe_args.actor,
                    probe_args.action,
                    probe_args.resource,
                    probe_args.step_up_fresh,
                    Duration::from_millis(probe_args.timeout_ms),
                )
                .await?;
            if opts.json {
                println!("{}", probe.to_json());
            } else {
                println!(
                    "CEDAR_PROBE seq={} decision={} actor={} action={} resource={} step_up={} policies={}",
                    probe.seq,
                    probe.decision,
                    probe.actor,
                    probe.action,
                    probe.resource,
                    probe.step_up_fresh,
                    probe.matched_policy_ids_json
                );
            }
            if let Some(expected) = probe_args.expect.as_deref()
                && probe.decision != expected
            {
                return Err(format!(
                    "cedar probe expectation failed: expected `{expected}`, got `{}` (policies={})",
                    probe.decision, probe.matched_policy_ids_json
                ));
            }
        }
        "task" => {
            let action = parse_fleet_task_action(&rest[1..])?;
            let (wire_sym, wire_arg) = action.to_wire_command();
            let resp = client
                .dispatch_fleet_task("marlin-scheme-ctl", &action)
                .await
                .map_err(|s| format!("dispatch_fleet_task({wire_sym}) failed: {s}"))?;
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
                    "TASK admitted={} action={wire_sym} arg={}",
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
                    print_governance_human(gov);
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


