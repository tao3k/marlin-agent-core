// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Authoritative AI/Tool Execution Circuit Breaker ("脱扣" / E-Stop),
//! monotonic trip-epoch lease fencing, armed-window tool budgets,
//! human-only re-arm control, and CaMeL IFC capability gating.
//!
//! Refined by:
//! - `threat.execution.agent-tool-circuit-breaker` (`AgentToolCircuitBreaker.qnt`)
//! - `threat.execution.agent-ifc-capability` (`AgentIfcCapability.qnt`)
//! - `dev/quint/threat/MarlinBackendExecutionBreakerOverlay.qnt`

use std::{
    collections::BTreeMap,
    error::Error,
    fmt,
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

macro_rules! breaker_id {
    ($name:ident, $label:literal) => {
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, BreakerRejectionReason> {
                let value = value.into();
                if value.is_empty() || value.chars().take(257).count() > 256 {
                    return Err(BreakerRejectionReason::InvalidIdentifier($label));
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

breaker_id!(BreakerActorId, "execution breaker actor identity");
breaker_id!(ToolExecutionLeaseId, "tool execution lease identity");
breaker_id!(BreakerTripReason, "execution breaker trip reason");

/// Monotonic trip epoch (`>= 1`). Every verified-human re-arm advances this counter
/// so any lease issued before the trip can never commit.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BreakerTripEpoch(u64);

impl BreakerTripEpoch {
    pub const INITIAL: Self = Self(1);

    pub fn new(value: u64) -> Result<Self, BreakerRejectionReason> {
        if value == 0 {
            return Err(BreakerRejectionReason::InvalidTripEpoch(value));
        }
        Ok(Self(value))
    }

    pub fn get(self) -> u64 {
        self.0
    }

    pub fn next(self) -> Result<Self, BreakerRejectionReason> {
        self.0
            .checked_add(1)
            .map(Self)
            .ok_or(BreakerRejectionReason::TripEpochExhausted)
    }
}

/// Maximum number of committed side effects permitted within one armed window.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ToolBudgetUnits(u32);

impl ToolBudgetUnits {
    pub const fn new(units: u32) -> Self {
        Self(units)
    }

    pub const fn get(self) -> u32 {
        self.0
    }
}

/// Authenticated actor requesting a breaker control-plane transition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BreakerActor {
    /// Step-up / cryptographically verified human operator.
    VerifiedHuman(BreakerActorId),
    /// Human identity without verified step-up / authentication proof.
    UnverifiedActor(BreakerActorId),
    /// Autonomous or delegated AI agent (strictly forbidden from re-arming).
    Agent(BreakerActorId),
}

impl BreakerActor {
    pub fn id(&self) -> &BreakerActorId {
        match self {
            Self::VerifiedHuman(id) | Self::UnverifiedActor(id) | Self::Agent(id) => id,
        }
    }
}

/// Information-flow control (CaMeL/FIDES) context evaluated before committing a side effect.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ToolIfcContext {
    pub plan_integrity_high: bool,
    pub tool_controlled_by_tainted_data: bool,
    pub carries_confidential_data: bool,
    pub declassification_approved: bool,
}

impl ToolIfcContext {
    pub const fn clean_trusted() -> Self {
        Self {
            plan_integrity_high: true,
            tool_controlled_by_tainted_data: false,
            carries_confidential_data: false,
            declassification_approved: false,
        }
    }

    pub const fn tainted_control_flow() -> Self {
        Self {
            plan_integrity_high: true,
            tool_controlled_by_tainted_data: true,
            carries_confidential_data: false,
            declassification_approved: false,
        }
    }

    pub const fn confidential_egress(declassification_approved: bool) -> Self {
        Self {
            plan_integrity_high: true,
            tool_controlled_by_tainted_data: false,
            carries_confidential_data: true,
            declassification_approved,
        }
    }
}

impl Default for ToolIfcContext {
    fn default() -> Self {
        Self::clean_trusted()
    }
}

/// Typed, fail-closed rejection reasons for the AI/tool execution circuit breaker.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BreakerRejectionReason {
    InvalidIdentifier(&'static str),
    InvalidTripEpoch(u64),
    TripEpochExhausted,
    BreakerTripped {
        trip_epoch: BreakerTripEpoch,
        reason: Option<BreakerTripReason>,
    },
    ActiveLeaseExists(ToolExecutionLeaseId),
    LeaseNotFound(ToolExecutionLeaseId),
    StaleLeaseEpoch {
        lease_epoch: BreakerTripEpoch,
        current_epoch: BreakerTripEpoch,
    },
    BudgetExhausted {
        used: u32,
        limit: u32,
    },
    NotTripped,
    HumanOnlyRearmRequired(BreakerActor),
    LowIntegrityPlanDenied,
    PromptInjectionControlHijackDenied,
    ConfidentialExfiltrationDenied,
    LockPoisoned,
}

impl fmt::Display for BreakerRejectionReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIdentifier(label) => write!(f, "invalid identifier for {label}"),
            Self::InvalidTripEpoch(epoch) => write!(f, "invalid trip epoch {epoch}"),
            Self::TripEpochExhausted => write!(f, "breaker trip epoch exhausted"),
            Self::BreakerTripped { trip_epoch, reason } => match reason {
                Some(reason) => write!(
                    f,
                    "execution breaker tripped at epoch {}: {reason}",
                    trip_epoch.get()
                ),
                None => write!(f, "execution breaker tripped at epoch {}", trip_epoch.get()),
            },
            Self::ActiveLeaseExists(id) => {
                write!(f, "active tool execution lease already held: {id}")
            }
            Self::LeaseNotFound(id) => write!(f, "tool execution lease not found: {id}"),
            Self::StaleLeaseEpoch {
                lease_epoch,
                current_epoch,
            } => write!(
                f,
                "stale tool lease epoch {} != current breaker epoch {}",
                lease_epoch.get(),
                current_epoch.get()
            ),
            Self::BudgetExhausted { used, limit } => {
                write!(f, "tool execution budget exhausted ({used}/{limit})")
            }
            Self::NotTripped => write!(f, "execution breaker is not tripped"),
            Self::HumanOnlyRearmRequired(actor) => {
                write!(f, "verified human operator required to re-arm breaker (got {actor:?})")
            }
            Self::LowIntegrityPlanDenied => {
                write!(f, "tool execution blocked: plan integrity is low")
            }
            Self::PromptInjectionControlHijackDenied => {
                write!(f, "tool execution blocked: CaMeL control-flow controlled by tainted data")
            }
            Self::ConfidentialExfiltrationDenied => {
                write!(
                    f,
                    "tool execution blocked: confidential egress requires explicit declassification"
                )
            }
            Self::LockPoisoned => write!(f, "execution circuit breaker lock poisoned"),
        }
    }
}

impl Error for BreakerRejectionReason {}

/// Epoch-fenced lease acquired prior to executing an irreversible tool side effect.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolExecutionLease {
    pub lease_id: ToolExecutionLeaseId,
    pub trip_epoch: BreakerTripEpoch,
    pub tool_label: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionBreakerSnapshot {
    pub tripped: bool,
    pub trip_epoch: BreakerTripEpoch,
    pub budget_used: u32,
    pub budget_limit: u32,
    pub active_lease_count: usize,
    pub last_trip_reason: Option<BreakerTripReason>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionBreakerCommitReceipt {
    pub lease_id: ToolExecutionLeaseId,
    pub trip_epoch: BreakerTripEpoch,
    pub budget_used: u32,
    pub budget_limit: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionBreakerTripReceipt {
    pub trip_epoch: BreakerTripEpoch,
    pub already_tripped: bool,
    pub fenced_active_leases: usize,
    pub reason: BreakerTripReason,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionBreakerRearmReceipt {
    pub previous_epoch: BreakerTripEpoch,
    pub new_epoch: BreakerTripEpoch,
    pub approved_by: BreakerActorId,
}

#[derive(Debug)]
struct BreakerInner {
    tripped: bool,
    trip_epoch: BreakerTripEpoch,
    budget_used: u32,
    budget_limit: u32,
    single_lease_enforced: bool,
    active_leases: BTreeMap<ToolExecutionLeaseId, BreakerTripEpoch>,
    last_trip_reason: Option<BreakerTripReason>,
}

/// Authoritative fail-closed AI/Tool Execution Circuit Breaker.
#[derive(Debug)]
pub struct ExecutionCircuitBreaker {
    inner: Mutex<BreakerInner>,
    auto_lease_seq: AtomicU64,
}

impl ExecutionCircuitBreaker {
    pub fn new(budget_limit: ToolBudgetUnits) -> Self {
        Self {
            inner: Mutex::new(BreakerInner {
                tripped: false,
                trip_epoch: BreakerTripEpoch::INITIAL,
                budget_used: 0,
                budget_limit: budget_limit.get(),
                single_lease_enforced: false,
                active_leases: BTreeMap::new(),
                last_trip_reason: None,
            }),
            auto_lease_seq: AtomicU64::new(1),
        }
    }

    /// Enables strict single-in-flight-lease semantics matching the finite Quint
    /// `AgentToolCircuitBreaker` specification.
    pub fn with_single_lease_enforced(self, enforced: bool) -> Self {
        if let Ok(mut guard) = self.inner.lock() {
            guard.single_lease_enforced = enforced;
        }
        self
    }

    pub fn snapshot(&self) -> Result<ExecutionBreakerSnapshot, BreakerRejectionReason> {
        let guard = self
            .inner
            .lock()
            .map_err(|_| BreakerRejectionReason::LockPoisoned)?;
        Ok(ExecutionBreakerSnapshot {
            tripped: guard.tripped,
            trip_epoch: guard.trip_epoch,
            budget_used: guard.budget_used,
            budget_limit: guard.budget_limit,
            active_lease_count: guard.active_leases.len(),
            last_trip_reason: guard.last_trip_reason.clone(),
        })
    }

    /// Phase 1: acquires a fenced tool-execution lease bound to the current `trip_epoch`.
    pub fn acquire_lease(
        &self,
        lease_id: ToolExecutionLeaseId,
        tool_label: impl Into<String>,
    ) -> Result<ToolExecutionLease, BreakerRejectionReason> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| BreakerRejectionReason::LockPoisoned)?;

        if guard.tripped {
            return Err(BreakerRejectionReason::BreakerTripped {
                trip_epoch: guard.trip_epoch,
                reason: guard.last_trip_reason.clone(),
            });
        }

        if guard.single_lease_enforced && !guard.active_leases.is_empty() {
            let existing = guard
                .active_leases
                .keys()
                .next()
                .cloned()
                .unwrap_or_else(|| lease_id.clone());
            return Err(BreakerRejectionReason::ActiveLeaseExists(existing));
        }

        if guard.active_leases.contains_key(&lease_id) {
            return Err(BreakerRejectionReason::ActiveLeaseExists(lease_id));
        }

        let epoch = guard.trip_epoch;
        guard.active_leases.insert(lease_id.clone(), epoch);
        Ok(ToolExecutionLease {
            lease_id,
            trip_epoch: epoch,
            tool_label: tool_label.into(),
        })
    }

    /// Phase 2: commits an irreversible side effect under a clean trusted IFC context.
    pub fn commit_effect(
        &self,
        lease: &ToolExecutionLease,
    ) -> Result<ExecutionBreakerCommitReceipt, BreakerRejectionReason> {
        self.commit_effect_with_ifc(lease, &ToolIfcContext::clean_trusted())
    }

    /// Phase 2 with CaMeL/FIDES IFC gate: verifies breaker armed, lease epoch current,
    /// budget available, control-data segregation, and confidential egress declassification.
    pub fn commit_effect_with_ifc(
        &self,
        lease: &ToolExecutionLease,
        ifc: &ToolIfcContext,
    ) -> Result<ExecutionBreakerCommitReceipt, BreakerRejectionReason> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| BreakerRejectionReason::LockPoisoned)?;

        if guard.tripped {
            return Err(BreakerRejectionReason::BreakerTripped {
                trip_epoch: guard.trip_epoch,
                reason: guard.last_trip_reason.clone(),
            });
        }

        let Some(&recorded_epoch) = guard.active_leases.get(&lease.lease_id) else {
            return Err(BreakerRejectionReason::LeaseNotFound(
                lease.lease_id.clone(),
            ));
        };

        if recorded_epoch != guard.trip_epoch || lease.trip_epoch != guard.trip_epoch {
            return Err(BreakerRejectionReason::StaleLeaseEpoch {
                lease_epoch: lease.trip_epoch,
                current_epoch: guard.trip_epoch,
            });
        }

        if guard.budget_used >= guard.budget_limit {
            return Err(BreakerRejectionReason::BudgetExhausted {
                used: guard.budget_used,
                limit: guard.budget_limit,
            });
        }

        if !ifc.plan_integrity_high {
            return Err(BreakerRejectionReason::LowIntegrityPlanDenied);
        }
        if ifc.tool_controlled_by_tainted_data {
            return Err(BreakerRejectionReason::PromptInjectionControlHijackDenied);
        }
        if ifc.carries_confidential_data && !ifc.declassification_approved {
            return Err(BreakerRejectionReason::ConfidentialExfiltrationDenied);
        }

        guard.active_leases.remove(&lease.lease_id);
        guard.budget_used = guard.budget_used.saturating_add(1);

        Ok(ExecutionBreakerCommitReceipt {
            lease_id: lease.lease_id.clone(),
            trip_epoch: guard.trip_epoch,
            budget_used: guard.budget_used,
            budget_limit: guard.budget_limit,
        })
    }

    /// Releases a lease that has been fenced by a trip or epoch advance.
    pub fn cancel_stale_lease(
        &self,
        lease_id: &ToolExecutionLeaseId,
    ) -> Result<bool, BreakerRejectionReason> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| BreakerRejectionReason::LockPoisoned)?;

        let Some(&lease_epoch) = guard.active_leases.get(lease_id) else {
            return Ok(false);
        };

        if guard.tripped || lease_epoch != guard.trip_epoch {
            guard.active_leases.remove(lease_id);
            return Ok(true);
        }

        Ok(false)
    }

    /// Emergency-Stop ("脱扣"): fail-closed latch that immediately halts all new leases
    /// and blocks commit of every in-flight lease.
    pub fn trip(
        &self,
        reason: BreakerTripReason,
    ) -> Result<ExecutionBreakerTripReceipt, BreakerRejectionReason> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| BreakerRejectionReason::LockPoisoned)?;

        let already_tripped = guard.tripped;
        guard.tripped = true;
        guard.last_trip_reason = Some(reason.clone());
        Ok(ExecutionBreakerTripReceipt {
            trip_epoch: guard.trip_epoch,
            already_tripped,
            fenced_active_leases: guard.active_leases.len(),
            reason,
        })
    }

    /// Verified human-only re-arm. Advances `trip_epoch` monotonically and resets
    /// the armed-window tool budget while permanently fencing all pre-trip leases.
    pub fn rearm(
        &self,
        actor: &BreakerActor,
    ) -> Result<ExecutionBreakerRearmReceipt, BreakerRejectionReason> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| BreakerRejectionReason::LockPoisoned)?;

        if !guard.tripped {
            return Err(BreakerRejectionReason::NotTripped);
        }

        let human_id = match actor {
            BreakerActor::VerifiedHuman(id) => id.clone(),
            BreakerActor::UnverifiedActor(_) | BreakerActor::Agent(_) => {
                return Err(BreakerRejectionReason::HumanOnlyRearmRequired(
                    actor.clone(),
                ));
            }
        };

        let previous_epoch = guard.trip_epoch;
        let new_epoch = previous_epoch.next()?;
        guard.tripped = false;
        guard.trip_epoch = new_epoch;
        guard.budget_used = 0;

        Ok(ExecutionBreakerRearmReceipt {
            previous_epoch,
            new_epoch,
            approved_by: human_id,
        })
    }

    /// Restores durable persisted circuit-breaker state across an OS process restart.
    ///
    /// Invariants:
    /// - Clears volatile in-flight leases (killed by process crash/restart).
    /// - Preserves monotonic `trip_epoch` (never regresses below current live epoch).
    /// - Preserves persisted `tripped` fail-closed latch (anti-restart-reset bypass).
    pub fn restore_durable_state(
        &self,
        tripped: bool,
        trip_epoch: BreakerTripEpoch,
        budget_used: u32,
        last_trip_reason: Option<BreakerTripReason>,
    ) -> Result<ExecutionBreakerSnapshot, BreakerRejectionReason> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| BreakerRejectionReason::LockPoisoned)?;

        if trip_epoch < guard.trip_epoch {
            return Err(BreakerRejectionReason::StaleLeaseEpoch {
                lease_epoch: trip_epoch,
                current_epoch: guard.trip_epoch,
            });
        }
        guard.active_leases.clear();
        guard.tripped = tripped;
        guard.trip_epoch = trip_epoch;
        guard.budget_used = budget_used.min(guard.budget_limit);
        guard.last_trip_reason = last_trip_reason;

        Ok(ExecutionBreakerSnapshot {
            tripped: guard.tripped,
            trip_epoch: guard.trip_epoch,
            budget_used: guard.budget_used,
            budget_limit: guard.budget_limit,
            active_lease_count: 0,
            last_trip_reason: guard.last_trip_reason.clone(),
        })
    }

    /// Atomic acquire + commit helper used at real OS side-effect boundaries
    /// (process spawn & workspace file write) so rejected commits clean up their lease.
    pub fn authorize_side_effect(
        &self,
        tool_label: impl Into<String>,
        ifc: &ToolIfcContext,
    ) -> Result<ExecutionBreakerCommitReceipt, BreakerRejectionReason> {
        let seq = self.auto_lease_seq.fetch_add(1, Ordering::Relaxed);
        let lease_id = ToolExecutionLeaseId::new(format!("side-effect-lease-{seq}"))?;
        let lease = self.acquire_lease(lease_id, tool_label)?;
        match self.commit_effect_with_ifc(&lease, ifc) {
            Ok(receipt) => Ok(receipt),
            Err(rejection) => {
                if let Ok(mut guard) = self.inner.lock() {
                    guard.active_leases.remove(&lease.lease_id);
                }
                Err(rejection)
            }
        }
    }
}
