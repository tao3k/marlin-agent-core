// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Live Quint -> ITF -> Rust Model-Based Testing (MBT) refinement suite for the
//! unified Rust backend governance plane (`marlin_agent_kernel::governance_plane`):
//! 1. Upstream `DelegatedAgentAuthz.qnt` + `MarlinBackendDelegatedAuthzOverlay.qnt`
//! 2. Upstream `MultiTenantIsolation.qnt` + `MarlinBackendMultiTenantSodOverlay.qnt`
//! 3. Upstream `ChatroomIngressGovernance.qnt` + `MarlinBackendChatroomIngressOverlay.qnt`
//! 4. Upstream `GovernanceSod.qnt`
//! 5. End-to-end async `TokioGraphLoopController` + `BackendSecurityGovernanceCoordinator`
//!    integration test verifying forged-ingress auto-trip, RFC 9470 step-up breaker re-arm,
//!    cross-tenant & zombie OBO refusal, and real OS side-effect cutoff.

use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use marlin_agent_kernel::{
    AgentFlowLoopProgramRuntimeHandoffExecutor, BackendSecurityGovernanceCoordinator,
    ChatroomGateStatus, ChatroomIngressGovernor, DelegatedAgentAuthzLedger, DelegatedAgentTicket,
    DelegationScopeLevel, DelegationTicketId, DualControlSodLedger, ExecutionCircuitBreaker,
    ExternalIngressRequest, GenericLoopMachineReceipt, GenericLoopMachineStepIndex,
    GovernanceActorId, GovernanceChannelId, GovernanceGateId, GovernancePlaneRejection,
    GovernancePrincipal, GovernanceProposalId, GovernanceTenantId, LoopProgramRuntimeHandoffExecutor,
    LoopProgramRuntimeHandoffPlan, LoopProgramRuntimeOwner, LoopProgramRuntimeSideEffectExecutor,
    LoopProgramRuntimeSideEffectStatus, LoopProgramToolProcessCommandTemplate,
    LoopProgramToolProcessProgram, MultiTenantIsolationFence, StaticLoopProgramToolProcessResolver,
    TenantExecutionTicket, TenantExecutionTicketId, TenantOutboxEntryId, TokioGraphLoopController,
    TokioGraphLoopKernel, ToolBudgetUnits, ToolIfcContext,
};
use marlin_agent_protocol::{
    LoopProgramActionKind, LoopProgramEventKind, LoopProgramId, LoopProgramStateId,
    LoopProgramTransitionId,
};
use marlin_agent_runtime::TokioAgentRuntime;

static QUINT_MBT_LOCK: Mutex<()> = Mutex::new(());

// ---------------------------------------------------------------------------
// Minimal zero-dependency JSON parser for Quint ITF (.itf.json) trace replay
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
enum JsonVal {
    Null,
    Bool(bool),
    Int(i64),
    Str(String),
    Arr(Vec<JsonVal>),
    Obj(BTreeMap<String, JsonVal>),
}

impl JsonVal {
    fn as_obj(&self) -> Option<&BTreeMap<String, JsonVal>> {
        match self {
            Self::Obj(map) => Some(map),
            _ => None,
        }
    }

    fn as_arr(&self) -> Option<&[JsonVal]> {
        match self {
            Self::Arr(arr) => Some(arr),
            _ => None,
        }
    }

    fn get(&self, key: &str) -> Option<&JsonVal> {
        self.as_obj()?.get(key)
    }

    fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }

    fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Int(n) => Some(*n),
            Self::Obj(map) => {
                if let Some(JsonVal::Str(s)) = map.get("#bigint") {
                    s.parse::<i64>().ok()
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

struct JsonParser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> JsonParser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            bytes: input.as_bytes(),
            pos: 0,
        }
    }

    fn skip_ws(&mut self) {
        while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
    }

    fn parse_val(&mut self) -> Result<JsonVal, String> {
        self.skip_ws();
        let &ch = self
            .bytes
            .get(self.pos)
            .ok_or_else(|| "unexpected EOF".to_owned())?;
        match ch {
            b'{' => self.parse_obj(),
            b'[' => self.parse_arr(),
            b'"' => self.parse_str().map(JsonVal::Str),
            b't' => self.expect_lit(b"true", JsonVal::Bool(true)),
            b'f' => self.expect_lit(b"false", JsonVal::Bool(false)),
            b'n' => self.expect_lit(b"null", JsonVal::Null),
            b'-' | b'0'..=b'9' => self.parse_num(),
            _ => Err(format!("unexpected byte '{}' at {}", ch as char, self.pos)),
        }
    }

    fn expect_lit(&mut self, lit: &[u8], val: JsonVal) -> Result<JsonVal, String> {
        if self.bytes[self.pos..].starts_with(lit) {
            self.pos += lit.len();
            Ok(val)
        } else {
            Err(format!("invalid literal at {}", self.pos))
        }
    }

    fn parse_str(&mut self) -> Result<String, String> {
        self.pos += 1;
        let mut out = String::new();
        while self.pos < self.bytes.len() {
            let ch = self.bytes[self.pos];
            self.pos += 1;
            match ch {
                b'"' => return Ok(out),
                b'\\' => {
                    let esc = *self
                        .bytes
                        .get(self.pos)
                        .ok_or_else(|| "unterminated escape".to_owned())?;
                    self.pos += 1;
                    match esc {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'b' => out.push('\u{0008}'),
                        b'f' => out.push('\u{000C}'),
                        b'u' => {
                            let hex = std::str::from_utf8(
                                self.bytes
                                    .get(self.pos..self.pos + 4)
                                    .ok_or_else(|| "truncated unicode escape".to_owned())?,
                            )
                            .map_err(|e| e.to_string())?;
                            self.pos += 4;
                            let code = u32::from_str_radix(hex, 16).map_err(|e| e.to_string())?;
                            out.push(char::from_u32(code).unwrap_or('\u{FFFD}'));
                        }
                        other => return Err(format!("unknown escape \\{}", other as char)),
                    }
                }
                _ => {
                    let start = self.pos - 1;
                    let mut end = self.pos;
                    while end < self.bytes.len()
                        && self.bytes[end] != b'"'
                        && self.bytes[end] != b'\\'
                    {
                        end += 1;
                    }
                    let slice = std::str::from_utf8(&self.bytes[start..end])
                        .map_err(|e| e.to_string())?;
                    out.push_str(slice);
                    self.pos = end;
                }
            }
        }
        Err("unterminated string".to_owned())
    }

    fn parse_num(&mut self) -> Result<JsonVal, String> {
        let start = self.pos;
        if self.bytes[self.pos] == b'-' {
            self.pos += 1;
        }
        while self.pos < self.bytes.len()
            && (self.bytes[self.pos].is_ascii_digit()
                || matches!(self.bytes[self.pos], b'.' | b'e' | b'E' | b'+' | b'-'))
        {
            self.pos += 1;
        }
        let text = std::str::from_utf8(&self.bytes[start..self.pos]).map_err(|e| e.to_string())?;
        let n = text
            .parse::<i64>()
            .or_else(|_| text.parse::<f64>().map(|f| f as i64))
            .map_err(|e| format!("invalid number '{text}': {e}"))?;
        Ok(JsonVal::Int(n))
    }

    fn parse_arr(&mut self) -> Result<JsonVal, String> {
        self.pos += 1;
        let mut items = Vec::new();
        self.skip_ws();
        if self.bytes.get(self.pos) == Some(&b']') {
            self.pos += 1;
            return Ok(JsonVal::Arr(items));
        }
        loop {
            items.push(self.parse_val()?);
            self.skip_ws();
            match self.bytes.get(self.pos) {
                Some(&b',') => self.pos += 1,
                Some(&b']') => {
                    self.pos += 1;
                    return Ok(JsonVal::Arr(items));
                }
                _ => return Err(format!("expected ',' or ']' at {}", self.pos)),
            }
        }
    }

    fn parse_obj(&mut self) -> Result<JsonVal, String> {
        self.pos += 1;
        let mut map = BTreeMap::new();
        self.skip_ws();
        if self.bytes.get(self.pos) == Some(&b'}') {
            self.pos += 1;
            return Ok(JsonVal::Obj(map));
        }
        loop {
            self.skip_ws();
            let key = self.parse_str()?;
            self.skip_ws();
            if self.bytes.get(self.pos) != Some(&b':') {
                return Err(format!("expected ':' at {}", self.pos));
            }
            self.pos += 1;
            let val = self.parse_val()?;
            map.insert(key, val);
            self.skip_ws();
            match self.bytes.get(self.pos) {
                Some(&b',') => self.pos += 1,
                Some(&b'}') => {
                    self.pos += 1;
                    return Ok(JsonVal::Obj(map));
                }
                _ => return Err(format!("expected ',' or '}}' at {}", self.pos)),
            }
        }
    }
}

fn parse_json(text: &str) -> JsonVal {
    JsonParser::new(text)
        .parse_val()
        .expect("valid Quint ITF JSON")
}

// ---------------------------------------------------------------------------
// Quint resolver & live ITF trace generation helpers
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(Path::parent)
        .expect("workspace root two levels above crate")
        .to_path_buf()
}

fn find_quint_tooling() -> Option<(PathBuf, PathBuf)> {
    let root = workspace_root();
    if let (Ok(cli), Ok(models)) = (
        env::var("MARLIN_QUINT_RESOLVER_CLI"),
        env::var("MARLIN_QUINT_MODELS_DIR"),
    ) {
        let cli_path = PathBuf::from(cli);
        let models_path = PathBuf::from(models);
        if cli_path.exists() && models_path.exists() {
            return Some((cli_path, models_path));
        }
    }

    let candidates = [
        root.join("node_modules/@deterministic-engineering"),
        root.join(".data/marlin-mygo/node_modules/@deterministic-engineering"),
    ];
    for base in candidates {
        let cli = base.join("quint-modules-resolver/src/cli.ts");
        let models = base.join("quint-models");
        if cli.exists() && models.exists() {
            return Some((cli, models));
        }
    }
    None
}

fn generate_itf_traces(
    model_path: &Path,
    main_module: &str,
    countermodel_steps: &[&str],
) -> Vec<(String, Vec<JsonVal>)> {
    let _guard = QUINT_MBT_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let (resolver_cli, models_dir) = find_quint_tooling()
        .expect("quint-modules-resolver and quint-models must be available locally");
    let resolve_root = models_dir
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .expect("project root containing node_modules");

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    let out_dir = env::temp_dir().join(format!(
        "marlin-govplane-mbt-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&out_dir).expect("create temp ITF dir");

    let output = Command::new("bun")
        .arg(&resolver_cli)
        .arg("mbt-audit")
        .arg(model_path)
        .arg("--out-dir")
        .arg(&out_dir)
        .arg("--min-coverage=0.8")
        .env("QUINT_RESOLVE_ROOT", resolve_root)
        .output()
        .expect("execute bun quint-modules-resolver mbt-audit");

    assert!(
        output.status.success(),
        "quint mbt-audit failed for {}:\nSTDOUT:\n{}\nSTDERR:\n{}",
        model_path.display(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let mut traces = Vec::new();
    let mut entries: Vec<_> = fs::read_dir(&out_dir)
        .expect("read ITF dir")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|s| s.to_str())
                .is_some_and(|name| name.ends_with(".itf.json"))
        })
        .collect();
    entries.sort();

    for path in entries {
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("trace")
            .to_owned();
        let raw = fs::read_to_string(&path).expect("read ITF file");
        let parsed = parse_json(&raw);
        let states = parsed
            .get("states")
            .and_then(JsonVal::as_arr)
            .expect("ITF states array")
            .to_vec();
        traces.push((name, states));
    }

    for step_action in countermodel_steps {
        let cm_file = out_dir.join(format!("cm_{step_action}.itf.json"));
        let cm_out = Command::new("bun")
            .arg(&resolver_cli)
            .arg("run")
            .arg(model_path)
            .arg(format!("--main={main_module}"))
            .arg(format!("--step={step_action}"))
            .arg("--invariant=safety")
            .arg("--max-steps=12")
            .arg("--max-samples=50000")
            .arg("--seed=7")
            .arg(format!("--out-itf={}", cm_file.display()))
            .env("QUINT_RESOLVE_ROOT", resolve_root)
            .output()
            .expect("execute bun quint-modules-resolver run for countermodel");

        assert!(
            !cm_out.status.success() && cm_file.exists(),
            "Quint did not emit violating countermodel ITF for {} ({step_action}):\nSTDOUT:\n{}\nSTDERR:\n{}",
            model_path.display(),
            String::from_utf8_lossy(&cm_out.stdout),
            String::from_utf8_lossy(&cm_out.stderr),
        );
        let raw = fs::read_to_string(&cm_file).expect("read countermodel ITF file");
        let parsed = parse_json(&raw);
        let states = parsed
            .get("states")
            .and_then(JsonVal::as_arr)
            .expect("countermodel ITF states array")
            .to_vec();
        traces.push((format!("cm_{step_action}.itf.json"), states));
    }

    let _ = fs::remove_dir_all(&out_dir);
    traces
}

fn state_h(state: &JsonVal) -> &JsonVal {
    state.get("h").expect("Quint state variable h")
}

fn h_bool(h: &JsonVal, key: &str) -> bool {
    h.get(key)
        .and_then(JsonVal::as_bool)
        .unwrap_or_else(|| panic!("missing bool field h.{key}"))
}

fn h_i64(h: &JsonVal, key: &str) -> i64 {
    h.get(key)
        .and_then(JsonVal::as_i64)
        .unwrap_or_else(|| panic!("missing int field h.{key}"))
}

// ---------------------------------------------------------------------------
// 1. OBO Delegated Agent Authz (Upstream + Backend Overlay)
// ---------------------------------------------------------------------------

fn replay_delegated_authz_traces(traces: &[(String, Vec<JsonVal>)], overlay_mode: bool) -> usize {
    let human_id = GovernanceActorId::new("alice").unwrap();
    let agent_id = GovernanceActorId::new("marlin-agent-1").unwrap();
    let human = GovernancePrincipal::Human(human_id.clone());
    let agent = GovernancePrincipal::DelegatedAgent {
        agent_id: agent_id.clone(),
        acting_for: human_id.clone(),
    };

    let mut countermodels_refused = 0usize;

    for (trace_name, states) in traces {
        let mut ledger = DelegatedAgentAuthzLedger::new(
            human_id.clone(),
            DelegationScopeLevel::WorkspaceWrite,
            true,
        )
        .with_single_delegation_enforced(true);
        let breaker = Arc::new(ExecutionCircuitBreaker::new(ToolBudgetUnits::new(4)));
        let mut active_ticket: Option<DelegatedAgentTicket> = None;
        let mut seq = 0u64;

        for pair in states.windows(2) {
            let prev = state_h(&pair[0]);
            let next = state_h(&pair[1]);

            let scope_amp = h_bool(next, "scopeAmplified") && !h_bool(prev, "scopeAmplified");
            let impersonated =
                h_bool(next, "impersonatedHuman") && !h_bool(prev, "impersonatedHuman");
            let zombie =
                h_bool(next, "zombieDelegatedUse") && !h_bool(prev, "zombieDelegatedUse");
            let unverified_priv =
                h_bool(next, "unverifiedPrivileged") && !h_bool(prev, "unverifiedPrivileged");
            let breaker_no_stepup = overlay_mode
                && h_bool(next, "breakerRearmedWithoutStepUp")
                && !h_bool(prev, "breakerRearmedWithoutStepUp");

            if scope_amp {
                seq += 1;
                let tid = DelegationTicketId::new(format!("amp-{seq}")).unwrap();
                let err = ledger
                    .delegate_to_agent(tid, agent_id.clone(), DelegationScopeLevel::PrivilegedAdmin)
                    .expect_err("OBO scope amplification must be refused");
                assert!(matches!(
                    err,
                    GovernancePlaneRejection::ScopeAmplificationDenied { .. }
                ));
                countermodels_refused += 1;
                break;
            }
            if impersonated {
                let err = ledger
                    .approve_human_only_gate(&agent)
                    .expect_err("AI agent impersonating human gate must be refused");
                assert!(matches!(
                    err,
                    GovernancePlaneRejection::AgentImpersonationDenied { .. }
                ));
                countermodels_refused += 1;
                break;
            }
            if zombie {
                let ticket = active_ticket.as_ref().expect("active ticket before zombie use");
                let err = ledger
                    .execute_delegated(ticket)
                    .expect_err("revoked zombie OBO delegation must be refused");
                assert!(matches!(
                    err,
                    GovernancePlaneRejection::ZombieDelegationDenied { .. }
                ));
                countermodels_refused += 1;
                break;
            }
            if unverified_priv {
                let err = ledger
                    .execute_privileged_human(&human)
                    .expect_err("privileged human action without step-up must be refused");
                assert!(matches!(
                    err,
                    GovernancePlaneRejection::StepUpVerificationRequired
                ));
                countermodels_refused += 1;
                break;
            }
            if breaker_no_stepup {
                // Violation can come either from an AI agent re-arming while the human has
                // fresh step-up, OR from a human lacking verified step-up/sign-in.
                if ledger.is_signed_in() && ledger.is_step_up_fresh() {
                    let err = ledger
                        .execute_privileged_human(&agent)
                        .expect_err("AI agent re-arming breaker even while human has step-up must be refused");
                    assert!(matches!(
                        err,
                        GovernancePlaneRejection::AgentImpersonationDenied { .. }
                    ));
                } else {
                    let err = ledger
                        .execute_privileged_human(&human)
                        .expect_err("breaker re-arm without RFC 9470 step-up must be refused");
                    assert!(matches!(
                        err,
                        GovernancePlaneRejection::StepUpVerificationRequired
                    ));
                }
                countermodels_refused += 1;
                break;
            }

            // Safe transitions:
            let prev_signed = h_bool(prev, "humanSignedIn");
            let next_signed = h_bool(next, "humanSignedIn");
            let prev_step = h_bool(prev, "stepUpFresh");
            let next_step = h_bool(next, "stepUpFresh");
            let prev_del = h_bool(prev, "delegatedActive");
            let next_del = h_bool(next, "delegatedActive");
            let prev_gates = h_i64(prev, "humanGatesApproved");
            let next_gates = h_i64(next, "humanGatesApproved");

            if prev_signed && !next_signed {
                let epoch = ledger.revoke_human_session().unwrap();
                assert_eq!(epoch.get() as i64, h_i64(next, "humanAuthEpoch"));
            } else if !prev_signed && next_signed {
                let epoch = ledger
                    .reauthenticate_human(h_bool(next, "emailVerified"))
                    .unwrap();
                assert_eq!(epoch.get() as i64, h_i64(next, "humanAuthEpoch"));
            } else if !prev_step && next_step {
                ledger
                    .perform_step_up()
                    .unwrap_or_else(|e| panic!("safe step-up failed in {trace_name}: {e}"));
            } else if !prev_del && next_del {
                seq += 1;
                let tid = DelegationTicketId::new(format!("obo-{seq}")).unwrap();
                let scope =
                    DelegationScopeLevel::from_level(h_i64(next, "delegatedScopeLevel") as u8)
                        .unwrap();
                let ticket = ledger
                    .delegate_to_agent(tid, agent_id.clone(), scope)
                    .unwrap_or_else(|e| panic!("safe delegate failed in {trace_name}: {e}"));
                active_ticket = Some(ticket);
            } else if prev_del && !next_del {
                let ticket = active_ticket.take().expect("ticket for safe execute");
                ledger
                    .execute_delegated(&ticket)
                    .unwrap_or_else(|e| panic!("safe execute_delegated failed in {trace_name}: {e}"));
            } else if next_gates == prev_gates + 1 {
                ledger.approve_human_only_gate(&human).unwrap();
            } else if prev_step && !next_step && next_signed {
                ledger.execute_privileged_human(&human).unwrap();
                if overlay_mode && h_bool(prev, "backendBreakerTripped") && !h_bool(next, "backendBreakerTripped") {
                    let b_actor = marlin_agent_kernel::BreakerActor::VerifiedHuman(
                        marlin_agent_kernel::BreakerActorId::new("alice").unwrap(),
                    );
                    breaker.rearm(&b_actor).unwrap();
                }
            } else if overlay_mode && !h_bool(prev, "backendBreakerTripped") && h_bool(next, "backendBreakerTripped") {
                breaker
                    .trip(marlin_agent_kernel::BreakerTripReason::new("overlay-trip").unwrap())
                    .unwrap();
            }
        }
    }

    countermodels_refused
}

#[test]
fn mbt_delegated_agent_authz_upstream_and_backend_overlay() {
    let (_, models_dir) = find_quint_tooling().expect("quint tooling");
    let upstream_path =
        models_dir.join("threat/identity/delegated-agent-authz/DelegatedAgentAuthz.qnt");
    let upstream_traces = generate_itf_traces(
        &upstream_path,
        "delegated_agent_authz",
        &[
            "stepScopeAmplification",
            "stepActorImpersonation",
            "stepZombieDelegation",
            "stepStepUpBypass",
        ],
    );
    let refused_up = replay_delegated_authz_traces(&upstream_traces, false);
    assert!(refused_up >= 4, "expected 4 upstream OBO countermodels refused, got {refused_up}");

    let overlay_path =
        workspace_root().join("dev/quint/threat/MarlinBackendDelegatedAuthzOverlay.qnt");
    let overlay_traces = generate_itf_traces(
        &overlay_path,
        "marlin_backend_delegated_authz_overlay",
        &[
            "stepScopeAmplification",
            "stepActorImpersonation",
            "stepZombieDelegation",
            "stepStepUpBypass",
            "stepBreakerRearmWithoutStepUp",
        ],
    );
    let refused_ov = replay_delegated_authz_traces(&overlay_traces, true);
    assert!(refused_ov >= 5, "expected 5 overlay OBO countermodels refused, got {refused_ov}");
}

// ---------------------------------------------------------------------------
// 2. Multi-Tenant Isolation + Dual-Control SoD (Upstream + Backend Overlay)
// ---------------------------------------------------------------------------

fn replay_multi_tenant_traces(traces: &[(String, Vec<JsonVal>)], overlay_mode: bool) -> usize {
    let t1 = GovernanceTenantId::new("tenant-1").unwrap();
    let t2 = GovernanceTenantId::new("tenant-2").unwrap();
    let actor1 = GovernanceActorId::new("admin-1").unwrap();
    let actor2 = GovernanceActorId::new("admin-2").unwrap();
    let prop_id = GovernanceProposalId::new("tenant-admin-op").unwrap();

    let tenant_for = |n: i64| if n == 1 { t1.clone() } else { t2.clone() };
    let actor_for = |n: i64| if n == 1 { actor1.clone() } else { actor2.clone() };

    let mut countermodels_refused = 0usize;

    for (trace_name, states) in traces {
        let mut fence = MultiTenantIsolationFence::new(t1.clone());
        fence.register_tenant(t2.clone());
        let mut sod = DualControlSodLedger::new();

        let mut active_ticket: Option<TenantExecutionTicket> = None;
        let mut pre_vetoed = false;
        let mut seq = 0u64;

        for pair in states.windows(2) {
            let prev = state_h(&pair[0]);
            let next = state_h(&pair[1]);

            let cross_tenant = h_bool(next, "crossTenantLeak") && !h_bool(prev, "crossTenantLeak");
            let stale_tenant = h_bool(next, "staleTenantUse") && !h_bool(prev, "staleTenantUse");
            let cross_flush =
                h_bool(next, "crossTenantFlush") && !h_bool(prev, "crossTenantFlush");
            let sod_self = overlay_mode
                && h_bool(next, "sodSelfApprovalLeak")
                && !h_bool(prev, "sodSelfApprovalLeak");
            let veto_override = overlay_mode
                && h_bool(next, "vetoOverrideLeak")
                && !h_bool(prev, "vetoOverrideLeak");

            if cross_tenant {
                let ticket = active_ticket.as_ref().expect("ticket for cross-tenant test");
                let wrong = if ticket.tenant_id == t1 { &t2 } else { &t1 };
                let err = fence
                    .enforce_ticket(ticket, wrong)
                    .expect_err("cross-tenant deputy must be refused");
                assert!(matches!(
                    err,
                    GovernancePlaneRejection::CrossTenantDeputyDenied { .. }
                ));
                countermodels_refused += 1;
                break;
            }
            if stale_tenant {
                let ticket = active_ticket.as_ref().expect("revoked ticket");
                let target = ticket.tenant_id.clone();
                let err = fence
                    .enforce_ticket(ticket, &target)
                    .expect_err("revoked tenant ticket must be refused");
                assert!(matches!(
                    err,
                    GovernancePlaneRejection::RevokedTenantTicketDenied { .. }
                ));
                countermodels_refused += 1;
                break;
            }
            if cross_flush {
                let err = fence
                    .flush_outbox()
                    .expect_err("cross-tenant outbox flush must be refused");
                assert!(matches!(
                    err,
                    GovernancePlaneRejection::CrossTenantOutboxFlushDenied { .. }
                ));
                countermodels_refused += 1;
                break;
            }
            if sod_self {
                let proposer = actor_for(h_i64(prev, "adminProposer"));
                let err = sod
                    .approve(&prop_id, proposer)
                    .expect_err("SoD self-approval must be refused");
                assert!(matches!(
                    err,
                    GovernancePlaneRejection::SodSelfApprovalDenied { .. }
                ));
                countermodels_refused += 1;
                break;
            }
            if veto_override {
                let err = sod
                    .execute(&prop_id)
                    .expect_err("executing vetoed SoD proposal must be refused");
                assert!(matches!(
                    err,
                    GovernancePlaneRejection::VetoedProposalDenied(_)
                ));
                countermodels_refused += 1;
                break;
            }

            // Safe transitions:
            if h_i64(prev, "activeTenant") != h_i64(next, "activeTenant") {
                fence
                    .switch_active_tenant(tenant_for(h_i64(next, "activeTenant")))
                    .unwrap();
            } else if !h_bool(prev, "tenant1Revoked") && h_bool(next, "tenant1Revoked") {
                fence.revoke_tenant(&t1).unwrap();
            } else if !h_bool(prev, "tenant2Revoked") && h_bool(next, "tenant2Revoked") {
                fence.revoke_tenant(&t2).unwrap();
            } else if !h_bool(prev, "ticketIssued") && h_bool(next, "ticketIssued") {
                seq += 1;
                let tid = TenantExecutionTicketId::new(format!("tt-{seq}")).unwrap();
                active_ticket = Some(fence.issue_ticket(tid).unwrap());
            } else if h_bool(prev, "ticketIssued")
                && !h_bool(next, "ticketIssued")
                && h_i64(next, "executedCount") == h_i64(prev, "executedCount") + 1
            {
                let ticket = active_ticket.take().unwrap();
                let target = ticket.tenant_id.clone();
                fence
                    .enforce_ticket(&ticket, &target)
                    .unwrap_or_else(|e| panic!("safe enforce_ticket failed in {trace_name}: {e}"));
            } else if !h_bool(prev, "outboxPending") && h_bool(next, "outboxPending") {
                seq += 1;
                let oid = TenantOutboxEntryId::new(format!("ob-{seq}")).unwrap();
                fence.enqueue_outbox(oid, "sync-event").unwrap();
            } else if h_bool(prev, "outboxPending") && !h_bool(next, "outboxPending") {
                fence
                    .flush_outbox()
                    .unwrap_or_else(|e| panic!("safe flush_outbox failed in {trace_name}: {e}"));
            } else if overlay_mode {
                if h_i64(prev, "adminProposer") == 0 && h_i64(next, "adminProposer") > 0 {
                    sod.propose(prop_id.clone(), actor_for(h_i64(next, "adminProposer")))
                        .unwrap();
                    if pre_vetoed {
                        sod.veto(&prop_id).unwrap();
                    }
                } else if h_i64(prev, "adminApprover") == 0 && h_i64(next, "adminApprover") > 0 {
                    sod.approve(&prop_id, actor_for(h_i64(next, "adminApprover")))
                        .unwrap();
                } else if !h_bool(prev, "adminVetoed") && h_bool(next, "adminVetoed") {
                    if sod.proposal(&prop_id).is_some() {
                        sod.veto(&prop_id).unwrap();
                    } else {
                        pre_vetoed = true;
                    }
                } else if !h_bool(prev, "adminExecuted") && h_bool(next, "adminExecuted") {
                    sod.execute(&prop_id).unwrap();
                }
            }
        }
    }

    countermodels_refused
}

#[test]
fn mbt_multi_tenant_isolation_and_sod_upstream_and_overlay() {
    let (_, models_dir) = find_quint_tooling().expect("quint tooling");

    // 1. Upstream MultiTenantIsolation
    let mt_path = models_dir.join("threat/zero-trust/multi-tenant-isolation/MultiTenantIsolation.qnt");
    let mt_traces = generate_itf_traces(
        &mt_path,
        "multi_tenant_isolation",
        &[
            "stepCrossTenantDeputy",
            "stepRevokedTenantUse",
            "stepCrossTenantFlush",
        ],
    );
    let refused_mt = replay_multi_tenant_traces(&mt_traces, false);
    assert!(refused_mt >= 3, "expected 3 multi-tenant countermodels refused, got {refused_mt}");

    // 2. Backend Overlay MultiTenant + Dual-Control SoD
    let ov_path =
        workspace_root().join("dev/quint/threat/MarlinBackendMultiTenantSodOverlay.qnt");
    let ov_traces = generate_itf_traces(
        &ov_path,
        "marlin_backend_multi_tenant_sod_overlay",
        &[
            "stepCrossTenantDeputy",
            "stepRevokedTenantUse",
            "stepCrossTenantFlush",
            "stepTenantAdminSelfApproval",
            "stepTenantAdminVetoOverride",
        ],
    );
    let refused_ov = replay_multi_tenant_traces(&ov_traces, true);
    assert!(refused_ov >= 5, "expected 5 overlay multi-tenant+SoD countermodels refused, got {refused_ov}");

    // 3. Upstream GovernanceSod
    let sod_path = models_dir.join("threat/zero-trust/governance-sod/GovernanceSod.qnt");
    let sod_traces = generate_itf_traces(
        &sod_path,
        "governance_sod",
        &["stepSelfApproval", "stepVetoOverride"],
    );
    let a1 = GovernanceActorId::new("alice").unwrap();
    let a2 = GovernanceActorId::new("bob").unwrap();
    let pid = GovernanceProposalId::new("sod-1").unwrap();
    let actor_for = |n: i64| if n == 1 { a1.clone() } else { a2.clone() };
    let mut sod_refused = 0usize;

    for (_name, states) in &sod_traces {
        let mut ledger = DualControlSodLedger::new();
        let mut pre_vetoed = false;
        for pair in states.windows(2) {
            let prev = state_h(&pair[0]);
            let next = state_h(&pair[1]);

            let next_self_exec = h_bool(next, "executed")
                && h_i64(next, "approver") == h_i64(next, "proposer")
                && h_i64(next, "proposer") > 0;
            let next_self_approve = h_i64(prev, "approver") == 0
                && h_i64(next, "approver") > 0
                && h_i64(next, "approver") == h_i64(next, "proposer");
            let next_veto_exec = h_bool(next, "executed") && h_bool(next, "vetoed");

            if next_self_approve || next_self_exec {
                let err = ledger
                    .approve(&pid, actor_for(h_i64(next, "proposer")))
                    .expect_err("GovernanceSod must refuse self-approval");
                assert!(matches!(
                    err,
                    GovernancePlaneRejection::SodSelfApprovalDenied { .. }
                ));
                sod_refused += 1;
                break;
            }
            if next_veto_exec {
                let err = ledger
                    .execute(&pid)
                    .expect_err("GovernanceSod must refuse veto override");
                assert!(matches!(
                    err,
                    GovernancePlaneRejection::VetoedProposalDenied(_)
                ));
                sod_refused += 1;
                break;
            }

            if h_i64(prev, "proposer") == 0 && h_i64(next, "proposer") > 0 {
                ledger
                    .propose(pid.clone(), actor_for(h_i64(next, "proposer")))
                    .unwrap();
                if pre_vetoed {
                    ledger.veto(&pid).unwrap();
                }
            } else if h_i64(prev, "approver") == 0 && h_i64(next, "approver") > 0 {
                ledger
                    .approve(&pid, actor_for(h_i64(next, "approver")))
                    .unwrap();
            } else if !h_bool(prev, "vetoed") && h_bool(next, "vetoed") {
                if ledger.proposal(&pid).is_some() {
                    ledger.veto(&pid).unwrap();
                } else {
                    pre_vetoed = true;
                }
            } else if !h_bool(prev, "executed") && h_bool(next, "executed") {
                ledger.execute(&pid).unwrap();
            }
        }
    }
    assert!(sod_refused >= 2, "expected 2 GovernanceSod countermodels refused, got {sod_refused}");
}

// ---------------------------------------------------------------------------
// 3. Chatroom Ingress Governance (Upstream + Backend Overlay)
// ---------------------------------------------------------------------------

fn replay_chatroom_traces(traces: &[(String, Vec<JsonVal>)], overlay_mode: bool) -> usize {
    let ch1 = GovernanceChannelId::new("ch-1").unwrap();
    let ch2 = GovernanceChannelId::new("ch-2").unwrap();
    let human = GovernancePrincipal::Human(GovernanceActorId::new("ops-human").unwrap());
    let agent = GovernancePrincipal::DelegatedAgent {
        agent_id: GovernanceActorId::new("chat-agent").unwrap(),
        acting_for: GovernanceActorId::new("ops-human").unwrap(),
    };

    let mut countermodels_refused = 0usize;

    for (trace_name, states) in traces {
        let mut gov = ChatroomIngressGovernor::new(ch1.clone());
        let breaker = Arc::new(ExecutionCircuitBreaker::new(ToolBudgetUnits::new(4)));
        let mut current_gate: Option<GovernanceGateId> = None;

        for pair in states.windows(2) {
            let prev = state_h(&pair[0]);
            let next = state_h(&pair[1]);

            let forged = h_bool(next, "forgedGateReceipt") && !h_bool(prev, "forgedGateReceipt");
            let cross_ch =
                h_bool(next, "crossChannelInjected") && !h_bool(prev, "crossChannelInjected");
            let gate_replay = h_bool(next, "gateReplayed") && !h_bool(prev, "gateReplayed");
            let agent_unblock =
                h_bool(next, "agentSelfUnblocked") && !h_bool(prev, "agentSelfUnblocked");
            let taint_bypass =
                h_bool(next, "taintedToolBypass") && !h_bool(prev, "taintedToolBypass");
            let forged_no_trip = overlay_mode
                && h_bool(next, "forgedIngressNoTrip")
                && !h_bool(prev, "forgedIngressNoTrip");

            if forged {
                let err = gov
                    .ingest_external(&ExternalIngressRequest {
                        origin_verified: false,
                        claims_gate_receipt: true,
                        target_channel: ch1.clone(),
                        thread_root_channel: ch1.clone(),
                    })
                    .expect_err("forged external ingress must be refused");
                assert_eq!(err, GovernancePlaneRejection::ForgedExternalIngressDenied);
                countermodels_refused += 1;
                break;
            }
            if cross_ch {
                let err = gov
                    .ingest_external(&ExternalIngressRequest {
                        origin_verified: true,
                        claims_gate_receipt: false,
                        target_channel: ch1.clone(),
                        thread_root_channel: ch2.clone(),
                    })
                    .expect_err("cross-channel injection must be refused");
                assert!(matches!(
                    err,
                    GovernancePlaneRejection::CrossChannelInjectionDenied { .. }
                ));
                countermodels_refused += 1;
                break;
            }
            if gate_replay {
                let gid = current_gate.as_ref().expect("gate id for replay check");
                if gov.gate_status() == ChatroomGateStatus::Pending {
                    // Countermodel was AI agent approving a pending gate.
                    let err = gov
                        .resolve_human_gate(&agent, gid, true)
                        .expect_err("AI agent resolving human gate must be refused");
                    assert!(matches!(
                        err,
                        GovernancePlaneRejection::AgentImpersonationDenied { .. }
                    ));
                } else {
                    // Countermodel was replaying a non-pending gate.
                    let err = gov
                        .resolve_human_gate(&human, gid, true)
                        .expect_err("replaying resolved gate must be refused");
                    assert!(matches!(
                        err,
                        GovernancePlaneRejection::GateReplayDenied { .. }
                    ));
                }
                countermodels_refused += 1;
                break;
            }
            if agent_unblock {
                let err = gov
                    .unblock_matter(&agent)
                    .expect_err("AI agent self-unblocking matter must be refused");
                assert!(matches!(
                    err,
                    GovernancePlaneRejection::AgentSelfUnblockDenied(_)
                ));
                countermodels_refused += 1;
                break;
            }
            if taint_bypass {
                let err = gov
                    .authorize_tool_execution()
                    .expect_err("tainted chatroom tool execution without approved gate must be refused");
                assert!(matches!(
                    err,
                    GovernancePlaneRejection::TaintedChatroomToolBypassDenied(_)
                ));
                countermodels_refused += 1;
                break;
            }
            if forged_no_trip {
                let coord = BackendSecurityGovernanceCoordinator::new(
                    Arc::clone(&breaker),
                    DelegatedAgentAuthzLedger::new(
                        GovernanceActorId::new("ops-human").unwrap(),
                        DelegationScopeLevel::WorkspaceWrite,
                        true,
                    ),
                    MultiTenantIsolationFence::new(GovernanceTenantId::new("t1").unwrap()),
                    ChatroomIngressGovernor::new(ch1.clone()),
                );
                let err = coord
                    .ingest_chatroom_event(&ExternalIngressRequest {
                        origin_verified: false,
                        claims_gate_receipt: true,
                        target_channel: ch1.clone(),
                        thread_root_channel: ch1.clone(),
                    })
                    .expect_err("forged ingress must fail closed");
                assert_eq!(err, GovernancePlaneRejection::ForgedExternalIngressDenied);
                assert!(
                    breaker.snapshot().unwrap().tripped,
                    "forged external ingress MUST auto-trip backend ExecutionCircuitBreaker"
                );
                countermodels_refused += 1;
                break;
            }

            // Safe transitions:
            let prev_taint = h_bool(prev, "taintedExternalMsg");
            let next_taint = h_bool(next, "taintedExternalMsg");
            let prev_gid = h_i64(prev, "gateId");
            let next_gid = h_i64(next, "gateId");
            let prev_gs = h_i64(prev, "gateStatus");
            let next_gs = h_i64(next, "gateStatus");
            let prev_blocked = h_bool(prev, "matterBlockedByGate");
            let next_blocked = h_bool(next, "matterBlockedByGate");
            let prev_tool = h_bool(prev, "toolExecuted");
            let next_tool = h_bool(next, "toolExecuted");

            if !prev_taint && next_taint {
                gov.ingest_external(&ExternalIngressRequest {
                    origin_verified: true,
                    claims_gate_receipt: false,
                    target_channel: ch1.clone(),
                    thread_root_channel: ch1.clone(),
                })
                .unwrap();
            } else if next_gid > prev_gid && next_gs == 1 {
                let gid = GovernanceGateId::new(format!("gate-{next_gid}")).unwrap();
                gov.request_human_gate(gid.clone())
                    .unwrap_or_else(|e| panic!("safe request_human_gate failed in {trace_name}: {e}"));
                current_gate = Some(gid);
            } else if prev_gs == 1 && (next_gs == 2 || next_gs == 3) {
                let gid = current_gate.as_ref().unwrap();
                gov.resolve_human_gate(&human, gid, next_gs == 2)
                    .unwrap_or_else(|e| panic!("safe resolve_human_gate failed in {trace_name}: {e}"));
            } else if prev_blocked && !next_blocked && prev_gs == next_gs {
                gov.unblock_matter(&human).unwrap();
            } else if !prev_tool && next_tool {
                gov.authorize_tool_execution()
                    .unwrap_or_else(|e| panic!("safe tool execution failed in {trace_name}: {e}"));
            } else if overlay_mode {
                if !h_bool(prev, "backendBreakerTripped") && h_bool(next, "backendBreakerTripped") {
                    breaker
                        .trip(marlin_agent_kernel::BreakerTripReason::new("forged-ingress").unwrap())
                        .unwrap();
                } else if h_bool(prev, "backendBreakerTripped")
                    && !h_bool(next, "backendBreakerTripped")
                {
                    let b_actor = marlin_agent_kernel::BreakerActor::VerifiedHuman(
                        marlin_agent_kernel::BreakerActorId::new("ops-human").unwrap(),
                    );
                    breaker.rearm(&b_actor).unwrap();
                }
            }
        }
    }

    countermodels_refused
}

#[test]
fn mbt_chatroom_ingress_governance_upstream_and_backend_overlay() {
    let (_, models_dir) = find_quint_tooling().expect("quint tooling");
    let up_path =
        models_dir.join("threat/protocol/chatroom-ingress-governance/ChatroomIngressGovernance.qnt");
    let up_traces = generate_itf_traces(
        &up_path,
        "chatroom_ingress_governance",
        &[
            "stepForgedExternalIngress",
            "stepCrossChannelInject",
            "stepGateReplayOrAgentApprove",
            "stepAgentSelfUnblock",
            "stepTaintedToolBypass",
        ],
    );
    let refused_up = replay_chatroom_traces(&up_traces, false);
    assert!(refused_up >= 5, "expected 5 upstream chatroom countermodels refused, got {refused_up}");

    let ov_path =
        workspace_root().join("dev/quint/threat/MarlinBackendChatroomIngressOverlay.qnt");
    let ov_traces = generate_itf_traces(
        &ov_path,
        "marlin_backend_chatroom_ingress_overlay",
        &[
            "stepForgedExternalIngress",
            "stepCrossChannelInject",
            "stepGateReplayOrAgentApprove",
            "stepAgentSelfUnblock",
            "stepTaintedToolBypass",
            "stepForgedIngressNoAutoTrip",
        ],
    );
    let refused_ov = replay_chatroom_traces(&ov_traces, true);
    assert!(refused_ov >= 6, "expected 6 overlay chatroom countermodels refused, got {refused_ov}");
}

// ---------------------------------------------------------------------------
// 4. End-to-End Unified Backend Coordinator + Controller Async Integration
// ---------------------------------------------------------------------------

#[cfg(unix)]
#[tokio::test]
async fn unified_backend_governance_coordinator_enforces_auto_trip_step_up_and_os_cutoff() {
    let tenant_a = GovernanceTenantId::new("tenant-alpha").unwrap();
    let tenant_b = GovernanceTenantId::new("tenant-beta").unwrap();
    let human_id = GovernanceActorId::new("ops-alice").unwrap();
    let agent_id = GovernanceActorId::new("marlin-agent-exec").unwrap();
    let ch_sec = GovernanceChannelId::new("ch-security").unwrap();

    let human = GovernancePrincipal::Human(human_id.clone());
    let agent = GovernancePrincipal::DelegatedAgent {
        agent_id: agent_id.clone(),
        acting_for: human_id.clone(),
    };

    let breaker = Arc::new(ExecutionCircuitBreaker::new(ToolBudgetUnits::new(4)));
    let mut tenants = MultiTenantIsolationFence::new(tenant_a.clone());
    tenants.register_tenant(tenant_b.clone());

    let coord = BackendSecurityGovernanceCoordinator::new(
        Arc::clone(&breaker),
        DelegatedAgentAuthzLedger::new(
            human_id.clone(),
            DelegationScopeLevel::WorkspaceWrite,
            true,
        ),
        tenants,
        ChatroomIngressGovernor::new(ch_sec.clone()),
    );

    // Build controller wired directly with the coordinator's authoritative breaker.
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let sentinel = temp_dir.path().join("coordinator-ran.sentinel");
    let resolver = StaticLoopProgramToolProcessResolver::new(
        vec![
            LoopProgramToolProcessCommandTemplate::new(
                "agent-flow.tool-intent",
                ["loop-program.dispatch-tools"],
                LoopProgramToolProcessProgram::new("/bin/sh"),
            )
            .with_args(["-c".to_owned(), format!("touch '{}'", sentinel.display())]),
        ]
        .into_boxed_slice(),
    );
    let executor = LoopProgramRuntimeSideEffectExecutor::new(resolver)
        .with_execution_breaker(coord.execution_breaker());
    let _controller =
        TokioGraphLoopController::new(TokioGraphLoopKernel::new("run-gov-e2e", "graph-gov-e2e"))
            .with_loop_program_side_effect_executor(executor.clone())
            .with_execution_breaker(coord.execution_breaker())
            .with_tool_ifc_context(ToolIfcContext::clean_trusted());

    let plan = LoopProgramRuntimeHandoffPlan::from_receipts(
        LoopProgramId::new("gov-e2e"),
        &[GenericLoopMachineReceipt {
            program_id: LoopProgramId::new("gov-e2e"),
            step_index: GenericLoopMachineStepIndex::new(1),
            transition_id: LoopProgramTransitionId::new("t-1"),
            from: LoopProgramStateId::new("start"),
            event: LoopProgramEventKind::RuntimeReceipt,
            action: LoopProgramActionKind::DispatchTools,
            to: LoopProgramStateId::new("done"),
            stopped: false,
        }],
    );
    let handoff = AgentFlowLoopProgramRuntimeHandoffExecutor::new(LoopProgramRuntimeOwner::new(
        "agent-flow-runtime",
    ))
    .execute_plan(&plan);

    // 1. Forged external chatroom ingress auto-trips the backend breaker & blocks real OS process!
    let forged_err = coord
        .ingest_chatroom_event(&ExternalIngressRequest {
            origin_verified: false,
            claims_gate_receipt: true,
            target_channel: ch_sec.clone(),
            thread_root_channel: ch_sec.clone(),
        })
        .expect_err("forged ingress must fail");
    assert_eq!(forged_err, GovernancePlaneRejection::ForgedExternalIngressDenied);
    assert!(coord.execution_breaker().snapshot().unwrap().tripped);

    let (runtime, _events) = TokioAgentRuntime::new(4);
    let blocked = executor.execute(&runtime.context(), &handoff).await;
    assert_eq!(blocked.status, LoopProgramRuntimeSideEffectStatus::Failed);
    assert!(!sentinel.exists(), "forged ingress auto-trip must block OS spawn");

    // 2. Neither AI agent nor human without RFC 9470 step-up can re-arm!
    assert!(matches!(
        coord.rearm_breaker_with_step_up(&agent),
        Err(GovernancePlaneRejection::AgentImpersonationDenied { .. })
    ));
    assert!(matches!(
        coord.rearm_breaker_with_step_up(&human),
        Err(GovernancePlaneRejection::StepUpVerificationRequired)
    ));

    // 3. Verified human performs step-up -> re-arms breaker (epoch 2).
    coord
        .with_ledgers(|del, _, _, _| del.perform_step_up())
        .unwrap();
    let rearm = coord.rearm_breaker_with_step_up(&human).unwrap();
    assert_eq!(rearm.new_epoch.get(), 2);

    // 4. Valid external chatroom ingress taints channel -> blocks side effect until human approves gate!
    coord
        .ingest_chatroom_event(&ExternalIngressRequest {
            origin_verified: true,
            claims_gate_receipt: false,
            target_channel: ch_sec.clone(),
            thread_root_channel: ch_sec.clone(),
        })
        .unwrap();

    let (tenant_ticket, obo_ticket) = coord
        .with_ledgers(|del, ten, _, _| {
            let tt = ten.issue_ticket(TenantExecutionTicketId::new("tt-alpha-1")?)?;
            let ot = del.delegate_to_agent(
                DelegationTicketId::new("obo-alpha-1")?,
                agent_id.clone(),
                DelegationScopeLevel::ReadOnly,
            )?;
            Ok((tt, ot))
        })
        .unwrap();

    // Cross-tenant deputy rejected!
    assert!(matches!(
        coord.authorize_governed_side_effect(
            &tenant_ticket,
            &tenant_b,
            Some(&obo_ticket),
            "tool-exec",
            &ToolIfcContext::clean_trusted(),
        ),
        Err(GovernancePlaneRejection::CrossTenantDeputyDenied { .. })
    ));

    // Same-tenant still blocked by tainted chatroom without human gate approval!
    assert!(matches!(
        coord.authorize_governed_side_effect(
            &tenant_ticket,
            &tenant_a,
            None,
            "tool-exec",
            &ToolIfcContext::clean_trusted(),
        ),
        Err(GovernancePlaneRejection::TaintedChatroomToolBypassDenied(_))
    ));

    // 5. Human opens & approves one-shot gate -> governed side effect & real OS process succeed!
    let gate_id = GovernanceGateId::new("gate-e2e-1").unwrap();
    coord
        .with_ledgers(|_, _, chat, _| {
            chat.request_human_gate(gate_id.clone())?;
            chat.resolve_human_gate(&human, &gate_id, true)?;
            Ok(())
        })
        .unwrap();

    let commit_receipt = coord
        .authorize_governed_side_effect(
            &tenant_ticket,
            &tenant_a,
            Some(&obo_ticket),
            "tool-exec",
            &ToolIfcContext::clean_trusted(),
        )
        .expect("governed side effect succeeds once tenant, OBO, human gate, and breaker all pass");
    assert_eq!(commit_receipt.trip_epoch.get(), 2);

    let ok = executor.execute(&runtime.context(), &handoff).await;
    assert_eq!(ok.status, LoopProgramRuntimeSideEffectStatus::Completed);
    assert!(sentinel.exists(), "real OS process should succeed after step-up re-arm");
}
