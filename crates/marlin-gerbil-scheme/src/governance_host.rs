// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Authoritative Governance Authority Host Controller for `marlin-scheme-host`.
//!
//! Bridges the live embedded Gambit/Gerbil Scheme VM (`:marlin-agent-core/policy/governance`)
//! through zero-JSON 14-slot rooted vectors into `marlin_agent_kernel::GovernanceAdmissionGate`
//! and `ExecutionCircuitBreaker`, enforces side-effect gating on `SchemeAtomicEffect::SandboxRun`,
//! publishes authoritative snapshots over `ProjectionQueryService` (`QueryProjectGraph` and
//! `WatchGraphTrajectory`), and enforces one-shot RFC 9470 human Step-Up Re-Arm strictly through
//! `ApprovalGateService` (never forgeable via `DispatchUiEvent`).

use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

use marlin_agent_app_server::{
    StorageCasStore,
    ui_grpc::MyGoBridgeHost,
    wire::marlin::{
        gate::v1::{GateKind, GraphHumanDecision, PendingGate},
        projection::v1::{GraphRecordView, GraphTrajectoryEvent, GraphVisibleSurface},
    },
};
use marlin_agent_kernel::{
    BreakerActor, BreakerActorId, CedarContextValue, CedarEntityRef, CedarEvaluationDecision,
    CedarEvaluationRequest, CedarPolicyId, DerivationId, ExecutionCircuitBreaker,
    GovernanceAdmissionError, GovernanceAdmissionGate, GovernanceGateSnapshot,
    GovernanceProofDigest, GovernanceProofEnvelope, GovernanceProofEpoch, GovernedSubjectName,
    SynthesizedCedarPolicy, ToolBudgetUnits, ToolIfcContext, governance_sha256_bytes,
    infer_actor_roles,
};
use marlin_agent_storage::AgentStorage;

use crate::{
    GovernanceDerivationSpec, GovernancePolicyVectorProjection, GovernancePolicyVectorRequest,
};

pub const GOVERNANCE_AUTHORITY_RECORD_ID: &str = "scheme:governance:authority-snapshot";
pub const GOVERNANCE_AUTHORITY_RECORD_KIND: &str = "scheme.governance.authority_snapshot";
pub const GOVERNANCE_AUTHORITY_RUN_ID: &str = "scheme-governance-authority";
pub const GOVERNANCE_AUTHORITY_NODE_KIND: &str = "scheme_governance_authority";
pub const GOVERNANCE_REARM_GATE_PREFIX: &str = "gov-rearm-epoch-";
pub const DEFAULT_GOVERNANCE_CHECKPOINT_KEY: &str = "checkpoint:governance-authority:v1";
pub const GOVERNANCE_FACT_STORE_MAGIC: &[u8; 4] = b"MGF1";
pub const GOVERNANCE_FACT_STORE_VERSION: u32 = 2;
/// Maximum entries per `GovernanceFactStore` collection matching `MGF1` deserialization cap (SM30).
pub const MAX_FACT_STORE_ENTRIES: usize = 4096;

fn constant_time_eq_32(lhs: &[u8], rhs: &[u8; 32]) -> bool {
    if lhs.len() != 32 {
        return false;
    }
    let mut diff = 0u8;
    for i in 0..32 {
        diff |= std::hint::black_box(lhs[i] ^ rhs[i]);
    }
    std::hint::black_box(diff) == 0
}

fn ensure_fact_capacity(
    current_len: usize,
    cap: usize,
    already_present: bool,
    label: &str,
) -> Result<(), String> {
    if !already_present && current_len >= cap {
        return Err(format!(
            "governance fact-graph capacity limit reached for {label}: {current_len} >= {cap}"
        ));
    }
    Ok(())
}

/// Companion Turso CAS key storing the tamper-evident `MGF1` runtime fact-graph checkpoint.
pub fn fact_store_checkpoint_key(checkpoint_key: &str) -> String {
    format!("{checkpoint_key}:facts")
}

static GOVERNANCE_TRAJECTORY_SEQ: AtomicU64 = AtomicU64::new(1);

/// Shared registry merging multiple non-authorizing `GraphRecordView` records
/// (e.g. `scheme:fleet:audit-capsule` + `scheme:governance:authority-snapshot`)
/// before publishing to `MyGoBridgeHost::set_graph_projection_records`.
#[derive(Default)]
pub struct SharedProjectionRecords {
    records: Mutex<BTreeMap<String, GraphRecordView>>,
}

impl SharedProjectionRecords {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn upsert_and_publish(&self, host: &MyGoBridgeHost, record: GraphRecordView) {
        let all = {
            let mut guard = self
                .records
                .lock()
                .expect("projection records mutex poisoned");
            guard.insert(record.record_id.clone(), record);
            guard.values().cloned().collect::<Vec<_>>()
        };
        host.set_graph_projection_records(all);
    }
}

/// Strictly allowlisted UI commands for the Governance Authority Host.
///
/// Security invariant: re-arm is deliberately EXCLUDED so UI presentation events can
/// NEVER bypass one-shot `ApprovalGateService` + RFC 9470 verified human step-up.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GovernanceUiCommand {
    OperatorEStop {
        actor: String,
        reason: String,
    },
    EvaluateIncident,
    EvaluateClean,
    DelegateActor {
        parent: String,
        child: String,
    },
    RevokeActor {
        actor: String,
    },
    ContextFlow {
        entry: String,
        src_scope: String,
        target: String,
        dst_scope: String,
    },
    DeclassifyContext {
        entry: String,
    },
    RetractPremise {
        subject: String,
    },
    AddTaintEdge {
        src: String,
        dst: String,
        mark_source: bool,
        mark_sink: bool,
    },
    AddTaintSanitizer {
        node: String,
    },
    RegisterDerivation {
        derivation_id: String,
        policy_id: String,
        subject_premises: Vec<String>,
        parent_derivations: Vec<String>,
    },
    CheckCedar {
        actor: String,
        action: String,
        resource: String,
        step_up_fresh: bool,
    },
}

/// Authoritative decision result emitted by live compiled-Cedar policy probes.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub struct CedarProbeResult {
    pub seq: u64,
    pub actor: String,
    pub action: String,
    pub resource: String,
    pub step_up_fresh: bool,
    pub decision: String,
    pub matched_policy_ids: Vec<String>,
}

fn parse_csv_tokens(raw: &str) -> Vec<String> {
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

/// Parses declarative `scheme:governance-*` commands dispatched via `DispatchUiEvent`.
/// Any re-arm or privileged bypass attempt is rejected fail-closed (`None`).
pub fn parse_governance_ui_command(raw: &str) -> Option<GovernanceUiCommand> {
    let body = raw.trim().strip_prefix("scheme:")?.trim();
    let (cmd, arg) = match body.split_once(char::is_whitespace) {
        Some((head, tail)) => (head, tail.trim()),
        None => (body, ""),
    };
    let parts: Vec<&str> = arg.split_whitespace().collect();
    match cmd {
        "governance-estop" => {
            let (actor, reason) = if let Some((a, r)) = arg.split_once(':') {
                (a.trim().to_owned(), r.trim().to_owned())
            } else if !arg.is_empty() {
                ("operator".to_owned(), arg.to_owned())
            } else {
                ("operator".to_owned(), "ui-emergency-stop".to_owned())
            };
            Some(GovernanceUiCommand::OperatorEStop { actor, reason })
        }
        "governance-eval-incident" => Some(GovernanceUiCommand::EvaluateIncident),
        "governance-eval-clean" => Some(GovernanceUiCommand::EvaluateClean),
        "governance-delegate" if parts.len() == 2 => Some(GovernanceUiCommand::DelegateActor {
            parent: parts[0].to_owned(),
            child: parts[1].to_owned(),
        }),
        "governance-revoke" if parts.len() == 1 => Some(GovernanceUiCommand::RevokeActor {
            actor: parts[0].to_owned(),
        }),
        "governance-context-flow" if parts.len() == 4 => Some(GovernanceUiCommand::ContextFlow {
            entry: parts[0].to_owned(),
            src_scope: parts[1].to_owned(),
            target: parts[2].to_owned(),
            dst_scope: parts[3].to_owned(),
        }),
        "governance-declassify" if parts.len() == 1 => {
            Some(GovernanceUiCommand::DeclassifyContext {
                entry: parts[0].to_owned(),
            })
        }
        "governance-retract" if parts.len() == 1 => Some(GovernanceUiCommand::RetractPremise {
            subject: parts[0].to_owned(),
        }),
        "governance-taint-edge" if parts.len() >= 2 && parts.len() <= 3 => {
            let role = parts.get(2).copied().unwrap_or("");
            Some(GovernanceUiCommand::AddTaintEdge {
                src: parts[0].to_owned(),
                dst: parts[1].to_owned(),
                mark_source: matches!(role, "source" | "both"),
                mark_sink: matches!(role, "sink" | "both"),
            })
        }
        "governance-sanitize" if parts.len() == 1 => Some(GovernanceUiCommand::AddTaintSanitizer {
            node: parts[0].to_owned(),
        }),
        "governance-register-derivation" if parts.len() >= 3 && parts.len() <= 4 => {
            let subject_premises = parse_csv_tokens(parts[2]);
            if subject_premises.is_empty() {
                return None;
            }
            let parent_derivations = parts
                .get(3)
                .map(|p| parse_csv_tokens(p))
                .unwrap_or_default();
            Some(GovernanceUiCommand::RegisterDerivation {
                derivation_id: parts[0].to_owned(),
                policy_id: parts[1].to_owned(),
                subject_premises,
                parent_derivations,
            })
        }
        "governance-check-cedar" if parts.len() >= 3 && parts.len() <= 4 => {
            let step_up_fresh = parts
                .get(3)
                .map(|s| s.eq_ignore_ascii_case("true") || *s == "1" || *s == "stepup=true")
                .unwrap_or(false);
            Some(GovernanceUiCommand::CheckCedar {
                actor: parts[0].to_owned(),
                action: parts[1].to_owned(),
                resource: parts[2].to_owned(),
                step_up_fresh,
            })
        }
        _ => None,
    }
}

/// Live mutable runtime security fact graph driving real-time Linked-AOT/VM Scheme
/// `POO × ASCENT × Bi-Temporal TM × Cedar` incremental re-evaluation.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct GovernanceFactStore {
    pub delegation_edges: BTreeSet<(String, String)>,
    pub revoked_actors: BTreeSet<String>,
    pub taint_edges: BTreeSet<(String, String)>,
    pub taint_sources: BTreeSet<String>,
    pub taint_sinks: BTreeSet<String>,
    pub taint_sanitizers: BTreeSet<String>,
    pub context_entries: BTreeMap<String, String>,
    pub context_flows: BTreeSet<(String, String)>,
    pub target_scopes: BTreeMap<String, String>,
    pub declassified_entries: BTreeSet<String>,
    pub tripped_breakers: BTreeSet<String>,
    pub breaker_tools: BTreeSet<(String, String)>,
    pub retracted_subjects: BTreeSet<String>,
    pub derivations: BTreeMap<String, GovernanceDerivationSpec>,
}

impl GovernanceFactStore {
    pub fn reset_from_request(&mut self, req: &GovernancePolicyVectorRequest) {
        self.delegation_edges = req.delegation_edges.iter().cloned().collect();
        self.revoked_actors = req.revoked_actors.iter().cloned().collect();
        self.taint_edges = req.taint_edges.iter().cloned().collect();
        self.taint_sources = req.taint_sources.iter().cloned().collect();
        self.taint_sinks = req.taint_sinks.iter().cloned().collect();
        self.taint_sanitizers = req.taint_sanitizers.iter().cloned().collect();
        self.context_entries = req.context_entries.iter().cloned().collect();
        self.context_flows = req.context_flows.iter().cloned().collect();
        self.target_scopes = req.target_scopes.iter().cloned().collect();
        self.declassified_entries = req.declassified_entries.iter().cloned().collect();
        self.tripped_breakers = req.tripped_breakers.iter().cloned().collect();
        self.breaker_tools = req.breaker_tools.iter().cloned().collect();
        self.retracted_subjects = req.retracted_subjects.iter().cloned().collect();
        self.derivations = req
            .derivation_specs
            .iter()
            .map(|d| (d.derivation_id.clone(), d.clone()))
            .collect();
    }

    pub fn to_vector_request(
        &self,
        revocation_epoch: u64,
        policy_revision: u64,
    ) -> GovernancePolicyVectorRequest {
        GovernancePolicyVectorRequest {
            delegation_edges: self.delegation_edges.iter().cloned().collect(),
            revoked_actors: self.revoked_actors.iter().cloned().collect(),
            taint_edges: self.taint_edges.iter().cloned().collect(),
            taint_sources: self.taint_sources.iter().cloned().collect(),
            taint_sinks: self.taint_sinks.iter().cloned().collect(),
            taint_sanitizers: self.taint_sanitizers.iter().cloned().collect(),
            context_entries: self
                .context_entries
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            context_flows: self.context_flows.iter().cloned().collect(),
            target_scopes: self
                .target_scopes
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            declassified_entries: self.declassified_entries.iter().cloned().collect(),
            tripped_breakers: self.tripped_breakers.iter().cloned().collect(),
            breaker_tools: self.breaker_tools.iter().cloned().collect(),
            retracted_subjects: self.retracted_subjects.iter().cloned().collect(),
            revocation_epoch,
            policy_revision,
            derivation_specs: self.derivations.values().cloned().collect(),
        }
    }

    /// Serializes the full deterministic runtime fact graph into canonical `MGF1` v2 binary
    /// bytes sealed with a trailing 32-byte FIPS 180-4 SHA-256 integrity digest.
    pub fn to_canonical_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(512);
        buf.extend_from_slice(GOVERNANCE_FACT_STORE_MAGIC);
        put_u32(&mut buf, GOVERNANCE_FACT_STORE_VERSION);
        put_pair_set(&mut buf, &self.delegation_edges);
        put_str_set(&mut buf, &self.revoked_actors);
        put_pair_set(&mut buf, &self.taint_edges);
        put_str_set(&mut buf, &self.taint_sources);
        put_str_set(&mut buf, &self.taint_sinks);
        put_str_set(&mut buf, &self.taint_sanitizers);
        put_str_map(&mut buf, &self.context_entries);
        put_pair_set(&mut buf, &self.context_flows);
        put_str_map(&mut buf, &self.target_scopes);
        put_str_set(&mut buf, &self.declassified_entries);
        put_str_set(&mut buf, &self.tripped_breakers);
        put_pair_set(&mut buf, &self.breaker_tools);
        put_str_set(&mut buf, &self.retracted_subjects);
        put_u32(&mut buf, self.derivations.len() as u32);
        for spec in self.derivations.values() {
            put_str(&mut buf, &spec.derivation_id);
            put_str(&mut buf, &spec.policy_id);
            put_str_vec(&mut buf, &spec.subject_premises);
            put_str_vec(&mut buf, &spec.parent_derivations);
        }
        let mac = governance_sha256_bytes(&buf);
        buf.extend_from_slice(&mac);
        buf
    }

    /// Validates magic `MGF1`, schema version (`1` or `2`), exact SHA-256 digest, bounds, and exact EOF.
    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() < 4 + 4 + 32 {
            return Err("fact-graph checkpoint truncated below minimum size".to_owned());
        }
        let (body, tag) = bytes.split_at(bytes.len() - 32);
        let expected = governance_sha256_bytes(body);
        if !constant_time_eq_32(tag, &expected) {
            return Err("fact-graph SHA-256 integrity verification mismatch".to_owned());
        }
        let mut cur = FactCursor::new(body);
        let magic = cur.read_exact(4)?;
        if magic != GOVERNANCE_FACT_STORE_MAGIC {
            return Err("invalid fact-graph checkpoint magic header".to_owned());
        }
        let version = cur.read_u32()?;
        if version != 1 && version != GOVERNANCE_FACT_STORE_VERSION {
            return Err(format!(
                "unsupported fact-graph checkpoint version {version}"
            ));
        }
        let delegation_edges = cur.read_pair_set(4096)?;
        let revoked_actors = cur.read_str_set(4096)?;
        let taint_edges = cur.read_pair_set(4096)?;
        let taint_sources = cur.read_str_set(4096)?;
        let taint_sinks = cur.read_str_set(4096)?;
        let taint_sanitizers = cur.read_str_set(4096)?;
        let context_entries = cur.read_str_map(4096)?;
        let context_flows = cur.read_pair_set(4096)?;
        let target_scopes = cur.read_str_map(4096)?;
        let declassified_entries = cur.read_str_set(4096)?;
        let tripped_breakers = cur.read_str_set(1024)?;
        let breaker_tools = cur.read_pair_set(4096)?;
        let retracted_subjects = cur.read_str_set(4096)?;
        let derivations = if version >= 2 {
            let count = cur.read_u32()? as usize;
            if count > 4096 {
                return Err(format!("excessive fact-graph derivation count {count}"));
            }
            let mut map = BTreeMap::new();
            for _ in 0..count {
                let derivation_id = cur.read_str()?;
                let policy_id = cur.read_str()?;
                let subject_premises = cur.read_str_vec(256)?;
                let parent_derivations = cur.read_str_vec(256)?;
                map.insert(
                    derivation_id.clone(),
                    GovernanceDerivationSpec {
                        derivation_id,
                        policy_id,
                        subject_premises,
                        parent_derivations,
                    },
                );
            }
            map
        } else {
            BTreeMap::new()
        };
        let store = Self {
            delegation_edges,
            revoked_actors,
            taint_edges,
            taint_sources,
            taint_sinks,
            taint_sanitizers,
            context_entries,
            context_flows,
            target_scopes,
            declassified_entries,
            tripped_breakers,
            breaker_tools,
            retracted_subjects,
            derivations,
        };
        if !cur.is_eof() {
            return Err("unexpected trailing bytes in fact-graph checkpoint".to_owned());
        }
        Ok(store)
    }
}

fn put_str_vec(buf: &mut Vec<u8>, items: &[String]) {
    put_u32(buf, items.len() as u32);
    for item in items {
        put_str(buf, item);
    }
}

fn put_u32(buf: &mut Vec<u8>, v: u32) {
    buf.extend_from_slice(&v.to_be_bytes());
}

fn put_str(buf: &mut Vec<u8>, s: &str) {
    let bytes = s.as_bytes();
    put_u32(buf, bytes.len() as u32);
    buf.extend_from_slice(bytes);
}

fn put_str_set(buf: &mut Vec<u8>, set: &BTreeSet<String>) {
    put_u32(buf, set.len() as u32);
    for item in set {
        put_str(buf, item);
    }
}

fn put_pair_set(buf: &mut Vec<u8>, set: &BTreeSet<(String, String)>) {
    put_u32(buf, set.len() as u32);
    for (a, b) in set {
        put_str(buf, a);
        put_str(buf, b);
    }
}

fn put_str_map(buf: &mut Vec<u8>, map: &BTreeMap<String, String>) {
    put_u32(buf, map.len() as u32);
    for (k, v) in map {
        put_str(buf, k);
        put_str(buf, v);
    }
}

struct FactCursor<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> FactCursor<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn is_eof(&self) -> bool {
        self.pos == self.data.len()
    }

    fn read_exact(&mut self, n: usize) -> Result<&'a [u8], String> {
        let end = self
            .pos
            .checked_add(n)
            .ok_or_else(|| "integer overflow in fact-graph cursor".to_owned())?;
        if end > self.data.len() {
            return Err("unexpected EOF in fact-graph checkpoint".to_owned());
        }
        let slice = &self.data[self.pos..end];
        self.pos = end;
        Ok(slice)
    }

    fn read_u32(&mut self) -> Result<u32, String> {
        let b = self.read_exact(4)?;
        Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn read_str(&mut self) -> Result<String, String> {
        let len = self.read_u32()? as usize;
        if len > 65_536 {
            return Err(format!("excessive fact-graph string length {len}"));
        }
        let bytes = self.read_exact(len)?;
        std::str::from_utf8(bytes)
            .map(str::to_owned)
            .map_err(|e| format!("invalid utf8 in fact-graph checkpoint: {e}"))
    }

    fn read_str_vec(&mut self, max_items: usize) -> Result<Vec<String>, String> {
        let count = self.read_u32()? as usize;
        if count > max_items {
            return Err(format!(
                "excessive fact-graph vec count {count} > {max_items}"
            ));
        }
        let mut out = Vec::with_capacity(count);
        for _ in 0..count {
            out.push(self.read_str()?);
        }
        Ok(out)
    }

    fn read_str_set(&mut self, max_items: usize) -> Result<BTreeSet<String>, String> {
        let count = self.read_u32()? as usize;
        if count > max_items {
            return Err(format!(
                "excessive fact-graph set count {count} > {max_items}"
            ));
        }
        let mut out = BTreeSet::new();
        for _ in 0..count {
            out.insert(self.read_str()?);
        }
        Ok(out)
    }

    fn read_pair_set(&mut self, max_items: usize) -> Result<BTreeSet<(String, String)>, String> {
        let count = self.read_u32()? as usize;
        if count > max_items {
            return Err(format!(
                "excessive fact-graph pair count {count} > {max_items}"
            ));
        }
        let mut out = BTreeSet::new();
        for _ in 0..count {
            let a = self.read_str()?;
            let b = self.read_str()?;
            out.insert((a, b));
        }
        Ok(out)
    }

    fn read_str_map(&mut self, max_items: usize) -> Result<BTreeMap<String, String>, String> {
        let count = self.read_u32()? as usize;
        if count > max_items {
            return Err(format!(
                "excessive fact-graph map count {count} > {max_items}"
            ));
        }
        let mut out = BTreeMap::new();
        for _ in 0..count {
            let k = self.read_str()?;
            let v = self.read_str()?;
            out.insert(k, v);
        }
        Ok(out)
    }
}

/// Validates that an RFC 9470 / NIST 800-63 Authentication Context Class Reference (`acr`)
/// meets the high-assurance human MFA floor required for E-Stop circuit-breaker re-arm (SM30).
pub fn is_verified_mfa_acr(acr: &str) -> bool {
    let trimmed = acr.trim();
    if matches!(
        trimmed,
        "urn:ietf:params:oauth:acr:mfa"
            | "urn:ietf:params:oauth:acr:phr"
            | "urn:ietf:params:oauth:acr:phrh"
            | "urn:nist:800-63:aal:2"
            | "urn:nist:800-63:aal:3"
    ) {
        return true;
    }
    if let Some(suffix) = trimmed.strip_prefix("urn:marlin:acr:mfa:") {
        return !suffix.is_empty()
            && suffix.len() <= 64
            && suffix
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'));
    }
    false
}

/// Parses RFC 9470 Step-Up attestation from an `ApprovalGateService` human decision note:
/// `stepup-verified:actor=<human-id>:acr=<acr-uri>`.
///
/// Missing step-up markers, duplicate keys, weak/unallowlisted ACR URNs, or non-human
/// identities (`agent*`/`bot*`/`llm*`/`worker*`/`scheme-sandbox*`/`system*`/`service*`)
/// are classified as `UnverifiedActor` / `Agent` so `ExecutionCircuitBreaker::rearm` fails closed.
pub fn parse_verified_step_up_actor(note: Option<&str>) -> Result<BreakerActor, String> {
    let raw = note.unwrap_or("").trim();
    let Some(rest) = raw.strip_prefix("stepup-verified:") else {
        let fallback_id = BreakerActorId::new("unverified-operator")
            .map_err(|e| format!("invalid fallback actor: {e}"))?;
        return Ok(BreakerActor::UnverifiedActor(fallback_id));
    };

    let mut actor: Option<String> = None;
    let mut acr: Option<String> = None;
    for segment in rest.split(':') {
        if let Some(v) = segment.strip_prefix("actor=") {
            if actor.is_some() {
                let fallback_id = BreakerActorId::new("duplicate-actor-param")
                    .map_err(|e| format!("invalid fallback actor: {e}"))?;
                return Ok(BreakerActor::UnverifiedActor(fallback_id));
            }
            actor = Some(v.trim().to_owned());
        } else if let Some(v) = segment.strip_prefix("acr=") {
            if acr.is_some() {
                let fallback_id = BreakerActorId::new("duplicate-acr-param")
                    .map_err(|e| format!("invalid fallback actor: {e}"))?;
                return Ok(BreakerActor::UnverifiedActor(fallback_id));
            }
            acr = Some(v.trim().to_owned());
        } else if let Some(acr_buf) = acr.as_mut() {
            // Preserve colons inside URN ACR values (e.g. `urn:ietf:params:oauth:acr:mfa`),
            // while still rejecting embedded `key=` smuggling.
            if segment.contains('=') {
                let fallback_id = BreakerActorId::new("smuggled-acr-param")
                    .map_err(|e| format!("invalid fallback actor: {e}"))?;
                return Ok(BreakerActor::UnverifiedActor(fallback_id));
            }
            acr_buf.push(':');
            acr_buf.push_str(segment);
        } else {
            let fallback_id = BreakerActorId::new("unexpected-stepup-segment")
                .map_err(|e| format!("invalid fallback actor: {e}"))?;
            return Ok(BreakerActor::UnverifiedActor(fallback_id));
        }
    }

    let actor = actor.unwrap_or_default();
    if actor.is_empty() {
        let fallback_id = BreakerActorId::new("missing-actor")
            .map_err(|e| format!("invalid missing actor id: {e}"))?;
        return Ok(BreakerActor::UnverifiedActor(fallback_id));
    }

    let actor_id = BreakerActorId::new(actor.clone()).map_err(|e| format!("invalid actor: {e}"))?;
    let lower = actor.to_ascii_lowercase();
    if !infer_actor_roles(&actor).is_empty()
        || lower.starts_with("system")
        || lower.starts_with("service")
    {
        return Ok(BreakerActor::Agent(actor_id));
    }
    let acr = acr.unwrap_or_default();
    if !is_verified_mfa_acr(&acr) {
        return Ok(BreakerActor::UnverifiedActor(actor_id));
    }
    Ok(BreakerActor::VerifiedHuman(actor_id))
}

/// Converts a zero-JSON 14-slot Gambit Scheme vector projection into a validated
/// kernel `GovernanceProofEnvelope`.
pub fn projection_to_kernel_envelope(
    proj: &GovernancePolicyVectorProjection,
) -> Result<GovernanceProofEnvelope, GovernanceAdmissionError> {
    let mut fenced_actors = Vec::with_capacity(proj.fenced_actors.len());
    for actor in &proj.fenced_actors {
        fenced_actors.push(GovernedSubjectName::new(actor.clone())?);
    }
    let mut blocked_tools = Vec::with_capacity(proj.blocked_tools.len());
    for tool in &proj.blocked_tools {
        blocked_tools.push(GovernedSubjectName::new(tool.clone())?);
    }
    let mut retracted_conclusions = Vec::with_capacity(proj.retracted_conclusions.len());
    for deriv in &proj.retracted_conclusions {
        retracted_conclusions.push(DerivationId::new(deriv.clone())?);
    }
    let mut cedar_policies = Vec::with_capacity(proj.cedar_policies.len());
    for (id, source) in &proj.cedar_policies {
        cedar_policies.push(SynthesizedCedarPolicy {
            policy_id: CedarPolicyId::new(id.clone())?,
            source: source.clone(),
        });
    }
    Ok(GovernanceProofEnvelope {
        schema_version: proj.schema_version,
        admitted: proj.admitted,
        revocation_epoch: GovernanceProofEpoch::new(proj.revocation_epoch)?,
        policy_revision: proj.policy_revision,
        assessment_digest: GovernanceProofDigest::new(proj.assessment_digest.clone())?,
        taint_proof_digest: GovernanceProofDigest::new(proj.taint_proof_digest.clone())?,
        fixpoint_proof_digest: GovernanceProofDigest::new(proj.fixpoint_proof_digest.clone())?,
        temporal_proof_digest: GovernanceProofDigest::new(proj.temporal_proof_digest.clone())?,
        invalidation_digest: GovernanceProofDigest::new(proj.invalidation_digest.clone())?,
        rejection_reasons: proj.rejection_reasons.clone(),
        fenced_actors,
        blocked_tools,
        retracted_conclusions,
        cedar_policies,
    })
}

/// Canonical multi-layer threat incident request fed to live Gerbil Scheme
/// (`POO × ASCENT Delegation/Taint/Breaker Fixpoint × Bi-Temporal TM Retraction × Cedar`).
pub fn default_incident_governance_request(
    revocation_epoch: u64,
    policy_revision: u64,
) -> GovernancePolicyVectorRequest {
    GovernancePolicyVectorRequest {
        delegation_edges: vec![
            ("alice".to_owned(), "agent-lead".to_owned()),
            ("agent-lead".to_owned(), "agent-sub".to_owned()),
        ],
        revoked_actors: vec!["alice".to_owned()],
        taint_edges: vec![
            ("chat-ext".to_owned(), "shared-ctx".to_owned()),
            ("shared-ctx".to_owned(), "os-shell".to_owned()),
        ],
        taint_sources: vec!["chat-ext".to_owned()],
        taint_sinks: vec!["os-shell".to_owned()],
        taint_sanitizers: Vec::new(),
        context_entries: vec![("secret-doc".to_owned(), "private".to_owned())],
        context_flows: vec![("secret-doc".to_owned(), "ext-webhook".to_owned())],
        target_scopes: vec![("ext-webhook".to_owned(), "external".to_owned())],
        declassified_entries: Vec::new(),
        tripped_breakers: vec!["exec-breaker-1".to_owned()],
        breaker_tools: vec![("exec-breaker-1".to_owned(), "os-shell".to_owned())],
        retracted_subjects: vec!["human-session-alice".to_owned()],
        revocation_epoch,
        policy_revision,
        derivation_specs: Vec::new(),
    }
}

/// Clean sanitized governance baseline request fed to live Gerbil Scheme.
pub fn default_clean_governance_request(
    revocation_epoch: u64,
    policy_revision: u64,
) -> GovernancePolicyVectorRequest {
    GovernancePolicyVectorRequest {
        delegation_edges: vec![("ops-admin".to_owned(), "ops-bob".to_owned())],
        revoked_actors: Vec::new(),
        taint_edges: vec![
            ("chat-ext".to_owned(), "human-gate".to_owned()),
            ("human-gate".to_owned(), "os-shell".to_owned()),
        ],
        taint_sources: vec!["chat-ext".to_owned()],
        taint_sinks: vec!["os-shell".to_owned()],
        taint_sanitizers: vec!["human-gate".to_owned()],
        context_entries: Vec::new(),
        context_flows: Vec::new(),
        target_scopes: Vec::new(),
        declassified_entries: Vec::new(),
        tripped_breakers: Vec::new(),
        breaker_tools: Vec::new(),
        retracted_subjects: Vec::new(),
        revocation_epoch,
        policy_revision,
        derivation_specs: Vec::new(),
    }
}

#[derive(Default)]
struct HostGovernanceMeta {
    active_rearm_gate_id: Option<String>,
    rearm_gate_seq: u64,
    policy_revision_seq: u64,
    cedar_probe_seq: u64,
    last_cedar_probe: Option<CedarProbeResult>,
    last_event_note: String,
    facts: GovernanceFactStore,
}

/// Thread-safe Rust control-plane controller synchronizing live Gambit Scheme
/// Governance Authority projections with `ExecutionCircuitBreaker`,
/// `ApprovalGateService` one-shot Step-Up Re-Arm gates, and `ProjectionQueryService`.
pub struct GovernanceHostController {
    gate: Arc<GovernanceAdmissionGate>,
    records: Arc<SharedProjectionRecords>,
    meta: Mutex<HostGovernanceMeta>,
}

impl GovernanceHostController {
    pub fn new(budget_limit: u32, records: Arc<SharedProjectionRecords>) -> Self {
        let breaker = Arc::new(ExecutionCircuitBreaker::new(ToolBudgetUnits::new(
            budget_limit.max(1),
        )));
        Self {
            gate: Arc::new(GovernanceAdmissionGate::new(breaker)),
            records,
            meta: Mutex::new(HostGovernanceMeta {
                active_rearm_gate_id: None,
                rearm_gate_seq: 0,
                policy_revision_seq: 1,
                cedar_probe_seq: 0,
                last_cedar_probe: None,
                last_event_note: "initialized".to_owned(),
                facts: GovernanceFactStore::default(),
            }),
        }
    }

    pub fn gate(&self) -> Arc<GovernanceAdmissionGate> {
        Arc::clone(&self.gate)
    }

    /// Synchronizes the internal dynamic fact graph from a baseline evaluation request.
    pub fn seed_fact_store(&self, req: &GovernancePolicyVectorRequest) {
        let mut meta = self.meta.lock().expect("host governance meta poisoned");
        meta.facts.reset_from_request(req);
        drop(meta);
        self.sync_gate_resource_scopes();
    }

    fn sync_gate_resource_scopes(&self) {
        let scopes = {
            let meta = self.meta.lock().expect("host governance meta poisoned");
            meta.facts
                .target_scopes
                .iter()
                .filter_map(|(target, scope)| {
                    GovernedSubjectName::new(target.clone())
                        .ok()
                        .map(|name| (name, scope.clone()))
                })
                .collect::<BTreeMap<_, _>>()
        };
        let _ = self.gate.replace_resource_scopes(scopes);
    }

    /// Applies one dynamic runtime governance mutation to the live fact store and returns
    /// a freshly timestamped `(GovernancePolicyVectorRequest, node_id, event_note)`
    /// ready for immediate Linked-AOT/VM Scheme evaluation.
    pub fn mutate_facts_and_build_request(
        &self,
        cmd: &GovernanceUiCommand,
    ) -> Result<Option<(GovernancePolicyVectorRequest, String, String)>, String> {
        let (epoch, rev) = self.next_evaluation_coords()?;
        let mut meta = self.meta.lock().expect("host governance meta poisoned");
        let (node_id, note) = match cmd {
            GovernanceUiCommand::EvaluateIncident => {
                let req = default_incident_governance_request(epoch, rev);
                meta.facts.reset_from_request(&req);
                return Ok(Some((
                    req,
                    "governance_eval_incident".to_owned(),
                    "live scheme governance incident evaluated".to_owned(),
                )));
            }
            GovernanceUiCommand::EvaluateClean => {
                let req = default_clean_governance_request(epoch, rev);
                meta.facts.reset_from_request(&req);
                return Ok(Some((
                    req,
                    "governance_eval_clean".to_owned(),
                    "live scheme governance clean baseline evaluated".to_owned(),
                )));
            }
            GovernanceUiCommand::DelegateActor { parent, child } => {
                let pair = (parent.clone(), child.clone());
                ensure_fact_capacity(
                    meta.facts.delegation_edges.len(),
                    MAX_FACT_STORE_ENTRIES,
                    meta.facts.delegation_edges.contains(&pair),
                    "delegation_edges",
                )?;
                meta.facts.delegation_edges.insert(pair);
                (
                    format!("governance_delegate:{parent}->{child}"),
                    format!("delegated {parent} -> {child} (live ASCENT re-eval)"),
                )
            }
            GovernanceUiCommand::RevokeActor { actor } => {
                ensure_fact_capacity(
                    meta.facts.revoked_actors.len(),
                    MAX_FACT_STORE_ENTRIES,
                    meta.facts.revoked_actors.contains(actor),
                    "revoked_actors",
                )?;
                meta.facts.revoked_actors.insert(actor.clone());
                (
                    format!("governance_revoke:{actor}"),
                    format!("revoked {actor} (live ASCENT transitive closure)"),
                )
            }
            GovernanceUiCommand::ContextFlow {
                entry,
                src_scope,
                target,
                dst_scope,
            } => {
                let flow_pair = (entry.clone(), target.clone());
                ensure_fact_capacity(
                    meta.facts.context_entries.len(),
                    MAX_FACT_STORE_ENTRIES,
                    meta.facts.context_entries.contains_key(entry),
                    "context_entries",
                )?;
                ensure_fact_capacity(
                    meta.facts.context_flows.len(),
                    MAX_FACT_STORE_ENTRIES,
                    meta.facts.context_flows.contains(&flow_pair),
                    "context_flows",
                )?;
                ensure_fact_capacity(
                    meta.facts.target_scopes.len(),
                    MAX_FACT_STORE_ENTRIES,
                    meta.facts.target_scopes.contains_key(target),
                    "target_scopes",
                )?;
                meta.facts
                    .context_entries
                    .insert(entry.clone(), src_scope.clone());
                meta.facts.context_flows.insert(flow_pair);
                meta.facts
                    .target_scopes
                    .insert(target.clone(), dst_scope.clone());
                (
                    format!("governance_context_flow:{entry}->{target}"),
                    format!("context flow {entry}({src_scope}) -> {target}({dst_scope})"),
                )
            }
            GovernanceUiCommand::DeclassifyContext { entry } => {
                ensure_fact_capacity(
                    meta.facts.declassified_entries.len(),
                    MAX_FACT_STORE_ENTRIES,
                    meta.facts.declassified_entries.contains(entry),
                    "declassified_entries",
                )?;
                meta.facts.declassified_entries.insert(entry.clone());
                (
                    format!("governance_declassify:{entry}"),
                    format!("declassified shared context entry {entry}"),
                )
            }
            GovernanceUiCommand::RetractPremise { subject } => {
                ensure_fact_capacity(
                    meta.facts.retracted_subjects.len(),
                    MAX_FACT_STORE_ENTRIES,
                    meta.facts.retracted_subjects.contains(subject),
                    "retracted_subjects",
                )?;
                meta.facts.retracted_subjects.insert(subject.clone());
                (
                    format!("governance_retract:{subject}"),
                    format!("retracted bi-temporal premise {subject}"),
                )
            }
            GovernanceUiCommand::AddTaintEdge {
                src,
                dst,
                mark_source,
                mark_sink,
            } => {
                let edge = (src.clone(), dst.clone());
                ensure_fact_capacity(
                    meta.facts.taint_edges.len(),
                    MAX_FACT_STORE_ENTRIES,
                    meta.facts.taint_edges.contains(&edge),
                    "taint_edges",
                )?;
                if *mark_source {
                    ensure_fact_capacity(
                        meta.facts.taint_sources.len(),
                        MAX_FACT_STORE_ENTRIES,
                        meta.facts.taint_sources.contains(src),
                        "taint_sources",
                    )?;
                }
                if *mark_sink {
                    ensure_fact_capacity(
                        meta.facts.taint_sinks.len(),
                        MAX_FACT_STORE_ENTRIES,
                        meta.facts.taint_sinks.contains(dst),
                        "taint_sinks",
                    )?;
                }
                meta.facts.taint_edges.insert(edge);
                if *mark_source {
                    meta.facts.taint_sources.insert(src.clone());
                }
                if *mark_sink {
                    meta.facts.taint_sinks.insert(dst.clone());
                }
                (
                    format!("governance_taint_edge:{src}->{dst}"),
                    format!("recorded CaMeL taint edge {src} -> {dst}"),
                )
            }
            GovernanceUiCommand::AddTaintSanitizer { node } => {
                ensure_fact_capacity(
                    meta.facts.taint_sanitizers.len(),
                    MAX_FACT_STORE_ENTRIES,
                    meta.facts.taint_sanitizers.contains(node),
                    "taint_sanitizers",
                )?;
                meta.facts.taint_sanitizers.insert(node.clone());
                (
                    format!("governance_sanitize:{node}"),
                    format!("registered CaMeL sanitizer {node}"),
                )
            }
            GovernanceUiCommand::RegisterDerivation {
                derivation_id,
                policy_id,
                subject_premises,
                parent_derivations,
            } => {
                ensure_fact_capacity(
                    meta.facts.derivations.len(),
                    MAX_FACT_STORE_ENTRIES,
                    meta.facts.derivations.contains_key(derivation_id),
                    "derivations",
                )?;
                meta.facts.derivations.insert(
                    derivation_id.clone(),
                    GovernanceDerivationSpec {
                        derivation_id: derivation_id.clone(),
                        policy_id: policy_id.clone(),
                        subject_premises: subject_premises.clone(),
                        parent_derivations: parent_derivations.clone(),
                    },
                );
                (
                    format!("governance_register_derivation:{derivation_id}"),
                    format!("registered bi-temporal derivation {derivation_id} ({policy_id})"),
                )
            }
            GovernanceUiCommand::OperatorEStop { .. } | GovernanceUiCommand::CheckCedar { .. } => {
                return Ok(None);
            }
        };
        let req = meta.facts.to_vector_request(epoch, rev);
        Ok(Some((req, node_id, note)))
    }

    /// Allocates strictly advancing `(revocation_epoch, policy_revision)` coordinates
    /// for the next live Gambit Scheme VM evaluation.
    pub fn next_evaluation_coords(&self) -> Result<(u64, u64), String> {
        let epoch = self
            .gate
            .next_revocation_epoch()
            .map_err(|e| format!("next revocation epoch: {e}"))?;
        let rev = {
            let mut meta = self.meta.lock().expect("host governance meta poisoned");
            meta.policy_revision_seq = meta.policy_revision_seq.saturating_add(1);
            meta.policy_revision_seq
        };
        Ok((epoch, rev))
    }

    /// Evaluates a live compiled-Cedar authorization probe directly against the
    /// synchronized `GovernanceAdmissionGate` policies (`forbid`-overrides-`permit`),
    /// records the structured decision on the host, and publishes it over gRPC.
    pub fn evaluate_cedar_probe(
        &self,
        host: &MyGoBridgeHost,
        actor: &str,
        action: &str,
        resource: &str,
        step_up_fresh: bool,
    ) -> Result<CedarProbeResult, String> {
        let mut principal_roles = infer_actor_roles(actor);
        if principal_roles.is_empty() && step_up_fresh {
            principal_roles.push(CedarEntityRef::role("VerifiedHuman"));
        }
        let (resource_groups, declassified_resource, tainted_resource) = {
            let meta = self.meta.lock().expect("host governance meta poisoned");
            let mut groups = Vec::new();
            if let Some(scope) = meta.facts.target_scopes.get(resource) {
                groups.push(CedarEntityRef::scope(scope.clone()));
            }
            let declassified = meta.facts.declassified_entries.contains(resource)
                || meta.facts.context_flows.iter().any(|(entry, target)| {
                    target == resource && meta.facts.declassified_entries.contains(entry)
                });
            let tainted = meta.facts.taint_sinks.contains(resource)
                && !meta.facts.taint_sanitizers.contains(resource);
            (groups, declassified, tainted)
        };
        let mut context = BTreeMap::new();
        context.insert(
            "step_up_fresh".to_owned(),
            CedarContextValue::Bool(step_up_fresh),
        );
        context.insert(
            "declassified".to_owned(),
            CedarContextValue::Bool(declassified_resource),
        );
        context.insert(
            "tainted".to_owned(),
            CedarContextValue::Bool(tainted_resource),
        );
        let req = CedarEvaluationRequest {
            principal: CedarEntityRef::actor(actor),
            principal_roles,
            action: CedarEntityRef::action(action),
            resource: CedarEntityRef::resource(resource),
            resource_groups,
            context,
        };
        let eval = self
            .gate
            .evaluate_cedar_request(&req)
            .map_err(|e| format!("evaluate compiled Cedar probe: {e}"))?;
        let (decision, matched_policy_ids) = match eval {
            CedarEvaluationDecision::Allow { matching_permits } => (
                "allow".to_owned(),
                matching_permits
                    .into_iter()
                    .map(|p| p.as_str().to_owned())
                    .collect(),
            ),
            CedarEvaluationDecision::Deny { matching_forbids } => (
                "deny".to_owned(),
                matching_forbids
                    .into_iter()
                    .map(|p| p.as_str().to_owned())
                    .collect(),
            ),
            CedarEvaluationDecision::ImplicitDeny => ("implicit_deny".to_owned(), Vec::new()),
        };
        let result = {
            let mut meta = self.meta.lock().expect("host governance meta poisoned");
            meta.cedar_probe_seq = meta.cedar_probe_seq.saturating_add(1);
            let probe = CedarProbeResult {
                seq: meta.cedar_probe_seq,
                actor: actor.to_owned(),
                action: action.to_owned(),
                resource: resource.to_owned(),
                step_up_fresh,
                decision: decision.clone(),
                matched_policy_ids: matched_policy_ids.clone(),
            };
            meta.last_cedar_probe = Some(probe.clone());
            probe
        };
        let note = format!(
            "cedar probe #{}: ({actor}, {action}, {resource}, step_up={step_up_fresh}) -> {decision}",
            result.seq
        );
        let _ =
            self.publish_snapshot(host, &format!("governance_cedar_probe:{decision}"), &note)?;
        Ok(result)
    }

    /// Gates a real OS `SandboxRun` atomic effect through `GovernanceAdmissionGate`,
    /// enforcing ASCENT fencing, breaker blocks, pinpoint Cedar forbids, AND registered
    /// `Marlin::Scope::"..."` `unless`-clause policies on the target program/resource.
    pub fn authorize_sandbox_effect(&self, actor: &str, program: &str) -> Result<(), String> {
        self.authorize_sandbox_effect_on_resource(actor, program, program)
    }

    /// Resource-explicit variant of `authorize_sandbox_effect` for callers passing a
    /// target resource identity distinct from the executable program name.
    pub fn authorize_sandbox_effect_on_resource(
        &self,
        actor: &str,
        program: &str,
        resource: &str,
    ) -> Result<(), String> {
        let actor_id = GovernedSubjectName::new(if actor.trim().is_empty() {
            "scheme-sandbox-worker"
        } else {
            actor.trim()
        })
        .map_err(|e| format!("invalid sandbox actor: {e}"))?;
        let tool_id = GovernedSubjectName::new(if program.trim().is_empty() {
            "os-shell"
        } else {
            program.trim()
        })
        .map_err(|e| format!("invalid sandbox tool: {e}"))?;
        let resource_id = GovernedSubjectName::new(if resource.trim().is_empty() {
            tool_id.as_str()
        } else {
            resource.trim()
        })
        .map_err(|e| format!("invalid sandbox resource: {e}"))?;
        self.gate
            .authorize_side_effect_on_resource(
                None,
                &actor_id,
                &tool_id,
                &resource_id,
                &ToolIfcContext::clean_trusted(),
            )
            .map(|_| ())
            .map_err(|e| format!("governance denied sandbox execution: {e}"))
    }

    /// Gates an external `LlmComplete` atomic effect through `GovernanceAdmissionGate`,
    /// failing closed under active Operator E-Stop, ASCENT fences, or Cedar forbids (SM30).
    pub fn authorize_llm_effect(&self, actor: &str, tool: &str) -> Result<(), String> {
        let actor_id = GovernedSubjectName::new(if actor.trim().is_empty() {
            "scheme-llm-worker"
        } else {
            actor.trim()
        })
        .map_err(|e| format!("invalid llm actor: {e}"))?;
        let tool_id = GovernedSubjectName::new(if tool.trim().is_empty() {
            "llm-complete"
        } else {
            tool.trim()
        })
        .map_err(|e| format!("invalid llm tool: {e}"))?;
        self.gate
            .authorize_side_effect_on_resource(
                None,
                &actor_id,
                &tool_id,
                &tool_id,
                &ToolIfcContext::clean_trusted(),
            )
            .map(|_| ())
            .map_err(|e| format!("governance denied llm completion: {e}"))
    }

    /// Persists BOTH the tamper-evident `MGV1` governance gate checkpoint AND the companion
    /// `MGF1` runtime fact-graph checkpoint to Turso CAS so incremental post-restart mutations
    /// retain full delegation, context-flow, taint, and retraction history.
    pub async fn persist_to_cas<S: AgentStorage + Clone + 'static>(
        &self,
        cas: &StorageCasStore<S>,
        checkpoint_key: &str,
    ) -> Result<(), String> {
        let gate_bytes = self
            .gate
            .export_durable_checkpoint_bytes()
            .map_err(|e| format!("export durable governance checkpoint: {e}"))?;
        let fact_bytes = {
            let meta = self.meta.lock().expect("host governance meta poisoned");
            meta.facts.to_canonical_bytes()
        };
        cas.put_blob(&fact_store_checkpoint_key(checkpoint_key), fact_bytes)
            .await
            .map_err(|e| format!("persist durable governance fact graph to CAS: {e}"))?;
        cas.put_blob(checkpoint_key, gate_bytes)
            .await
            .map_err(|e| format!("persist durable governance checkpoint to CAS: {e}"))
    }

    /// Restores tamper-evident `MGV1` gate state and companion `MGF1` fact-graph state from
    /// Turso CAS on daemon boot.
    ///
    /// Returns:
    /// - `Ok(None)` when no prior checkpoint exists in CAS (cold boot).
    /// - `Ok(Some((snap, "restored")))` when valid persisted checkpoints were restored.
    /// - `Ok(Some((snap, "corrupt_fail_closed")))` when either `MGV1` or `MGF1` failed SHA-256
    ///   or schema integrity checks and the host automatically failed closed (`tripped = true`).
    pub async fn restore_from_cas<S: AgentStorage + Clone + 'static>(
        &self,
        host: &MyGoBridgeHost,
        cas: &StorageCasStore<S>,
        checkpoint_key: &str,
    ) -> Result<Option<(GovernanceGateSnapshot, &'static str)>, String> {
        let Some(bytes) = cas
            .get_blob(checkpoint_key)
            .await
            .map_err(|e| format!("read durable governance checkpoint from CAS: {e}"))?
        else {
            return Ok(None);
        };

        match self.gate.restore_durable_checkpoint_bytes(&bytes) {
            Ok(snap) => {
                let fact_key = fact_store_checkpoint_key(checkpoint_key);
                if let Some(fact_bytes) = cas
                    .get_blob(&fact_key)
                    .await
                    .map_err(|e| format!("read durable governance fact graph from CAS: {e}"))?
                {
                    match GovernanceFactStore::from_canonical_bytes(&fact_bytes) {
                        Ok(restored_facts) => {
                            {
                                let mut meta =
                                    self.meta.lock().expect("host governance meta poisoned");
                                meta.facts = restored_facts;
                            }
                            self.sync_gate_resource_scopes();
                        }
                        Err(detail) => {
                            let tripped_snap = self
                                .gate
                                .trip_corrupt_durable_state(&format!("fact-graph:{detail}"))
                                .map_err(|e| format!("trip corrupt fact-graph fail-closed: {e}"))?;
                            let note = format!(
                                "corrupt durable governance fact-graph checkpoint (fail-closed): {detail}"
                            );
                            self.ensure_rearm_gate_enqueued(
                                host,
                                tripped_snap.breaker.trip_epoch.get(),
                                &note,
                            );
                            let published = self.publish_snapshot(
                                host,
                                "governance_boot_corrupt_fail_closed",
                                &note,
                            )?;
                            return Ok(Some((published, "corrupt_fail_closed")));
                        }
                    }
                }
                {
                    let mut meta = self.meta.lock().expect("host governance meta poisoned");
                    meta.policy_revision_seq = meta.policy_revision_seq.max(snap.policy_revision);
                    if !snap.breaker.tripped {
                        meta.active_rearm_gate_id = None;
                    }
                }
                let note = format!(
                    "restored durable governance checkpoint (trip_epoch={}, rev_epoch={})",
                    snap.breaker.trip_epoch.get(),
                    snap.revocation_epoch.get()
                );
                if snap.breaker.tripped {
                    self.ensure_rearm_gate_enqueued(host, snap.breaker.trip_epoch.get(), &note);
                }
                let published = self.publish_snapshot(host, "governance_boot_restored", &note)?;
                Ok(Some((published, "restored")))
            }
            Err(GovernanceAdmissionError::CorruptDurableCheckpoint(detail)) => {
                let snap = self
                    .gate
                    .snapshot()
                    .map_err(|e| format!("snapshot fail-closed corrupt recovery: {e}"))?;
                let note = format!("corrupt durable governance checkpoint (fail-closed): {detail}");
                self.ensure_rearm_gate_enqueued(host, snap.breaker.trip_epoch.get(), &note);
                let published =
                    self.publish_snapshot(host, "governance_boot_corrupt_fail_closed", &note)?;
                Ok(Some((published, "corrupt_fail_closed")))
            }
            Err(other) => Err(format!("restore durable governance checkpoint: {other}")),
        }
    }

    /// Crash-consistent durable variant of `apply_scheme_projection`: flushes the updated
    /// tamper-evident checkpoint to Turso CAS BEFORE publishing to gRPC clients.
    pub async fn apply_scheme_projection_durable<S: AgentStorage + Clone + 'static>(
        &self,
        host: &MyGoBridgeHost,
        cas: Option<&StorageCasStore<S>>,
        checkpoint_key: Option<&str>,
        proj: &GovernancePolicyVectorProjection,
        node_id: &str,
        note: &str,
    ) -> Result<GovernanceGateSnapshot, String> {
        let envelope = projection_to_kernel_envelope(proj)
            .map_err(|e| format!("validate scheme proof envelope: {e}"))?;
        let _receipt = self
            .gate
            .synchronize_proof_envelope(&envelope)
            .map_err(|e| format!("synchronize scheme proof envelope: {e}"))?;
        self.sync_gate_resource_scopes();
        if let (Some(store), Some(key)) = (cas, checkpoint_key) {
            self.persist_to_cas(store, key).await?;
        }
        let snap = self
            .gate
            .snapshot()
            .map_err(|e| format!("snapshot governance gate: {e}"))?;
        if snap.breaker.tripped {
            self.ensure_rearm_gate_enqueued(host, snap.breaker.trip_epoch.get(), note);
        } else {
            let mut meta = self.meta.lock().expect("host governance meta poisoned");
            meta.active_rearm_gate_id = None;
            meta.last_event_note = note.to_owned();
        }
        self.publish_snapshot(host, node_id, note)
    }

    /// Crash-consistent durable variant of `apply_operator_estop`: flushes E-Stop latch
    /// to Turso CAS BEFORE publishing to gRPC clients.
    pub async fn apply_operator_estop_durable<S: AgentStorage + Clone + 'static>(
        &self,
        host: &MyGoBridgeHost,
        cas: Option<&StorageCasStore<S>>,
        checkpoint_key: Option<&str>,
        actor: &str,
        reason: &str,
    ) -> Result<GovernanceGateSnapshot, String> {
        let snap = self
            .gate
            .trip_operator_estop(actor, reason)
            .map_err(|e| format!("trip operator e-stop: {e}"))?;
        if let (Some(store), Some(key)) = (cas, checkpoint_key) {
            self.persist_to_cas(store, key).await?;
        }
        let note = format!("operator e-stop tripped by {actor}: {reason}");
        self.ensure_rearm_gate_enqueued(host, snap.breaker.trip_epoch.get(), &note);
        self.publish_snapshot(host, "governance_operator_estop", &note)
    }

    /// Crash-consistent durable variant of `handle_claimed_gate_decision`: flushes updated
    /// breaker/epoch state to Turso CAS BEFORE publishing to gRPC clients.
    pub async fn handle_claimed_gate_decision_durable<S: AgentStorage + Clone + 'static>(
        &self,
        host: &MyGoBridgeHost,
        cas: Option<&StorageCasStore<S>>,
        checkpoint_key: Option<&str>,
        gate_id: &str,
        decision: i32,
        note: Option<&str>,
    ) -> bool {
        if !gate_id.starts_with(GOVERNANCE_REARM_GATE_PREFIX) {
            return false;
        }
        let expected_match = {
            let meta = self.meta.lock().expect("host governance meta poisoned");
            meta.active_rearm_gate_id.as_deref() == Some(gate_id)
        };
        if !expected_match {
            let _ = self.publish_snapshot(
                host,
                "governance_rearm_stale_gate",
                &format!("ignored stale re-arm gate {gate_id}"),
            );
            return true;
        }

        if decision != GraphHumanDecision::Approve as i32 {
            let msg = format!("step-up re-arm rejected on {gate_id}");
            if let Ok(snap) = self.gate.snapshot()
                && snap.breaker.tripped
            {
                self.rotate_rearm_gate(host, snap.breaker.trip_epoch.get(), &msg);
            }
            let _ = self.publish_snapshot(host, "governance_rearm_rejected", &msg);
            return true;
        }

        let actor = match parse_verified_step_up_actor(note) {
            Ok(a) => a,
            Err(err) => {
                let msg = format!("step-up attestation invalid on {gate_id}: {err}");
                if let Ok(snap) = self.gate.snapshot()
                    && snap.breaker.tripped
                {
                    self.rotate_rearm_gate(host, snap.breaker.trip_epoch.get(), &msg);
                }
                let _ = self.publish_snapshot(host, "governance_rearm_denied", &msg);
                return true;
            }
        };

        match self.gate.rearm_by_verified_human(actor) {
            Ok((receipt, _snap)) => {
                if let (Some(store), Some(key)) = (cas, checkpoint_key)
                    && let Err(persist_err) = self.persist_to_cas(store, key).await
                {
                    let msg = format!(
                        "step-up re-arm aborted fail-closed (durable CAS persist failed): {persist_err}"
                    );
                    let _ = self.gate.trip_operator_estop(
                        "governance-host",
                        &format!("rearm-cas-persist-failed:{persist_err}"),
                    );
                    if let Ok(snap) = self.gate.snapshot()
                        && snap.breaker.tripped
                    {
                        self.rotate_rearm_gate(host, snap.breaker.trip_epoch.get(), &msg);
                    }
                    let _ = self.publish_snapshot(host, "governance_rearm_persist_failed", &msg);
                    return true;
                }
                {
                    let mut meta = self.meta.lock().expect("host governance meta poisoned");
                    meta.active_rearm_gate_id = None;
                }
                let msg = format!(
                    "step-up re-arm approved by {} (epoch {} -> {})",
                    receipt.approved_by,
                    receipt.previous_epoch.get(),
                    receipt.new_epoch.get()
                );
                let _ = self.publish_snapshot(host, "governance_rearm_approved", &msg);
            }
            Err(err) => {
                let msg = format!("step-up re-arm denied fail-closed: {err}");
                if let Ok(snap) = self.gate.snapshot()
                    && snap.breaker.tripped
                {
                    self.rotate_rearm_gate(host, snap.breaker.trip_epoch.get(), &msg);
                }
                let _ = self.publish_snapshot(host, "governance_rearm_denied", &msg);
            }
        }
        true
    }

    /// Synchronizes a live Scheme VM 14-slot projection into Rust's `GovernanceAdmissionGate`,
    /// enqueues a one-shot `ApprovalGateService` Step-Up Re-Arm gate if tripped, and publishes
    /// the authoritative snapshot over `ProjectionQueryService`.
    pub fn apply_scheme_projection(
        &self,
        host: &MyGoBridgeHost,
        proj: &GovernancePolicyVectorProjection,
        node_id: &str,
        note: &str,
    ) -> Result<GovernanceGateSnapshot, String> {
        let envelope = projection_to_kernel_envelope(proj)
            .map_err(|e| format!("validate scheme proof envelope: {e}"))?;
        let _receipt = self
            .gate
            .synchronize_proof_envelope(&envelope)
            .map_err(|e| format!("synchronize scheme proof envelope: {e}"))?;
        self.sync_gate_resource_scopes();
        let snap = self
            .gate
            .snapshot()
            .map_err(|e| format!("snapshot governance gate: {e}"))?;
        if snap.breaker.tripped {
            self.ensure_rearm_gate_enqueued(host, snap.breaker.trip_epoch.get(), note);
        } else {
            let mut meta = self.meta.lock().expect("host governance meta poisoned");
            meta.active_rearm_gate_id = None;
            meta.last_event_note = note.to_owned();
        }
        self.publish_snapshot(host, node_id, note)
    }

    /// Trips the authoritative `ExecutionCircuitBreaker` fail-closed on operator E-Stop,
    /// enqueues a one-shot `ApprovalGateService` Step-Up Re-Arm gate, and publishes the update.
    pub fn apply_operator_estop(
        &self,
        host: &MyGoBridgeHost,
        actor: &str,
        reason: &str,
    ) -> Result<GovernanceGateSnapshot, String> {
        let snap = self
            .gate
            .trip_operator_estop(actor, reason)
            .map_err(|e| format!("trip operator e-stop: {e}"))?;
        let note = format!("operator e-stop tripped by {actor}: {reason}");
        self.ensure_rearm_gate_enqueued(host, snap.breaker.trip_epoch.get(), &note);
        self.publish_snapshot(host, "governance_operator_estop", &note)
    }

    /// Processes a claimed one-shot `ApprovalGateService::submit_graph_human_decision` event.
    /// Returns `true` if `gate_id` belonged to this controller's Step-Up Re-Arm workflow.
    pub fn handle_claimed_gate_decision(
        &self,
        host: &MyGoBridgeHost,
        gate_id: &str,
        decision: i32,
        note: Option<&str>,
    ) -> bool {
        if !gate_id.starts_with(GOVERNANCE_REARM_GATE_PREFIX) {
            return false;
        }
        let expected_match = {
            let meta = self.meta.lock().expect("host governance meta poisoned");
            meta.active_rearm_gate_id.as_deref() == Some(gate_id)
        };
        if !expected_match {
            let _ = self.publish_snapshot(
                host,
                "governance_rearm_stale_gate",
                &format!("ignored stale re-arm gate {gate_id}"),
            );
            return true;
        }

        if decision != GraphHumanDecision::Approve as i32 {
            let msg = format!("step-up re-arm rejected on {gate_id}");
            if let Ok(snap) = self.gate.snapshot()
                && snap.breaker.tripped
            {
                self.rotate_rearm_gate(host, snap.breaker.trip_epoch.get(), &msg);
            }
            let _ = self.publish_snapshot(host, "governance_rearm_rejected", &msg);
            return true;
        }

        let actor = match parse_verified_step_up_actor(note) {
            Ok(a) => a,
            Err(err) => {
                let msg = format!("step-up attestation invalid on {gate_id}: {err}");
                if let Ok(snap) = self.gate.snapshot()
                    && snap.breaker.tripped
                {
                    self.rotate_rearm_gate(host, snap.breaker.trip_epoch.get(), &msg);
                }
                let _ = self.publish_snapshot(host, "governance_rearm_denied", &msg);
                return true;
            }
        };

        match self.gate.rearm_by_verified_human(actor) {
            Ok((receipt, _snap)) => {
                {
                    let mut meta = self.meta.lock().expect("host governance meta poisoned");
                    meta.active_rearm_gate_id = None;
                }
                let msg = format!(
                    "step-up re-arm approved by {} (epoch {} -> {})",
                    receipt.approved_by,
                    receipt.previous_epoch.get(),
                    receipt.new_epoch.get()
                );
                let _ = self.publish_snapshot(host, "governance_rearm_approved", &msg);
            }
            Err(err) => {
                let msg = format!("step-up re-arm denied fail-closed: {err}");
                if let Ok(snap) = self.gate.snapshot()
                    && snap.breaker.tripped
                {
                    self.rotate_rearm_gate(host, snap.breaker.trip_epoch.get(), &msg);
                }
                let _ = self.publish_snapshot(host, "governance_rearm_denied", &msg);
            }
        }
        true
    }

    fn ensure_rearm_gate_enqueued(
        &self,
        host: &MyGoBridgeHost,
        trip_epoch: u64,
        prompt_note: &str,
    ) {
        let existing = {
            let meta = self.meta.lock().expect("host governance meta poisoned");
            meta.active_rearm_gate_id.clone()
        };
        if existing.is_some() {
            return;
        }
        self.rotate_rearm_gate(host, trip_epoch, prompt_note);
    }

    fn rotate_rearm_gate(&self, host: &MyGoBridgeHost, trip_epoch: u64, prompt_note: &str) {
        let gate_id = {
            let mut meta = self.meta.lock().expect("host governance meta poisoned");
            meta.rearm_gate_seq = meta.rearm_gate_seq.saturating_add(1);
            let id = if meta.rearm_gate_seq == 1 {
                format!("{GOVERNANCE_REARM_GATE_PREFIX}{trip_epoch}")
            } else {
                format!(
                    "{GOVERNANCE_REARM_GATE_PREFIX}{trip_epoch}-r{}",
                    meta.rearm_gate_seq
                )
            };
            meta.active_rearm_gate_id = Some(id.clone());
            id
        };
        host.enqueue_pending_gate(PendingGate {
            gate_id,
            kind: GateKind::GraphHumanGate as i32,
            turn: None,
            call_id: None,
            tool_or_node_name: "governance.breaker.rearm".to_owned(),
            prompt: format!(
                "RFC 9470 Human Step-Up required to re-arm ExecutionCircuitBreaker (epoch {trip_epoch}): {prompt_note}"
            ),
            details_heading: "Rust+Scheme Governance Authority E-Stop Re-Arm".to_owned(),
            choices: Vec::new(),
            card: None,
            created_at_unix_ms: 1,
            expires_at_unix_ms: None,
        });
    }

    /// Publishes the authoritative Rust+Scheme governance snapshot into both
    /// `QueryProjectGraph` (`GraphVisibleSurface::Evidence`) and live `WatchGraphTrajectory`.
    pub fn publish_snapshot(
        &self,
        host: &MyGoBridgeHost,
        node_id: &str,
        note: &str,
    ) -> Result<GovernanceGateSnapshot, String> {
        let snap = self
            .gate
            .snapshot()
            .map_err(|e| format!("snapshot governance gate: {e}"))?;
        let (rearm_gate_id, facts_snapshot, last_probe) = {
            let mut meta = self.meta.lock().expect("host governance meta poisoned");
            meta.last_event_note = note.to_owned();
            (
                meta.active_rearm_gate_id.clone().unwrap_or_default(),
                meta.facts.clone(),
                meta.last_cedar_probe.clone(),
            )
        };

        let facets = encode_snapshot_facets(
            &snap,
            &facts_snapshot,
            last_probe.as_ref(),
            &rearm_gate_id,
            note,
        );
        let record = GraphRecordView {
            record_id: GOVERNANCE_AUTHORITY_RECORD_ID.to_owned(),
            surface: GraphVisibleSurface::Evidence as i32,
            kind: GOVERNANCE_AUTHORITY_RECORD_KIND.to_owned(),
            title: "Scheme Governance Authority Snapshot".to_owned(),
            summary: format!(
                "admitted={} breaker_tripped={} trip_epoch={} rev_epoch={} fenced={} retracted={} cedar={}",
                snap.admitted,
                snap.breaker.tripped,
                snap.breaker.trip_epoch.get(),
                snap.revocation_epoch.get(),
                snap.fenced_actors.len(),
                snap.retracted_conclusions.len(),
                snap.cedar_policies.len(),
            ),
            source_path: Some("scheme/policy/governance.ss".to_owned()),
            start_line: None,
            end_line: None,
            facets: facets.clone(),
        };
        self.records.upsert_and_publish(host, record);

        let seq = GOVERNANCE_TRAJECTORY_SEQ.fetch_add(1, Ordering::Relaxed);
        host.publish_graph_trajectory_event(GraphTrajectoryEvent {
            event_id: format!("scheme-gov-ev-{seq}"),
            run_id: GOVERNANCE_AUTHORITY_RUN_ID.to_owned(),
            node_id: node_id.to_owned(),
            node_kind: GOVERNANCE_AUTHORITY_NODE_KIND.to_owned(),
            status: if snap.breaker.tripped {
                "tripped".to_owned()
            } else {
                "armed".to_owned()
            },
            node_receipt: None,
            metadata: facets,
        });
        Ok(snap)
    }
}

fn encode_snapshot_facets(
    snap: &GovernanceGateSnapshot,
    facts: &GovernanceFactStore,
    last_probe: Option<&CedarProbeResult>,
    rearm_gate_id: &str,
    note: &str,
) -> HashMap<String, String> {
    #[derive(serde::Serialize)]
    struct CedarWireEntry<'a> {
        policy_id: &'a str,
        source: &'a str,
    }
    #[derive(serde::Serialize)]
    struct DelegationWireEntry<'a> {
        parent: &'a str,
        child: &'a str,
    }
    #[derive(serde::Serialize)]
    struct TaintEdgeWireEntry<'a> {
        src: &'a str,
        dst: &'a str,
    }
    #[derive(serde::Serialize)]
    struct ContextFlowWireEntry<'a> {
        entry: &'a str,
        src_scope: &'a str,
        target: &'a str,
        dst_scope: &'a str,
    }
    #[derive(serde::Serialize)]
    struct DerivationWireEntry<'a> {
        derivation_id: &'a str,
        policy_id: &'a str,
        subject_premises: &'a [String],
        parent_derivations: &'a [String],
    }

    let fenced: Vec<&str> = snap
        .fenced_actors
        .iter()
        .map(GovernedSubjectName::as_str)
        .collect();
    let blocked: Vec<&str> = snap
        .blocked_tools
        .iter()
        .map(GovernedSubjectName::as_str)
        .collect();
    let retracted: Vec<&str> = snap
        .retracted_conclusions
        .iter()
        .map(DerivationId::as_str)
        .collect();
    let cedar: Vec<CedarWireEntry<'_>> = snap
        .cedar_policies
        .iter()
        .map(|p| CedarWireEntry {
            policy_id: p.policy_id.as_str(),
            source: p.source.as_str(),
        })
        .collect();
    let fact_delegations: Vec<DelegationWireEntry<'_>> = facts
        .delegation_edges
        .iter()
        .map(|(parent, child)| DelegationWireEntry {
            parent: parent.as_str(),
            child: child.as_str(),
        })
        .collect();
    let fact_revoked: Vec<&str> = facts.revoked_actors.iter().map(String::as_str).collect();
    let fact_flows: Vec<ContextFlowWireEntry<'_>> = facts
        .context_flows
        .iter()
        .map(|(entry, target)| ContextFlowWireEntry {
            entry: entry.as_str(),
            src_scope: facts
                .context_entries
                .get(entry)
                .map(String::as_str)
                .unwrap_or(""),
            target: target.as_str(),
            dst_scope: facts
                .target_scopes
                .get(target)
                .map(String::as_str)
                .unwrap_or(""),
        })
        .collect();
    let fact_declassified: Vec<&str> = facts
        .declassified_entries
        .iter()
        .map(String::as_str)
        .collect();
    let fact_retracted: Vec<&str> = facts
        .retracted_subjects
        .iter()
        .map(String::as_str)
        .collect();
    let fact_derivations: Vec<DerivationWireEntry<'_>> = facts
        .derivations
        .values()
        .map(|d| DerivationWireEntry {
            derivation_id: d.derivation_id.as_str(),
            policy_id: d.policy_id.as_str(),
            subject_premises: &d.subject_premises,
            parent_derivations: &d.parent_derivations,
        })
        .collect();
    let fact_taint_edges: Vec<TaintEdgeWireEntry<'_>> = facts
        .taint_edges
        .iter()
        .map(|(src, dst)| TaintEdgeWireEntry {
            src: src.as_str(),
            dst: dst.as_str(),
        })
        .collect();
    let fact_taint_sources: Vec<&str> = facts.taint_sources.iter().map(String::as_str).collect();
    let fact_taint_sinks: Vec<&str> = facts.taint_sinks.iter().map(String::as_str).collect();
    let fact_taint_sanitizers: Vec<&str> =
        facts.taint_sanitizers.iter().map(String::as_str).collect();

    let mut facets = HashMap::new();
    facets.insert("admitted".to_owned(), snap.admitted.to_string());
    facets.insert(
        "revocation_epoch".to_owned(),
        snap.revocation_epoch.get().to_string(),
    );
    facets.insert(
        "policy_revision".to_owned(),
        snap.policy_revision.to_string(),
    );
    facets.insert(
        "breaker_tripped".to_owned(),
        snap.breaker.tripped.to_string(),
    );
    facets.insert(
        "breaker_trip_epoch".to_owned(),
        snap.breaker.trip_epoch.get().to_string(),
    );
    facets.insert(
        "breaker_budget_used".to_owned(),
        snap.breaker.budget_used.to_string(),
    );
    facets.insert(
        "breaker_budget_limit".to_owned(),
        snap.breaker.budget_limit.to_string(),
    );
    facets.insert(
        "breaker_active_leases".to_owned(),
        snap.breaker.active_lease_count.to_string(),
    );
    facets.insert("rearm_gate_id".to_owned(), rearm_gate_id.to_owned());
    facets.insert(
        "assessment_digest".to_owned(),
        snap.assessment_digest
            .as_ref()
            .map(|d| d.as_str().to_owned())
            .unwrap_or_default(),
    );
    facets.insert(
        "taint_proof_digest".to_owned(),
        snap.taint_proof_digest
            .as_ref()
            .map(|d| d.as_str().to_owned())
            .unwrap_or_default(),
    );
    facets.insert(
        "fixpoint_proof_digest".to_owned(),
        snap.fixpoint_proof_digest
            .as_ref()
            .map(|d| d.as_str().to_owned())
            .unwrap_or_default(),
    );
    facets.insert(
        "temporal_proof_digest".to_owned(),
        snap.temporal_proof_digest
            .as_ref()
            .map(|d| d.as_str().to_owned())
            .unwrap_or_default(),
    );
    facets.insert(
        "invalidation_digest".to_owned(),
        snap.invalidation_digest
            .as_ref()
            .map(|d| d.as_str().to_owned())
            .unwrap_or_default(),
    );
    facets.insert(
        "rejection_reasons_json".to_owned(),
        serde_json::to_string(&snap.rejection_reasons).unwrap_or_else(|_| "[]".to_owned()),
    );
    facets.insert(
        "fenced_actors_json".to_owned(),
        serde_json::to_string(&fenced).unwrap_or_else(|_| "[]".to_owned()),
    );
    facets.insert(
        "blocked_tools_json".to_owned(),
        serde_json::to_string(&blocked).unwrap_or_else(|_| "[]".to_owned()),
    );
    facets.insert(
        "retracted_conclusions_json".to_owned(),
        serde_json::to_string(&retracted).unwrap_or_else(|_| "[]".to_owned()),
    );
    facets.insert(
        "cedar_policies_json".to_owned(),
        serde_json::to_string(&cedar).unwrap_or_else(|_| "[]".to_owned()),
    );
    facets.insert(
        "fact_delegations_json".to_owned(),
        serde_json::to_string(&fact_delegations).unwrap_or_else(|_| "[]".to_owned()),
    );
    facets.insert(
        "fact_revoked_actors_json".to_owned(),
        serde_json::to_string(&fact_revoked).unwrap_or_else(|_| "[]".to_owned()),
    );
    facets.insert(
        "fact_context_flows_json".to_owned(),
        serde_json::to_string(&fact_flows).unwrap_or_else(|_| "[]".to_owned()),
    );
    facets.insert(
        "fact_declassified_entries_json".to_owned(),
        serde_json::to_string(&fact_declassified).unwrap_or_else(|_| "[]".to_owned()),
    );
    facets.insert(
        "fact_retracted_subjects_json".to_owned(),
        serde_json::to_string(&fact_retracted).unwrap_or_else(|_| "[]".to_owned()),
    );
    facets.insert(
        "fact_derivations_json".to_owned(),
        serde_json::to_string(&fact_derivations).unwrap_or_else(|_| "[]".to_owned()),
    );
    facets.insert(
        "fact_taint_edges_json".to_owned(),
        serde_json::to_string(&fact_taint_edges).unwrap_or_else(|_| "[]".to_owned()),
    );
    facets.insert(
        "fact_taint_sources_json".to_owned(),
        serde_json::to_string(&fact_taint_sources).unwrap_or_else(|_| "[]".to_owned()),
    );
    facets.insert(
        "fact_taint_sinks_json".to_owned(),
        serde_json::to_string(&fact_taint_sinks).unwrap_or_else(|_| "[]".to_owned()),
    );
    facets.insert(
        "fact_taint_sanitizers_json".to_owned(),
        serde_json::to_string(&fact_taint_sanitizers).unwrap_or_else(|_| "[]".to_owned()),
    );
    facets.insert(
        "last_cedar_probe_json".to_owned(),
        match last_probe {
            Some(p) => serde_json::to_string(p)
                .unwrap_or_else(|e| format!("{{\"serialization_error\":{e:?}}}")),
            None => String::new(),
        },
    );
    facets.insert("last_event_note".to_owned(), note.to_owned());
    facets
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ui_command_parser_denies_rearm_bypass_and_allows_estop_and_eval() {
        for forbidden in [
            "scheme:governance-rearm ops-alice",
            "scheme:governance-rearm",
            "governance-rearm",
            "scheme:task-human-approve",
            "scheme:unknown",
            "",
        ] {
            assert_eq!(
                parse_governance_ui_command(forbidden),
                None,
                "privileged bypass `{forbidden}` must fail closed"
            );
        }

        assert_eq!(
            parse_governance_ui_command("scheme:governance-estop ops-alice:red-button"),
            Some(GovernanceUiCommand::OperatorEStop {
                actor: "ops-alice".to_owned(),
                reason: "red-button".to_owned(),
            })
        );
        assert_eq!(
            parse_governance_ui_command("scheme:governance-eval-incident"),
            Some(GovernanceUiCommand::EvaluateIncident)
        );
        assert_eq!(
            parse_governance_ui_command("scheme:governance-eval-clean"),
            Some(GovernanceUiCommand::EvaluateClean)
        );
        assert_eq!(
            parse_governance_ui_command("scheme:governance-delegate ops-lead agent-coder"),
            Some(GovernanceUiCommand::DelegateActor {
                parent: "ops-lead".to_owned(),
                child: "agent-coder".to_owned(),
            })
        );
        assert_eq!(
            parse_governance_ui_command("scheme:governance-revoke ops-lead"),
            Some(GovernanceUiCommand::RevokeActor {
                actor: "ops-lead".to_owned(),
            })
        );
        assert_eq!(
            parse_governance_ui_command(
                "scheme:governance-context-flow payroll-csv private public-webhook external"
            ),
            Some(GovernanceUiCommand::ContextFlow {
                entry: "payroll-csv".to_owned(),
                src_scope: "private".to_owned(),
                target: "public-webhook".to_owned(),
                dst_scope: "external".to_owned(),
            })
        );
        assert_eq!(
            parse_governance_ui_command("scheme:governance-declassify payroll-csv"),
            Some(GovernanceUiCommand::DeclassifyContext {
                entry: "payroll-csv".to_owned(),
            })
        );
        assert_eq!(
            parse_governance_ui_command("scheme:governance-retract human-session-alice"),
            Some(GovernanceUiCommand::RetractPremise {
                subject: "human-session-alice".to_owned(),
            })
        );
        assert_eq!(
            parse_governance_ui_command(
                "scheme:governance-register-derivation D-PLAN pol.plan workspace-ok D-TOKEN"
            ),
            Some(GovernanceUiCommand::RegisterDerivation {
                derivation_id: "D-PLAN".to_owned(),
                policy_id: "pol.plan".to_owned(),
                subject_premises: vec!["workspace-ok".to_owned()],
                parent_derivations: vec!["D-TOKEN".to_owned()],
            })
        );
        assert_eq!(
            parse_governance_ui_command(
                "scheme:governance-register-derivation D-TOKEN pol.cert token-cert-v1 none"
            ),
            Some(GovernanceUiCommand::RegisterDerivation {
                derivation_id: "D-TOKEN".to_owned(),
                policy_id: "pol.cert".to_owned(),
                subject_premises: vec!["token-cert-v1".to_owned()],
                parent_derivations: Vec::new(),
            })
        );
        assert_eq!(
            parse_governance_ui_command(
                "scheme:governance-check-cedar agent-worker rearm_breaker workspace true"
            ),
            Some(GovernanceUiCommand::CheckCedar {
                actor: "agent-worker".to_owned(),
                action: "rearm_breaker".to_owned(),
                resource: "workspace".to_owned(),
                step_up_fresh: true,
            })
        );
    }

    #[test]
    fn step_up_actor_parser_enforces_human_and_rfc9470_acr() {
        assert!(matches!(
            parse_verified_step_up_actor(None).unwrap(),
            BreakerActor::UnverifiedActor(_)
        ));
        assert!(matches!(
            parse_verified_step_up_actor(Some("plain-approval-without-stepup")).unwrap(),
            BreakerActor::UnverifiedActor(_)
        ));
        assert!(matches!(
            parse_verified_step_up_actor(Some(
                "stepup-verified:actor=agent-rogue:acr=urn:ietf:params:oauth:acr:mfa"
            ))
            .unwrap(),
            BreakerActor::Agent(_)
        ));
        assert!(matches!(
            parse_verified_step_up_actor(Some("stepup-verified:actor=ops-alice:acr=")).unwrap(),
            BreakerActor::UnverifiedActor(_)
        ));
        assert!(matches!(
            parse_verified_step_up_actor(Some(
                "stepup-verified:actor=ops-alice:acr=urn:ietf:params:oauth:acr:mfa"
            ))
            .unwrap(),
            BreakerActor::VerifiedHuman(id) if id.as_str() == "ops-alice"
        ));
    }

    #[test]
    fn host_controller_blocks_sandbox_on_estop_and_requires_verified_human_step_up_rearm() {
        let records = Arc::new(SharedProjectionRecords::new());
        let host = MyGoBridgeHost::new_default_trust(|_| Ok(()));
        let ctrl = GovernanceHostController::new(8, records);

        // Armed initially -> clean sandbox call admitted.
        ctrl.authorize_sandbox_effect("ops-bob", "/bin/echo")
            .expect("armed controller allows clean sandbox call");

        // Operator trips E-Stop -> sandbox blocked, one-shot re-arm gate issued.
        let tripped = ctrl
            .apply_operator_estop(&host, "ops-alice", "incident-halt")
            .expect("trip operator estop");
        assert!(tripped.breaker.tripped);
        assert!(!tripped.admitted);
        assert!(
            ctrl.authorize_sandbox_effect("ops-bob", "/bin/echo")
                .is_err(),
            "tripped breaker must block sandbox execution"
        );

        let gate_id = ctrl
            .meta
            .lock()
            .unwrap()
            .active_rearm_gate_id
            .clone()
            .expect("rearm gate enqueued");
        assert_eq!(gate_id, "gov-rearm-epoch-1");

        // Agent impersonation attempt on gate -> denied fail-closed, gate rotated!
        assert!(ctrl.handle_claimed_gate_decision(
            &host,
            &gate_id,
            GraphHumanDecision::Approve as i32,
            Some("stepup-verified:actor=agent-rogue:acr=urn:ietf:params:oauth:acr:mfa"),
        ));
        assert!(ctrl.gate().snapshot().unwrap().breaker.tripped);

        // Next rotated gate approved by verified human with RFC 9470 ACR -> re-armed!
        let next_gate_id = ctrl
            .meta
            .lock()
            .unwrap()
            .active_rearm_gate_id
            .clone()
            .expect("rotated rearm gate");
        assert_ne!(next_gate_id, gate_id);
        assert!(ctrl.handle_claimed_gate_decision(
            &host,
            &next_gate_id,
            GraphHumanDecision::Approve as i32,
            Some("stepup-verified:actor=ops-alice:acr=urn:ietf:params:oauth:acr:mfa"),
        ));
        let armed = ctrl.gate().snapshot().unwrap();
        assert!(!armed.breaker.tripped);
        assert_eq!(armed.breaker.trip_epoch.get(), 2);
        ctrl.authorize_sandbox_effect("ops-bob", "/bin/echo")
            .expect("re-armed controller allows clean sandbox call");
    }

    #[tokio::test]
    async fn host_controller_persists_restores_and_fails_closed_on_tampered_cas_blob() {
        use marlin_agent_storage::{AgentId, InMemoryAgentStorage, ProjectId, SessionId};

        let storage = InMemoryAgentStorage::default();
        let cas = StorageCasStore::new(
            storage,
            ProjectId::new("default").unwrap(),
            SessionId::new("sess-durable-test").unwrap(),
            AgentId::new("agent-durable-test").unwrap(),
        );
        let key = DEFAULT_GOVERNANCE_CHECKPOINT_KEY;

        let host1 = MyGoBridgeHost::new_default_trust(|_| Ok(()));
        let ctrl1 = GovernanceHostController::new(8, Arc::new(SharedProjectionRecords::new()));
        ctrl1
            .apply_operator_estop_durable(&host1, Some(&cas), Some(key), "ops-alice", "pre-crash")
            .await
            .expect("persist estop");

        // Fresh post-restart host restores tripped latch & enqueues one-shot gate.
        let host2 = MyGoBridgeHost::new_default_trust(|_| Ok(()));
        let ctrl2 = GovernanceHostController::new(8, Arc::new(SharedProjectionRecords::new()));
        let (snap2, mode2) = ctrl2
            .restore_from_cas(&host2, &cas, key)
            .await
            .expect("restore valid cas")
            .expect("checkpoint present");
        assert_eq!(mode2, "restored");
        assert!(snap2.breaker.tripped);
        assert!(
            ctrl2
                .authorize_sandbox_effect("ops-bob", "/bin/echo")
                .is_err()
        );

        // Tamper with stored CAS bytes -> fresh host MUST boot fail-closed (`corrupt_fail_closed`)!
        let mut raw = cas.get_blob(key).await.unwrap().unwrap();
        raw[10] ^= 0xAA;
        cas.put_blob(key, raw).await.unwrap();

        let host3 = MyGoBridgeHost::new_default_trust(|_| Ok(()));
        let ctrl3 = GovernanceHostController::new(8, Arc::new(SharedProjectionRecords::new()));
        let (snap3, mode3) = ctrl3
            .restore_from_cas(&host3, &cas, key)
            .await
            .expect("corrupt recovery succeeds fail-closed")
            .expect("corrupt checkpoint observed");
        assert_eq!(mode3, "corrupt_fail_closed");
        assert!(snap3.breaker.tripped);
        assert!(!snap3.admitted);
        assert!(
            ctrl3
                .authorize_sandbox_effect("ops-bob", "/bin/echo")
                .is_err(),
            "corrupted durable checkpoint MUST physically block sandbox execution"
        );
    }

    #[tokio::test]
    async fn host_controller_persists_fact_graph_mgf1_and_fails_closed_on_tampered_facts() {
        use marlin_agent_storage::{AgentId, InMemoryAgentStorage, ProjectId, SessionId};

        let storage = InMemoryAgentStorage::default();
        let cas = StorageCasStore::new(
            storage,
            ProjectId::new("default").unwrap(),
            SessionId::new("sess-mgf1-test").unwrap(),
            AgentId::new("agent-mgf1-test").unwrap(),
        );
        let key = "checkpoint:governance-mgf1-test:v1";

        let ctrl1 = GovernanceHostController::new(8, Arc::new(SharedProjectionRecords::new()));
        ctrl1.seed_fact_store(&default_clean_governance_request(1, 1));
        ctrl1
            .mutate_facts_and_build_request(&GovernanceUiCommand::DelegateActor {
                parent: "ops-root".to_owned(),
                child: "agent-lead".to_owned(),
            })
            .unwrap();
        ctrl1
            .mutate_facts_and_build_request(&GovernanceUiCommand::DelegateActor {
                parent: "agent-lead".to_owned(),
                child: "agent-worker".to_owned(),
            })
            .unwrap();
        ctrl1
            .mutate_facts_and_build_request(&GovernanceUiCommand::RegisterDerivation {
                derivation_id: "D-TOKEN".to_owned(),
                policy_id: "pol.cert".to_owned(),
                subject_premises: vec!["token-cert-v1".to_owned()],
                parent_derivations: Vec::new(),
            })
            .unwrap();
        ctrl1.persist_to_cas(&cas, key).await.unwrap();

        // Fresh post-restart host restores BOTH MGV1 gate and MGF1 v2 fact graph (including derivations)!
        let host2 = MyGoBridgeHost::new_default_trust(|_| Ok(()));
        let ctrl2 = GovernanceHostController::new(8, Arc::new(SharedProjectionRecords::new()));
        let (_snap2, mode2) = ctrl2
            .restore_from_cas(&host2, &cas, key)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(mode2, "restored");

        // Incremental post-restart revoke builds a request retaining PRE-RESTART delegation edges & derivations!
        let (req, _, _) = ctrl2
            .mutate_facts_and_build_request(&GovernanceUiCommand::RevokeActor {
                actor: "agent-lead".to_owned(),
            })
            .unwrap()
            .unwrap();
        assert!(
            req.delegation_edges
                .contains(&("ops-root".to_owned(), "agent-lead".to_owned()))
                && req
                    .delegation_edges
                    .contains(&("agent-lead".to_owned(), "agent-worker".to_owned())),
            "restored MGF1 fact store must preserve pre-restart delegation edges: {:?}",
            req.delegation_edges
        );
        assert!(
            req.derivation_specs.iter().any(
                |d| d.derivation_id == "D-TOKEN" && d.subject_premises == vec!["token-cert-v1"]
            ),
            "restored MGF1 v2 fact store must preserve pre-restart dynamic derivations: {:?}",
            req.derivation_specs
        );
        assert!(req.revoked_actors.contains(&"agent-lead".to_owned()));

        // Bit-flip tamper in companion `:facts` CAS blob MUST auto-trip fail-closed!
        let fact_key = fact_store_checkpoint_key(key);
        let mut raw_facts = cas.get_blob(&fact_key).await.unwrap().unwrap();
        raw_facts[8] ^= 0x55;
        cas.put_blob(&fact_key, raw_facts).await.unwrap();

        let host3 = MyGoBridgeHost::new_default_trust(|_| Ok(()));
        let ctrl3 = GovernanceHostController::new(8, Arc::new(SharedProjectionRecords::new()));
        let (snap3, mode3) = ctrl3
            .restore_from_cas(&host3, &cas, key)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(mode3, "corrupt_fail_closed");
        assert!(snap3.breaker.tripped && !snap3.admitted);
        assert!(
            ctrl3
                .authorize_sandbox_effect("ops-bob", "/bin/echo")
                .is_err()
        );
    }

    #[test]
    fn host_controller_evaluates_live_cedar_probes_with_forbid_override() {
        let host = MyGoBridgeHost::new_default_trust(|_| Ok(()));
        let ctrl = GovernanceHostController::new(8, Arc::new(SharedProjectionRecords::new()));
        let d = |ch: char| format!("sha256:{}", std::iter::repeat_n(ch, 64).collect::<String>());
        let proj = GovernancePolicyVectorProjection {
            schema_version: 1,
            admitted: true,
            revocation_epoch: 2,
            policy_revision: 1,
            assessment_digest: d('a'),
            taint_proof_digest: d('b'),
            fixpoint_proof_digest: d('c'),
            temporal_proof_digest: d('d'),
            invalidation_digest: d('e'),
            rejection_reasons: Vec::new(),
            fenced_actors: Vec::new(),
            blocked_tools: Vec::new(),
            retracted_conclusions: Vec::new(),
            cedar_policies: vec![
                (
                    "marlin.forbid.agent-privileged-governance".to_owned(),
                    "forbid (principal in Marlin::Role::\"Agent\", action in [Marlin::Action::\"rearm_breaker\"], resource);".to_owned(),
                ),
                (
                    "marlin.permit.verified-human-governance".to_owned(),
                    "permit (principal in Marlin::Role::\"VerifiedHuman\", action in [Marlin::Action::\"rearm_breaker\"], resource) when { context.step_up_fresh == true };".to_owned(),
                ),
                (
                    "marlin.ascent.forbid.exfil.secret.ext-webhook".to_owned(),
                    "forbid (principal, action, resource == Marlin::Resource::\"ext-webhook\");".to_owned(),
                ),
                (
                    "marlin.forbid.external-scope-unless-declassified".to_owned(),
                    "forbid (principal, action, resource in Marlin::Scope::\"external\") unless { context.declassified == true };".to_owned(),
                ),
            ],
        };
        ctrl.apply_scheme_projection(&host, &proj, "probe-seed", "seed cedar")
            .unwrap();

        // Register a live 5-scope context flow into `ext-partner` (scope = external):
        let _ = ctrl
            .mutate_facts_and_build_request(&GovernanceUiCommand::ContextFlow {
                entry: "partner-doc".to_owned(),
                src_scope: "private".to_owned(),
                target: "ext-partner".to_owned(),
                dst_scope: "external".to_owned(),
            })
            .unwrap();

        let p1 = ctrl
            .evaluate_cedar_probe(&host, "agent-worker", "rearm_breaker", "workspace", true)
            .unwrap();
        assert_eq!(p1.decision, "deny");
        assert_eq!(
            p1.matched_policy_ids,
            vec!["marlin.forbid.agent-privileged-governance".to_owned()]
        );

        let p2 = ctrl
            .evaluate_cedar_probe(&host, "ops-alice", "rearm_breaker", "workspace", true)
            .unwrap();
        assert_eq!(p2.decision, "allow");
        assert_eq!(
            p2.matched_policy_ids,
            vec!["marlin.permit.verified-human-governance".to_owned()]
        );

        let p3 = ctrl
            .evaluate_cedar_probe(&host, "ops-alice", "rearm_breaker", "workspace", false)
            .unwrap();
        assert_eq!(p3.decision, "implicit_deny");

        let p4 = ctrl
            .evaluate_cedar_probe(&host, "ops-alice", "rearm_breaker", "ext-webhook", true)
            .unwrap();
        assert_eq!(p4.decision, "deny");
        assert_eq!(
            p4.matched_policy_ids,
            vec!["marlin.ascent.forbid.exfil.secret.ext-webhook".to_owned()]
        );

        // Scope-based `forbid ... resource in Marlin::Scope::"external" unless { context.declassified == true }`
        // blocks `ext-partner` BEFORE declassification, then allows AFTER `DeclassifyContext("partner-doc")`!
        let p5 = ctrl
            .evaluate_cedar_probe(&host, "ops-alice", "rearm_breaker", "ext-partner", true)
            .unwrap();
        assert_eq!(p5.decision, "deny");
        assert_eq!(
            p5.matched_policy_ids,
            vec!["marlin.forbid.external-scope-unless-declassified".to_owned()]
        );

        let _ = ctrl
            .mutate_facts_and_build_request(&GovernanceUiCommand::DeclassifyContext {
                entry: "partner-doc".to_owned(),
            })
            .unwrap();
        let p6 = ctrl
            .evaluate_cedar_probe(&host, "ops-alice", "rearm_breaker", "ext-partner", true)
            .unwrap();
        assert_eq!(p6.decision, "allow");
        assert_eq!(
            p6.matched_policy_ids,
            vec!["marlin.permit.verified-human-governance".to_owned()]
        );
    }

    #[test]
    fn mgf1_v2_exhaustive_bitflip_truncation_and_ui_command_injection_refusal() {
        let mut store = GovernanceFactStore::default();
        store
            .delegation_edges
            .insert(("ops-root".to_owned(), "agent-lead".to_owned()));
        store.revoked_actors.insert("agent-lead".to_owned());
        store
            .taint_edges
            .insert(("chat-ext".to_owned(), "os-shell".to_owned()));
        store.taint_sources.insert("chat-ext".to_owned());
        store.taint_sinks.insert("os-shell".to_owned());
        store.taint_sanitizers.insert("human-gate".to_owned());
        store
            .context_entries
            .insert("vault-key".to_owned(), "private".to_owned());
        store
            .context_flows
            .insert(("vault-key".to_owned(), "ext-hook".to_owned()));
        store
            .target_scopes
            .insert("ext-hook".to_owned(), "external".to_owned());
        store.declassified_entries.insert("public-note".to_owned());
        store.retracted_subjects.insert("cert-v1".to_owned());
        store.derivations.insert(
            "D-DEPLOY".to_owned(),
            GovernanceDerivationSpec {
                derivation_id: "D-DEPLOY".to_owned(),
                policy_id: "pol.deploy".to_owned(),
                subject_premises: vec!["cert-v1".to_owned()],
                parent_derivations: vec!["D-TOKEN".to_owned()],
            },
        );

        let valid_blob = store.to_canonical_bytes();
        assert_eq!(
            GovernanceFactStore::from_canonical_bytes(&valid_blob).unwrap(),
            store
        );

        // 1. Every single byte bit-flip across the full MGF1 v2 blob MUST fail SHA-256/schema check!
        for idx in 0..valid_blob.len() {
            let mut tampered = valid_blob.clone();
            tampered[idx] ^= 0x80;
            assert!(
                GovernanceFactStore::from_canonical_bytes(&tampered).is_err(),
                "MGF1 v2 bit-flip at byte {idx} MUST be rejected"
            );
        }

        // 2. Every truncation length 0..len and trailing-byte append MUST be rejected!
        for cut in 0..valid_blob.len() {
            assert!(
                GovernanceFactStore::from_canonical_bytes(&valid_blob[..cut]).is_err(),
                "MGF1 v2 truncation at {cut} MUST be rejected"
            );
        }
        let mut extended = valid_blob.clone();
        extended.push(0x42);
        assert!(GovernanceFactStore::from_canonical_bytes(&extended).is_err());

        // 3. Adversarial UI command injection sweep: re-arm bypass & malformed syntax MUST return None!
        for hostile in [
            "",
            "scheme:",
            "scheme:governance-rearm",
            "scheme:governance-rearm ops-alice",
            "scheme:GOVERNANCE-REARM ops-alice",
            "scheme:governance-rearm\tstepup=true",
            "scheme:governance-rearm\nops-alice",
            "scheme:governance-estop-rearm",
            "scheme:governance-delegate only-one",
            "scheme:governance-delegate a b c",
            "scheme:governance-context-flow a b c",
            "scheme:governance-register-derivation D1 pol -",
            "scheme:governance-register-derivation D1 pol none",
            "scheme:governance-check-cedar a b",
            "governance-eval-clean",
            "scheme:unknown-cmd\0payload",
        ] {
            assert_eq!(
                parse_governance_ui_command(hostile),
                None,
                "hostile UI command {hostile:?} must be rejected fail-closed"
            );
        }
    }

    #[tokio::test]
    async fn mgv1_v2_preserves_sandbox_external_scope_unless_enforcement_without_facts_blob() {
        use marlin_agent_storage::{AgentId, InMemoryAgentStorage, ProjectId, SessionId};

        let storage = InMemoryAgentStorage::default();
        let cas = StorageCasStore::new(
            storage,
            ProjectId::new("default").unwrap(),
            SessionId::new("sess-mgv1-v2-scope").unwrap(),
            AgentId::new("agent-mgv1-v2-scope").unwrap(),
        );
        let key = "checkpoint:gov-scope-mgv1-v2";

        let host1 = MyGoBridgeHost::new_default_trust(|_| Ok(()));
        let ctrl1 = GovernanceHostController::new(8, Arc::new(SharedProjectionRecords::new()));
        let d = |ch: char| format!("sha256:{}", std::iter::repeat_n(ch, 64).collect::<String>());
        let proj = GovernancePolicyVectorProjection {
            schema_version: 1,
            admitted: true,
            revocation_epoch: 2,
            policy_revision: 1,
            assessment_digest: d('a'),
            taint_proof_digest: d('b'),
            fixpoint_proof_digest: d('c'),
            temporal_proof_digest: d('d'),
            invalidation_digest: d('e'),
            rejection_reasons: Vec::new(),
            fenced_actors: Vec::new(),
            blocked_tools: Vec::new(),
            retracted_conclusions: Vec::new(),
            cedar_policies: vec![(
                "marlin.ascent.forbid.scope.external-unless-declassified".to_owned(),
                "forbid (principal, action, resource in Marlin::Scope::\"external\") unless { context.declassified == true };".to_owned(),
            )],
        };
        let _ = ctrl1
            .mutate_facts_and_build_request(&GovernanceUiCommand::ContextFlow {
                entry: "public-report".to_owned(),
                src_scope: "workspace".to_owned(),
                target: "ext-curl".to_owned(),
                dst_scope: "external".to_owned(),
            })
            .unwrap();
        ctrl1
            .apply_scheme_projection_durable(
                &host1,
                Some(&cas),
                Some(key),
                &proj,
                "scope-seed",
                "seed external scope",
            )
            .await
            .unwrap();

        // Simulate reboot where ONLY `MGV1` v2 is persisted (companion `MGF1` `:facts` key absent!).
        let mgv1_bytes = cas.get_blob(key).await.unwrap().unwrap();
        let mgv1_only_key = "checkpoint:gov-scope-mgv1-only";
        cas.put_blob(mgv1_only_key, mgv1_bytes).await.unwrap();

        let host2 = MyGoBridgeHost::new_default_trust(|_| Ok(()));
        let ctrl2 = GovernanceHostController::new(8, Arc::new(SharedProjectionRecords::new()));
        let (_, mode2) = ctrl2
            .restore_from_cas(&host2, &cas, mgv1_only_key)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(mode2, "restored");

        // Sandbox execution targeting `ext-curl` MUST STILL be denied by `MGV1` v2's persisted
        // `resource_scopes` + `marlin.ascent.forbid.scope.external-unless-declassified`!
        let err = ctrl2
            .authorize_sandbox_effect("ops-bob", "ext-curl")
            .expect_err("MGV1 v2 must preserve external-scope sandbox denial even without MGF1");
        assert!(
            err.contains("marlin.ascent.forbid.scope.external-unless-declassified"),
            "unexpected sandbox denial message: {err}"
        );

        // Non-external sandbox binary remains admitted.
        ctrl2
            .authorize_sandbox_effect("ops-bob", "/bin/echo")
            .expect("non-external sandbox program should remain admitted");
    }
}
