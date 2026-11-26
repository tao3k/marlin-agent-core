// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Live Quint -> ITF -> Rust Model-Based Testing (MBT) refinement suite for
//! `marlin_agent_kernel::higher_order_bridge::HigherOrderGovernanceAdmissionGate`
//! against `dev/quint/threat/MarlinBackendCausalRetractionOverlay.qnt`.
//!
//! Proves all 6 composed countermodels (4 inherited ExecutionCircuitBreaker +
//! 2 Bi-Temporal Causal Retraction & Monotonic Proof-Epoch countermodels) are
//! deterministically refused by real Rust code.

use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use marlin_agent_kernel::{
    BreakerActor, BreakerActorId, BreakerRejectionReason, BreakerTripReason, CedarPolicyId,
    DerivationId, ExecutionCircuitBreaker, GovernanceProofDigest, GovernedSubjectName,
    HIGHER_ORDER_GOVERNANCE_SCHEMA_VERSION, HigherOrderBridgeError,
    HigherOrderGovernanceAdmissionGate, HigherOrderGovernanceProofEnvelope, HigherOrderProofEpoch,
    SynthesizedCedarPolicy, ToolBudgetUnits, ToolExecutionLease, ToolExecutionLeaseId,
    ToolIfcContext,
};

static QUINT_MBT_LOCK: Mutex<()> = Mutex::new(());

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
            Self::Obj(m) => Some(m),
            _ => None,
        }
    }
    fn as_arr(&self) -> Option<&[JsonVal]> {
        match self {
            Self::Arr(a) => Some(a),
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
            Self::Obj(m) => match m.get("#bigint") {
                Some(JsonVal::Str(s)) => s.parse::<i64>().ok(),
                _ => None,
            },
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
        let &ch = self.bytes.get(self.pos).ok_or_else(|| "eof".to_owned())?;
        match ch {
            b'{' => self.parse_obj(),
            b'[' => self.parse_arr(),
            b'"' => self.parse_str().map(JsonVal::Str),
            b't' => self.expect(b"true", JsonVal::Bool(true)),
            b'f' => self.expect(b"false", JsonVal::Bool(false)),
            b'n' => self.expect(b"null", JsonVal::Null),
            b'-' | b'0'..=b'9' => self.parse_num(),
            _ => Err(format!("bad byte {}", ch as char)),
        }
    }
    fn expect(&mut self, lit: &[u8], val: JsonVal) -> Result<JsonVal, String> {
        if self.bytes[self.pos..].starts_with(lit) {
            self.pos += lit.len();
            Ok(val)
        } else {
            Err("bad lit".to_owned())
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
                    let esc = self.bytes[self.pos];
                    self.pos += 1;
                    match esc {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            let hex = std::str::from_utf8(&self.bytes[self.pos..self.pos + 4])
                                .map_err(|e| e.to_string())?;
                            self.pos += 4;
                            let code = u32::from_str_radix(hex, 16).map_err(|e| e.to_string())?;
                            out.push(char::from_u32(code).unwrap_or('\u{FFFD}'));
                        }
                        _ => return Err("bad esc".to_owned()),
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
                    out.push_str(
                        std::str::from_utf8(&self.bytes[start..end]).map_err(|e| e.to_string())?,
                    );
                    self.pos = end;
                }
            }
        }
        Err("unterminated str".to_owned())
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
        let txt = std::str::from_utf8(&self.bytes[start..self.pos]).map_err(|e| e.to_string())?;
        Ok(JsonVal::Int(txt.parse::<i64>().unwrap_or(0)))
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
                _ => return Err("bad arr".to_owned()),
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
            let k = self.parse_str()?;
            self.skip_ws();
            self.pos += 1; // ':'
            let v = self.parse_val()?;
            map.insert(k, v);
            self.skip_ws();
            match self.bytes.get(self.pos) {
                Some(&b',') => self.pos += 1,
                Some(&b'}') => {
                    self.pos += 1;
                    return Ok(JsonVal::Obj(map));
                }
                _ => return Err("bad obj".to_owned()),
            }
        }
    }
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

fn find_quint_tooling() -> (PathBuf, PathBuf) {
    let root = workspace_root();
    for base in [
        root.join("node_modules/@deterministic-engineering"),
        root.join(".data/marlin-mygo/node_modules/@deterministic-engineering"),
    ] {
        let cli = base.join("quint-modules-resolver/src/cli.ts");
        let models = base.join("quint-models");
        if cli.exists() && models.exists() {
            return (cli, models);
        }
    }
    panic!("quint-modules-resolver not found")
}

fn generate_itf_traces(
    model_path: &Path,
    main_module: &str,
    countermodel_steps: &[&str],
) -> Vec<(String, Vec<JsonVal>)> {
    let _guard = QUINT_MBT_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let (cli, models_dir) = find_quint_tooling();
    let resolve_root = models_dir
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .expect("resolve root");

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let out_dir = env::temp_dir().join(format!("marlin-causal-mbt-{}-{nanos}", std::process::id()));
    fs::create_dir_all(&out_dir).unwrap();

    let audit = Command::new("bun")
        .arg(&cli)
        .arg("mbt-audit")
        .arg(model_path)
        .arg("--out-dir")
        .arg(&out_dir)
        .arg("--min-coverage=0.8")
        .env("QUINT_RESOLVE_ROOT", resolve_root)
        .output()
        .unwrap();
    assert!(
        audit.status.success(),
        "mbt-audit failed:\n{}\n{}",
        String::from_utf8_lossy(&audit.stdout),
        String::from_utf8_lossy(&audit.stderr)
    );

    let mut traces = Vec::new();
    let mut files: Vec<_> = fs::read_dir(&out_dir)
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|s| s.to_str())
                .is_some_and(|n| n.ends_with(".itf.json"))
        })
        .collect();
    files.sort();
    for path in files {
        let raw = fs::read_to_string(&path).unwrap();
        let val = JsonParser::new(&raw).parse_val().unwrap();
        traces.push((
            path.file_name().unwrap().to_string_lossy().into_owned(),
            val.get("states").unwrap().as_arr().unwrap().to_vec(),
        ));
    }

    for step_action in countermodel_steps {
        let cm_file = out_dir.join(format!("cm_{step_action}.itf.json"));
        let out = Command::new("bun")
            .arg(&cli)
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
            .unwrap();
        assert!(
            !out.status.success() && cm_file.exists(),
            "countermodel {step_action} failed to trigger:\n{}\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        let raw = fs::read_to_string(&cm_file).unwrap();
        let val = JsonParser::new(&raw).parse_val().unwrap();
        traces.push((
            format!("cm_{step_action}.itf.json"),
            val.get("states").unwrap().as_arr().unwrap().to_vec(),
        ));
    }

    let _ = fs::remove_dir_all(&out_dir);
    traces
}

fn digest(ch: char) -> GovernanceProofDigest {
    GovernanceProofDigest::new(format!("sha256:{}", std::iter::repeat_n(ch, 64).collect::<String>()))
        .unwrap()
}

fn envelope(admitted: bool, epoch: u64, retracted: Vec<DerivationId>) -> HigherOrderGovernanceProofEnvelope {
    HigherOrderGovernanceProofEnvelope {
        schema_version: HIGHER_ORDER_GOVERNANCE_SCHEMA_VERSION,
        admitted,
        revocation_epoch: HigherOrderProofEpoch::new(epoch).unwrap(),
        policy_revision: 1,
        assessment_digest: digest('a'),
        taint_proof_digest: digest('b'),
        fixpoint_proof_digest: digest('c'),
        temporal_proof_digest: digest('d'),
        invalidation_digest: digest('e'),
        rejection_reasons: if admitted {
            vec![]
        } else {
            vec!["bitemporal-premise-retracted".to_owned()]
        },
        fenced_actors: vec![],
        blocked_tools: vec![],
        retracted_conclusions: retracted,
        cedar_policies: vec![SynthesizedCedarPolicy {
            policy_id: CedarPolicyId::new("marlin.baseline").unwrap(),
            source: "forbid (principal in Marlin::Role::\"Agent\", action, resource);".to_owned(),
        }],
    }
}

#[test]
fn mbt_causal_retraction_overlay_refines_higher_order_admission_gate() {
    let model_path =
        workspace_root().join("dev/quint/threat/MarlinBackendCausalRetractionOverlay.qnt");
    let traces = generate_itf_traces(
        &model_path,
        "marlin_backend_causal_retraction_overlay",
        &[
            "stepExecWhileTripped",
            "stepStaleLeaseAfterTrip",
            "stepAgentSelfRearm",
            "stepBudgetRunaway",
            "stepCommitAfterPremiseRetraction",
            "stepAcceptStaleProofEnvelope",
        ],
    );

    let human = BreakerActor::VerifiedHuman(BreakerActorId::new("ops-human").unwrap());
    let agent = BreakerActor::Agent(BreakerActorId::new("agent-auto").unwrap());
    let actor = GovernedSubjectName::new("worker-alice").unwrap();
    let tool = GovernedSubjectName::new("dispatch-tool").unwrap();
    let reason = BreakerTripReason::new("manual-trip").unwrap();

    let mut countermodels_refused = 0usize;

    for (trace_name, states) in &traces {
        let breaker = Arc::new(
            ExecutionCircuitBreaker::new(ToolBudgetUnits::new(1)).with_single_lease_enforced(true),
        );
        let gate = HigherOrderGovernanceAdmissionGate::new(Arc::clone(&breaker));
        let mut active_lease: Option<(DerivationId, ToolExecutionLease)> = None;
        let mut seq = 0u64;

        for pair in states.windows(2) {
            let prev = pair[0].get("h").unwrap();
            let next = pair[1].get("h").unwrap();
            let b = |o: &JsonVal, k: &str| o.get(k).unwrap().as_bool().unwrap();
            let i = |o: &JsonVal, k: &str| o.get(k).unwrap().as_i64().unwrap();

            let exec_tripped = b(next, "execWhileTripped") && !b(prev, "execWhileTripped");
            let stale_lease = b(next, "staleLeaseCommit") && !b(prev, "staleLeaseCommit");
            let agent_rearm = b(next, "agentSelfRearmed") && !b(prev, "agentSelfRearmed");
            let budget_over = b(next, "budgetOverrun") && !b(prev, "budgetOverrun");
            let retracted_commit =
                b(next, "retractedEffectCommit") && !b(prev, "retractedEffectCommit");
            let stale_proof = b(next, "staleProofAccepted") && !b(prev, "staleProofAccepted");

            if exec_tripped {
                let (did, _) = active_lease.as_ref().unwrap();
                let err = gate
                    .commit_derived_effect(did, &actor, &tool, &ToolIfcContext::clean_trusted())
                    .expect_err("must refuse commit while breaker tripped");
                assert!(matches!(
                    err,
                    HigherOrderBridgeError::ExecutionBreaker(
                        BreakerRejectionReason::BreakerTripped { .. }
                    ) | HigherOrderBridgeError::DerivationRetractedDenied(_)
                ));
                countermodels_refused += 1;
                break;
            }
            if stale_lease {
                let (did, _) = active_lease.as_ref().unwrap();
                let err = gate
                    .commit_derived_effect(did, &actor, &tool, &ToolIfcContext::clean_trusted())
                    .expect_err("must refuse stale pre-trip lease after re-arm");
                assert!(matches!(
                    err,
                    HigherOrderBridgeError::ExecutionBreaker(
                        BreakerRejectionReason::StaleLeaseEpoch { .. }
                    ) | HigherOrderBridgeError::DerivationRetractedDenied(_)
                ));
                countermodels_refused += 1;
                break;
            }
            if agent_rearm {
                let err = breaker
                    .rearm(&agent)
                    .expect_err("must refuse AI agent self-rearm");
                assert!(matches!(
                    err,
                    BreakerRejectionReason::HumanOnlyRearmRequired(_)
                ));
                countermodels_refused += 1;
                break;
            }
            if budget_over {
                let (did, _) = active_lease.as_ref().unwrap();
                let err = gate
                    .commit_derived_effect(did, &actor, &tool, &ToolIfcContext::clean_trusted())
                    .expect_err("must refuse budget overrun");
                assert!(matches!(
                    err,
                    HigherOrderBridgeError::ExecutionBreaker(
                        BreakerRejectionReason::BudgetExhausted { .. }
                    )
                ));
                countermodels_refused += 1;
                break;
            }
            if retracted_commit {
                let (did, _) = active_lease.as_ref().unwrap();
                let err = gate
                    .commit_derived_effect(did, &actor, &tool, &ToolIfcContext::clean_trusted())
                    .expect_err("must refuse committing a retracted causal derivation");
                assert!(matches!(
                    err,
                    HigherOrderBridgeError::DerivationRetractedDenied(_)
                ));
                countermodels_refused += 1;
                break;
            }
            if stale_proof {
                let floor = gate.min_revocation_epoch().unwrap().get();
                assert!(floor >= 2);
                let err = gate
                    .synchronize_proof_envelope(&envelope(true, floor - 1, vec![]))
                    .expect_err("must refuse stale pre-retraction proof envelope");
                assert!(matches!(
                    err,
                    HigherOrderBridgeError::StaleProofEpoch { .. }
                ));
                countermodels_refused += 1;
                break;
            }

            // Safe transitions refined 1:1 against Rust gate & breaker:
            let prev_retracted = b(prev, "premiseRetracted");
            let next_retracted = b(next, "premiseRetracted");
            let prev_tripped = b(prev, "breakerTripped");
            let next_tripped = b(next, "breakerTripped");
            let prev_lease = b(prev, "leaseActive");
            let next_lease = b(next, "leaseActive");
            let prev_budget = i(prev, "budgetUsed");
            let next_budget = i(next, "budgetUsed");

            if !prev_retracted && next_retracted {
                let new_epoch = i(next, "proofEpochFloor") as u64;
                let retracted_ids = active_lease
                    .as_ref()
                    .map(|(did, _)| vec![did.clone()])
                    .unwrap_or_default();
                let sync = gate
                    .synchronize_proof_envelope(&envelope(false, new_epoch, retracted_ids))
                    .unwrap_or_else(|e| panic!("retract sync failed in {trace_name}: {e}"));
                assert!(sync.breaker_auto_tripped);
            } else if prev_retracted && !next_retracted {
                let new_epoch = i(next, "proofEpochFloor") as u64;
                gate.synchronize_proof_envelope(&envelope(true, new_epoch, vec![]))
                    .unwrap_or_else(|e| panic!("reinstate sync failed in {trace_name}: {e}"));
            } else if !prev_lease && next_lease {
                seq += 1;
                let lid = ToolExecutionLeaseId::new(format!("hl-{seq}")).unwrap();
                let did = DerivationId::new(format!("D-EXEC-{seq}")).unwrap();
                let lease = breaker.acquire_lease(lid, "derived-tool").unwrap();
                gate.register_derived_lease(did.clone(), lease.clone()).unwrap();
                active_lease = Some((did, lease));
            } else if !prev_tripped && next_tripped {
                breaker.trip(reason.clone()).unwrap();
            } else if prev_tripped && !next_tripped {
                breaker.rearm(&human).unwrap();
            } else if prev_lease && !next_lease && next_budget == prev_budget + 1 {
                let (did, _) = active_lease.take().unwrap();
                gate.commit_derived_effect(&did, &actor, &tool, &ToolIfcContext::clean_trusted())
                    .unwrap_or_else(|e| panic!("safe derived commit failed in {trace_name}: {e}"));
            } else if prev_lease && !next_lease {
                let (_, lease) = active_lease.take().unwrap();
                let _ = breaker.cancel_stale_lease(&lease.lease_id);
            }
        }
    }

    assert!(
        countermodels_refused >= 6,
        "expected all 6 causal-retraction overlay countermodels refused, got {countermodels_refused}"
    );
}
