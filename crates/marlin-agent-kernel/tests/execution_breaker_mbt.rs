// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Live Quint -> ITF -> Rust Model-Based Testing (MBT) refinement suite for
//! `marlin_agent_kernel::ExecutionCircuitBreaker` and live async side-effect
//! cutoff integration in `LoopProgramRuntimeSideEffectExecutor`.
//!
//! Executes `quint-modules-resolver mbt-audit` against:
//! 1. Upstream `AgentToolCircuitBreaker.qnt`
//! 2. Backend overlay `dev/quint/threat/MarlinBackendExecutionBreakerOverlay.qnt`
//! 3. Upstream `AgentIfcCapability.qnt`
//!
//! Also proves with real async `tokio::test` execution that tripping the breaker
//! or exhausting its armed-window budget blocks REAL OS process spawns and
//! REAL disk file writes inside `LoopProgramRuntimeSideEffectExecutor`.

use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

static QUINT_APALACHE_LOCK: Mutex<()> = Mutex::new(());

use marlin_agent_kernel::{
    AgentFlowLoopProgramRuntimeHandoffExecutor, BreakerActor, BreakerActorId,
    BreakerRejectionReason, BreakerTripReason, ExecutionCircuitBreaker, GenericLoopMachineReceipt,
    GenericLoopMachineStepIndex, LoopProgramFileSandbox, LoopProgramFileWriteSideEffectStatus,
    LoopProgramFileWriteTemplate, LoopProgramRuntimeHandoffExecutor, LoopProgramRuntimeHandoffPlan,
    LoopProgramRuntimeOwner, LoopProgramRuntimeSideEffectExecutor,
    LoopProgramRuntimeSideEffectStatus, LoopProgramToolProcessCommandTemplate,
    LoopProgramToolProcessProgram, LoopProgramToolProcessSideEffectStatus,
    StaticLoopProgramFileWriteResolver, StaticLoopProgramToolProcessResolver, ToolBudgetUnits,
    ToolExecutionLease, ToolExecutionLeaseId, ToolIfcContext,
};
use marlin_agent_protocol::{
    LoopProgramActionKind, LoopProgramEventKind, LoopProgramId, LoopProgramStateId,
    LoopProgramTransitionId,
};
use marlin_agent_runtime::TokioAgentRuntime;

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
// Quint resolver & live ITF trace generation helpers (no hardcoded host paths)
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

    let candidate_node_modules = [
        root.join("node_modules/@deterministic-engineering"),
        root.join(".data/marlin-mygo/node_modules/@deterministic-engineering"),
    ];
    for base in candidate_node_modules {
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
    let _apalache_guard = QUINT_APALACHE_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let (resolver_cli, models_dir) = find_quint_tooling()
        .expect("quint-modules-resolver and quint-models must be installed under node_modules or .data/marlin-mygo/node_modules");
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
        "marlin-kernel-mbt-{}-{nanos}",
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

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "quint mbt-audit failed for {}:\nSTDOUT:\n{}\nSTDERR:\n{}",
        model_path.display(),
        stdout,
        stderr
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
    assert!(
        !traces.is_empty(),
        "mbt-audit produced zero ITF traces for {}",
        model_path.display()
    );
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
// 1. Live Quint -> ITF -> Rust MBT: Upstream AgentToolCircuitBreaker.qnt
// ---------------------------------------------------------------------------

#[test]
fn mbt_upstream_agent_tool_circuit_breaker_refines_rust_breaker() {
    let (_, models_dir) = find_quint_tooling().expect("local quint-models");
    let model_path =
        models_dir.join("threat/execution/agent-tool-circuit-breaker/AgentToolCircuitBreaker.qnt");
    let traces = generate_itf_traces(
        &model_path,
        "agent_tool_circuit_breaker",
        &[
            "stepExecWhileTripped",
            "stepStaleLeaseAfterTrip",
            "stepAgentSelfRearm",
            "stepBudgetRunaway",
        ],
    );

    let human = BreakerActor::VerifiedHuman(BreakerActorId::new("ops-human").unwrap());
    let agent = BreakerActor::Agent(BreakerActorId::new("agent-autonomous").unwrap());
    let unverified = BreakerActor::UnverifiedActor(BreakerActorId::new("unverified-user").unwrap());
    let reason = BreakerTripReason::new("emergency-stop-triggered").unwrap();

    let mut countermodels_refused = 0usize;

    for (trace_name, states) in &traces {
        let breaker =
            ExecutionCircuitBreaker::new(ToolBudgetUnits::new(1)).with_single_lease_enforced(true);
        let mut active_lease: Option<ToolExecutionLease> = None;
        let mut lease_seq = 0u64;

        for pair in states.windows(2) {
            let prev = state_h(&pair[0]);
            let next = state_h(&pair[1]);

            let exec_tripped =
                h_bool(next, "execWhileTripped") && !h_bool(prev, "execWhileTripped");
            let stale_commit =
                h_bool(next, "staleLeaseCommit") && !h_bool(prev, "staleLeaseCommit");
            let agent_rearm =
                h_bool(next, "agentSelfRearmed") && !h_bool(prev, "agentSelfRearmed");
            let budget_overrun =
                h_bool(next, "budgetOverrun") && !h_bool(prev, "budgetOverrun");

            if exec_tripped {
                let lease = active_lease.as_ref().expect("active lease before tripped commit");
                let err = breaker
                    .commit_effect(lease)
                    .expect_err("Rust breaker must refuse commit while tripped");
                assert!(matches!(err, BreakerRejectionReason::BreakerTripped { .. }));
                countermodels_refused += 1;
                break;
            }
            if stale_commit {
                let lease = active_lease.as_ref().expect("stale lease after re-arm");
                let err = breaker
                    .commit_effect(lease)
                    .expect_err("Rust breaker must refuse stale pre-trip lease after re-arm");
                assert!(matches!(err, BreakerRejectionReason::StaleLeaseEpoch { .. }));
                countermodels_refused += 1;
                break;
            }
            if agent_rearm {
                let err_agent = breaker
                    .rearm(&agent)
                    .expect_err("Rust breaker must refuse AI agent self-rearm");
                assert!(matches!(
                    err_agent,
                    BreakerRejectionReason::HumanOnlyRearmRequired(_)
                ));
                let err_unverified = breaker
                    .rearm(&unverified)
                    .expect_err("Rust breaker must refuse unverified actor re-arm");
                assert!(matches!(
                    err_unverified,
                    BreakerRejectionReason::HumanOnlyRearmRequired(_)
                ));
                countermodels_refused += 1;
                break;
            }
            if budget_overrun {
                let lease = active_lease.as_ref().expect("lease at budget overrun");
                let err = breaker
                    .commit_effect(lease)
                    .expect_err("Rust breaker must refuse tool budget overrun");
                assert!(matches!(err, BreakerRejectionReason::BudgetExhausted { .. }));
                countermodels_refused += 1;
                break;
            }

            let prev_tripped = h_bool(prev, "breakerTripped");
            let next_tripped = h_bool(next, "breakerTripped");
            let prev_lease = h_bool(prev, "leaseActive");
            let next_lease = h_bool(next, "leaseActive");
            let prev_budget = h_i64(prev, "budgetUsed");
            let next_budget = h_i64(next, "budgetUsed");

            if !prev_lease && next_lease {
                lease_seq += 1;
                let lid = ToolExecutionLeaseId::new(format!("lease-{lease_seq}")).unwrap();
                let lease = breaker
                    .acquire_lease(lid, "tool-exec")
                    .unwrap_or_else(|e| panic!("safe acquire failed in {trace_name}: {e}"));
                assert_eq!(lease.trip_epoch.get() as i64, h_i64(next, "leaseEpoch"));
                active_lease = Some(lease);
            } else if !prev_tripped && next_tripped {
                breaker.trip(reason.clone()).unwrap();
            } else if prev_tripped && !next_tripped {
                let receipt = breaker
                    .rearm(&human)
                    .unwrap_or_else(|e| panic!("safe human rearm failed in {trace_name}: {e}"));
                assert_eq!(receipt.new_epoch.get() as i64, h_i64(next, "tripEpoch"));
            } else if prev_lease && !next_lease && next_budget == prev_budget + 1 {
                let lease = active_lease.take().expect("lease for safe commit");
                let receipt = breaker
                    .commit_effect(&lease)
                    .unwrap_or_else(|e| panic!("safe commit failed in {trace_name}: {e}"));
                assert_eq!(receipt.budget_used as i64, next_budget);
            } else if prev_lease && !next_lease {
                let lease = active_lease.take().expect("stale lease for cancel");
                assert!(breaker.cancel_stale_lease(&lease.lease_id).unwrap());
            }
        }
    }

    assert!(
        countermodels_refused >= 4,
        "expected all 4 upstream breaker countermodels refused, got {countermodels_refused}"
    );
}

// ---------------------------------------------------------------------------
// 2. Live Quint -> ITF -> Rust MBT: Backend Execution Breaker Overlay
// ---------------------------------------------------------------------------

#[test]
fn mbt_marlin_backend_execution_breaker_overlay_refines_rust_breaker() {
    let model_path =
        workspace_root().join("dev/quint/threat/MarlinBackendExecutionBreakerOverlay.qnt");
    let traces = generate_itf_traces(
        &model_path,
        "marlin_backend_execution_breaker_overlay",
        &[
            "stepExecWhileTripped",
            "stepStaleLeaseAfterTrip",
            "stepAgentSelfRearm",
            "stepBudgetRunaway",
            "stepBackendTaintedCommit",
        ],
    );

    let human = BreakerActor::VerifiedHuman(BreakerActorId::new("ops-human").unwrap());
    let agent = BreakerActor::Agent(BreakerActorId::new("agent-backend").unwrap());
    let reason = BreakerTripReason::new("backend-estop").unwrap();

    let mut countermodels_refused = 0usize;

    for (trace_name, states) in &traces {
        let breaker =
            ExecutionCircuitBreaker::new(ToolBudgetUnits::new(1)).with_single_lease_enforced(true);
        let mut active_lease: Option<ToolExecutionLease> = None;
        let mut lease_seq = 0u64;

        for pair in states.windows(2) {
            let prev = state_h(&pair[0]);
            let next = state_h(&pair[1]);

            let exec_tripped =
                h_bool(next, "execWhileTripped") && !h_bool(prev, "execWhileTripped");
            let stale_commit =
                h_bool(next, "staleLeaseCommit") && !h_bool(prev, "staleLeaseCommit");
            let agent_rearm =
                h_bool(next, "agentSelfRearmed") && !h_bool(prev, "agentSelfRearmed");
            let budget_overrun =
                h_bool(next, "budgetOverrun") && !h_bool(prev, "budgetOverrun");
            let tainted_commit = h_bool(next, "taintedSideEffectCommit")
                && !h_bool(prev, "taintedSideEffectCommit");

            if exec_tripped {
                let lease = active_lease.as_ref().unwrap();
                let err = breaker.commit_effect(lease).expect_err("must block tripped spawn");
                assert!(matches!(err, BreakerRejectionReason::BreakerTripped { .. }));
                countermodels_refused += 1;
                break;
            }
            if stale_commit {
                let lease = active_lease.as_ref().unwrap();
                let err = breaker
                    .commit_effect(lease)
                    .expect_err("must block stale pre-trip file write");
                assert!(matches!(err, BreakerRejectionReason::StaleLeaseEpoch { .. }));
                countermodels_refused += 1;
                break;
            }
            if agent_rearm {
                let err = breaker
                    .rearm(&agent)
                    .expect_err("must refuse agent self-rearm");
                assert!(matches!(
                    err,
                    BreakerRejectionReason::HumanOnlyRearmRequired(_)
                ));
                countermodels_refused += 1;
                break;
            }
            if budget_overrun {
                let lease = active_lease.as_ref().unwrap();
                let err = breaker
                    .commit_effect(lease)
                    .expect_err("must refuse tool budget overrun");
                assert!(matches!(err, BreakerRejectionReason::BudgetExhausted { .. }));
                countermodels_refused += 1;
                break;
            }
            if tainted_commit {
                let lease = active_lease.as_ref().unwrap();
                let err = breaker
                    .commit_effect_with_ifc(lease, &ToolIfcContext::tainted_control_flow())
                    .expect_err("must block CaMeL tainted side-effect commit");
                assert_eq!(err, BreakerRejectionReason::PromptInjectionControlHijackDenied);
                countermodels_refused += 1;
                break;
            }

            let prev_tripped = h_bool(prev, "breakerTripped");
            let next_tripped = h_bool(next, "breakerTripped");
            let prev_lease = h_bool(prev, "leaseActive");
            let next_lease = h_bool(next, "leaseActive");
            let prev_budget = h_i64(prev, "budgetUsed");
            let next_budget = h_i64(next, "budgetUsed");

            if !prev_lease && next_lease {
                lease_seq += 1;
                let lid = ToolExecutionLeaseId::new(format!("ov-lease-{lease_seq}")).unwrap();
                active_lease = Some(breaker.acquire_lease(lid, "side-effect").unwrap());
            } else if !prev_tripped && next_tripped {
                breaker.trip(reason.clone()).unwrap();
            } else if prev_tripped && !next_tripped {
                breaker
                    .rearm(&human)
                    .unwrap_or_else(|e| panic!("overlay rearm failed in {trace_name}: {e}"));
            } else if prev_lease && !next_lease && next_budget == prev_budget + 1 {
                let lease = active_lease.take().unwrap();
                breaker
                    .commit_effect_with_ifc(&lease, &ToolIfcContext::clean_trusted())
                    .unwrap_or_else(|e| panic!("overlay commit failed in {trace_name}: {e}"));
            } else if prev_lease && !next_lease {
                let lease = active_lease.take().unwrap();
                assert!(breaker.cancel_stale_lease(&lease.lease_id).unwrap());
            }
        }
    }

    assert!(
        countermodels_refused >= 5,
        "expected all 5 composed overlay breaker countermodels (4 inherited + 1 CaMeL taint) refused, got {countermodels_refused}"
    );
}

// ---------------------------------------------------------------------------
// 3. Live Quint -> ITF -> Rust MBT: Upstream AgentIfcCapability.qnt (CaMeL IFC)
// ---------------------------------------------------------------------------

#[test]
fn mbt_upstream_agent_ifc_capability_refines_rust_breaker_ifc() {
    let (_, models_dir) = find_quint_tooling().expect("local quint-models");
    let model_path =
        models_dir.join("threat/execution/agent-ifc-capability/AgentIfcCapability.qnt");
    let traces = generate_itf_traces(
        &model_path,
        "agent_ifc_capability",
        &["stepPromptInjectionHijack", "stepConfidentialExfiltration"],
    );

    let mut countermodels_refused = 0usize;

    for (_trace_name, states) in &traces {
        let breaker = ExecutionCircuitBreaker::new(ToolBudgetUnits::new(8));
        let mut lease_seq = 0u64;

        for pair in states.windows(2) {
            let prev = state_h(&pair[0]);
            let next = state_h(&pair[1]);

            let prev_taint_hijack = h_bool(prev, "toolInvoked") && h_bool(prev, "toolControlledByTainted");
            let next_taint_hijack = h_bool(next, "toolInvoked") && h_bool(next, "toolControlledByTainted");
            let prev_secret_exfil = h_bool(prev, "toolInvoked")
                && h_bool(prev, "sinkObservedSecret")
                && !h_bool(prev, "sinkObservedDeclassified");
            let next_secret_exfil = h_bool(next, "toolInvoked")
                && h_bool(next, "sinkObservedSecret")
                && !h_bool(next, "sinkObservedDeclassified");
            let clean_invoked_now = h_bool(next, "toolInvoked") && !h_bool(prev, "toolInvoked");

            let taint_hijack_now = next_taint_hijack && !prev_taint_hijack;
            let secret_exfil_now = next_secret_exfil && !prev_secret_exfil;

            if !taint_hijack_now && !secret_exfil_now && !clean_invoked_now {
                continue;
            }

            let controlled_by_tainted = h_bool(next, "toolControlledByTainted");
            let sink_secret = h_bool(next, "sinkObservedSecret");
            let sink_declass = h_bool(next, "sinkObservedDeclassified");

            lease_seq += 1;
            let lease = breaker
                .acquire_lease(
                    ToolExecutionLeaseId::new(format!("ifc-lease-{lease_seq}")).unwrap(),
                    "ifc-tool",
                )
                .unwrap();

            let ifc = ToolIfcContext {
                plan_integrity_high: h_bool(next, "planIntegrityHigh"),
                tool_controlled_by_tainted_data: controlled_by_tainted,
                carries_confidential_data: sink_secret,
                declassification_approved: sink_declass,
            };

            if taint_hijack_now {
                let err = breaker
                    .commit_effect_with_ifc(&lease, &ifc)
                    .expect_err("CaMeL IFC must block prompt-injection control hijack");
                assert_eq!(err, BreakerRejectionReason::PromptInjectionControlHijackDenied);
                countermodels_refused += 1;
                break;
            } else if secret_exfil_now {
                let err = breaker
                    .commit_effect_with_ifc(&lease, &ifc)
                    .expect_err("CaMeL IFC must block confidential egress without declassification");
                assert_eq!(err, BreakerRejectionReason::ConfidentialExfiltrationDenied);
                countermodels_refused += 1;
                break;
            } else {
                breaker
                    .commit_effect_with_ifc(&lease, &ifc)
                    .expect("valid clean or declassified IFC tool invocation must pass");
            }
        }
    }

    assert!(
        countermodels_refused >= 2,
        "expected both CaMeL IFC countermodels refused, got {countermodels_refused}"
    );
}

// ---------------------------------------------------------------------------
// 4. Real Async Side-Effect Cutoff Integration Tests (OS Process + Disk File Write)
// ---------------------------------------------------------------------------

fn dispatch_tools_handoff() -> marlin_agent_kernel::LoopProgramRuntimeHandoffExecutionReceipt {
    let plan = LoopProgramRuntimeHandoffPlan::from_receipts(
        LoopProgramId::new("breaker-side-effect-program"),
        &[GenericLoopMachineReceipt {
            program_id: LoopProgramId::new("breaker-side-effect-program"),
            step_index: GenericLoopMachineStepIndex::new(1),
            transition_id: LoopProgramTransitionId::new("dispatch-1"),
            from: LoopProgramStateId::new("start"),
            event: LoopProgramEventKind::RuntimeReceipt,
            action: LoopProgramActionKind::DispatchTools,
            to: LoopProgramStateId::new("await-tool"),
            stopped: false,
        }],
    );
    AgentFlowLoopProgramRuntimeHandoffExecutor::new(LoopProgramRuntimeOwner::new(
        "agent-flow-runtime",
    ))
    .execute_plan(&plan)
}

fn shell_tool_resolver(script: String) -> StaticLoopProgramToolProcessResolver {
    StaticLoopProgramToolProcessResolver::new(
        vec![
            LoopProgramToolProcessCommandTemplate::new(
                "agent-flow.tool-intent",
                ["loop-program.dispatch-tools"],
                LoopProgramToolProcessProgram::new("/bin/sh"),
            )
            .with_args(["-c".to_owned(), script]),
        ]
        .into_boxed_slice(),
    )
}

fn single_file_write_resolver(
    relative_path: &'static str,
    contents: &'static str,
) -> StaticLoopProgramFileWriteResolver {
    StaticLoopProgramFileWriteResolver::new(
        vec![LoopProgramFileWriteTemplate::new(
            "agent-flow.tool-intent",
            ["loop-program.dispatch-tools"],
            relative_path,
            contents.as_bytes().to_vec(),
        )]
        .into_boxed_slice(),
    )
}

#[cfg(unix)]
#[tokio::test]
async fn tripped_breaker_blocks_real_os_process_spawn_and_disk_file_write_until_human_rearm() {
    let (runtime, _events) = TokioAgentRuntime::new(4);
    let handoff = dispatch_tools_handoff();

    let temp_dir = tempfile::tempdir().expect("temp workspace");
    let sentinel = temp_dir.path().join("os-process-ran.sentinel");
    let target_file = temp_dir.path().join("notes/output.txt");

    let breaker = Arc::new(ExecutionCircuitBreaker::new(ToolBudgetUnits::new(2)));
    breaker
        .trip(BreakerTripReason::new("operator-emergency-stop").unwrap())
        .unwrap();

    let executor = LoopProgramRuntimeSideEffectExecutor::new(shell_tool_resolver(format!(
        "touch '{}'",
        sentinel.display()
    )))
    .with_file_write_resolver(single_file_write_resolver(
        "notes/output.txt",
        "must-not-write-while-tripped",
    ))
    .with_file_sandbox(
        LoopProgramFileSandbox::new(temp_dir.path())
            .with_allowed_relative_paths(["notes/output.txt"]),
    )
    .with_execution_breaker(Arc::clone(&breaker));

    // 1. While tripped: NEITHER the OS process NOR the disk file write may execute!
    let blocked_receipt = executor.execute(&runtime.context(), &handoff).await;
    assert_eq!(blocked_receipt.status, LoopProgramRuntimeSideEffectStatus::Failed);
    assert_eq!(blocked_receipt.tool_processes.len(), 1);
    assert_eq!(
        blocked_receipt.tool_processes[0].status,
        LoopProgramToolProcessSideEffectStatus::Failed
    );
    assert!(blocked_receipt.tool_processes[0].spawn_receipt.is_none());
    assert!(
        blocked_receipt.tool_processes[0]
            .diagnostic
            .as_deref()
            .unwrap_or("")
            .contains("breaker_blocked")
    );
    assert_eq!(blocked_receipt.file_writes.len(), 1);
    assert_eq!(
        blocked_receipt.file_writes[0].status,
        LoopProgramFileWriteSideEffectStatus::Denied
    );
    assert!(blocked_receipt.file_writes[0].write_receipt.is_none());
    assert!(
        !sentinel.exists(),
        "OS process MUST NOT spawn when circuit breaker is tripped"
    );
    assert!(
        !target_file.exists(),
        "disk file MUST NOT be written when circuit breaker is tripped"
    );

    // 2. AI agent cannot self-rearm!
    let agent = BreakerActor::Agent(BreakerActorId::new("marlin-agent-loop").unwrap());
    assert!(matches!(
        breaker.rearm(&agent),
        Err(BreakerRejectionReason::HumanOnlyRearmRequired(_))
    ));

    // 3. Verified human operator re-arms -> trip_epoch advances to 2 -> real side effects succeed!
    let human = BreakerActor::VerifiedHuman(BreakerActorId::new("ops-oncall").unwrap());
    let rearm = breaker.rearm(&human).expect("verified human re-arm");
    assert_eq!(rearm.new_epoch.get(), 2);

    let ok_receipt = executor.execute(&runtime.context(), &handoff).await;
    assert_eq!(ok_receipt.status, LoopProgramRuntimeSideEffectStatus::Completed);
    assert!(sentinel.exists(), "OS process should run after verified human re-arm");
    assert_eq!(
        fs::read_to_string(&target_file).unwrap(),
        "must-not-write-while-tripped"
    );

    // 4. Budget cap (2 units: 1 process + 1 file write) is now exhausted -> subsequent execution fails closed!
    fs::remove_file(&sentinel).unwrap();
    let budget_blocked = executor.execute(&runtime.context(), &handoff).await;
    assert_eq!(budget_blocked.status, LoopProgramRuntimeSideEffectStatus::Failed);
    assert!(
        !sentinel.exists(),
        "OS process MUST NOT spawn after armed-window budget is exhausted"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn camel_tainted_ifc_context_blocks_real_side_effect_executor() {
    let (runtime, _events) = TokioAgentRuntime::new(4);
    let handoff = dispatch_tools_handoff();
    let temp_dir = tempfile::tempdir().expect("temp workspace");
    let sentinel = temp_dir.path().join("tainted-process-ran.sentinel");

    let breaker = Arc::new(ExecutionCircuitBreaker::new(ToolBudgetUnits::new(4)));
    let tainted_executor = LoopProgramRuntimeSideEffectExecutor::new(shell_tool_resolver(format!(
        "touch '{}'",
        sentinel.display()
    )))
    .with_execution_breaker(Arc::clone(&breaker))
    .with_ifc_context(ToolIfcContext::tainted_control_flow());

    let receipt = tainted_executor.execute(&runtime.context(), &handoff).await;
    assert_eq!(receipt.status, LoopProgramRuntimeSideEffectStatus::Failed);
    assert!(receipt.tool_processes[0].spawn_receipt.is_none());
    assert!(
        receipt.tool_processes[0]
            .diagnostic
            .as_deref()
            .unwrap_or("")
            .contains("tainted")
    );
    assert!(
        !sentinel.exists(),
        "OS process MUST NOT spawn when CaMeL IFC context is tainted"
    );
}
