// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Authoritative Higher-Order Governance Host Controller for `marlin-scheme-host`.
//!
//! Bridges the live embedded Gambit/Gerbil Scheme VM (`:marlin-agent-core/policy/higher-order-governance`)
//! through zero-JSON 14-slot rooted vectors into `marlin_agent_kernel::HigherOrderGovernanceAdmissionGate`
//! + `ExecutionCircuitBreaker`, enforces side-effect gating on `SchemeAtomicEffect::SandboxRun`,
//! publishes authoritative snapshots over `ProjectionQueryService` (`QueryProjectGraph` +
//! `WatchGraphTrajectory`), and enforces one-shot RFC 9470 human Step-Up Re-Arm strictly through
//! `ApprovalGateService` (never forgeable via `DispatchUiEvent`).

use std::{
    collections::{BTreeMap, HashMap},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

use marlin_agent_app_server::{
    ui_grpc::MyGoBridgeHost,
    wire::marlin::{
        gate::v1::{GateKind, GraphHumanDecision, PendingGate},
        projection::v1::{GraphRecordView, GraphTrajectoryEvent, GraphVisibleSurface},
    },
};
use marlin_agent_kernel::{
    BreakerActor, BreakerActorId, CedarPolicyId, DerivationId, ExecutionCircuitBreaker,
    GovernanceProofDigest, GovernedSubjectName, HigherOrderBridgeError, HigherOrderGateSnapshot,
    HigherOrderGovernanceAdmissionGate, HigherOrderGovernanceProofEnvelope, HigherOrderProofEpoch,
    SynthesizedCedarPolicy, ToolBudgetUnits, ToolIfcContext,
};

use crate::{HigherOrderGovernanceVectorProjection, HigherOrderGovernanceVectorRequest};

pub const HIGHER_ORDER_GOVERNANCE_RECORD_ID: &str = "scheme:higher-order:governance-snapshot";
pub const HIGHER_ORDER_GOVERNANCE_RECORD_KIND: &str = "scheme.higher_order.governance_snapshot";
pub const HIGHER_ORDER_GOVERNANCE_RUN_ID: &str = "scheme-higher-order-governance";
pub const HIGHER_ORDER_GOVERNANCE_NODE_KIND: &str = "scheme_higher_order_governance";
pub const HIGHER_ORDER_GOVERNANCE_REARM_GATE_PREFIX: &str = "gov-rearm-epoch-";

static GOVERNANCE_TRAJECTORY_SEQ: AtomicU64 = AtomicU64::new(1);

/// Shared registry merging multiple non-authorizing `GraphRecordView` records
/// (e.g. `scheme:fleet:audit-capsule` + `scheme:higher-order:governance-snapshot`)
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
            let mut guard = self.records.lock().expect("projection records mutex poisoned");
            guard.insert(record.record_id.clone(), record);
            guard.values().cloned().collect::<Vec<_>>()
        };
        host.set_graph_projection_records(all);
    }
}

/// Strictly allowlisted UI commands for the Higher-Order Governance Host.
///
/// Security invariant: re-arm is deliberately EXCLUDED so UI presentation events can
/// NEVER bypass one-shot `ApprovalGateService` + RFC 9470 verified human step-up.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HigherOrderUiCommand {
    OperatorEStop { actor: String, reason: String },
    EvaluateIncident,
    EvaluateClean,
}

/// Parses declarative `scheme:governance-*` commands dispatched via `DispatchUiEvent`.
/// Any re-arm or privileged bypass attempt is rejected fail-closed (`None`).
pub fn parse_higher_order_governance_ui_command(raw: &str) -> Option<HigherOrderUiCommand> {
    let body = raw.trim().strip_prefix("scheme:")?.trim();
    let (cmd, arg) = match body.split_once(char::is_whitespace) {
        Some((head, tail)) => (head, tail.trim()),
        None => (body, ""),
    };
    match cmd {
        "governance-estop" => {
            let (actor, reason) = if let Some((a, r)) = arg.split_once(':') {
                (a.trim().to_owned(), r.trim().to_owned())
            } else if !arg.is_empty() {
                ("operator".to_owned(), arg.to_owned())
            } else {
                ("operator".to_owned(), "ui-emergency-stop".to_owned())
            };
            Some(HigherOrderUiCommand::OperatorEStop { actor, reason })
        }
        "governance-eval-incident" => Some(HigherOrderUiCommand::EvaluateIncident),
        "governance-eval-clean" => Some(HigherOrderUiCommand::EvaluateClean),
        _ => None,
    }
}

/// Parses RFC 9470 Step-Up attestation from an `ApprovalGateService` human decision note:
/// `stepup-verified:actor=<human-id>:acr=<acr-uri>`.
///
/// Missing step-up markers, empty ACR, or agent-prefixed identities are classified as
/// `UnverifiedActor` / `Agent` so `ExecutionCircuitBreaker::rearm` fails closed.
pub fn parse_verified_step_up_actor(note: Option<&str>) -> Result<BreakerActor, String> {
    let raw = note.unwrap_or("").trim();
    let Some(rest) = raw.strip_prefix("stepup-verified:") else {
        let fallback_id = BreakerActorId::new("unverified-operator")
            .map_err(|e| format!("invalid fallback actor: {e}"))?;
        return Ok(BreakerActor::UnverifiedActor(fallback_id));
    };

    let mut actor = String::new();
    let mut acr = String::new();
    for segment in rest.split(':') {
        if let Some(v) = segment.strip_prefix("actor=") {
            actor = v.trim().to_owned();
        } else if let Some(v) = segment.strip_prefix("acr=") {
            acr = v.trim().to_owned();
        } else if !acr.is_empty() {
            // Preserve colons inside URN ACR values (e.g. `urn:ietf:params:oauth:acr:mfa`).
            acr.push(':');
            acr.push_str(segment);
        }
    }

    if actor.is_empty() {
        let fallback_id = BreakerActorId::new("missing-actor")
            .map_err(|e| format!("invalid missing actor id: {e}"))?;
        return Ok(BreakerActor::UnverifiedActor(fallback_id));
    }

    let actor_id = BreakerActorId::new(actor.clone()).map_err(|e| format!("invalid actor: {e}"))?;
    let lower = actor.to_ascii_lowercase();
    if lower.starts_with("agent") || lower.starts_with("bot") || lower.starts_with("llm") {
        return Ok(BreakerActor::Agent(actor_id));
    }
    if acr.trim().is_empty() {
        return Ok(BreakerActor::UnverifiedActor(actor_id));
    }
    Ok(BreakerActor::VerifiedHuman(actor_id))
}

/// Converts a zero-JSON 14-slot Gambit Scheme vector projection into a validated
/// kernel `HigherOrderGovernanceProofEnvelope`.
pub fn projection_to_kernel_envelope(
    proj: &HigherOrderGovernanceVectorProjection,
) -> Result<HigherOrderGovernanceProofEnvelope, HigherOrderBridgeError> {
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
    Ok(HigherOrderGovernanceProofEnvelope {
        schema_version: proj.schema_version,
        admitted: proj.admitted,
        revocation_epoch: HigherOrderProofEpoch::new(proj.revocation_epoch)?,
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
) -> HigherOrderGovernanceVectorRequest {
    HigherOrderGovernanceVectorRequest {
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
    }
}

/// Clean sanitized governance baseline request fed to live Gerbil Scheme.
pub fn default_clean_governance_request(
    revocation_epoch: u64,
    policy_revision: u64,
) -> HigherOrderGovernanceVectorRequest {
    HigherOrderGovernanceVectorRequest {
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
    }
}

#[derive(Default)]
struct HostGovernanceMeta {
    active_rearm_gate_id: Option<String>,
    rearm_gate_seq: u64,
    policy_revision_seq: u64,
    last_event_note: String,
}

/// Thread-safe Rust control-plane controller synchronizing live Gambit Scheme
/// Higher-Order Governance projections with `ExecutionCircuitBreaker`,
/// `ApprovalGateService` one-shot Step-Up Re-Arm gates, and `ProjectionQueryService`.
pub struct HigherOrderHostController {
    gate: Arc<HigherOrderGovernanceAdmissionGate>,
    records: Arc<SharedProjectionRecords>,
    meta: Mutex<HostGovernanceMeta>,
}

impl HigherOrderHostController {
    pub fn new(budget_limit: u32, records: Arc<SharedProjectionRecords>) -> Self {
        let breaker = Arc::new(ExecutionCircuitBreaker::new(ToolBudgetUnits::new(
            budget_limit.max(1),
        )));
        Self {
            gate: Arc::new(HigherOrderGovernanceAdmissionGate::new(breaker)),
            records,
            meta: Mutex::new(HostGovernanceMeta {
                active_rearm_gate_id: None,
                rearm_gate_seq: 0,
                policy_revision_seq: 1,
                last_event_note: "initialized".to_owned(),
            }),
        }
    }

    pub fn gate(&self) -> Arc<HigherOrderGovernanceAdmissionGate> {
        Arc::clone(&self.gate)
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

    /// Gates a real OS `SandboxRun` atomic effect through `HigherOrderGovernanceAdmissionGate`.
    pub fn authorize_sandbox_effect(&self, actor: &str, program: &str) -> Result<(), String> {
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
        self.gate
            .authorize_side_effect(None, &actor_id, &tool_id, &ToolIfcContext::clean_trusted())
            .map(|_| ())
            .map_err(|e| format!("higher-order governance denied sandbox execution: {e}"))
    }

    /// Synchronizes a live Scheme VM 14-slot projection into Rust's `HigherOrderGovernanceAdmissionGate`,
    /// enqueues a one-shot `ApprovalGateService` Step-Up Re-Arm gate if tripped, and publishes
    /// the authoritative snapshot over `ProjectionQueryService`.
    pub fn apply_scheme_projection(
        &self,
        host: &MyGoBridgeHost,
        proj: &HigherOrderGovernanceVectorProjection,
        node_id: &str,
        note: &str,
    ) -> Result<HigherOrderGateSnapshot, String> {
        let envelope = projection_to_kernel_envelope(proj)
            .map_err(|e| format!("validate scheme proof envelope: {e}"))?;
        let _receipt = self
            .gate
            .synchronize_proof_envelope(&envelope)
            .map_err(|e| format!("synchronize scheme proof envelope: {e}"))?;
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
    ) -> Result<HigherOrderGateSnapshot, String> {
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
        if !gate_id.starts_with(HIGHER_ORDER_GOVERNANCE_REARM_GATE_PREFIX) {
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

    fn ensure_rearm_gate_enqueued(&self, host: &MyGoBridgeHost, trip_epoch: u64, prompt_note: &str) {
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
                format!("{HIGHER_ORDER_GOVERNANCE_REARM_GATE_PREFIX}{trip_epoch}")
            } else {
                format!(
                    "{HIGHER_ORDER_GOVERNANCE_REARM_GATE_PREFIX}{trip_epoch}-r{}",
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
            details_heading: "Rust+Scheme Higher-Order Governance E-Stop Re-Arm".to_owned(),
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
    ) -> Result<HigherOrderGateSnapshot, String> {
        let snap = self
            .gate
            .snapshot()
            .map_err(|e| format!("snapshot governance gate: {e}"))?;
        let rearm_gate_id = {
            let mut meta = self.meta.lock().expect("host governance meta poisoned");
            meta.last_event_note = note.to_owned();
            meta.active_rearm_gate_id.clone().unwrap_or_default()
        };

        let facets = encode_snapshot_facets(&snap, &rearm_gate_id, note);
        let record = GraphRecordView {
            record_id: HIGHER_ORDER_GOVERNANCE_RECORD_ID.to_owned(),
            surface: GraphVisibleSurface::Evidence as i32,
            kind: HIGHER_ORDER_GOVERNANCE_RECORD_KIND.to_owned(),
            title: "Scheme Higher-Order Governance Authority Snapshot".to_owned(),
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
            source_path: Some("scheme/policy/higher-order-governance.ss".to_owned()),
            start_line: None,
            end_line: None,
            facets: facets.clone(),
        };
        self.records.upsert_and_publish(host, record);

        let seq = GOVERNANCE_TRAJECTORY_SEQ.fetch_add(1, Ordering::Relaxed);
        host.publish_graph_trajectory_event(GraphTrajectoryEvent {
            event_id: format!("scheme-gov-ev-{seq}"),
            run_id: HIGHER_ORDER_GOVERNANCE_RUN_ID.to_owned(),
            node_id: node_id.to_owned(),
            node_kind: HIGHER_ORDER_GOVERNANCE_NODE_KIND.to_owned(),
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
    snap: &HigherOrderGateSnapshot,
    rearm_gate_id: &str,
    note: &str,
) -> HashMap<String, String> {
    #[derive(serde::Serialize)]
    struct CedarWireEntry<'a> {
        policy_id: &'a str,
        source: &'a str,
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
                parse_higher_order_governance_ui_command(forbidden),
                None,
                "privileged bypass `{forbidden}` must fail closed"
            );
        }

        assert_eq!(
            parse_higher_order_governance_ui_command("scheme:governance-estop ops-alice:red-button"),
            Some(HigherOrderUiCommand::OperatorEStop {
                actor: "ops-alice".to_owned(),
                reason: "red-button".to_owned(),
            })
        );
        assert_eq!(
            parse_higher_order_governance_ui_command("scheme:governance-eval-incident"),
            Some(HigherOrderUiCommand::EvaluateIncident)
        );
        assert_eq!(
            parse_higher_order_governance_ui_command("scheme:governance-eval-clean"),
            Some(HigherOrderUiCommand::EvaluateClean)
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
        let ctrl = HigherOrderHostController::new(8, records);

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
}
