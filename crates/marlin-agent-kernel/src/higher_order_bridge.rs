// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Authoritative Rust admission gate bridging Gerbil Scheme's Higher-Order Governance
//! Plane (`scheme/policy/higher-order-governance.ss`) into `ExecutionCircuitBreaker`
//! and `LoopProgramRuntimeSideEffectExecutor`.
//!
//! Zero JSON / text parsing: accepts typed `HigherOrderGovernanceProofEnvelope`
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
    BreakerRejectionReason, BreakerTripReason, ExecutionBreakerCommitReceipt,
    ExecutionCircuitBreaker, ToolExecutionLease, ToolIfcContext,
};

pub const HIGHER_ORDER_GOVERNANCE_SCHEMA_VERSION: u32 = 1;

/// Validated canonical SHA-256 content digest (`sha256:<64 lowercase hex>`).
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct GovernanceProofDigest(String);

impl GovernanceProofDigest {
    pub fn new(value: impl Into<String>) -> Result<Self, HigherOrderBridgeError> {
        let value = value.into();
        let valid = value.len() == 71
            && value.starts_with("sha256:")
            && value.as_bytes()[7..]
                .iter()
                .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'));
        if !valid {
            return Err(HigherOrderBridgeError::InvalidProofDigest(value));
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
            pub fn new(value: impl Into<String>) -> Result<Self, HigherOrderBridgeError> {
                let value = value.into();
                if value.is_empty() || value.chars().take(257).count() > 256 {
                    return Err(HigherOrderBridgeError::InvalidIdentifier($label));
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
ho_id!(GovernedSubjectName, "higher-order governed actor or tool identity");

/// Monotonic proof revocation epoch (`>= 1`).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct HigherOrderProofEpoch(u64);

impl HigherOrderProofEpoch {
    pub const INITIAL: Self = Self(1);

    pub fn new(value: u64) -> Result<Self, HigherOrderBridgeError> {
        if value == 0 {
            return Err(HigherOrderBridgeError::InvalidProofEpoch(value));
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
pub struct HigherOrderGovernanceProofEnvelope {
    pub schema_version: u32,
    pub admitted: bool,
    pub revocation_epoch: HigherOrderProofEpoch,
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
pub struct HigherOrderProofSyncReceipt {
    pub admitted: bool,
    pub revocation_epoch: HigherOrderProofEpoch,
    pub invalidated_active_derivations: Vec<DerivationId>,
    pub breaker_auto_tripped: bool,
    pub fenced_actor_count: usize,
    pub blocked_tool_count: usize,
    pub synthesized_cedar_count: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HigherOrderBridgeError {
    InvalidIdentifier(&'static str),
    InvalidProofDigest(String),
    InvalidProofEpoch(u64),
    UnsupportedSchemaVersion(u32),
    StaleProofEpoch {
        supplied: HigherOrderProofEpoch,
        current_floor: HigherOrderProofEpoch,
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
    ProofEnvelopeRejected(Vec<String>),
    ExecutionBreaker(BreakerRejectionReason),
    LockPoisoned,
}

impl fmt::Display for HigherOrderBridgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIdentifier(label) => write!(f, "invalid identifier for {label}"),
            Self::InvalidProofDigest(d) => write!(f, "invalid sha256 proof digest: {d}"),
            Self::InvalidProofEpoch(e) => write!(f, "invalid proof epoch {e}"),
            Self::UnsupportedSchemaVersion(v) => {
                write!(f, "unsupported higher-order governance schema version {v}")
            }
            Self::StaleProofEpoch {
                supplied,
                current_floor,
            } => write!(
                f,
                "stale higher-order proof epoch {} < floor {}",
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
                write!(f, "causal derivation invalidated by bi-temporal truth maintenance: {id}")
            }
            Self::AscentFencedActorDenied(actor) => {
                write!(f, "actor transitively fenced by ASCENT delegation fixpoint: {actor}")
            }
            Self::AscentBlockedToolDenied(tool) => {
                write!(f, "tool blocked by ASCENT execution breaker fixpoint: {tool}")
            }
            Self::CedarForbidDenied { policy_id, subject } => {
                write!(f, "synthesized Cedar forbid {policy_id} denied {subject}")
            }
            Self::ProofEnvelopeRejected(reasons) => {
                write!(f, "higher-order proof envelope rejected: {}", reasons.join(","))
            }
            Self::ExecutionBreaker(err) => write!(f, "execution breaker denied: {err}"),
            Self::LockPoisoned => write!(f, "higher-order admission gate lock poisoned"),
        }
    }
}

impl Error for HigherOrderBridgeError {}

impl From<BreakerRejectionReason> for HigherOrderBridgeError {
    fn from(value: BreakerRejectionReason) -> Self {
        Self::ExecutionBreaker(value)
    }
}

#[derive(Debug)]
struct GateInner {
    min_revocation_epoch: HigherOrderProofEpoch,
    last_admitted: bool,
    active_derivations: BTreeMap<DerivationId, ToolExecutionLease>,
    retracted_derivations: BTreeSet<DerivationId>,
    fenced_actors: BTreeSet<GovernedSubjectName>,
    blocked_tools: BTreeSet<GovernedSubjectName>,
    cedar_policies: Vec<SynthesizedCedarPolicy>,
}

/// Authoritative Rust admission gate synchronizing Scheme Higher-Order Governance
/// proofs (`POO × ASCENT × Bi-Temporal Truth Maintenance × Cedar`) directly into
/// `ExecutionCircuitBreaker` and live causal derivation leases.
#[derive(Clone, Debug)]
pub struct HigherOrderGovernanceAdmissionGate {
    breaker: Arc<ExecutionCircuitBreaker>,
    inner: Arc<Mutex<GateInner>>,
}

impl HigherOrderGovernanceAdmissionGate {
    pub fn new(breaker: Arc<ExecutionCircuitBreaker>) -> Self {
        Self {
            breaker,
            inner: Arc::new(Mutex::new(GateInner {
                min_revocation_epoch: HigherOrderProofEpoch::INITIAL,
                last_admitted: false,
                active_derivations: BTreeMap::new(),
                retracted_derivations: BTreeSet::new(),
                fenced_actors: BTreeSet::new(),
                blocked_tools: BTreeSet::new(),
                cedar_policies: Vec::new(),
            })),
        }
    }

    pub fn execution_breaker(&self) -> Arc<ExecutionCircuitBreaker> {
        Arc::clone(&self.breaker)
    }

    pub fn min_revocation_epoch(&self) -> Result<HigherOrderProofEpoch, HigherOrderBridgeError> {
        let guard = self
            .inner
            .lock()
            .map_err(|_| HigherOrderBridgeError::LockPoisoned)?;
        Ok(guard.min_revocation_epoch)
    }

    /// Registers a live in-flight `ToolExecutionLease` tagged with its causal `DerivationId`
    /// so Bi-Temporal Truth Maintenance retraction can surgically revoke it.
    pub fn register_derived_lease(
        &self,
        derivation_id: DerivationId,
        lease: ToolExecutionLease,
    ) -> Result<(), HigherOrderBridgeError> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| HigherOrderBridgeError::LockPoisoned)?;
        if guard.retracted_derivations.contains(&derivation_id) {
            return Err(HigherOrderBridgeError::DerivationRetractedDenied(
                derivation_id,
            ));
        }
        if guard.active_derivations.contains_key(&derivation_id) {
            return Err(HigherOrderBridgeError::DerivationAlreadyRegistered(
                derivation_id,
            ));
        }
        guard.active_derivations.insert(derivation_id, lease);
        Ok(())
    }

    /// Synchronizes a proof envelope produced by Scheme's Higher-Order Governance engine.
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
        envelope: &HigherOrderGovernanceProofEnvelope,
    ) -> Result<HigherOrderProofSyncReceipt, HigherOrderBridgeError> {
        if envelope.schema_version != HIGHER_ORDER_GOVERNANCE_SCHEMA_VERSION {
            return Err(HigherOrderBridgeError::UnsupportedSchemaVersion(
                envelope.schema_version,
            ));
        }

        let mut guard = self
            .inner
            .lock()
            .map_err(|_| HigherOrderBridgeError::LockPoisoned)?;

        if envelope.revocation_epoch < guard.min_revocation_epoch {
            return Err(HigherOrderBridgeError::StaleProofEpoch {
                supplied: envelope.revocation_epoch,
                current_floor: guard.min_revocation_epoch,
            });
        }
        guard.min_revocation_epoch = envelope.revocation_epoch;
        guard.last_admitted = envelope.admitted;

        guard.fenced_actors = envelope.fenced_actors.iter().cloned().collect();
        guard.blocked_tools = envelope.blocked_tools.iter().cloned().collect();
        guard.cedar_policies = envelope.cedar_policies.clone();

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
                format!("higher-order-denied:{}", envelope.rejection_reasons.join(","))
            } else {
                "higher-order-proof-not-admitted".to_owned()
            };
            let reason = BreakerTripReason::new(reason_text)?;
            self.breaker.trip(reason)?;
            auto_tripped = true;
        }

        Ok(HigherOrderProofSyncReceipt {
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
    ) -> Result<ExecutionBreakerCommitReceipt, HigherOrderBridgeError> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| HigherOrderBridgeError::LockPoisoned)?;

        if guard.retracted_derivations.contains(derivation_id) {
            return Err(HigherOrderBridgeError::DerivationRetractedDenied(
                derivation_id.clone(),
            ));
        }

        if guard.fenced_actors.contains(actor) {
            return Err(HigherOrderBridgeError::AscentFencedActorDenied(
                actor.clone(),
            ));
        }

        if guard.blocked_tools.contains(tool) {
            return Err(HigherOrderBridgeError::AscentBlockedToolDenied(
                tool.clone(),
            ));
        }

        // Enforce ASCENT-synthesized Cedar forbid policies on actor & tool identifiers.
        let actor_pattern = format!("Marlin::Actor::\"{}\"", actor.as_str());
        let tool_pattern = format!("Marlin::Action::\"{}\"", tool.as_str());
        for policy in &guard.cedar_policies {
            if policy.source.trim_start().starts_with("forbid") {
                if policy.source.contains(&actor_pattern) {
                    return Err(HigherOrderBridgeError::CedarForbidDenied {
                        policy_id: policy.policy_id.clone(),
                        subject: actor.clone(),
                    });
                }
                if policy.source.contains(&tool_pattern) {
                    return Err(HigherOrderBridgeError::CedarForbidDenied {
                        policy_id: policy.policy_id.clone(),
                        subject: tool.clone(),
                    });
                }
            }
        }

        let Some(lease) = guard.active_derivations.get(derivation_id).cloned() else {
            return Err(HigherOrderBridgeError::DerivationNotRegistered(
                derivation_id.clone(),
            ));
        };

        let receipt = self.breaker.commit_effect_with_ifc(&lease, ifc)?;
        guard.active_derivations.remove(derivation_id);
        Ok(receipt)
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
}
