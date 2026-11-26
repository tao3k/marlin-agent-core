// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Live Quint -> ITF -> Rust Model-Based Testing (MBT) refinement suite for
//! `marlin_agent_context::shared_scope::SharedContextGovernanceLedger`.
//!
//! Executes `quint-modules-resolver mbt-audit` against:
//! 1. Upstream `@deterministic-engineering/quint-models/.../SharedContextGovernance.qnt`
//! 2. Backend overlay `dev/quint/threat/MarlinBackendSharedContextOverlay.qnt`
//!
//! Zero external JSON/crate dependencies are required so workspace `Cargo.toml`
//! and `Cargo.lock` remain untouched.

use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    path::{Path, PathBuf},
    process::Command,
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

static QUINT_APALACHE_LOCK: Mutex<()> = Mutex::new(());

use marlin_agent_context::shared_scope::{
    ContextAccessHandle, ContextChannelId, ContextEntryId, ContextHandleId, ContextPrincipal,
    ContextPrincipalId, ContextScopeRejection, ContextSensitivity, ContextTenantId,
    SharedContextEntry, SharedContextGovernanceLedger, SharedContextGovernanceService,
    SharedContextScope,
};

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
        self.pos += 1; // '"'
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
                byte => {
                    // Preserve UTF-8 multi-byte sequences cleanly.
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
                    let _ = byte;
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
        self.pos += 1; // '['
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
        self.pos += 1; // '{'
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
        "marlin-context-mbt-{}-{nanos}",
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

    // Generate live Quint countermodel ITF traces for each adversarial step action.
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
        "mbt-audit produced zero ITF trace files for {}",
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
// 1. Live Quint -> ITF -> Rust MBT: Upstream SharedContextGovernance.qnt
// ---------------------------------------------------------------------------

#[test]
fn mbt_upstream_shared_context_governance_refines_rust_ledger() {
    let (_, models_dir) = find_quint_tooling().expect("local quint-models");
    let model_path =
        models_dir.join("threat/zero-trust/shared-context-governance/SharedContextGovernance.qnt");
    let traces = generate_itf_traces(
        &model_path,
        "shared_context_governance",
        &[
            "stepPrivateContextLeak",
            "stepUnauthorizedPromotion",
            "stepRevokedMemberRead",
            "stepMultiPartyTaintHijack",
        ],
    );

    let mut replayed_traces = 0usize;
    let mut countermodels_refused = 0usize;

    let tenant = ContextTenantId::new("tenant-marlin").unwrap();
    let owner_id = ContextPrincipalId::new("alice").unwrap();
    let peer_id = ContextPrincipalId::new("bob").unwrap();
    let channel_id = ContextChannelId::new("ch-shared").unwrap();
    let entry_id = ContextEntryId::new("ctx-secret-1").unwrap();

    let owner = ContextPrincipal::Human {
        id: owner_id.clone(),
        tenant_id: tenant.clone(),
    };
    let peer = ContextPrincipal::Human {
        id: peer_id.clone(),
        tenant_id: tenant.clone(),
    };
    let agent = ContextPrincipal::DelegatedAgent {
        agent_id: ContextPrincipalId::new("agent-bob").unwrap(),
        acting_for: peer_id.clone(),
        tenant_id: tenant.clone(),
    };

    for (trace_name, states) in &traces {
        replayed_traces += 1;
        let mut ledger = SharedContextGovernanceLedger::new();
        ledger.create_channel(
            tenant.clone(),
            channel_id.clone(),
            [owner_id.clone(), peer_id.clone()],
        );
        ledger
            .register_entry(SharedContextEntry {
                entry_id: entry_id.clone(),
                tenant_id: tenant.clone(),
                owner_id: owner_id.clone(),
                channel_id: Some(channel_id.clone()),
                dm_participants: BTreeSet::new(),
                scope: SharedContextScope::Private,
                sensitivity: ContextSensitivity::Confidential,
                declassified: false,
                tainted: false,
                content: "top-secret-design-doc".to_owned(),
            })
            .unwrap();

        let mut peer_handle: Option<ContextAccessHandle> = None;
        let mut handle_seq = 0u64;

        for pair in states.windows(2) {
            let prev = state_h(&pair[0]);
            let next = state_h(&pair[1]);

            let prev_scope = h_i64(prev, "entryScope");
            let next_scope = h_i64(next, "entryScope");
            let prev_handle = h_bool(prev, "peerHandleIssued");
            let next_handle = h_bool(next, "peerHandleIssued");
            let prev_member = h_bool(prev, "peerInChannel");
            let next_member = h_bool(next, "peerInChannel");
            let prev_reads = h_i64(prev, "peerReads");
            let next_reads = h_i64(next, "peerReads");
            let prev_tainted = h_bool(prev, "channelTainted");
            let next_tainted = h_bool(next, "channelTainted");
            let prev_tools = h_i64(prev, "agentToolCalls");
            let next_tools = h_i64(next, "agentToolCalls");

            let private_leak = h_bool(next, "privateLeak") && !h_bool(prev, "privateLeak");
            let unauth_promote =
                h_bool(next, "unauthorizedPromote") && !h_bool(prev, "unauthorizedPromote");
            let revoked_read =
                h_bool(next, "revokedReadLeak") && !h_bool(prev, "revokedReadLeak");
            let taint_hijack =
                h_bool(next, "taintedToolHijack") && !h_bool(prev, "taintedToolHijack");

            if unauth_promote {
                // Countermodel: non-owner OR confidential without explicit declassification MUST fail in Rust.
                let err_non_owner = ledger
                    .promote_scope(&peer, &entry_id, SharedContextScope::Channel, true)
                    .expect_err("Rust ledger must reject non-owner scope promotion");
                assert!(matches!(
                    err_non_owner,
                    ContextScopeRejection::NonOwnerPromotionDenied { .. }
                ));
                let err_no_declass = ledger
                    .promote_scope(&owner, &entry_id, SharedContextScope::Channel, false)
                    .expect_err("Rust ledger must reject confidential promotion without declassification");
                assert!(matches!(
                    err_no_declass,
                    ContextScopeRejection::DeclassificationRequired(_)
                ));
                countermodels_refused += 1;
                break;
            }

            if private_leak {
                // Countermodel: peer reading Private-scope entry MUST fail in Rust.
                let err = ledger
                    .read_entry(&peer, &entry_id, peer_handle.as_ref())
                    .expect_err("Rust ledger must block private-scope leak");
                assert!(matches!(err, ContextScopeRejection::PrivateScopeDenied { .. }));
                countermodels_refused += 1;
                break;
            }

            if revoked_read {
                // Countermodel: revoked member using stale handle MUST fail in Rust.
                let err = ledger
                    .read_entry(&peer, &entry_id, peer_handle.as_ref())
                    .expect_err("Rust ledger must block revoked-member stale handle read");
                assert!(matches!(
                    err,
                    ContextScopeRejection::NotChannelMember { .. }
                        | ContextScopeRejection::StaleHandleEpoch { .. }
                ));
                countermodels_refused += 1;
                break;
            }

            if taint_hijack {
                // Countermodel: agent tool call on tainted channel without human gate MUST fail in Rust.
                let err = ledger
                    .authorize_agent_tool_call(&agent, &channel_id, false)
                    .expect_err("Rust ledger must block tainted channel tool hijack");
                assert!(matches!(
                    err,
                    ContextScopeRejection::TaintedContextRequiresHumanGate(_)
                ));
                countermodels_refused += 1;
                break;
            }

            // Safe state transitions refined 1:1 against Rust ledger:
            if prev_scope == 0 && next_scope == 1 {
                let explicit = h_bool(next, "entryDeclassified");
                let receipt = ledger
                    .promote_scope(&owner, &entry_id, SharedContextScope::Channel, explicit)
                    .unwrap_or_else(|e| panic!("safe promote failed in {trace_name}: {e}"));
                assert_eq!(receipt.new_scope, SharedContextScope::Channel);
            } else if !prev_handle && next_handle {
                handle_seq += 1;
                let hid = ContextHandleId::new(format!("handle-{handle_seq}")).unwrap();
                let handle = ledger
                    .mint_channel_handle(&peer, &channel_id, hid)
                    .unwrap_or_else(|e| panic!("safe mint handle failed in {trace_name}: {e}"));
                assert_eq!(handle.acl_epoch.get() as i64, h_i64(next, "peerHandleEpoch"));
                peer_handle = Some(handle);
            } else if prev_member && !next_member {
                let epoch = ledger
                    .revoke_member(&channel_id, &peer_id)
                    .unwrap_or_else(|e| panic!("safe revoke failed in {trace_name}: {e}"));
                assert_eq!(epoch.get() as i64, h_i64(next, "aclEpoch"));
            } else if !prev_member && next_member {
                let epoch = ledger
                    .rejoin_member(&channel_id, peer_id.clone())
                    .unwrap_or_else(|e| panic!("safe rejoin failed in {trace_name}: {e}"));
                assert_eq!(epoch.get() as i64, h_i64(next, "aclEpoch"));
                if !next_handle {
                    peer_handle = None;
                }
            } else if !prev_tainted && next_tainted {
                ledger.poison_channel_context(&channel_id).unwrap();
            } else if next_reads == prev_reads + 1 {
                let receipt = ledger
                    .read_entry(&peer, &entry_id, peer_handle.as_ref())
                    .unwrap_or_else(|e| panic!("safe peer read failed in {trace_name}: {e}"));
                assert_eq!(receipt.scope, SharedContextScope::Channel);
            } else if next_tools == prev_tools + 1 {
                let human_gate = ledger.is_channel_tainted(&channel_id).unwrap();
                let receipt = ledger
                    .authorize_agent_tool_call(&agent, &channel_id, human_gate)
                    .unwrap_or_else(|e| panic!("safe tool call failed in {trace_name}: {e}"));
                assert_eq!(receipt.tool_calls_in_channel as i64, next_tools);
            }
        }
    }

    assert!(replayed_traces >= 4, "expected at least 4 ITF traces");
    assert!(
        countermodels_refused >= 4,
        "expected all 4 upstream countermodels to be deterministically refused by Rust ledger, got {countermodels_refused}"
    );
}

// ---------------------------------------------------------------------------
// 2. Live Quint -> ITF -> Rust MBT: Backend Overlay (5-Scope + Multi-Tenant + DM + Exfiltration)
// ---------------------------------------------------------------------------

#[test]
fn mbt_marlin_backend_shared_context_overlay_refines_rust_ledger() {
    let model_path =
        workspace_root().join("dev/quint/threat/MarlinBackendSharedContextOverlay.qnt");
    let traces = generate_itf_traces(
        &model_path,
        "marlin_backend_shared_context_overlay",
        &[
            "stepPrivateContextLeak",
            "stepUnauthorizedPromotion",
            "stepRevokedMemberRead",
            "stepMultiPartyTaintHijack",
            "stepCrossTenantReadLeak",
            "stepDmOutsiderReadLeak",
            "stepPromptScopeExfiltration",
        ],
    );

    let tenant_a = ContextTenantId::new("tenant-a").unwrap();
    let tenant_b = ContextTenantId::new("tenant-b").unwrap();
    let alice_id = ContextPrincipalId::new("alice").unwrap();
    let bob_id = ContextPrincipalId::new("bob").unwrap();
    let outsider_id = ContextPrincipalId::new("carol").unwrap();
    let channel_id = ContextChannelId::new("ch-backend").unwrap();
    let entry_id = ContextEntryId::new("ctx-overlay-1").unwrap();

    let alice = ContextPrincipal::Human {
        id: alice_id.clone(),
        tenant_id: tenant_a.clone(),
    };
    let bob = ContextPrincipal::Human {
        id: bob_id.clone(),
        tenant_id: tenant_a.clone(),
    };
    let bob_agent = ContextPrincipal::DelegatedAgent {
        agent_id: ContextPrincipalId::new("agent-bob").unwrap(),
        acting_for: bob_id.clone(),
        tenant_id: tenant_a.clone(),
    };
    let carol_outsider = ContextPrincipal::Human {
        id: outsider_id.clone(),
        tenant_id: tenant_a.clone(),
    };
    let mallory_tenant_b = ContextPrincipal::Human {
        id: alice_id.clone(),
        tenant_id: tenant_b.clone(),
    };

    let mut countermodels_refused = 0usize;

    for (trace_name, states) in &traces {
        let mut ledger = SharedContextGovernanceLedger::new();
        ledger.create_channel(
            tenant_a.clone(),
            channel_id.clone(),
            [alice_id.clone(), bob_id.clone()],
        );
        let dm_participants = BTreeSet::from([bob_id.clone()]);

        ledger
            .register_entry(SharedContextEntry {
                entry_id: entry_id.clone(),
                tenant_id: tenant_a.clone(),
                owner_id: alice_id.clone(),
                channel_id: Some(channel_id.clone()),
                dm_participants,
                scope: SharedContextScope::Private,
                sensitivity: ContextSensitivity::Confidential,
                declassified: false,
                tainted: false,
                content: "backend-governed-context".to_owned(),
            })
            .unwrap();

        let mut bob_handle: Option<ContextAccessHandle> = None;
        let mut handle_seq = 0u64;

        for pair in states.windows(2) {
            let prev = state_h(&pair[0]);
            let next = state_h(&pair[1]);

            let private_leak = h_bool(next, "privateLeak") && !h_bool(prev, "privateLeak");
            let unauth_promote =
                h_bool(next, "unauthorizedPromote") && !h_bool(prev, "unauthorizedPromote");
            let revoked_leak =
                h_bool(next, "revokedReadLeak") && !h_bool(prev, "revokedReadLeak");
            let taint_hijack =
                h_bool(next, "taintedToolHijack") && !h_bool(prev, "taintedToolHijack");
            let cross_tenant_leak =
                h_bool(next, "crossTenantLeak") && !h_bool(prev, "crossTenantLeak");
            let dm_outsider_leak =
                h_bool(next, "dmOutsiderLeak") && !h_bool(prev, "dmOutsiderLeak");
            let exfil_leak =
                h_bool(next, "exfiltrationLeak") && !h_bool(prev, "exfiltrationLeak");

            if private_leak {
                let err = ledger
                    .read_entry(&bob, &entry_id, bob_handle.as_ref())
                    .expect_err("overlay must refuse private-scope leak");
                assert!(matches!(err, ContextScopeRejection::PrivateScopeDenied { .. }));
                countermodels_refused += 1;
                break;
            }
            if unauth_promote {
                let err = ledger
                    .promote_scope(&bob, &entry_id, SharedContextScope::Channel, true)
                    .expect_err("overlay must refuse unauthorized promotion");
                assert!(matches!(
                    err,
                    ContextScopeRejection::NonOwnerPromotionDenied { .. }
                ));
                countermodels_refused += 1;
                break;
            }
            if revoked_leak {
                let err = ledger
                    .read_entry(&bob, &entry_id, bob_handle.as_ref())
                    .expect_err("overlay must refuse revoked channel handle");
                assert!(matches!(
                    err,
                    ContextScopeRejection::NotChannelMember { .. }
                        | ContextScopeRejection::StaleHandleEpoch { .. }
                ));
                countermodels_refused += 1;
                break;
            }
            if taint_hijack {
                let err = ledger
                    .authorize_agent_tool_call(&bob_agent, &channel_id, false)
                    .expect_err("overlay must block tainted tool call without human gate");
                assert!(matches!(
                    err,
                    ContextScopeRejection::TaintedContextRequiresHumanGate(_)
                ));
                countermodels_refused += 1;
                break;
            }
            if cross_tenant_leak {
                let err = ledger
                    .read_entry(&mallory_tenant_b, &entry_id, None)
                    .expect_err("overlay must refuse cross-tenant read");
                assert!(matches!(err, ContextScopeRejection::CrossTenantDenied { .. }));
                countermodels_refused += 1;
                break;
            }
            if dm_outsider_leak {
                let err = ledger
                    .read_entry(&carol_outsider, &entry_id, None)
                    .expect_err("overlay must refuse DM outsider read");
                assert!(matches!(
                    err,
                    ContextScopeRejection::DirectMessageParticipantDenied { .. }
                ));
                countermodels_refused += 1;
                break;
            }
            if exfil_leak {
                let err = ledger
                    .verify_scope_flow(
                        &alice,
                        &entry_id,
                        SharedContextScope::Channel,
                        bob_handle.as_ref(),
                    )
                    .expect_err("overlay must block Private/DM exfiltration into Channel prompt");
                assert!(matches!(
                    err,
                    ContextScopeRejection::PromptExfiltrationBlocked { .. }
                ));
                countermodels_refused += 1;
                break;
            }

            // Safe transitions (base + overlay):
            if !h_bool(prev, "dmShared") && h_bool(next, "dmShared") {
                let explicit = h_bool(next, "entryDeclassified");
                ledger
                    .promote_scope(&alice, &entry_id, SharedContextScope::DirectMessage, explicit)
                    .unwrap_or_else(|e| panic!("overlay DM promote failed in {trace_name}: {e}"));
            } else if h_i64(prev, "entryScope") == 0 && h_i64(next, "entryScope") == 1 {
                let explicit = h_bool(next, "entryDeclassified");
                ledger
                    .promote_scope(&alice, &entry_id, SharedContextScope::Channel, explicit)
                    .unwrap_or_else(|e| panic!("overlay Channel promote failed in {trace_name}: {e}"));
            } else if !h_bool(prev, "peerHandleIssued") && h_bool(next, "peerHandleIssued") {
                handle_seq += 1;
                let hid = ContextHandleId::new(format!("ov-handle-{handle_seq}")).unwrap();
                bob_handle = Some(ledger.mint_channel_handle(&bob, &channel_id, hid).unwrap());
            } else if h_bool(prev, "peerInChannel") && !h_bool(next, "peerInChannel") {
                ledger.revoke_member(&channel_id, &bob_id).unwrap();
            } else if !h_bool(prev, "peerInChannel") && h_bool(next, "peerInChannel") {
                ledger.rejoin_member(&channel_id, bob_id.clone()).unwrap();
                if !h_bool(next, "peerHandleIssued") {
                    bob_handle = None;
                }
            } else if !h_bool(prev, "channelTainted") && h_bool(next, "channelTainted") {
                ledger.poison_channel_context(&channel_id).unwrap();
            } else if h_i64(next, "peerReads") == h_i64(prev, "peerReads") + 1 {
                ledger
                    .read_entry(&bob, &entry_id, bob_handle.as_ref())
                    .unwrap_or_else(|e| panic!("overlay valid read failed in {trace_name}: {e}"));
            } else if h_i64(next, "agentToolCalls") == h_i64(prev, "agentToolCalls") + 1 {
                let human_gate = ledger.is_channel_tainted(&channel_id).unwrap();
                ledger
                    .authorize_agent_tool_call(&bob_agent, &channel_id, human_gate)
                    .unwrap();
            }
        }
    }

    assert!(
        countermodels_refused >= 7,
        "expected all 7 composed overlay countermodels (4 inherited + 3 backend) refused, got {countermodels_refused}"
    );
}

// ---------------------------------------------------------------------------
// 3. Authoritative Rust 5-Scope & Anti-Exfiltration Service Unit Verification
// ---------------------------------------------------------------------------

#[test]
fn five_scope_bundle_redacts_narrower_entries_and_forbids_agent_promotion() {
    let tenant = ContextTenantId::new("tenant-prod").unwrap();
    let alice_id = ContextPrincipalId::new("alice").unwrap();
    let bob_id = ContextPrincipalId::new("bob").unwrap();
    let channel_id = ContextChannelId::new("ch-eng").unwrap();

    let alice = ContextPrincipal::Human {
        id: alice_id.clone(),
        tenant_id: tenant.clone(),
    };
    let alice_agent = ContextPrincipal::DelegatedAgent {
        agent_id: ContextPrincipalId::new("marlin-agent-1").unwrap(),
        acting_for: alice_id.clone(),
        tenant_id: tenant.clone(),
    };

    let service = SharedContextGovernanceService::new(SharedContextGovernanceLedger::new());
    let priv_id = ContextEntryId::new("e-private").unwrap();
    let dm_id = ContextEntryId::new("e-dm").unwrap();
    let chan_id = ContextEntryId::new("e-channel").unwrap();
    let ws_id = ContextEntryId::new("e-workspace").unwrap();

    service
        .write_with(|ledger| {
            ledger.create_channel(
                tenant.clone(),
                channel_id.clone(),
                [alice_id.clone(), bob_id.clone()],
            );
            ledger.register_entry(SharedContextEntry {
                entry_id: priv_id.clone(),
                tenant_id: tenant.clone(),
                owner_id: alice_id.clone(),
                channel_id: Some(channel_id.clone()),
                dm_participants: BTreeSet::new(),
                scope: SharedContextScope::Private,
                sensitivity: ContextSensitivity::Confidential,
                declassified: false,
                tainted: false,
                content: "alice-ssh-key-notes".to_owned(),
            })?;
            ledger.register_entry(SharedContextEntry {
                entry_id: dm_id.clone(),
                tenant_id: tenant.clone(),
                owner_id: alice_id.clone(),
                channel_id: Some(channel_id.clone()),
                dm_participants: BTreeSet::from([bob_id.clone()]),
                scope: SharedContextScope::DirectMessage,
                sensitivity: ContextSensitivity::Internal,
                declassified: false,
                tainted: false,
                content: "alice-bob-1on1".to_owned(),
            })?;
            ledger.register_entry(SharedContextEntry {
                entry_id: chan_id.clone(),
                tenant_id: tenant.clone(),
                owner_id: alice_id.clone(),
                channel_id: Some(channel_id.clone()),
                dm_participants: BTreeSet::new(),
                scope: SharedContextScope::Channel,
                sensitivity: ContextSensitivity::Internal,
                declassified: true,
                tainted: false,
                content: "channel-sprint-notes".to_owned(),
            })?;
            ledger.register_entry(SharedContextEntry {
                entry_id: ws_id.clone(),
                tenant_id: tenant.clone(),
                owner_id: alice_id.clone(),
                channel_id: None,
                dm_participants: BTreeSet::new(),
                scope: SharedContextScope::Workspace,
                sensitivity: ContextSensitivity::Public,
                declassified: true,
                tainted: false,
                content: "workspace-handbook".to_owned(),
            })
        })
        .unwrap();

    // Delegated AI agent is strictly forbidden from promoting Private -> Channel.
    let agent_err = service
        .write_with(|ledger| {
            ledger.promote_scope(&alice_agent, &priv_id, SharedContextScope::Channel, true)
        })
        .expect_err("agent must not promote context scope");
    assert!(matches!(
        agent_err,
        ContextScopeRejection::AgentPrivilegeDenied { .. }
    ));

    // Assembling a Channel-scoped prompt bundle must redact Private and DirectMessage entries!
    let bundle = service
        .read_with(|ledger| {
            let handle = ledger.mint_channel_handle(
                &alice,
                &channel_id,
                ContextHandleId::new("alice-ch-h1")?,
            )?;
            Ok(ledger.assemble_prompt_bundle(
                &alice,
                SharedContextScope::Channel,
                &[priv_id.clone(), dm_id.clone(), chan_id.clone(), ws_id.clone()],
                Some(&handle),
            ))
        })
        .unwrap();

    assert_eq!(bundle.included_entry_ids, vec![chan_id, ws_id]);
    assert_eq!(bundle.redacted_entry_ids, vec![priv_id, dm_id]);
    assert!(!bundle.tainted);
}
