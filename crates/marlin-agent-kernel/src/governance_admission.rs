// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Authoritative Rust admission gate bridging Gerbil Scheme's Governance Authority
//! Plane (`scheme/policy/governance.ss`) into `ExecutionCircuitBreaker`
//! and `LoopProgramRuntimeSideEffectExecutor`.
//!
//! Zero JSON / text parsing: accepts typed `GovernanceProofEnvelope`
//! values decoded directly from rooted Gambit Scheme vectors by `marlin-gerbil-scheme`,
//! verifies all cryptographic `sha256:` proof bindings and monotonic revocation epochs,
//! enforces Bi-Temporal Truth Maintenance causal retraction blast-radius invalidation
//! (automatically tripping `ExecutionCircuitBreaker` if an active in-flight derivation
//! loses causal support), and enforces ASCENT-derived transitive fencing & Cedar forbids.

use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
    sync::{Arc, Mutex},
};

use crate::execution_breaker::{
    BreakerActor, BreakerRejectionReason, BreakerTripEpoch, BreakerTripReason,
    ExecutionBreakerCommitReceipt, ExecutionBreakerRearmReceipt, ExecutionBreakerSnapshot,
    ExecutionCircuitBreaker, ToolExecutionLease, ToolIfcContext,
};
use crate::governance_cedar::{
    CedarEvaluationDecision, CedarEvaluationRequest, CompiledCedarPolicy, compile_cedar_policy,
    evaluate_compiled_cedar_policies,
};

pub const GOVERNANCE_ADMISSION_SCHEMA_VERSION: u32 = 1;
pub const GOVERNANCE_DURABLE_CHECKPOINT_MAGIC: &[u8; 4] = b"MGV1";
pub const GOVERNANCE_DURABLE_CHECKPOINT_VERSION: u32 = 1;

/// Validated canonical SHA-256 content digest (`sha256:<64 lowercase hex>`).
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct GovernanceProofDigest(String);

impl GovernanceProofDigest {
    pub fn new(value: impl Into<String>) -> Result<Self, GovernanceAdmissionError> {
        let value = value.into();
        let valid = value.len() == 71
            && value.starts_with("sha256:")
            && value.as_bytes()[7..]
                .iter()
                .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'));
        if !valid {
            return Err(GovernanceAdmissionError::InvalidProofDigest(value));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

macro_rules! ho_id {
    ($name:ident, $label:literal) => {
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, GovernanceAdmissionError> {
                let value = value.into();
                if value.is_empty() || value.chars().take(257).count() > 256 {
                    return Err(GovernanceAdmissionError::InvalidIdentifier($label));
                }
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

ho_id!(DerivationId, "truth-maintenance derivation identity");
ho_id!(CedarPolicyId, "synthesized Cedar policy identity");
ho_id!(GovernedSubjectName, "governed actor or tool identity");

/// Monotonic proof revocation epoch (`>= 1`).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct GovernanceProofEpoch(u64);

impl GovernanceProofEpoch {
    pub const INITIAL: Self = Self(1);

    pub fn new(value: u64) -> Result<Self, GovernanceAdmissionError> {
        if value == 0 {
            return Err(GovernanceAdmissionError::InvalidProofEpoch(value));
        }
        Ok(Self(value))
    }

    pub fn get(self) -> u64 {
        self.0
    }
}

/// One Cedar policy synthesized from ASCENT fixpoint + POO governance baselines.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SynthesizedCedarPolicy {
    pub policy_id: CedarPolicyId,
    pub source: String,
}

/// Typed Rust projection of the 14-slot rooted Scheme governance proof vector.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GovernanceProofEnvelope {
    pub schema_version: u32,
    pub admitted: bool,
    pub revocation_epoch: GovernanceProofEpoch,
    pub policy_revision: u64,
    pub assessment_digest: GovernanceProofDigest,
    pub taint_proof_digest: GovernanceProofDigest,
    pub fixpoint_proof_digest: GovernanceProofDigest,
    pub temporal_proof_digest: GovernanceProofDigest,
    pub invalidation_digest: GovernanceProofDigest,
    pub rejection_reasons: Vec<String>,
    pub fenced_actors: Vec<GovernedSubjectName>,
    pub blocked_tools: Vec<GovernedSubjectName>,
    pub retracted_conclusions: Vec<DerivationId>,
    pub cedar_policies: Vec<SynthesizedCedarPolicy>,
}

/// Typed admission receipt emitted when Rust synchronizes a Scheme proof envelope.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GovernanceProofSyncReceipt {
    pub admitted: bool,
    pub revocation_epoch: GovernanceProofEpoch,
    pub invalidated_active_derivations: Vec<DerivationId>,
    pub breaker_auto_tripped: bool,
    pub fenced_actor_count: usize,
    pub blocked_tool_count: usize,
    pub synthesized_cedar_count: usize,
}

/// Authoritative snapshot of the combined Rust `ExecutionCircuitBreaker` and
/// Gerbil Scheme Governance Authority proof state for control-plane projections.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GovernanceGateSnapshot {
    pub admitted: bool,
    pub revocation_epoch: GovernanceProofEpoch,
    pub policy_revision: u64,
    pub breaker: ExecutionBreakerSnapshot,
    pub assessment_digest: Option<GovernanceProofDigest>,
    pub taint_proof_digest: Option<GovernanceProofDigest>,
    pub fixpoint_proof_digest: Option<GovernanceProofDigest>,
    pub temporal_proof_digest: Option<GovernanceProofDigest>,
    pub invalidation_digest: Option<GovernanceProofDigest>,
    pub rejection_reasons: Vec<String>,
    pub fenced_actors: Vec<GovernedSubjectName>,
    pub blocked_tools: Vec<GovernedSubjectName>,
    pub retracted_conclusions: Vec<DerivationId>,
    pub cedar_policies: Vec<SynthesizedCedarPolicy>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GovernanceAdmissionError {
    InvalidIdentifier(&'static str),
    InvalidProofDigest(String),
    InvalidProofEpoch(u64),
    UnsupportedSchemaVersion(u32),
    StaleProofEpoch {
        supplied: GovernanceProofEpoch,
        current_floor: GovernanceProofEpoch,
    },
    DerivationAlreadyRegistered(DerivationId),
    DerivationNotRegistered(DerivationId),
    DerivationRetractedDenied(DerivationId),
    AscentFencedActorDenied(GovernedSubjectName),
    AscentBlockedToolDenied(GovernedSubjectName),
    CedarForbidDenied {
        policy_id: CedarPolicyId,
        subject: GovernedSubjectName,
    },
    InvalidCedarPolicy {
        policy_id: CedarPolicyId,
        detail: String,
    },
    ProofEnvelopeRejected(Vec<String>),
    CorruptDurableCheckpoint(String),
    ExecutionBreaker(BreakerRejectionReason),
    LockPoisoned,
}

impl fmt::Display for GovernanceAdmissionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIdentifier(label) => write!(f, "invalid identifier for {label}"),
            Self::InvalidProofDigest(d) => write!(f, "invalid sha256 proof digest: {d}"),
            Self::InvalidProofEpoch(e) => write!(f, "invalid proof epoch {e}"),
            Self::UnsupportedSchemaVersion(v) => {
                write!(f, "unsupported governance admission schema version {v}")
            }
            Self::StaleProofEpoch {
                supplied,
                current_floor,
            } => write!(
                f,
                "stale governance proof epoch {} < floor {}",
                supplied.get(),
                current_floor.get()
            ),
            Self::DerivationAlreadyRegistered(id) => {
                write!(f, "causal derivation already registered: {id}")
            }
            Self::DerivationNotRegistered(id) => {
                write!(f, "causal derivation lease not registered: {id}")
            }
            Self::DerivationRetractedDenied(id) => {
                write!(
                    f,
                    "causal derivation invalidated by bi-temporal truth maintenance: {id}"
                )
            }
            Self::AscentFencedActorDenied(actor) => {
                write!(
                    f,
                    "actor transitively fenced by ASCENT delegation fixpoint: {actor}"
                )
            }
            Self::AscentBlockedToolDenied(tool) => {
                write!(
                    f,
                    "tool blocked by ASCENT execution breaker fixpoint: {tool}"
                )
            }
            Self::CedarForbidDenied { policy_id, subject } => {
                write!(f, "synthesized Cedar forbid {policy_id} denied {subject}")
            }
            Self::InvalidCedarPolicy { policy_id, detail } => {
                write!(
                    f,
                    "invalid synthesized Cedar policy {policy_id} (fail-closed): {detail}"
                )
            }
            Self::ProofEnvelopeRejected(reasons) => {
                write!(
                    f,
                    "governance proof envelope rejected: {}",
                    reasons.join(",")
                )
            }
            Self::CorruptDurableCheckpoint(detail) => {
                write!(
                    f,
                    "corrupt durable governance checkpoint (fail-closed): {detail}"
                )
            }
            Self::ExecutionBreaker(err) => write!(f, "execution breaker denied: {err}"),
            Self::LockPoisoned => write!(f, "governance admission gate lock poisoned"),
        }
    }
}

impl Error for GovernanceAdmissionError {}

impl From<BreakerRejectionReason> for GovernanceAdmissionError {
    fn from(value: BreakerRejectionReason) -> Self {
        Self::ExecutionBreaker(value)
    }
}

#[derive(Debug)]
struct GateInner {
    min_revocation_epoch: GovernanceProofEpoch,
    policy_revision: u64,
    last_admitted: bool,
    assessment_digest: Option<GovernanceProofDigest>,
    taint_proof_digest: Option<GovernanceProofDigest>,
    fixpoint_proof_digest: Option<GovernanceProofDigest>,
    temporal_proof_digest: Option<GovernanceProofDigest>,
    invalidation_digest: Option<GovernanceProofDigest>,
    rejection_reasons: Vec<String>,
    active_derivations: BTreeMap<DerivationId, ToolExecutionLease>,
    retracted_derivations: BTreeSet<DerivationId>,
    fenced_actors: BTreeSet<GovernedSubjectName>,
    blocked_tools: BTreeSet<GovernedSubjectName>,
    cedar_policies: Vec<SynthesizedCedarPolicy>,
    compiled_cedar_policies: Vec<CompiledCedarPolicy>,
    resource_scopes: BTreeMap<GovernedSubjectName, String>,
}

/// Authoritative Rust admission gate synchronizing Scheme Governance Authority
/// proofs (`POO × ASCENT × Bi-Temporal Truth Maintenance × Cedar`) directly into
/// `ExecutionCircuitBreaker` and live causal derivation leases.
#[derive(Clone, Debug)]
pub struct GovernanceAdmissionGate {
    breaker: Arc<ExecutionCircuitBreaker>,
    inner: Arc<Mutex<GateInner>>,
}

impl GovernanceAdmissionGate {
    pub fn new(breaker: Arc<ExecutionCircuitBreaker>) -> Self {
        Self {
            breaker,
            inner: Arc::new(Mutex::new(GateInner {
                min_revocation_epoch: GovernanceProofEpoch::INITIAL,
                policy_revision: 0,
                last_admitted: true,
                assessment_digest: None,
                taint_proof_digest: None,
                fixpoint_proof_digest: None,
                temporal_proof_digest: None,
                invalidation_digest: None,
                rejection_reasons: Vec::new(),
                active_derivations: BTreeMap::new(),
                retracted_derivations: BTreeSet::new(),
                fenced_actors: BTreeSet::new(),
                blocked_tools: BTreeSet::new(),
                cedar_policies: Vec::new(),
                compiled_cedar_policies: Vec::new(),
                resource_scopes: BTreeMap::new(),
            })),
        }
    }

    /// Registers one resource's 5-scope classification (e.g. `"external"`, `"private"`)
    /// so `authorize_side_effect_on_resource` automatically attaches `Marlin::Scope::"<scope>"`
    /// during compiled Cedar evaluation.
    pub fn set_resource_scope(
        &self,
        resource: GovernedSubjectName,
        scope: impl Into<String>,
    ) -> Result<(), GovernanceAdmissionError> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| GovernanceAdmissionError::LockPoisoned)?;
        let scope_str = scope.into();
        if !scope_str.trim().is_empty() {
            guard.resource_scopes.insert(resource, scope_str);
        }
        Ok(())
    }

    /// Atomically replaces the gate's live resource-to-scope map (e.g. synced from `MGF1` v2).
    pub fn replace_resource_scopes(
        &self,
        scopes: BTreeMap<GovernedSubjectName, String>,
    ) -> Result<(), GovernanceAdmissionError> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| GovernanceAdmissionError::LockPoisoned)?;
        guard.resource_scopes = scopes;
        Ok(())
    }

    pub fn execution_breaker(&self) -> Arc<ExecutionCircuitBreaker> {
        Arc::clone(&self.breaker)
    }

    pub fn min_revocation_epoch(&self) -> Result<GovernanceProofEpoch, GovernanceAdmissionError> {
        let guard = self
            .inner
            .lock()
            .map_err(|_| GovernanceAdmissionError::LockPoisoned)?;
        Ok(guard.min_revocation_epoch)
    }

    /// Returns the next monotonic revocation epoch strictly greater than the current floor.
    pub fn next_revocation_epoch(&self) -> Result<u64, GovernanceAdmissionError> {
        let guard = self
            .inner
            .lock()
            .map_err(|_| GovernanceAdmissionError::LockPoisoned)?;
        Ok(guard.min_revocation_epoch.get().saturating_add(1))
    }

    /// Captures an immutable snapshot of the combined Rust breaker and Scheme proof state.
    pub fn snapshot(&self) -> Result<GovernanceGateSnapshot, GovernanceAdmissionError> {
        let guard = self
            .inner
            .lock()
            .map_err(|_| GovernanceAdmissionError::LockPoisoned)?;
        let breaker = self.breaker.snapshot()?;
        Ok(GovernanceGateSnapshot {
            admitted: guard.last_admitted && !breaker.tripped,
            revocation_epoch: guard.min_revocation_epoch,
            policy_revision: guard.policy_revision,
            breaker,
            assessment_digest: guard.assessment_digest.clone(),
            taint_proof_digest: guard.taint_proof_digest.clone(),
            fixpoint_proof_digest: guard.fixpoint_proof_digest.clone(),
            temporal_proof_digest: guard.temporal_proof_digest.clone(),
            invalidation_digest: guard.invalidation_digest.clone(),
            rejection_reasons: guard.rejection_reasons.clone(),
            fenced_actors: guard.fenced_actors.iter().cloned().collect(),
            blocked_tools: guard.blocked_tools.iter().cloned().collect(),
            retracted_conclusions: guard.retracted_derivations.iter().cloned().collect(),
            cedar_policies: guard.cedar_policies.clone(),
        })
    }

    /// Trips the underlying `ExecutionCircuitBreaker` fail-closed on behalf of a human
    /// operator E-Stop action and records the reason on the gate snapshot.
    pub fn trip_operator_estop(
        &self,
        actor: &str,
        reason: &str,
    ) -> Result<GovernanceGateSnapshot, GovernanceAdmissionError> {
        let actor_clean = actor.trim();
        let actor_label = if actor_clean.is_empty() {
            "operator"
        } else {
            actor_clean
        };
        let reason_clean = reason.trim();
        let reason_label = if reason_clean.is_empty() {
            "operator-emergency-stop"
        } else {
            reason_clean
        };
        let formatted = format!("operator-estop:{actor_label}:{reason_label}");
        let trip_reason = BreakerTripReason::new(formatted.clone())?;
        self.breaker.trip(trip_reason)?;
        {
            let mut guard = self
                .inner
                .lock()
                .map_err(|_| GovernanceAdmissionError::LockPoisoned)?;
            guard.last_admitted = false;
            guard.rejection_reasons.insert(0, formatted);
            if guard.rejection_reasons.len() > 16 {
                guard.rejection_reasons.truncate(16);
            }
        }
        self.snapshot()
    }

    /// Re-arms a tripped `ExecutionCircuitBreaker` strictly for a verified human operator
    /// (enforcing human-only re-arm), advances the proof revocation floor monotonically,
    /// clears transient trip rejections, and preserves ASCENT/Cedar per-actor fences until
    /// superseded by a fresh Scheme governance proof.
    pub fn rearm_by_verified_human(
        &self,
        actor: BreakerActor,
    ) -> Result<(ExecutionBreakerRearmReceipt, GovernanceGateSnapshot), GovernanceAdmissionError>
    {
        let receipt = self.breaker.rearm(&actor)?;
        {
            let mut guard = self
                .inner
                .lock()
                .map_err(|_| GovernanceAdmissionError::LockPoisoned)?;
            let next_epoch =
                GovernanceProofEpoch::new(guard.min_revocation_epoch.get().saturating_add(1))?;
            guard.min_revocation_epoch = next_epoch;
            guard.last_admitted = true;
            guard.rejection_reasons.clear();
            guard.retracted_derivations.clear();
        }
        Ok((receipt, self.snapshot()?))
    }

    /// Registers a live in-flight `ToolExecutionLease` tagged with its causal `DerivationId`
    /// so Bi-Temporal Truth Maintenance retraction can surgically revoke it.
    pub fn register_derived_lease(
        &self,
        derivation_id: DerivationId,
        lease: ToolExecutionLease,
    ) -> Result<(), GovernanceAdmissionError> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| GovernanceAdmissionError::LockPoisoned)?;
        if guard.retracted_derivations.contains(&derivation_id) {
            return Err(GovernanceAdmissionError::DerivationRetractedDenied(
                derivation_id,
            ));
        }
        if guard.active_derivations.contains_key(&derivation_id) {
            return Err(GovernanceAdmissionError::DerivationAlreadyRegistered(
                derivation_id,
            ));
        }
        guard.active_derivations.insert(derivation_id, lease);
        Ok(())
    }

    /// Synchronizes a proof envelope produced by Scheme's Governance Authority engine.
    ///
    /// Fail-closed invariants:
    /// 1. Rejects unsupported schema versions and stale `revocation_epoch` replays.
    /// 2. Immediately fences every active derivation listed in `retracted_conclusions`
    ///    (exact Bi-Temporal Truth Maintenance reverse-dependency blast radius).
    /// 3. Automatically trips `ExecutionCircuitBreaker` if `!envelope.admitted` OR if any
    ///    in-flight active derivation was invalidated by a causal retraction.
    /// 4. Installs ASCENT-derived `fenced_actors`, `blocked_tools`, and synthesized Cedar policies.
    pub fn synchronize_proof_envelope(
        &self,
        envelope: &GovernanceProofEnvelope,
    ) -> Result<GovernanceProofSyncReceipt, GovernanceAdmissionError> {
        if envelope.schema_version != GOVERNANCE_ADMISSION_SCHEMA_VERSION {
            return Err(GovernanceAdmissionError::UnsupportedSchemaVersion(
                envelope.schema_version,
            ));
        }

        // Compile every Scheme-synthesized Cedar policy BEFORE mutating live gate state
        // so malformed policy syntax fails closed atomically.
        let mut compiled_cedar = Vec::with_capacity(envelope.cedar_policies.len());
        for policy in &envelope.cedar_policies {
            compiled_cedar.push(compile_cedar_policy(policy)?);
        }

        let mut guard = self
            .inner
            .lock()
            .map_err(|_| GovernanceAdmissionError::LockPoisoned)?;

        if envelope.revocation_epoch < guard.min_revocation_epoch {
            return Err(GovernanceAdmissionError::StaleProofEpoch {
                supplied: envelope.revocation_epoch,
                current_floor: guard.min_revocation_epoch,
            });
        }
        guard.min_revocation_epoch = envelope.revocation_epoch;
        guard.policy_revision = envelope.policy_revision;
        guard.last_admitted = envelope.admitted;
        guard.assessment_digest = Some(envelope.assessment_digest.clone());
        guard.taint_proof_digest = Some(envelope.taint_proof_digest.clone());
        guard.fixpoint_proof_digest = Some(envelope.fixpoint_proof_digest.clone());
        guard.temporal_proof_digest = Some(envelope.temporal_proof_digest.clone());
        guard.invalidation_digest = Some(envelope.invalidation_digest.clone());
        guard.rejection_reasons = envelope.rejection_reasons.clone();

        guard.fenced_actors = envelope.fenced_actors.iter().cloned().collect();
        guard.blocked_tools = envelope.blocked_tools.iter().cloned().collect();
        guard.cedar_policies = envelope.cedar_policies.clone();
        guard.compiled_cedar_policies = compiled_cedar;

        let mut invalidated_active = Vec::new();
        for deriv_id in &envelope.retracted_conclusions {
            guard.retracted_derivations.insert(deriv_id.clone());
            if let Some(lease) = guard.active_derivations.remove(deriv_id) {
                let _ = self.breaker.cancel_stale_lease(&lease.lease_id);
                invalidated_active.push(deriv_id.clone());
            }
        }

        let mut auto_tripped = false;
        if !envelope.admitted || !invalidated_active.is_empty() {
            let reason_text = if !invalidated_active.is_empty() {
                format!(
                    "causal-retraction:{}",
                    invalidated_active
                        .iter()
                        .map(DerivationId::as_str)
                        .collect::<Vec<_>>()
                        .join(",")
                )
            } else if !envelope.rejection_reasons.is_empty() {
                format!("governance-denied:{}", envelope.rejection_reasons.join(","))
            } else {
                "governance-proof-not-admitted".to_owned()
            };
            let reason = BreakerTripReason::new(reason_text)?;
            self.breaker.trip(reason)?;
            auto_tripped = true;
        }

        Ok(GovernanceProofSyncReceipt {
            admitted: envelope.admitted,
            revocation_epoch: envelope.revocation_epoch,
            invalidated_active_derivations: invalidated_active,
            breaker_auto_tripped: auto_tripped,
            fenced_actor_count: guard.fenced_actors.len(),
            blocked_tool_count: guard.blocked_tools.len(),
            synthesized_cedar_count: guard.cedar_policies.len(),
        })
    }

    /// Commits a causally derived side effect ONLY IF:
    /// 1. The derivation has NOT been invalidated by Bi-Temporal Truth Maintenance;
    /// 2. The actor is NOT transitively fenced by ASCENT delegation closure;
    /// 3. The tool is NOT blocked by ASCENT breaker fixpoint;
    /// 4. No synthesized Cedar `forbid` policy denies the actor or tool;
    /// 5. The authoritative `ExecutionCircuitBreaker` is armed, epoch-current, within budget,
    ///    and CaMeL IFC clean/declassified.
    pub fn commit_derived_effect(
        &self,
        derivation_id: &DerivationId,
        actor: &GovernedSubjectName,
        tool: &GovernedSubjectName,
        ifc: &ToolIfcContext,
    ) -> Result<ExecutionBreakerCommitReceipt, GovernanceAdmissionError> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| GovernanceAdmissionError::LockPoisoned)?;

        if guard.retracted_derivations.contains(derivation_id) {
            return Err(GovernanceAdmissionError::DerivationRetractedDenied(
                derivation_id.clone(),
            ));
        }

        if guard.fenced_actors.contains(actor) {
            return Err(GovernanceAdmissionError::AscentFencedActorDenied(
                actor.clone(),
            ));
        }

        if guard.blocked_tools.contains(tool) {
            return Err(GovernanceAdmissionError::AscentBlockedToolDenied(
                tool.clone(),
            ));
        }

        // Evaluate compiled Scheme-synthesized Cedar policies under strict forbid-overrides-permit.
        let default_resource = GovernedSubjectName::new("workspace")?;
        let cedar_req = CedarEvaluationRequest::for_side_effect(
            actor,
            tool,
            &default_resource,
            ifc.tool_controlled_by_tainted_data,
            ifc.declassification_approved,
        );
        if let CedarEvaluationDecision::Deny { matching_forbids } =
            evaluate_compiled_cedar_policies(&guard.compiled_cedar_policies, &cedar_req)
        {
            return Err(GovernanceAdmissionError::CedarForbidDenied {
                policy_id: matching_forbids[0].clone(),
                subject: actor.clone(),
            });
        }

        let Some(lease) = guard.active_derivations.get(derivation_id).cloned() else {
            return Err(GovernanceAdmissionError::DerivationNotRegistered(
                derivation_id.clone(),
            ));
        };

        let receipt = self.breaker.commit_effect_with_ifc(&lease, ifc)?;
        guard.active_derivations.remove(derivation_id);
        Ok(receipt)
    }

    /// Evaluates an explicit structured Cedar authorization request against the currently
    /// synchronized compiled Cedar policies using true `forbid`-overrides-`permit` semantics.
    pub fn evaluate_cedar_request(
        &self,
        req: &CedarEvaluationRequest,
    ) -> Result<CedarEvaluationDecision, GovernanceAdmissionError> {
        let guard = self
            .inner
            .lock()
            .map_err(|_| GovernanceAdmissionError::LockPoisoned)?;
        Ok(evaluate_compiled_cedar_policies(
            &guard.compiled_cedar_policies,
            req,
        ))
    }

    /// Authorizes an atomic side effect directly from `LoopProgramRuntimeSideEffectExecutor`:
    /// enforces Bi-Temporal Truth Maintenance retraction, ASCENT transitive fencing,
    /// ASCENT breaker-blocked tools, compiled Scheme-synthesized Cedar `forbid` policies,
    /// and the authoritative `ExecutionCircuitBreaker` + CaMeL IFC context in one call.
    pub fn authorize_side_effect(
        &self,
        derivation_id: Option<&DerivationId>,
        actor: &GovernedSubjectName,
        tool: &GovernedSubjectName,
        ifc: &ToolIfcContext,
    ) -> Result<ExecutionBreakerCommitReceipt, GovernanceAdmissionError> {
        let default_resource = GovernedSubjectName::new("workspace")?;
        self.authorize_side_effect_on_resource(derivation_id, actor, tool, &default_resource, ifc)
    }

    /// Resource-aware variant of `authorize_side_effect` enforcing compiled Cedar resource-scoped
    /// `forbid` policies (such as ASCENT 5-scope exfiltration rules on `Marlin::Resource::"<target>"`).
    pub fn authorize_side_effect_on_resource(
        &self,
        derivation_id: Option<&DerivationId>,
        actor: &GovernedSubjectName,
        tool: &GovernedSubjectName,
        resource: &GovernedSubjectName,
        ifc: &ToolIfcContext,
    ) -> Result<ExecutionBreakerCommitReceipt, GovernanceAdmissionError> {
        let guard = self
            .inner
            .lock()
            .map_err(|_| GovernanceAdmissionError::LockPoisoned)?;

        if let Some(did) = derivation_id
            && guard.retracted_derivations.contains(did)
        {
            return Err(GovernanceAdmissionError::DerivationRetractedDenied(
                did.clone(),
            ));
        }

        if guard.fenced_actors.contains(actor) {
            return Err(GovernanceAdmissionError::AscentFencedActorDenied(
                actor.clone(),
            ));
        }

        if guard.blocked_tools.contains(tool) {
            return Err(GovernanceAdmissionError::AscentBlockedToolDenied(
                tool.clone(),
            ));
        }

        let mut cedar_req = CedarEvaluationRequest::for_side_effect(
            actor,
            tool,
            resource,
            ifc.tool_controlled_by_tainted_data,
            ifc.declassification_approved,
        );
        if let Some(scope) = guard.resource_scopes.get(resource) {
            cedar_req = cedar_req.with_resource_scope(scope.clone());
        }
        if let CedarEvaluationDecision::Deny { matching_forbids } =
            evaluate_compiled_cedar_policies(&guard.compiled_cedar_policies, &cedar_req)
        {
            return Err(GovernanceAdmissionError::CedarForbidDenied {
                policy_id: matching_forbids[0].clone(),
                subject: resource.clone(),
            });
        }
        drop(guard);

        Ok(self.breaker.authorize_side_effect(tool.as_str(), ifc)?)
    }

    pub fn is_derivation_retracted(&self, derivation_id: &DerivationId) -> bool {
        self.inner
            .lock()
            .map(|g| g.retracted_derivations.contains(derivation_id))
            .unwrap_or(true)
    }

    pub fn is_actor_fenced(&self, actor: &GovernedSubjectName) -> bool {
        self.inner
            .lock()
            .map(|g| g.fenced_actors.contains(actor))
            .unwrap_or(true)
    }

    /// Captures a complete, self-contained durable checkpoint of both the Rust
    /// `ExecutionCircuitBreaker` and Scheme Governance Authority gate state.
    pub fn export_durable_checkpoint(
        &self,
    ) -> Result<GovernanceDurableCheckpoint, GovernanceAdmissionError> {
        let guard = self
            .inner
            .lock()
            .map_err(|_| GovernanceAdmissionError::LockPoisoned)?;
        let breaker = self.breaker.snapshot()?;
        Ok(GovernanceDurableCheckpoint {
            schema_version: GOVERNANCE_DURABLE_CHECKPOINT_VERSION,
            last_admitted: guard.last_admitted,
            min_revocation_epoch: guard.min_revocation_epoch,
            policy_revision: guard.policy_revision,
            breaker_tripped: breaker.tripped,
            breaker_trip_epoch: breaker.trip_epoch,
            breaker_budget_used: breaker.budget_used,
            breaker_last_trip_reason: breaker.last_trip_reason,
            assessment_digest: guard.assessment_digest.clone(),
            taint_proof_digest: guard.taint_proof_digest.clone(),
            fixpoint_proof_digest: guard.fixpoint_proof_digest.clone(),
            temporal_proof_digest: guard.temporal_proof_digest.clone(),
            invalidation_digest: guard.invalidation_digest.clone(),
            rejection_reasons: guard.rejection_reasons.clone(),
            fenced_actors: guard.fenced_actors.iter().cloned().collect(),
            blocked_tools: guard.blocked_tools.iter().cloned().collect(),
            retracted_conclusions: guard.retracted_derivations.iter().cloned().collect(),
            cedar_policies: guard.cedar_policies.clone(),
        })
    }

    /// Exports the durable checkpoint as canonical length-prefixed `MGV1` bytes
    /// protected by a trailing 32-byte FIPS 180-4 SHA-256 integrity digest.
    pub fn export_durable_checkpoint_bytes(&self) -> Result<Vec<u8>, GovernanceAdmissionError> {
        Ok(self.export_durable_checkpoint()?.to_canonical_bytes())
    }

    /// Restores durable persisted governance state across an OS process restart.
    ///
    /// Anti-Restart-Reset & Tamper Fail-Closed invariants:
    /// 1. If `bytes` fail magic/version/SHA-256 integrity or schema validation, the
    ///    underlying `ExecutionCircuitBreaker` is AUTOMATICALLY TRIPPED fail-closed
    ///    (`corrupt-durable-governance-checkpoint`) so WAL tampering can NEVER fail open.
    /// 2. Rejects any checkpoint whose `min_revocation_epoch` or `breaker_trip_epoch`
    ///    would regress below the live gate's monotonic floor.
    /// 3. Preserves persisted `breaker_tripped`, monotonic epochs, ASCENT `fenced_actors`,
    ///    `blocked_tools`, Bi-Temporal TM `retracted_conclusions`, and synthesized Cedar rules.
    pub fn restore_durable_checkpoint_bytes(
        &self,
        bytes: &[u8],
    ) -> Result<GovernanceGateSnapshot, GovernanceAdmissionError> {
        let cp = match GovernanceDurableCheckpoint::from_canonical_bytes(bytes) {
            Ok(cp) => cp,
            Err(err) => {
                let _ = self.trip_corrupt_checkpoint_fail_closed(&err.to_string());
                return Err(err);
            }
        };

        let compiled_cedar = match cp
            .cedar_policies
            .iter()
            .map(compile_cedar_policy)
            .collect::<Result<Vec<_>, _>>()
        {
            Ok(compiled) => compiled,
            Err(err) => {
                let detail = err.to_string();
                let _ = self.trip_corrupt_checkpoint_fail_closed(&detail);
                return Err(GovernanceAdmissionError::CorruptDurableCheckpoint(detail));
            }
        };

        {
            let guard = self
                .inner
                .lock()
                .map_err(|_| GovernanceAdmissionError::LockPoisoned)?;
            if cp.min_revocation_epoch < guard.min_revocation_epoch {
                return Err(GovernanceAdmissionError::StaleProofEpoch {
                    supplied: cp.min_revocation_epoch,
                    current_floor: guard.min_revocation_epoch,
                });
            }
        }

        self.breaker.restore_durable_state(
            cp.breaker_tripped,
            cp.breaker_trip_epoch,
            cp.breaker_budget_used,
            cp.breaker_last_trip_reason.clone(),
        )?;

        {
            let mut guard = self
                .inner
                .lock()
                .map_err(|_| GovernanceAdmissionError::LockPoisoned)?;
            guard.active_derivations.clear();
            guard.min_revocation_epoch = cp.min_revocation_epoch;
            guard.policy_revision = cp.policy_revision;
            guard.last_admitted = cp.last_admitted;
            guard.assessment_digest = cp.assessment_digest;
            guard.taint_proof_digest = cp.taint_proof_digest;
            guard.fixpoint_proof_digest = cp.fixpoint_proof_digest;
            guard.temporal_proof_digest = cp.temporal_proof_digest;
            guard.invalidation_digest = cp.invalidation_digest;
            guard.rejection_reasons = cp.rejection_reasons;
            guard.fenced_actors = cp.fenced_actors.into_iter().collect();
            guard.blocked_tools = cp.blocked_tools.into_iter().collect();
            guard.retracted_derivations = cp.retracted_conclusions.into_iter().collect();
            guard.cedar_policies = cp.cedar_policies;
            guard.compiled_cedar_policies = compiled_cedar;
        }

        self.snapshot()
    }

    /// Trips the admission gate fail-closed when a companion durable artifact
    /// (e.g. an `MGF1` runtime fact-graph checkpoint in Turso CAS) fails integrity verification.
    pub fn trip_corrupt_durable_state(
        &self,
        detail: &str,
    ) -> Result<GovernanceGateSnapshot, GovernanceAdmissionError> {
        self.trip_corrupt_checkpoint_fail_closed(detail)?;
        self.snapshot()
    }

    fn trip_corrupt_checkpoint_fail_closed(
        &self,
        detail: &str,
    ) -> Result<(), GovernanceAdmissionError> {
        let reason_str = format!("corrupt-durable-governance-checkpoint:{detail}");
        let truncated: String = reason_str.chars().take(240).collect();
        let trip_reason = BreakerTripReason::new(truncated.clone())?;
        self.breaker.trip(trip_reason)?;
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| GovernanceAdmissionError::LockPoisoned)?;
        guard.last_admitted = false;
        guard
            .rejection_reasons
            .insert(0, "corrupt-durable-governance-checkpoint".to_owned());
        if guard.rejection_reasons.len() > 16 {
            guard.rejection_reasons.truncate(16);
        }
        Ok(())
    }
}

/// Durable tamper-evident checkpoint snapshot of `GovernanceAdmissionGate`
/// + `ExecutionCircuitBreaker` persisted to Turso CAS/WAL across OS restarts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GovernanceDurableCheckpoint {
    pub schema_version: u32,
    pub last_admitted: bool,
    pub min_revocation_epoch: GovernanceProofEpoch,
    pub policy_revision: u64,
    pub breaker_tripped: bool,
    pub breaker_trip_epoch: BreakerTripEpoch,
    pub breaker_budget_used: u32,
    pub breaker_last_trip_reason: Option<BreakerTripReason>,
    pub assessment_digest: Option<GovernanceProofDigest>,
    pub taint_proof_digest: Option<GovernanceProofDigest>,
    pub fixpoint_proof_digest: Option<GovernanceProofDigest>,
    pub temporal_proof_digest: Option<GovernanceProofDigest>,
    pub invalidation_digest: Option<GovernanceProofDigest>,
    pub rejection_reasons: Vec<String>,
    pub fenced_actors: Vec<GovernedSubjectName>,
    pub blocked_tools: Vec<GovernedSubjectName>,
    pub retracted_conclusions: Vec<DerivationId>,
    pub cedar_policies: Vec<SynthesizedCedarPolicy>,
}

impl GovernanceDurableCheckpoint {
    /// Serializes into canonical `MGV1` length-prefixed binary bytes followed by a
    /// 32-byte FIPS 180-4 SHA-256 integrity digest over all preceding bytes.
    pub fn to_canonical_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(512);
        buf.extend_from_slice(GOVERNANCE_DURABLE_CHECKPOINT_MAGIC);
        put_u32(&mut buf, self.schema_version);
        buf.push(u8::from(self.last_admitted));
        put_u64(&mut buf, self.min_revocation_epoch.get());
        put_u64(&mut buf, self.policy_revision);
        buf.push(u8::from(self.breaker_tripped));
        put_u64(&mut buf, self.breaker_trip_epoch.get());
        put_u32(&mut buf, self.breaker_budget_used);
        put_opt_str(
            &mut buf,
            self.breaker_last_trip_reason
                .as_ref()
                .map(BreakerTripReason::as_str),
        );
        put_opt_str(
            &mut buf,
            self.assessment_digest
                .as_ref()
                .map(GovernanceProofDigest::as_str),
        );
        put_opt_str(
            &mut buf,
            self.taint_proof_digest
                .as_ref()
                .map(GovernanceProofDigest::as_str),
        );
        put_opt_str(
            &mut buf,
            self.fixpoint_proof_digest
                .as_ref()
                .map(GovernanceProofDigest::as_str),
        );
        put_opt_str(
            &mut buf,
            self.temporal_proof_digest
                .as_ref()
                .map(GovernanceProofDigest::as_str),
        );
        put_opt_str(
            &mut buf,
            self.invalidation_digest
                .as_ref()
                .map(GovernanceProofDigest::as_str),
        );
        put_str_vec(&mut buf, &self.rejection_reasons);
        put_str_iter(
            &mut buf,
            self.fenced_actors.len(),
            self.fenced_actors.iter().map(GovernedSubjectName::as_str),
        );
        put_str_iter(
            &mut buf,
            self.blocked_tools.len(),
            self.blocked_tools.iter().map(GovernedSubjectName::as_str),
        );
        put_str_iter(
            &mut buf,
            self.retracted_conclusions.len(),
            self.retracted_conclusions.iter().map(DerivationId::as_str),
        );
        put_u32(&mut buf, self.cedar_policies.len() as u32);
        for p in &self.cedar_policies {
            put_str(&mut buf, p.policy_id.as_str());
            put_str(&mut buf, &p.source);
        }
        let mac = sha256_bytes(&buf);
        buf.extend_from_slice(&mac);
        buf
    }

    /// Validates magic, schema version, exact SHA-256 integrity tag, exact EOF, and
    /// every domain newtype invariant.
    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, GovernanceAdmissionError> {
        if bytes.len() < 4 + 4 + 32 {
            return Err(GovernanceAdmissionError::CorruptDurableCheckpoint(
                "checkpoint truncated below minimum envelope size".to_owned(),
            ));
        }
        let (body, tag) = bytes.split_at(bytes.len() - 32);
        let expected_tag = sha256_bytes(body);
        if tag != expected_tag {
            return Err(GovernanceAdmissionError::CorruptDurableCheckpoint(
                "SHA-256 integrity verification mismatch".to_owned(),
            ));
        }

        let mut cur = ByteCursor::new(body);
        let magic = cur.read_exact(4)?;
        if magic != GOVERNANCE_DURABLE_CHECKPOINT_MAGIC {
            return Err(GovernanceAdmissionError::CorruptDurableCheckpoint(
                "invalid checkpoint magic header".to_owned(),
            ));
        }
        let schema_version = cur.read_u32()?;
        if schema_version != GOVERNANCE_DURABLE_CHECKPOINT_VERSION {
            return Err(GovernanceAdmissionError::CorruptDurableCheckpoint(format!(
                "unsupported durable checkpoint version {schema_version}"
            )));
        }
        let last_admitted = cur.read_bool()?;
        let min_revocation_epoch = GovernanceProofEpoch::new(cur.read_u64()?)
            .map_err(|e| GovernanceAdmissionError::CorruptDurableCheckpoint(e.to_string()))?;
        let policy_revision = cur.read_u64()?;
        let breaker_tripped = cur.read_bool()?;
        let breaker_trip_epoch = BreakerTripEpoch::new(cur.read_u64()?)
            .map_err(|e| GovernanceAdmissionError::CorruptDurableCheckpoint(e.to_string()))?;
        let breaker_budget_used = cur.read_u32()?;
        let breaker_last_trip_reason =
            match cur.read_opt_str()? {
                Some(s) => Some(BreakerTripReason::new(s).map_err(|e| {
                    GovernanceAdmissionError::CorruptDurableCheckpoint(e.to_string())
                })?),
                None => None,
            };
        let read_opt_digest = |c: &mut ByteCursor<'_>| -> Result<
            Option<GovernanceProofDigest>,
            GovernanceAdmissionError,
        > {
            match c.read_opt_str()? {
                Some(s) => Ok(Some(GovernanceProofDigest::new(s).map_err(|e| {
                    GovernanceAdmissionError::CorruptDurableCheckpoint(e.to_string())
                })?)),
                None => Ok(None),
            }
        };
        let assessment_digest = read_opt_digest(&mut cur)?;
        let taint_proof_digest = read_opt_digest(&mut cur)?;
        let fixpoint_proof_digest = read_opt_digest(&mut cur)?;
        let temporal_proof_digest = read_opt_digest(&mut cur)?;
        let invalidation_digest = read_opt_digest(&mut cur)?;
        let rejection_reasons = cur.read_str_vec(128)?;

        let raw_fenced = cur.read_str_vec(1024)?;
        let mut fenced_actors = Vec::with_capacity(raw_fenced.len());
        for s in raw_fenced {
            fenced_actors.push(
                GovernedSubjectName::new(s).map_err(|e| {
                    GovernanceAdmissionError::CorruptDurableCheckpoint(e.to_string())
                })?,
            );
        }
        let raw_blocked = cur.read_str_vec(1024)?;
        let mut blocked_tools = Vec::with_capacity(raw_blocked.len());
        for s in raw_blocked {
            blocked_tools.push(
                GovernedSubjectName::new(s).map_err(|e| {
                    GovernanceAdmissionError::CorruptDurableCheckpoint(e.to_string())
                })?,
            );
        }
        let raw_retracted = cur.read_str_vec(1024)?;
        let mut retracted_conclusions = Vec::with_capacity(raw_retracted.len());
        for s in raw_retracted {
            retracted_conclusions.push(
                DerivationId::new(s).map_err(|e| {
                    GovernanceAdmissionError::CorruptDurableCheckpoint(e.to_string())
                })?,
            );
        }
        let cedar_count = cur.read_u32()? as usize;
        if cedar_count > 1024 {
            return Err(GovernanceAdmissionError::CorruptDurableCheckpoint(format!(
                "excessive synthesized cedar policy count {cedar_count}"
            )));
        }
        let mut cedar_policies = Vec::with_capacity(cedar_count);
        for _ in 0..cedar_count {
            let pid = CedarPolicyId::new(cur.read_str()?)
                .map_err(|e| GovernanceAdmissionError::CorruptDurableCheckpoint(e.to_string()))?;
            let src = cur.read_str()?;
            cedar_policies.push(SynthesizedCedarPolicy {
                policy_id: pid,
                source: src,
            });
        }
        if !cur.is_eof() {
            return Err(GovernanceAdmissionError::CorruptDurableCheckpoint(
                "unexpected trailing bytes in durable checkpoint".to_owned(),
            ));
        }
        Ok(Self {
            schema_version,
            last_admitted,
            min_revocation_epoch,
            policy_revision,
            breaker_tripped,
            breaker_trip_epoch,
            breaker_budget_used,
            breaker_last_trip_reason,
            assessment_digest,
            taint_proof_digest,
            fixpoint_proof_digest,
            temporal_proof_digest,
            invalidation_digest,
            rejection_reasons,
            fenced_actors,
            blocked_tools,
            retracted_conclusions,
            cedar_policies,
        })
    }
}

fn put_u32(buf: &mut Vec<u8>, v: u32) {
    buf.extend_from_slice(&v.to_be_bytes());
}

fn put_u64(buf: &mut Vec<u8>, v: u64) {
    buf.extend_from_slice(&v.to_be_bytes());
}

fn put_str(buf: &mut Vec<u8>, s: &str) {
    let bytes = s.as_bytes();
    put_u32(buf, bytes.len() as u32);
    buf.extend_from_slice(bytes);
}

fn put_opt_str(buf: &mut Vec<u8>, opt: Option<&str>) {
    match opt {
        Some(s) => {
            buf.push(1);
            put_str(buf, s);
        }
        None => buf.push(0),
    }
}

fn put_str_vec(buf: &mut Vec<u8>, items: &[String]) {
    put_str_iter(buf, items.len(), items.iter().map(String::as_str));
}

fn put_str_iter<'a>(buf: &mut Vec<u8>, len: usize, iter: impl Iterator<Item = &'a str>) {
    put_u32(buf, len as u32);
    for s in iter {
        put_str(buf, s);
    }
}

struct ByteCursor<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> ByteCursor<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn is_eof(&self) -> bool {
        self.pos == self.data.len()
    }

    fn read_exact(&mut self, n: usize) -> Result<&'a [u8], GovernanceAdmissionError> {
        let end = self.pos.checked_add(n).ok_or_else(|| {
            GovernanceAdmissionError::CorruptDurableCheckpoint(
                "integer overflow in cursor".to_owned(),
            )
        })?;
        if end > self.data.len() {
            return Err(GovernanceAdmissionError::CorruptDurableCheckpoint(
                "unexpected EOF in durable checkpoint".to_owned(),
            ));
        }
        let slice = &self.data[self.pos..end];
        self.pos = end;
        Ok(slice)
    }

    fn read_bool(&mut self) -> Result<bool, GovernanceAdmissionError> {
        match self.read_exact(1)?[0] {
            0 => Ok(false),
            1 => Ok(true),
            other => Err(GovernanceAdmissionError::CorruptDurableCheckpoint(format!(
                "invalid bool tag {other}"
            ))),
        }
    }

    fn read_u32(&mut self) -> Result<u32, GovernanceAdmissionError> {
        let b = self.read_exact(4)?;
        Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn read_u64(&mut self) -> Result<u64, GovernanceAdmissionError> {
        let b = self.read_exact(8)?;
        Ok(u64::from_be_bytes([
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        ]))
    }

    fn read_str(&mut self) -> Result<String, GovernanceAdmissionError> {
        let len = self.read_u32()? as usize;
        if len > 65_536 {
            return Err(GovernanceAdmissionError::CorruptDurableCheckpoint(format!(
                "excessive string length {len}"
            )));
        }
        let bytes = self.read_exact(len)?;
        std::str::from_utf8(bytes).map(str::to_owned).map_err(|e| {
            GovernanceAdmissionError::CorruptDurableCheckpoint(format!("invalid utf8: {e}"))
        })
    }

    fn read_opt_str(&mut self) -> Result<Option<String>, GovernanceAdmissionError> {
        if self.read_bool()? {
            Ok(Some(self.read_str()?))
        } else {
            Ok(None)
        }
    }

    fn read_str_vec(&mut self, max_items: usize) -> Result<Vec<String>, GovernanceAdmissionError> {
        let count = self.read_u32()? as usize;
        if count > max_items {
            return Err(GovernanceAdmissionError::CorruptDurableCheckpoint(format!(
                "excessive vector count {count} > {max_items}"
            )));
        }
        let mut out = Vec::with_capacity(count);
        for _ in 0..count {
            out.push(self.read_str()?);
        }
        Ok(out)
    }
}

/// Zero-dependency FIPS 180-4 SHA-256 exposed for sealing canonical durable governance artifacts
/// (`MGV1` admission checkpoints and `MGF1` runtime fact-graph checkpoints).
pub fn governance_sha256_bytes(input: &[u8]) -> [u8; 32] {
    sha256_bytes(input)
}

/// Zero-dependency FIPS 180-4 SHA-256 used to seal canonical `MGV1` durable checkpoints.
fn sha256_bytes(input: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];

    let bit_len = (input.len() as u64).wrapping_mul(8);
    let mut msg = Vec::with_capacity(input.len() + 72);
    msg.extend_from_slice(input);
    msg.push(0x80);
    while (msg.len() % 64) != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in msg.chunks_exact(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = h;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }

    let mut out = [0u8; 32];
    for (i, word) in h.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

#[cfg(test)]
mod durable_checkpoint_tests {
    use super::*;
    use crate::execution_breaker::ToolBudgetUnits;

    fn hex(bytes: &[u8]) -> String {
        let mut s = String::with_capacity(bytes.len() * 2);
        for b in bytes {
            use std::fmt::Write;
            let _ = write!(&mut s, "{b:02x}");
        }
        s
    }

    #[test]
    fn sha256_matches_nist_fips_180_4_vectors() {
        assert_eq!(
            hex(&sha256_bytes(b"")),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            hex(&sha256_bytes(b"abc")),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn durable_checkpoint_roundtrips_and_tampered_bytes_fail_closed() {
        let b1 = Arc::new(ExecutionCircuitBreaker::new(ToolBudgetUnits::new(8)));
        let g1 = GovernanceAdmissionGate::new(b1);
        let d = |ch: char| {
            GovernanceProofDigest::new(format!(
                "sha256:{}",
                std::iter::repeat_n(ch, 64).collect::<String>()
            ))
            .unwrap()
        };
        let env = GovernanceProofEnvelope {
            schema_version: GOVERNANCE_ADMISSION_SCHEMA_VERSION,
            admitted: false,
            revocation_epoch: GovernanceProofEpoch::new(4).unwrap(),
            policy_revision: 7,
            assessment_digest: d('a'),
            taint_proof_digest: d('b'),
            fixpoint_proof_digest: d('c'),
            temporal_proof_digest: d('d'),
            invalidation_digest: d('e'),
            rejection_reasons: vec!["ascent-camel-taint-alert".to_owned()],
            fenced_actors: vec![GovernedSubjectName::new("agent-sub").unwrap()],
            blocked_tools: vec![GovernedSubjectName::new("os-shell").unwrap()],
            retracted_conclusions: vec![DerivationId::new("D-TOOL-EXEC").unwrap()],
            cedar_policies: vec![SynthesizedCedarPolicy {
                policy_id: CedarPolicyId::new("marlin.ascent.forbid.fenced.agent-sub").unwrap(),
                source: "forbid(principal == Marlin::Actor::\"agent-sub\", action, resource);"
                    .to_owned(),
            }],
        };
        g1.synchronize_proof_envelope(&env).unwrap();
        let bytes = g1.export_durable_checkpoint_bytes().unwrap();

        // Fresh post-restart gate restores tripped latch, epochs, fences, retractions, Cedar rules!
        let b2 = Arc::new(ExecutionCircuitBreaker::new(ToolBudgetUnits::new(8)));
        let g2 = GovernanceAdmissionGate::new(b2);
        let restored = g2.restore_durable_checkpoint_bytes(&bytes).unwrap();
        assert!(restored.breaker.tripped);
        assert_eq!(restored.revocation_epoch.get(), 4);
        assert_eq!(restored.policy_revision, 7);
        assert!(g2.is_actor_fenced(&GovernedSubjectName::new("agent-sub").unwrap()));
        assert!(g2.is_derivation_retracted(&DerivationId::new("D-TOOL-EXEC").unwrap()));
        assert_eq!(restored.cedar_policies.len(), 1);

        // Single-bit tamper in persisted checkpoint MUST fail closed & auto-trip breaker!
        let mut tampered = bytes.clone();
        tampered[10] ^= 0x01;
        let b3 = Arc::new(ExecutionCircuitBreaker::new(ToolBudgetUnits::new(8)));
        let g3 = GovernanceAdmissionGate::new(b3);
        let err = g3
            .restore_durable_checkpoint_bytes(&tampered)
            .expect_err("bit-flipped checkpoint must be rejected");
        assert!(matches!(
            err,
            GovernanceAdmissionError::CorruptDurableCheckpoint(_)
        ));
        let snap3 = g3.snapshot().unwrap();
        assert!(
            snap3.breaker.tripped && !snap3.admitted,
            "corrupted durable checkpoint MUST auto-trip breaker fail-closed"
        );
    }

    #[test]
    fn gate_enforces_compiled_cedar_resource_and_role_policies_and_rejects_malformed_cedar() {
        use crate::governance_cedar::{CedarContextValue, CedarEntityRef};

        let breaker = Arc::new(ExecutionCircuitBreaker::new(ToolBudgetUnits::new(8)));
        let gate = GovernanceAdmissionGate::new(breaker);
        let d = |ch: char| {
            GovernanceProofDigest::new(format!(
                "sha256:{}",
                std::iter::repeat_n(ch, 64).collect::<String>()
            ))
            .unwrap()
        };

        // 1. Malformed synthesized Cedar policy MUST fail closed before modifying gate state.
        let malformed = GovernanceProofEnvelope {
            schema_version: GOVERNANCE_ADMISSION_SCHEMA_VERSION,
            admitted: true,
            revocation_epoch: GovernanceProofEpoch::new(2).unwrap(),
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
            cedar_policies: vec![SynthesizedCedarPolicy {
                policy_id: CedarPolicyId::new("bad.syntax").unwrap(),
                source: "forbid (principal == Marlin::Actor::\"x\"".to_owned(),
            }],
        };
        assert!(matches!(
            gate.synchronize_proof_envelope(&malformed),
            Err(GovernanceAdmissionError::InvalidCedarPolicy { .. })
        ));

        // 2. Valid compiled Cedar policies enforce role forbid + resource exfiltration forbid.
        let valid = GovernanceProofEnvelope {
            cedar_policies: vec![
                SynthesizedCedarPolicy {
                    policy_id: CedarPolicyId::new("marlin.forbid.agent-privileged-governance")
                        .unwrap(),
                    source: "forbid (principal in Marlin::Role::\"Agent\", action in [Marlin::Action::\"rearm_breaker\", Marlin::Action::\"promote_context_scope\"], resource);".to_owned(),
                },
                SynthesizedCedarPolicy {
                    policy_id: CedarPolicyId::new("marlin.permit.verified-human-governance")
                        .unwrap(),
                    source: "permit (principal in Marlin::Role::\"VerifiedHuman\", action in [Marlin::Action::\"rearm_breaker\"], resource) when { context.step_up_fresh == true };".to_owned(),
                },
                SynthesizedCedarPolicy {
                    policy_id: CedarPolicyId::new("marlin.ascent.forbid.exfil.secret-doc.ext-webhook")
                        .unwrap(),
                    source: "forbid (principal, action, resource == Marlin::Resource::\"ext-webhook\");".to_owned(),
                },
            ],
            ..malformed
        };
        gate.synchronize_proof_envelope(&valid).unwrap();

        let agent = GovernedSubjectName::new("agent-worker").unwrap();
        let human = GovernedSubjectName::new("ops-alice").unwrap();
        let rearm_action = GovernedSubjectName::new("rearm_breaker").unwrap();
        let read_action = GovernedSubjectName::new("http-post").unwrap();
        let workspace = GovernedSubjectName::new("workspace").unwrap();
        let ext_webhook = GovernedSubjectName::new("ext-webhook").unwrap();

        // Agent attempting privileged `rearm_breaker` is denied by compiled Cedar role forbid!
        assert!(matches!(
            gate.authorize_side_effect_on_resource(
                None,
                &agent,
                &rearm_action,
                &workspace,
                &ToolIfcContext::clean_trusted(),
            ),
            Err(GovernanceAdmissionError::CedarForbidDenied { .. })
        ));

        // Even human targeting exfiltration-forbid resource `ext-webhook` is denied by compiled Cedar!
        assert!(matches!(
            gate.authorize_side_effect_on_resource(
                None,
                &human,
                &read_action,
                &ext_webhook,
                &ToolIfcContext::clean_trusted(),
            ),
            Err(GovernanceAdmissionError::CedarForbidDenied { .. })
        ));

        // Clean call on `workspace` succeeds.
        gate.authorize_side_effect_on_resource(
            None,
            &human,
            &read_action,
            &workspace,
            &ToolIfcContext::clean_trusted(),
        )
        .expect("non-forbidden resource should pass compiled Cedar evaluation");

        // Direct Cedar request evaluation verifies `when { context.step_up_fresh == true }`.
        let human_req = CedarEvaluationRequest {
            principal: CedarEntityRef::actor("ops-alice"),
            principal_roles: vec![CedarEntityRef::role("VerifiedHuman")],
            action: CedarEntityRef::action("rearm_breaker"),
            resource: CedarEntityRef::resource("workspace"),
            resource_groups: Vec::new(),
            context: BTreeMap::from([("step_up_fresh".to_owned(), CedarContextValue::Bool(true))]),
        };
        assert!(matches!(
            gate.evaluate_cedar_request(&human_req).unwrap(),
            CedarEvaluationDecision::Allow { .. }
        ));
    }

    #[test]
    fn mgv1_exhaustive_single_byte_bitflip_and_truncation_always_fail_closed() {
        let breaker = Arc::new(ExecutionCircuitBreaker::new(ToolBudgetUnits::new(8)));
        let gate = GovernanceAdmissionGate::new(Arc::clone(&breaker));
        let digest = |ch: char| {
            GovernanceProofDigest::new(format!(
                "sha256:{}",
                std::iter::repeat_n(ch, 64).collect::<String>()
            ))
            .unwrap()
        };
        let env = GovernanceProofEnvelope {
            schema_version: GOVERNANCE_ADMISSION_SCHEMA_VERSION,
            admitted: false,
            revocation_epoch: GovernanceProofEpoch::new(4).unwrap(),
            policy_revision: 7,
            assessment_digest: digest('a'),
            taint_proof_digest: digest('b'),
            fixpoint_proof_digest: digest('c'),
            temporal_proof_digest: digest('d'),
            invalidation_digest: digest('e'),
            rejection_reasons: vec!["ascent-scope-exfiltration-alert".to_owned()],
            fenced_actors: vec![GovernedSubjectName::new("agent-sub").unwrap()],
            blocked_tools: vec![GovernedSubjectName::new("os-shell").unwrap()],
            retracted_conclusions: vec![DerivationId::new("D-TOOL-EXEC").unwrap()],
            cedar_policies: vec![
                SynthesizedCedarPolicy {
                    policy_id: CedarPolicyId::new("marlin.ascent.forbid.fenced.agent-sub").unwrap(),
                    source: "forbid (principal == Marlin::Actor::\"agent-sub\", action, resource);"
                        .to_owned(),
                },
                SynthesizedCedarPolicy {
                    policy_id: CedarPolicyId::new(
                        "marlin.ascent.forbid.scope.external-unless-declassified",
                    )
                    .unwrap(),
                    source: "forbid (principal, action, resource in Marlin::Scope::\"external\") unless { context.declassified == true };".to_owned(),
                },
            ],
        };
        gate.synchronize_proof_envelope(&env).unwrap();
        let valid_blob = gate.export_durable_checkpoint_bytes().unwrap();
        assert!(valid_blob.len() > 128);

        // 1. Every single byte position bit-flip MUST return CorruptDurableCheckpoint AND auto-trip fail-closed!
        for idx in 0..valid_blob.len() {
            let mut tampered = valid_blob.clone();
            tampered[idx] ^= 0x01;
            let fresh_b = Arc::new(ExecutionCircuitBreaker::new(ToolBudgetUnits::new(8)));
            let fresh_g = GovernanceAdmissionGate::new(fresh_b);
            let err = fresh_g
                .restore_durable_checkpoint_bytes(&tampered)
                .expect_err("single-byte MGV1 bit-flip must fail closed");
            assert!(
                matches!(err, GovernanceAdmissionError::CorruptDurableCheckpoint(_)),
                "byte {idx}: unexpected error {err:?}"
            );
            assert!(
                fresh_g.snapshot().unwrap().breaker.tripped,
                "byte {idx}: corrupt MGV1 must auto-trip breaker"
            );
        }

        // 2. Every truncation length 0..len and trailing-byte append MUST fail closed!
        for cut in 0..valid_blob.len() {
            let fresh_b = Arc::new(ExecutionCircuitBreaker::new(ToolBudgetUnits::new(8)));
            let fresh_g = GovernanceAdmissionGate::new(fresh_b);
            assert!(matches!(
                fresh_g.restore_durable_checkpoint_bytes(&valid_blob[..cut]),
                Err(GovernanceAdmissionError::CorruptDurableCheckpoint(_))
            ));
            assert!(fresh_g.snapshot().unwrap().breaker.tripped);
        }
        let mut extended = valid_blob.clone();
        extended.push(0x00);
        let fresh_b = Arc::new(ExecutionCircuitBreaker::new(ToolBudgetUnits::new(8)));
        let fresh_g = GovernanceAdmissionGate::new(fresh_b);
        assert!(matches!(
            fresh_g.restore_durable_checkpoint_bytes(&extended),
            Err(GovernanceAdmissionError::CorruptDurableCheckpoint(_))
        ));
        assert!(fresh_g.snapshot().unwrap().breaker.tripped);
    }

    #[test]
    fn gate_enforces_scope_unless_cedar_policy_on_registered_resource_scopes() {
        let breaker = Arc::new(ExecutionCircuitBreaker::new(ToolBudgetUnits::new(8)));
        let gate = GovernanceAdmissionGate::new(Arc::clone(&breaker));
        let digest = |ch: char| {
            GovernanceProofDigest::new(format!(
                "sha256:{}",
                std::iter::repeat_n(ch, 64).collect::<String>()
            ))
            .unwrap()
        };
        let env = GovernanceProofEnvelope {
            schema_version: GOVERNANCE_ADMISSION_SCHEMA_VERSION,
            admitted: true,
            revocation_epoch: GovernanceProofEpoch::new(2).unwrap(),
            policy_revision: 1,
            assessment_digest: digest('a'),
            taint_proof_digest: digest('b'),
            fixpoint_proof_digest: digest('c'),
            temporal_proof_digest: digest('d'),
            invalidation_digest: digest('e'),
            rejection_reasons: Vec::new(),
            fenced_actors: Vec::new(),
            blocked_tools: Vec::new(),
            retracted_conclusions: Vec::new(),
            cedar_policies: vec![SynthesizedCedarPolicy {
                policy_id: CedarPolicyId::new(
                    "marlin.ascent.forbid.scope.external-unless-declassified",
                )
                .unwrap(),
                source: "forbid (principal, action, resource in Marlin::Scope::\"external\") unless { context.declassified == true };".to_owned(),
            }],
        };
        gate.synchronize_proof_envelope(&env).unwrap();

        let ext_sibling = GovernedSubjectName::new("ext-partner-mirror.txt").unwrap();
        let workspace_doc = GovernedSubjectName::new("safe-report.md").unwrap();
        gate.set_resource_scope(ext_sibling.clone(), "external")
            .unwrap();
        gate.set_resource_scope(workspace_doc.clone(), "workspace")
            .unwrap();

        let human = GovernedSubjectName::new("ops-alice").unwrap();
        let tool = GovernedSubjectName::new("file-write").unwrap();

        // 1. Un-declassified write to external-scope sibling is DENIED by scope-level `unless` forbid!
        let err = gate
            .authorize_side_effect_on_resource(
                None,
                &human,
                &tool,
                &ext_sibling,
                &ToolIfcContext::clean_trusted(),
            )
            .expect_err("external-scope sibling must be denied without declassification");
        assert!(matches!(
            err,
            GovernanceAdmissionError::CedarForbidDenied { ref policy_id, .. }
                if policy_id.as_str() == "marlin.ascent.forbid.scope.external-unless-declassified"
        ));

        // 2. Workspace-scope resource succeeds cleanly.
        gate.authorize_side_effect_on_resource(
            None,
            &human,
            &tool,
            &workspace_doc,
            &ToolIfcContext::clean_trusted(),
        )
        .expect("workspace resource must pass");

        // 3. Declassification-approved context satisfies `unless { context.declassified == true }` -> admitted!
        let declass_ifc = ToolIfcContext {
            declassification_approved: true,
            ..ToolIfcContext::clean_trusted()
        };
        gate.authorize_side_effect_on_resource(None, &human, &tool, &ext_sibling, &declass_ifc)
            .expect("explicitly declassified write should satisfy unless clause");
    }
}
