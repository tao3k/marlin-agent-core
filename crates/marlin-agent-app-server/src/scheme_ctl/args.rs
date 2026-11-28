// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Argument parsing and human-readable formatting helpers for `marlin-scheme-ctl`.

use crate::{FleetTaskAction, GovernanceAuthoritySnapshotView, GovernanceFactAction};

#[derive(Debug, Eq, PartialEq)]
pub struct CliOptions {
    pub endpoint: Option<String>,
    pub token: Option<String>,
    pub json: bool,
    pub help: bool,
    pub rest: Vec<String>,
}

pub fn parse_cli_args<I>(
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


pub struct WatchFlags {
    pub count: Option<usize>,
    pub timeout_ms: Option<u64>,
    pub session: Option<String>,
}

pub fn print_governance_human(gov: &GovernanceAuthoritySnapshotView) {
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

pub struct WaitReadyFlags {
    pub timeout_ms: u64,
    pub interval_ms: u64,
}

pub struct RecordsFlags {
    pub project: Option<String>,
    pub kind_prefix: Option<String>,
}

pub fn parse_wait_ready_flags(args: &[String]) -> Result<WaitReadyFlags, String> {
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

pub fn parse_csv_arg(raw: &str) -> Vec<String> {
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

pub fn parse_gov_fact_action(args: &[String]) -> Result<GovernanceFactAction, String> {
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

pub fn parse_fleet_task_action(args: &[String]) -> Result<FleetTaskAction, String> {
    let sub = args.first().map(String::as_str).ok_or_else(|| {
        "usage: task <drill|propose-branch|detour-blocker|detour-sidequest|detour-exit|peek|reopen|fleet-step|sandbox-verify> [args...]"
            .to_owned()
    })?;
    let join_opt = |slice: &[String]| -> Option<String> {
        let joined = slice.join(" ");
        let trimmed = joined.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_owned())
        }
    };
    match sub {
        "drill" if args.len() == 2 => {
            let level = args[1]
                .parse::<u8>()
                .map_err(|_| format!("invalid drill level `{}` (expected 0..=3)", args[1]))?;
            if level > 3 {
                return Err(format!("invalid drill level `{level}` (expected 0..=3)"));
            }
            Ok(FleetTaskAction::Drill { level })
        }
        "propose-branch" => Ok(FleetTaskAction::ProposeBranch {
            label: join_opt(&args[1..]),
        }),
        "detour-blocker" => Ok(FleetTaskAction::DetourBlocker {
            note: join_opt(&args[1..]),
        }),
        "detour-sidequest" => Ok(FleetTaskAction::DetourSidequest {
            note: join_opt(&args[1..]),
        }),
        "detour-exit" if args.len() == 2 => match args[1].as_str() {
            "return" => Ok(FleetTaskAction::DetourExitReturn),
            "absorb" => Ok(FleetTaskAction::DetourExitAbsorb),
            "abort" => Ok(FleetTaskAction::DetourExitAbort),
            "sibling" => Ok(FleetTaskAction::DetourExitSibling),
            other => Err(format!(
                "invalid detour-exit disposition `{other}` (expected return|absorb|abort|sibling)"
            )),
        },
        "peek" if args.len() <= 2 => Ok(FleetTaskAction::Peek {
            actor_id: args.get(1).cloned(),
        }),
        "reopen" if args.len() <= 2 => Ok(FleetTaskAction::Reopen {
            actor_id: args.get(1).cloned(),
        }),
        "fleet-step" if args.len() == 1 => Ok(FleetTaskAction::FleetStep),
        "sandbox-verify" if args.len() == 1 => Ok(FleetTaskAction::SandboxVerify),
        other => Err(format!(
            "invalid task subaction `{other}` or wrong arity (expected drill|propose-branch|detour-blocker|detour-sidequest|detour-exit|peek|reopen|fleet-step|sandbox-verify)"
        )),
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct ProbeCedarArgs {
    pub actor: String,
    pub action: String,
    pub resource: String,
    pub step_up_fresh: bool,
    pub expect: Option<String>,
    pub timeout_ms: u64,
}

pub fn parse_probe_cedar_args(args: &[String]) -> Result<ProbeCedarArgs, String> {
    let mut positional: Vec<String> = Vec::new();
    let mut expect: Option<String> = None;
    let mut timeout_ms = 5_000u64;
    let mut idx = 0;
    while idx < args.len() {
        match args[idx].as_str() {
            "--expect" => {
                idx += 1;
                let raw = args
                    .get(idx)
                    .ok_or_else(|| "missing value for --expect".to_owned())?;
                if !matches!(raw.as_str(), "allow" | "deny" | "implicit_deny") {
                    return Err(format!(
                        "invalid --expect `{raw}` (expected allow|deny|implicit_deny)"
                    ));
                }
                expect = Some(raw.clone());
            }
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
            other if other.starts_with('-') => {
                return Err(format!("unknown probe-cedar option `{other}`"));
            }
            pos => positional.push(pos.to_owned()),
        }
        idx += 1;
    }
    if positional.len() < 3 || positional.len() > 4 {
        return Err(
            "usage: probe-cedar <actor> <action> <resource> [true|false] [--expect allow|deny|implicit_deny] [--timeout-ms MS]"
                .to_owned(),
        );
    }
    let step_up_fresh = match positional.get(3).map(String::as_str) {
        None | Some("false") => false,
        Some("true") => true,
        Some(other) => {
            return Err(format!(
                "invalid probe-cedar step_up_fresh `{other}` (expected true|false)"
            ));
        }
    };
    Ok(ProbeCedarArgs {
        actor: positional[0].clone(),
        action: positional[1].clone(),
        resource: positional[2].clone(),
        step_up_fresh,
        expect,
        timeout_ms,
    })
}

pub fn parse_records_flags(args: &[String]) -> Result<RecordsFlags, String> {
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

pub fn parse_watch_flags(args: &[String]) -> Result<WatchFlags, String> {
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


