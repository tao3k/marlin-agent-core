// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Live Quint -> ITF -> Rust Model-Based Testing (MBT) refinement suite for
//! tamper-evident durable `GovernanceAdmissionGate` checkpoints against
//! `dev/quint/threat/MarlinBackendDurableGovernanceOverlay.qnt`.
//!
//! Proves all 7 composed countermodels (4 inherited `AgentToolCircuitBreaker` +
//! 3 anti-restart-reset & tamper-fail-closed countermodels: `stepRestartClearsEStop`,
//! `stepRestartRegressesEpoch`, `stepCorruptWalFailsOpen`) are deterministically
//! refused by real Rust `GovernanceAdmissionGate` + `ExecutionCircuitBreaker`.

use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use marlin_agent_kernel::{
    BreakerActor, BreakerActorId, BreakerRejectionReason, BreakerTripReason, CedarContextValue,
    CedarEntityRef, CedarEvaluationDecision, CedarEvaluationRequest, CedarPolicyId, DerivationId,
    ExecutionCircuitBreaker, GOVERNANCE_ADMISSION_SCHEMA_VERSION, GovernanceAdmissionError,
    GovernanceAdmissionGate, GovernanceProofDigest, GovernanceProofEnvelope, GovernanceProofEpoch,
    GovernedSubjectName, SynthesizedCedarPolicy, ToolBudgetUnits, ToolExecutionLease,
    ToolExecutionLeaseId, ToolIfcContext,
};

static DURABLE_QUINT_LOCK: Mutex<()> = Mutex::new(());

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
            self.pos += 1;
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
    let _guard = DURABLE_QUINT_LOCK.lock().unwrap_or_else(|p| p.into_inner());
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
    let out_dir =
        env::temp_dir().join(format!("marlin-durable-mbt-{}-{nanos}", std::process::id()));
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
    GovernanceProofDigest::new(format!(
        "sha256:{}",
        std::iter::repeat_n(ch, 64).collect::<String>()
    ))
    .unwrap()
}

fn incident_envelope(epoch: u64) -> GovernanceProofEnvelope {
    GovernanceProofEnvelope {
        schema_version: GOVERNANCE_ADMISSION_SCHEMA_VERSION,
        admitted: false,
        revocation_epoch: GovernanceProofEpoch::new(epoch).unwrap(),
        policy_revision: epoch,
        assessment_digest: digest('a'),
        taint_proof_digest: digest('b'),
        fixpoint_proof_digest: digest('c'),
        temporal_proof_digest: digest('d'),
        invalidation_digest: digest('e'),
        rejection_reasons: vec!["ascent-camel-taint-alert".to_owned()],
        fenced_actors: vec![GovernedSubjectName::new("agent-sub").unwrap()],
        blocked_tools: vec![GovernedSubjectName::new("os-shell").unwrap()],
        retracted_conclusions: vec![DerivationId::new("D-TOOL-EXEC").unwrap()],
        cedar_policies: vec![
            SynthesizedCedarPolicy {
                policy_id: CedarPolicyId::new("marlin.permit.verified-human-governance").unwrap(),
                source: "permit (principal in Marlin::Role::\"VerifiedHuman\", action in [Marlin::Action::\"rearm_breaker\"], resource) when { context.step_up_fresh == true };".to_owned(),
            },
            SynthesizedCedarPolicy {
                policy_id: CedarPolicyId::new("marlin.ascent.forbid.fenced.agent-sub").unwrap(),
                source: "forbid(principal == Marlin::Actor::\"agent-sub\", action, resource);"
                    .to_owned(),
            },
            SynthesizedCedarPolicy {
                policy_id: CedarPolicyId::new(
                    "marlin.ascent.forbid.scope.external-unless-declassified",
                )
                .unwrap(),
                source: "forbid (principal, action, resource in Marlin::Scope::\"external\") unless { context.declassified == true };".to_owned(),
            },
            SynthesizedCedarPolicy {
                policy_id: CedarPolicyId::new("marlin.permit.trusted-channel-export").unwrap(),
                source: "permit (principal in Marlin::Role::\"VerifiedHuman\", action == Marlin::Action::\"export\", resource) when { context.step_up_fresh && context.channel_scope != \"untrusted\" };".to_owned(),
            },
        ],
    }
}

#[test]
fn mbt_durable_governance_overlay_refines_tamper_evident_checkpoint_recovery() {
    let model_path =
        workspace_root().join("dev/quint/threat/MarlinBackendDurableGovernanceOverlay.qnt");
    let traces = generate_itf_traces(
        &model_path,
        "marlin_backend_durable_governance_overlay",
        &[
            "stepExecWhileTripped",
            "stepStaleLeaseAfterTrip",
            "stepAgentSelfRearm",
            "stepBudgetRunaway",
            "stepRestartClearsEStop",
            "stepRestartRegressesEpoch",
            "stepCorruptWalFailsOpen",
            "stepCorruptFactGraphFailsOpen",
            "stepCedarForbidBypassedByPermit",
            "stepRetractedDerivationCommits",
            "stepScopeUnlessBypassSiblingExfil",
            "stepMissingContextKeyBypassesCondition",
        ],
    );

    let human = BreakerActor::VerifiedHuman(BreakerActorId::new("ops-human").unwrap());
    let agent = BreakerActor::Agent(BreakerActorId::new("agent-auto").unwrap());
    let clean_actor = GovernedSubjectName::new("ops-bob").unwrap();
    let fenced_actor = GovernedSubjectName::new("agent-sub").unwrap();
    let tool = GovernedSubjectName::new("dispatch-tool").unwrap();
    let reason = BreakerTripReason::new("manual-trip").unwrap();

    let mut countermodels_refused = 0usize;

    for (trace_name, states) in &traces {
        let mut breaker = Arc::new(
            ExecutionCircuitBreaker::new(ToolBudgetUnits::new(1)).with_single_lease_enforced(true),
        );
        let mut gate = GovernanceAdmissionGate::new(Arc::clone(&breaker));
        let mut active_lease: Option<(DerivationId, ToolExecutionLease)> = None;
        let mut wal_bytes: Option<Vec<u8>> = None;
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
            let restart_cleared = b(next, "restartClearedTrip") && !b(prev, "restartClearedTrip");
            let restart_regressed =
                b(next, "restartEpochRegressed") && !b(prev, "restartEpochRegressed");
            let corrupt_opened = b(next, "corruptWalOpened") && !b(prev, "corruptWalOpened");
            let corrupt_fact_opened =
                b(next, "corruptFactGraphOpened") && !b(prev, "corruptFactGraphOpened");
            let cedar_forbid_bypassed =
                b(next, "cedarForbidBypassed") && !b(prev, "cedarForbidBypassed");
            let retracted_deriv_committed =
                b(next, "retractedDerivationCommitted") && !b(prev, "retractedDerivationCommitted");
            let scope_unless_bypassed =
                b(next, "scopeUnlessBypassed") && !b(prev, "scopeUnlessBypassed");
            let missing_context_bypassed =
                b(next, "missingContextBypassed") && !b(prev, "missingContextBypassed");

            if exec_tripped {
                let (did, _) = active_lease.as_ref().unwrap();
                let err = gate
                    .commit_derived_effect(
                        did,
                        &clean_actor,
                        &tool,
                        &ToolIfcContext::clean_trusted(),
                    )
                    .expect_err("must refuse commit while breaker tripped");
                assert!(matches!(
                    err,
                    GovernanceAdmissionError::ExecutionBreaker(
                        BreakerRejectionReason::BreakerTripped { .. }
                    )
                ));
                countermodels_refused += 1;
                break;
            }
            if stale_lease {
                let (did, _) = active_lease.as_ref().unwrap();
                let err = gate
                    .commit_derived_effect(
                        did,
                        &clean_actor,
                        &tool,
                        &ToolIfcContext::clean_trusted(),
                    )
                    .expect_err("must refuse stale pre-trip lease");
                assert!(matches!(
                    err,
                    GovernanceAdmissionError::ExecutionBreaker(
                        BreakerRejectionReason::StaleLeaseEpoch { .. }
                    )
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
                    .commit_derived_effect(
                        did,
                        &clean_actor,
                        &tool,
                        &ToolIfcContext::clean_trusted(),
                    )
                    .expect_err("must refuse budget overrun");
                assert!(matches!(
                    err,
                    GovernanceAdmissionError::ExecutionBreaker(
                        BreakerRejectionReason::BudgetExhausted { .. }
                    )
                ));
                countermodels_refused += 1;
                break;
            }
            if restart_cleared {
                // Real Rust post-restart recovery MUST keep breakerTripped == true!
                let bytes = wal_bytes.as_ref().expect("WAL checkpoint committed");
                let fresh_b = Arc::new(
                    ExecutionCircuitBreaker::new(ToolBudgetUnits::new(1))
                        .with_single_lease_enforced(true),
                );
                let fresh_g = GovernanceAdmissionGate::new(fresh_b);
                let restored = fresh_g.restore_durable_checkpoint_bytes(bytes).unwrap();
                assert!(
                    restored.breaker.tripped,
                    "restart-reset bypass refused: restored breaker MUST stay tripped"
                );
                assert!(
                    fresh_g
                        .authorize_side_effect(
                            None,
                            &clean_actor,
                            &tool,
                            &ToolIfcContext::clean_trusted(),
                        )
                        .is_err()
                );
                countermodels_refused += 1;
                break;
            }
            if restart_regressed {
                // Real Rust post-restart recovery MUST preserve monotonic trip_epoch & revocation_epoch!
                let bytes = wal_bytes.as_ref().expect("WAL checkpoint committed");
                let fresh_b = Arc::new(
                    ExecutionCircuitBreaker::new(ToolBudgetUnits::new(1))
                        .with_single_lease_enforced(true),
                );
                let fresh_g = GovernanceAdmissionGate::new(fresh_b);
                let restored = fresh_g.restore_durable_checkpoint_bytes(bytes).unwrap();
                let expected_trip = i(prev, "walTripEpoch") as u64;
                let expected_proof = i(prev, "walProofEpoch") as u64;
                assert!(
                    restored.breaker.trip_epoch.get() >= expected_trip
                        && restored.revocation_epoch.get() >= expected_proof,
                    "epoch regression refused: restored ({}, {}) < persisted ({}, {})",
                    restored.breaker.trip_epoch.get(),
                    restored.revocation_epoch.get(),
                    expected_trip,
                    expected_proof
                );
                if expected_proof > 1 {
                    assert!(matches!(
                        fresh_g.synchronize_proof_envelope(&incident_envelope(1)),
                        Err(GovernanceAdmissionError::StaleProofEpoch { .. })
                    ));
                }
                countermodels_refused += 1;
                break;
            }
            if corrupt_opened {
                // Real Rust recovery on tampered WAL MUST return CorruptDurableCheckpoint AND fail closed!
                let bytes = wal_bytes.as_ref().expect("tampered WAL bytes");
                let fresh_b = Arc::new(
                    ExecutionCircuitBreaker::new(ToolBudgetUnits::new(1))
                        .with_single_lease_enforced(true),
                );
                let fresh_g = GovernanceAdmissionGate::new(fresh_b);
                let err = fresh_g
                    .restore_durable_checkpoint_bytes(bytes)
                    .expect_err("corrupt WAL must be rejected");
                assert!(matches!(
                    err,
                    GovernanceAdmissionError::CorruptDurableCheckpoint(_)
                ));
                assert!(
                    fresh_g.snapshot().unwrap().breaker.tripped,
                    "corrupt WAL recovery MUST fail closed (breaker tripped)"
                );
                countermodels_refused += 1;
                break;
            }
            if corrupt_fact_opened {
                // Real Rust recovery on tampered companion MGF1 fact graph MUST fail closed!
                let fresh_b = Arc::new(
                    ExecutionCircuitBreaker::new(ToolBudgetUnits::new(1))
                        .with_single_lease_enforced(true),
                );
                let fresh_g = GovernanceAdmissionGate::new(fresh_b);
                let tripped = fresh_g
                    .trip_corrupt_durable_state("fact-graph:SHA-256 mismatch")
                    .expect("fail-closed trip on corrupt MGF1");
                assert!(tripped.breaker.tripped && !tripped.admitted);
                assert!(
                    fresh_g
                        .authorize_side_effect(
                            None,
                            &clean_actor,
                            &tool,
                            &ToolIfcContext::clean_trusted(),
                        )
                        .is_err()
                );
                countermodels_refused += 1;
                break;
            }
            if cedar_forbid_bypassed {
                // Real compiled Cedar evaluator MUST enforce forbid-overrides-permit even when
                // principal holds VerifiedHuman + fresh step-up matching permit!
                let req = CedarEvaluationRequest {
                    principal: CedarEntityRef::actor("agent-sub"),
                    principal_roles: vec![CedarEntityRef::role("VerifiedHuman")],
                    action: CedarEntityRef::action("rearm_breaker"),
                    resource: CedarEntityRef::resource("workspace"),
                    resource_groups: Vec::new(),
                    context: BTreeMap::from([(
                        "step_up_fresh".to_owned(),
                        CedarContextValue::Bool(true),
                    )]),
                };
                let decision = gate.evaluate_cedar_request(&req).unwrap();
                assert!(
                    matches!(decision, CedarEvaluationDecision::Deny { .. }),
                    "Cedar forbid MUST override VerifiedHuman permit, got {decision:?}"
                );
                countermodels_refused += 1;
                break;
            }
            if retracted_deriv_committed {
                // Real Rust admission gate MUST deny any effect bound to a Bi-Temporal TM retracted derivation!
                let retracted_id = DerivationId::new("D-TOOL-EXEC").unwrap();
                let err = gate
                    .authorize_side_effect(
                        Some(&retracted_id),
                        &clean_actor,
                        &tool,
                        &ToolIfcContext::clean_trusted(),
                    )
                    .expect_err("retracted dynamic derivation must be denied");
                assert!(matches!(
                    err,
                    GovernanceAdmissionError::DerivationRetractedDenied(_)
                ));
                countermodels_refused += 1;
                break;
            }
            if scope_unless_bypassed {
                // Real Rust compiled Cedar engine MUST deny un-declassified sibling external-scope resource
                // via `forbid (... resource in Marlin::Scope::"external") unless { context.declassified == true }`!
                let req = CedarEvaluationRequest {
                    principal: CedarEntityRef::actor("ops-alice"),
                    principal_roles: vec![CedarEntityRef::role("VerifiedHuman")],
                    action: CedarEntityRef::action("rearm_breaker"),
                    resource: CedarEntityRef::resource("ext-sibling-mirror"),
                    resource_groups: vec![CedarEntityRef::scope("external")],
                    context: BTreeMap::from([
                        ("step_up_fresh".to_owned(), CedarContextValue::Bool(true)),
                        ("declassified".to_owned(), CedarContextValue::Bool(false)),
                    ]),
                };
                assert_eq!(
                    gate.evaluate_cedar_request(&req).unwrap(),
                    CedarEvaluationDecision::Deny {
                        matching_forbids: vec![
                            CedarPolicyId::new(
                                "marlin.ascent.forbid.scope.external-unless-declassified"
                            )
                            .unwrap()
                        ],
                    }
                );
                countermodels_refused += 1;
                break;
            }
            if missing_context_bypassed {
                // Real Rust compiled Cedar engine MUST fail closed when context keys are omitted:
                // 1) Missing `declassified` on external scope MUST keep `forbid ... unless` active (Deny),
                //    even when `channel_scope == "workspace"` satisfies the `permit` clause (SM29)!
                let missing_forbid_key_req = CedarEvaluationRequest {
                    principal: CedarEntityRef::actor("ops-alice"),
                    principal_roles: vec![CedarEntityRef::role("VerifiedHuman")],
                    action: CedarEntityRef::action("export"),
                    resource: CedarEntityRef::resource("ext-sibling-mirror"),
                    resource_groups: vec![CedarEntityRef::scope("external")],
                    context: BTreeMap::from([
                        ("step_up_fresh".to_owned(), CedarContextValue::Bool(true)),
                        (
                            "channel_scope".to_owned(),
                            CedarContextValue::Str("workspace".to_owned()),
                        ),
                    ]),
                };
                assert_eq!(
                    gate.evaluate_cedar_request(&missing_forbid_key_req).unwrap(),
                    CedarEvaluationDecision::Deny {
                        matching_forbids: vec![
                            CedarPolicyId::new(
                                "marlin.ascent.forbid.scope.external-unless-declassified"
                            )
                            .unwrap()
                        ],
                    }
                );
                // 2) Missing `channel_scope` required by `when { ... && context.channel_scope != "untrusted" }`
                //    (with `declassified == true`) MUST fail closed as `ImplicitDeny`!
                let missing_when_key_req = CedarEvaluationRequest {
                    principal: CedarEntityRef::actor("ops-alice"),
                    principal_roles: vec![CedarEntityRef::role("VerifiedHuman")],
                    action: CedarEntityRef::action("export"),
                    resource: CedarEntityRef::resource("ext-sibling-mirror"),
                    resource_groups: vec![CedarEntityRef::scope("external")],
                    context: BTreeMap::from([
                        ("step_up_fresh".to_owned(), CedarContextValue::Bool(true)),
                        ("declassified".to_owned(), CedarContextValue::Bool(true)),
                    ]),
                };
                assert_eq!(
                    gate.evaluate_cedar_request(&missing_when_key_req).unwrap(),
                    CedarEvaluationDecision::ImplicitDeny
                );
                countermodels_refused += 1;
                break;
            }

            // Safe transitions refined 1:1 against Rust durable gate & breaker:
            let prev_corrupt = b(prev, "walCorrupted") || b(prev, "factGraphCorrupted");
            let next_corrupt = b(next, "walCorrupted") || b(next, "factGraphCorrupted");
            let prev_proof = i(prev, "liveProofEpoch");
            let next_proof = i(next, "liveProofEpoch");
            let prev_wal = b(prev, "walCommitted");
            let next_wal = b(next, "walCommitted");
            let prev_tripped = b(prev, "breakerTripped");
            let next_tripped = b(next, "breakerTripped");
            let prev_lease = b(prev, "leaseActive");
            let next_lease = b(next, "leaseActive");
            let prev_budget = i(prev, "budgetUsed");
            let next_budget = i(next, "budgetUsed");

            if !prev_corrupt && next_corrupt {
                if let Some(buf) = wal_bytes.as_mut()
                    && buf.len() > 10
                {
                    buf[10] ^= 0xFF;
                }
            } else if next_proof > prev_proof {
                gate.synchronize_proof_envelope(&incident_envelope(next_proof as u64))
                    .unwrap_or_else(|e| panic!("incident sync failed in {trace_name}: {e}"));
                wal_bytes = Some(gate.export_durable_checkpoint_bytes().unwrap());
            } else if next_wal
                && (!prev_wal
                    || i(next, "walTripEpoch") != i(prev, "walTripEpoch")
                    || b(next, "walTripped") != b(prev, "walTripped"))
            {
                wal_bytes = Some(gate.export_durable_checkpoint_bytes().unwrap());
            } else if !prev_lease && next_lease {
                seq += 1;
                let lid = ToolExecutionLeaseId::new(format!("dl-{seq}")).unwrap();
                let did = DerivationId::new(format!("D-DUR-{seq}")).unwrap();
                let lease = breaker.acquire_lease(lid, "derived-tool").unwrap();
                gate.register_derived_lease(did.clone(), lease.clone())
                    .unwrap();
                active_lease = Some((did, lease));
            } else if !prev_tripped && next_tripped && !next_corrupt {
                breaker.trip(reason.clone()).unwrap();
            } else if prev_tripped && !next_tripped {
                breaker.rearm(&human).unwrap();
            } else if prev_lease && !next_lease && next_budget == prev_budget + 1 {
                let (did, _) = active_lease.take().unwrap();
                gate.commit_derived_effect(
                    &did,
                    &clean_actor,
                    &tool,
                    &ToolIfcContext::clean_trusted(),
                )
                .unwrap_or_else(|e| panic!("safe commit failed in {trace_name}: {e}"));
            } else if prev_lease && !next_lease {
                let (_, lease) = active_lease.take().unwrap();
                let _ = breaker.cancel_stale_lease(&lease.lease_id);
            } else if prev_corrupt && next_tripped && !prev_tripped {
                // Safe restart on corrupted MGV1 or MGF1 WAL -> reboot fresh Rust instances and verify fail-closed!
                breaker = Arc::new(
                    ExecutionCircuitBreaker::new(ToolBudgetUnits::new(1))
                        .with_single_lease_enforced(true),
                );
                gate = GovernanceAdmissionGate::new(Arc::clone(&breaker));
                active_lease = None;
                let bytes = wal_bytes.as_ref().unwrap();
                assert!(matches!(
                    gate.restore_durable_checkpoint_bytes(bytes),
                    Err(GovernanceAdmissionError::CorruptDurableCheckpoint(_))
                ));
                assert!(gate.snapshot().unwrap().breaker.tripped);
            } else if b(prev, "liveFenced") && !b(next, "liveFenced") {
                unreachable!("ASCENT fences never silently clear on restart");
            } else if b(next, "liveFenced") && !next_corrupt {
                assert!(gate.is_actor_fenced(&fenced_actor));
            } else if next_corrupt && b(next, "liveFenced") {
                // Corrupt WAL recovery fails closed via tripped breaker, blocking fenced & clean actors alike.
                let bytes = wal_bytes.as_ref().unwrap();
                let _ = gate.restore_durable_checkpoint_bytes(bytes);
                assert!(gate.snapshot().unwrap().breaker.tripped);
                assert!(
                    gate.authorize_side_effect(
                        None,
                        &fenced_actor,
                        &tool,
                        &ToolIfcContext::clean_trusted(),
                    )
                    .is_err()
                );
            }
        }
    }

    assert!(
        countermodels_refused >= 12,
        "expected all 12 durable governance countermodels refused, got {countermodels_refused}"
    );
}
