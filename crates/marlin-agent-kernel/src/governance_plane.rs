// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Unified Rust Backend Security Governance Plane:
//! 1. `DelegatedAgentAuthzLedger` — RFC 8693 OBO scope attenuation, actor provenance,
//!    monotonic session-revocation fencing, and RFC 9470 step-up verification floor
//!    (`threat.identity.delegated-agent-authz`).
//! 2. `MultiTenantIsolationFence` — Tenant-resource binding, monotonic tenant revocation
//!    epochs, and tenant-pinned offline outbox flushing (`threat.zero-trust.multi-tenant-isolation`).
//! 3. `ChatroomIngressGovernor` — External ingress origin/forgery verification, channel-thread
//!    affinity, one-shot human gates, matter gate locks, and tainted-tool human gating
//!    (`threat.protocol.chatroom-ingress-governance`).
//! 4. `DualControlSodLedger` — Separation of duties (`proposer != approver`) and veto
//!    precedence (`threat.zero-trust.governance-sod`).
//! 5. `BackendSecurityGovernanceCoordinator` — Composes all four ledgers with
//!    `ExecutionCircuitBreaker` so side effects and E-Stop re-arm are fail-closed end to end.

use std::{
    collections::BTreeMap,
    error::Error,
    fmt,
    sync::{Arc, Mutex},
};

use crate::execution_breaker::{
    BreakerActor, BreakerActorId, BreakerRejectionReason, BreakerTripReason,
    ExecutionBreakerCommitReceipt, ExecutionBreakerRearmReceipt, ExecutionCircuitBreaker,
    ToolIfcContext,
};

macro_rules! gov_plane_id {
    ($name:ident, $label:literal) => {
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, GovernancePlaneRejection> {
                let value = value.into();
                if value.is_empty() || value.chars().take(257).count() > 256 {
                    return Err(GovernancePlaneRejection::InvalidIdentifier($label));
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

gov_plane_id!(GovernanceTenantId, "governance tenant identity");
gov_plane_id!(GovernanceActorId, "governance actor identity");
gov_plane_id!(DelegationTicketId, "OBO delegation ticket identity");
gov_plane_id!(TenantExecutionTicketId, "tenant execution ticket identity");
gov_plane_id!(TenantOutboxEntryId, "tenant outbox entry identity");
gov_plane_id!(GovernanceChannelId, "governance chatroom channel identity");
gov_plane_id!(GovernanceGateId, "governance human gate identity");
gov_plane_id!(GovernanceProposalId, "dual-control SoD proposal identity");

/// Monotonic authority/revocation epoch (`>= 1`).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct AuthorityEpoch(u64);

impl AuthorityEpoch {
    pub const INITIAL: Self = Self(1);

    pub fn new(value: u64) -> Result<Self, GovernancePlaneRejection> {
        if value == 0 {
            return Err(GovernancePlaneRejection::InvalidEpoch(value));
        }
        Ok(Self(value))
    }

    pub fn get(self) -> u64 {
        self.0
    }

    pub fn next(self) -> Result<Self, GovernancePlaneRejection> {
        self.0
            .checked_add(1)
            .map(Self)
            .ok_or(GovernancePlaneRejection::EpochExhausted)
    }
}

/// Ordered delegation privilege scope (`ReadOnly(1) < WorkspaceWrite(2) < PrivilegedAdmin(3)`).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum DelegationScopeLevel {
    ReadOnly = 1,
    WorkspaceWrite = 2,
    PrivilegedAdmin = 3,
}

impl DelegationScopeLevel {
    pub fn from_level(level: u8) -> Result<Self, GovernancePlaneRejection> {
        match level {
            1 => Ok(Self::ReadOnly),
            2 => Ok(Self::WorkspaceWrite),
            3 => Ok(Self::PrivilegedAdmin),
            _ => Err(GovernancePlaneRejection::InvalidScopeLevel(level)),
        }
    }

    pub fn level(self) -> u8 {
        self as u8
    }
}

/// Typed actor distinguishing human principals from delegated AI agents.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GovernancePrincipal {
    Human(GovernanceActorId),
    DelegatedAgent {
        agent_id: GovernanceActorId,
        acting_for: GovernanceActorId,
    },
}

impl GovernancePrincipal {
    pub fn actor_id(&self) -> &GovernanceActorId {
        match self {
            Self::Human(id) => id,
            Self::DelegatedAgent { agent_id, .. } => agent_id,
        }
    }

    pub fn is_agent(&self) -> bool {
        matches!(self, Self::DelegatedAgent { .. })
    }
}

/// Typed fail-closed denial reasons across the entire backend governance plane.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GovernancePlaneRejection {
    InvalidIdentifier(&'static str),
    InvalidEpoch(u64),
    EpochExhausted,
    InvalidScopeLevel(u8),
    HumanSessionSignedOut,
    ActiveDelegationExists(DelegationTicketId),
    DelegationNotFound(DelegationTicketId),
    ScopeAmplificationDenied {
        requested: DelegationScopeLevel,
        human_ceiling: DelegationScopeLevel,
    },
    AgentImpersonationDenied {
        action: &'static str,
        agent_id: GovernanceActorId,
    },
    ZombieDelegationDenied {
        ticket_epoch: AuthorityEpoch,
        current_epoch: AuthorityEpoch,
    },
    StepUpVerificationRequired,
    StepUpAlreadyFresh,
    TenantNotFound(GovernanceTenantId),
    TenantRevoked(GovernanceTenantId),
    CrossTenantDeputyDenied {
        ticket_tenant: GovernanceTenantId,
        target_tenant: GovernanceTenantId,
    },
    RevokedTenantTicketDenied {
        tenant_id: GovernanceTenantId,
        ticket_epoch: AuthorityEpoch,
        current_epoch: AuthorityEpoch,
    },
    CrossTenantOutboxFlushDenied {
        outbox_tenant: GovernanceTenantId,
        active_tenant: GovernanceTenantId,
    },
    OutboxEmpty,
    OutboxAlreadyPending,
    ForgedExternalIngressDenied,
    CrossChannelInjectionDenied {
        target_channel: GovernanceChannelId,
        thread_root_channel: GovernanceChannelId,
    },
    GateNotFound(GovernanceGateId),
    GateAlreadyPending(GovernanceGateId),
    GateReplayDenied {
        gate_id: GovernanceGateId,
        current_status: ChatroomGateStatus,
    },
    AgentSelfUnblockDenied(GovernanceActorId),
    MatterNotBlocked,
    TaintedChatroomToolBypassDenied(GovernanceChannelId),
    ProposalAlreadyExists(GovernanceProposalId),
    ProposalNotFound(GovernanceProposalId),
    SodSelfApprovalDenied {
        proposal_id: GovernanceProposalId,
        actor_id: GovernanceActorId,
    },
    ProposalAlreadyApproved(GovernanceProposalId),
    ProposalNotApproved(GovernanceProposalId),
    ProposalAlreadyExecuted(GovernanceProposalId),
    VetoedProposalDenied(GovernanceProposalId),
    ExecutionBreaker(BreakerRejectionReason),
    LockPoisoned,
}

impl fmt::Display for GovernancePlaneRejection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIdentifier(label) => write!(f, "invalid identifier for {label}"),
            Self::InvalidEpoch(e) => write!(f, "invalid authority epoch {e}"),
            Self::EpochExhausted => write!(f, "authority epoch overflow"),
            Self::InvalidScopeLevel(lvl) => write!(f, "invalid delegation scope level {lvl}"),
            Self::HumanSessionSignedOut => write!(f, "human authority session is signed out"),
            Self::ActiveDelegationExists(id) => write!(f, "active OBO delegation already exists: {id}"),
            Self::DelegationNotFound(id) => write!(f, "OBO delegation ticket not found: {id}"),
            Self::ScopeAmplificationDenied {
                requested,
                human_ceiling,
            } => write!(
                f,
                "OBO scope amplification denied: requested {requested:?} > human ceiling {human_ceiling:?}"
            ),
            Self::AgentImpersonationDenied { action, agent_id } => {
                write!(f, "delegated agent {agent_id} cannot impersonate human for {action}")
            }
            Self::ZombieDelegationDenied {
                ticket_epoch,
                current_epoch,
            } => write!(
                f,
                "revoked/stale OBO delegation epoch {} != current session epoch {}",
                ticket_epoch.get(),
                current_epoch.get()
            ),
            Self::StepUpVerificationRequired => {
                write!(f, "verified email + fresh RFC 9470 step-up session required")
            }
            Self::StepUpAlreadyFresh => write!(f, "step-up session is already fresh"),
            Self::TenantNotFound(t) => write!(f, "tenant not found: {t}"),
            Self::TenantRevoked(t) => write!(f, "tenant is revoked: {t}"),
            Self::CrossTenantDeputyDenied {
                ticket_tenant,
                target_tenant,
            } => write!(
                f,
                "cross-tenant deputy denied: ticket tenant {ticket_tenant} != target {target_tenant}"
            ),
            Self::RevokedTenantTicketDenied {
                tenant_id,
                ticket_epoch,
                current_epoch,
            } => write!(
                f,
                "stale/revoked ticket for tenant {tenant_id}: epoch {} != {}",
                ticket_epoch.get(),
                current_epoch.get()
            ),
            Self::CrossTenantOutboxFlushDenied {
                outbox_tenant,
                active_tenant,
            } => write!(
                f,
                "cross-tenant outbox flush denied: entry pinned to {outbox_tenant}, active {active_tenant}"
            ),
            Self::OutboxEmpty => write!(f, "tenant outbox has no pending entry"),
            Self::OutboxAlreadyPending => write!(f, "tenant outbox already has a pending entry"),
            Self::ForgedExternalIngressDenied => {
                write!(f, "forged or unverified external chatroom ingress denied")
            }
            Self::CrossChannelInjectionDenied {
                target_channel,
                thread_root_channel,
            } => write!(
                f,
                "cross-channel thread injection denied: target {target_channel} != root {thread_root_channel}"
            ),
            Self::GateNotFound(id) => write!(f, "chatroom human gate not found: {id}"),
            Self::GateAlreadyPending(id) => write!(f, "chatroom human gate already pending: {id}"),
            Self::GateReplayDenied {
                gate_id,
                current_status,
            } => write!(
                f,
                "one-shot human gate {gate_id} cannot be replayed from status {current_status:?}"
            ),
            Self::AgentSelfUnblockDenied(id) => {
                write!(f, "delegated agent {id} cannot self-unblock gate-locked matter")
            }
            Self::MatterNotBlocked => write!(f, "matter is not blocked by a gate"),
            Self::TaintedChatroomToolBypassDenied(ch) => write!(
                f,
                "tainted external ingress in channel {ch} requires approved human gate before tool execution"
            ),
            Self::ProposalAlreadyExists(id) => write!(f, "SoD proposal already exists: {id}"),
            Self::ProposalNotFound(id) => write!(f, "SoD proposal not found: {id}"),
            Self::SodSelfApprovalDenied {
                proposal_id,
                actor_id,
            } => write!(
                f,
                "separation of duties violated on {proposal_id}: proposer {actor_id} cannot self-approve"
            ),
            Self::ProposalAlreadyApproved(id) => write!(f, "SoD proposal already approved: {id}"),
            Self::ProposalNotApproved(id) => write!(f, "SoD proposal has not been approved: {id}"),
            Self::ProposalAlreadyExecuted(id) => write!(f, "SoD proposal already executed: {id}"),
            Self::VetoedProposalDenied(id) => write!(f, "vetoed SoD proposal {id} cannot proceed"),
            Self::ExecutionBreaker(err) => write!(f, "execution breaker denied: {err}"),
            Self::LockPoisoned => write!(f, "governance plane lock poisoned"),
        }
    }
}

impl Error for GovernancePlaneRejection {}

impl From<BreakerRejectionReason> for GovernancePlaneRejection {
    fn from(value: BreakerRejectionReason) -> Self {
        Self::ExecutionBreaker(value)
    }
}

// ===========================================================================
// 1. OBO Delegated Agent Authorization & RFC 9470 Step-Up Ledger
// ===========================================================================

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DelegatedAgentTicket {
    pub ticket_id: DelegationTicketId,
    pub agent_id: GovernanceActorId,
    pub acting_for: GovernanceActorId,
    pub auth_epoch: AuthorityEpoch,
    pub scope_level: DelegationScopeLevel,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DelegatedExecutionReceipt {
    pub ticket_id: DelegationTicketId,
    pub executed_scope: DelegationScopeLevel,
    pub auth_epoch: AuthorityEpoch,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PrivilegedHumanReceipt {
    pub actor_id: GovernanceActorId,
    pub auth_epoch: AuthorityEpoch,
}

#[derive(Clone, Debug)]
pub struct DelegatedAgentAuthzLedger {
    human_actor_id: GovernanceActorId,
    human_auth_epoch: AuthorityEpoch,
    human_signed_in: bool,
    human_scope_level: DelegationScopeLevel,
    email_verified: bool,
    step_up_fresh: bool,
    single_delegation_enforced: bool,
    active_tickets: BTreeMap<DelegationTicketId, DelegatedAgentTicket>,
    human_gates_approved: u64,
}

impl DelegatedAgentAuthzLedger {
    pub fn new(
        human_actor_id: GovernanceActorId,
        human_scope_level: DelegationScopeLevel,
        email_verified: bool,
    ) -> Self {
        Self {
            human_actor_id,
            human_auth_epoch: AuthorityEpoch::INITIAL,
            human_signed_in: true,
            human_scope_level,
            email_verified,
            step_up_fresh: false,
            single_delegation_enforced: false,
            active_tickets: BTreeMap::new(),
            human_gates_approved: 0,
        }
    }

    pub fn with_single_delegation_enforced(mut self, enforced: bool) -> Self {
        self.single_delegation_enforced = enforced;
        self
    }

    pub fn auth_epoch(&self) -> AuthorityEpoch {
        self.human_auth_epoch
    }

    pub fn is_signed_in(&self) -> bool {
        self.human_signed_in
    }

    pub fn is_step_up_fresh(&self) -> bool {
        self.step_up_fresh
    }

    pub fn human_gates_approved(&self) -> u64 {
        self.human_gates_approved
    }

    pub fn perform_step_up(&mut self) -> Result<(), GovernancePlaneRejection> {
        if !self.human_signed_in {
            return Err(GovernancePlaneRejection::HumanSessionSignedOut);
        }
        if !self.email_verified {
            return Err(GovernancePlaneRejection::StepUpVerificationRequired);
        }
        if self.step_up_fresh {
            return Err(GovernancePlaneRejection::StepUpAlreadyFresh);
        }
        self.step_up_fresh = true;
        Ok(())
    }

    pub fn revoke_human_session(&mut self) -> Result<AuthorityEpoch, GovernancePlaneRejection> {
        if !self.human_signed_in {
            return Err(GovernancePlaneRejection::HumanSessionSignedOut);
        }
        self.human_signed_in = false;
        self.step_up_fresh = false;
        self.human_auth_epoch = self.human_auth_epoch.next()?;
        Ok(self.human_auth_epoch)
    }

    pub fn reauthenticate_human(
        &mut self,
        email_verified: bool,
    ) -> Result<AuthorityEpoch, GovernancePlaneRejection> {
        self.human_signed_in = true;
        self.email_verified = email_verified;
        self.step_up_fresh = false;
        self.human_auth_epoch = self.human_auth_epoch.next()?;
        Ok(self.human_auth_epoch)
    }

    pub fn delegate_to_agent(
        &mut self,
        ticket_id: DelegationTicketId,
        agent_id: GovernanceActorId,
        requested_scope: DelegationScopeLevel,
    ) -> Result<DelegatedAgentTicket, GovernancePlaneRejection> {
        if !self.human_signed_in {
            return Err(GovernancePlaneRejection::HumanSessionSignedOut);
        }
        if self.single_delegation_enforced && !self.active_tickets.is_empty() {
            let existing = self
                .active_tickets
                .keys()
                .next()
                .cloned()
                .unwrap_or(ticket_id);
            return Err(GovernancePlaneRejection::ActiveDelegationExists(existing));
        }
        if requested_scope > self.human_scope_level {
            return Err(GovernancePlaneRejection::ScopeAmplificationDenied {
                requested: requested_scope,
                human_ceiling: self.human_scope_level,
            });
        }
        let ticket = DelegatedAgentTicket {
            ticket_id: ticket_id.clone(),
            agent_id,
            acting_for: self.human_actor_id.clone(),
            auth_epoch: self.human_auth_epoch,
            scope_level: requested_scope,
        };
        self.active_tickets.insert(ticket_id, ticket.clone());
        Ok(ticket)
    }

    pub fn execute_delegated(
        &mut self,
        ticket: &DelegatedAgentTicket,
    ) -> Result<DelegatedExecutionReceipt, GovernancePlaneRejection> {
        let Some(recorded) = self.active_tickets.get(&ticket.ticket_id) else {
            return Err(GovernancePlaneRejection::DelegationNotFound(
                ticket.ticket_id.clone(),
            ));
        };
        if !self.human_signed_in
            || recorded.auth_epoch != self.human_auth_epoch
            || ticket.auth_epoch != self.human_auth_epoch
        {
            return Err(GovernancePlaneRejection::ZombieDelegationDenied {
                ticket_epoch: ticket.auth_epoch,
                current_epoch: self.human_auth_epoch,
            });
        }
        let consumed = self
            .active_tickets
            .remove(&ticket.ticket_id)
            .expect("verified ticket exists");
        Ok(DelegatedExecutionReceipt {
            ticket_id: consumed.ticket_id,
            executed_scope: consumed.scope_level,
            auth_epoch: self.human_auth_epoch,
        })
    }

    pub fn approve_human_only_gate(
        &mut self,
        actor: &GovernancePrincipal,
    ) -> Result<u64, GovernancePlaneRejection> {
        if !self.human_signed_in {
            return Err(GovernancePlaneRejection::HumanSessionSignedOut);
        }
        if let GovernancePrincipal::DelegatedAgent { agent_id, .. } = actor {
            return Err(GovernancePlaneRejection::AgentImpersonationDenied {
                action: "approve_human_only_gate",
                agent_id: agent_id.clone(),
            });
        }
        self.human_gates_approved = self.human_gates_approved.saturating_add(1);
        Ok(self.human_gates_approved)
    }

    pub fn execute_privileged_human(
        &mut self,
        actor: &GovernancePrincipal,
    ) -> Result<PrivilegedHumanReceipt, GovernancePlaneRejection> {
        if let GovernancePrincipal::DelegatedAgent { agent_id, .. } = actor {
            return Err(GovernancePlaneRejection::AgentImpersonationDenied {
                action: "execute_privileged_human",
                agent_id: agent_id.clone(),
            });
        }
        if !self.human_signed_in || !self.email_verified || !self.step_up_fresh {
            return Err(GovernancePlaneRejection::StepUpVerificationRequired);
        }
        self.step_up_fresh = false;
        Ok(PrivilegedHumanReceipt {
            actor_id: actor.actor_id().clone(),
            auth_epoch: self.human_auth_epoch,
        })
    }
}

// ===========================================================================
// 2. Multi-Tenant Isolation & Offline Outbox Pinning Fence
// ===========================================================================

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TenantExecutionTicket {
    pub ticket_id: TenantExecutionTicketId,
    pub tenant_id: GovernanceTenantId,
    pub epoch: AuthorityEpoch,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TenantOutboxEntry {
    pub entry_id: TenantOutboxEntryId,
    pub tenant_id: GovernanceTenantId,
    pub payload_label: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct TenantRecord {
    epoch: AuthorityEpoch,
    revoked: bool,
}

#[derive(Clone, Debug)]
pub struct MultiTenantIsolationFence {
    active_tenant: GovernanceTenantId,
    tenants: BTreeMap<GovernanceTenantId, TenantRecord>,
    active_tickets: BTreeMap<TenantExecutionTicketId, TenantExecutionTicket>,
    pending_outbox: Option<TenantOutboxEntry>,
    executed_count: u64,
}

impl MultiTenantIsolationFence {
    pub fn new(initial_active_tenant: GovernanceTenantId) -> Self {
        let mut tenants = BTreeMap::new();
        tenants.insert(
            initial_active_tenant.clone(),
            TenantRecord {
                epoch: AuthorityEpoch::INITIAL,
                revoked: false,
            },
        );
        Self {
            active_tenant: initial_active_tenant,
            tenants,
            active_tickets: BTreeMap::new(),
            pending_outbox: None,
            executed_count: 0,
        }
    }

    pub fn register_tenant(&mut self, tenant_id: GovernanceTenantId) {
        self.tenants.entry(tenant_id).or_insert(TenantRecord {
            epoch: AuthorityEpoch::INITIAL,
            revoked: false,
        });
    }

    pub fn switch_active_tenant(
        &mut self,
        tenant_id: GovernanceTenantId,
    ) -> Result<(), GovernancePlaneRejection> {
        if !self.tenants.contains_key(&tenant_id) {
            return Err(GovernancePlaneRejection::TenantNotFound(tenant_id));
        }
        self.active_tenant = tenant_id;
        Ok(())
    }

    pub fn active_tenant(&self) -> &GovernanceTenantId {
        &self.active_tenant
    }

    pub fn executed_count(&self) -> u64 {
        self.executed_count
    }

    pub fn issue_ticket(
        &mut self,
        ticket_id: TenantExecutionTicketId,
    ) -> Result<TenantExecutionTicket, GovernancePlaneRejection> {
        let tenant_id = self.active_tenant.clone();
        let rec = self
            .tenants
            .get(&tenant_id)
            .ok_or_else(|| GovernancePlaneRejection::TenantNotFound(tenant_id.clone()))?;
        if rec.revoked {
            return Err(GovernancePlaneRejection::TenantRevoked(tenant_id));
        }
        let ticket = TenantExecutionTicket {
            ticket_id: ticket_id.clone(),
            tenant_id,
            epoch: rec.epoch,
        };
        self.active_tickets.insert(ticket_id, ticket.clone());
        Ok(ticket)
    }

    pub fn revoke_tenant(
        &mut self,
        tenant_id: &GovernanceTenantId,
    ) -> Result<AuthorityEpoch, GovernancePlaneRejection> {
        let rec = self
            .tenants
            .get_mut(tenant_id)
            .ok_or_else(|| GovernancePlaneRejection::TenantNotFound(tenant_id.clone()))?;
        rec.revoked = true;
        rec.epoch = rec.epoch.next()?;
        Ok(rec.epoch)
    }

    pub fn enforce_ticket(
        &mut self,
        ticket: &TenantExecutionTicket,
        target_tenant: &GovernanceTenantId,
    ) -> Result<u64, GovernancePlaneRejection> {
        if &ticket.tenant_id != target_tenant {
            return Err(GovernancePlaneRejection::CrossTenantDeputyDenied {
                ticket_tenant: ticket.tenant_id.clone(),
                target_tenant: target_tenant.clone(),
            });
        }
        let rec = self
            .tenants
            .get(&ticket.tenant_id)
            .ok_or_else(|| GovernancePlaneRejection::TenantNotFound(ticket.tenant_id.clone()))?;
        if rec.revoked || ticket.epoch != rec.epoch {
            return Err(GovernancePlaneRejection::RevokedTenantTicketDenied {
                tenant_id: ticket.tenant_id.clone(),
                ticket_epoch: ticket.epoch,
                current_epoch: rec.epoch,
            });
        }
        self.active_tickets.remove(&ticket.ticket_id);
        self.executed_count = self.executed_count.saturating_add(1);
        Ok(self.executed_count)
    }

    pub fn enqueue_outbox(
        &mut self,
        entry_id: TenantOutboxEntryId,
        payload_label: impl Into<String>,
    ) -> Result<TenantOutboxEntry, GovernancePlaneRejection> {
        if self.pending_outbox.is_some() {
            return Err(GovernancePlaneRejection::OutboxAlreadyPending);
        }
        let tenant_id = self.active_tenant.clone();
        let rec = self
            .tenants
            .get(&tenant_id)
            .ok_or_else(|| GovernancePlaneRejection::TenantNotFound(tenant_id.clone()))?;
        if rec.revoked {
            return Err(GovernancePlaneRejection::TenantRevoked(tenant_id));
        }
        let entry = TenantOutboxEntry {
            entry_id,
            tenant_id,
            payload_label: payload_label.into(),
        };
        self.pending_outbox = Some(entry.clone());
        Ok(entry)
    }

    pub fn flush_outbox(&mut self) -> Result<TenantOutboxEntry, GovernancePlaneRejection> {
        let entry = self
            .pending_outbox
            .as_ref()
            .ok_or(GovernancePlaneRejection::OutboxEmpty)?;
        if entry.tenant_id != self.active_tenant {
            return Err(GovernancePlaneRejection::CrossTenantOutboxFlushDenied {
                outbox_tenant: entry.tenant_id.clone(),
                active_tenant: self.active_tenant.clone(),
            });
        }
        let rec = self
            .tenants
            .get(&entry.tenant_id)
            .ok_or_else(|| GovernancePlaneRejection::TenantNotFound(entry.tenant_id.clone()))?;
        if rec.revoked {
            return Err(GovernancePlaneRejection::TenantRevoked(
                entry.tenant_id.clone(),
            ));
        }
        Ok(self.pending_outbox.take().expect("verified pending outbox"))
    }
}

// ===========================================================================
// 3. Multi-Party Chatroom Ingress Governance & One-Shot Human Gate
// ===========================================================================

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChatroomGateStatus {
    None,
    Pending,
    Approved,
    Rejected,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExternalIngressRequest {
    pub origin_verified: bool,
    pub claims_gate_receipt: bool,
    pub target_channel: GovernanceChannelId,
    pub thread_root_channel: GovernanceChannelId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChatroomIngressReceipt {
    pub channel_id: GovernanceChannelId,
    pub tainted: bool,
}

#[derive(Clone, Debug)]
pub struct ChatroomIngressGovernor {
    channel_id: GovernanceChannelId,
    current_gate_id: Option<GovernanceGateId>,
    gate_status: ChatroomGateStatus,
    matter_blocked_by_gate: bool,
    tainted_external_msg: bool,
    tools_executed: u64,
}

impl ChatroomIngressGovernor {
    pub fn new(channel_id: GovernanceChannelId) -> Self {
        Self {
            channel_id,
            current_gate_id: None,
            gate_status: ChatroomGateStatus::None,
            matter_blocked_by_gate: false,
            tainted_external_msg: false,
            tools_executed: 0,
        }
    }

    pub fn gate_status(&self) -> ChatroomGateStatus {
        self.gate_status
    }

    pub fn is_matter_blocked(&self) -> bool {
        self.matter_blocked_by_gate
    }

    pub fn is_tainted(&self) -> bool {
        self.tainted_external_msg
    }

    pub fn ingest_external(
        &mut self,
        request: &ExternalIngressRequest,
    ) -> Result<ChatroomIngressReceipt, GovernancePlaneRejection> {
        if !request.origin_verified || request.claims_gate_receipt {
            return Err(GovernancePlaneRejection::ForgedExternalIngressDenied);
        }
        if request.target_channel != request.thread_root_channel {
            return Err(GovernancePlaneRejection::CrossChannelInjectionDenied {
                target_channel: request.target_channel.clone(),
                thread_root_channel: request.thread_root_channel.clone(),
            });
        }
        self.tainted_external_msg = true;
        Ok(ChatroomIngressReceipt {
            channel_id: request.target_channel.clone(),
            tainted: true,
        })
    }

    pub fn request_human_gate(
        &mut self,
        gate_id: GovernanceGateId,
    ) -> Result<(), GovernancePlaneRejection> {
        if self.gate_status == ChatroomGateStatus::Pending {
            let existing = self.current_gate_id.clone().unwrap_or(gate_id);
            return Err(GovernancePlaneRejection::GateAlreadyPending(existing));
        }
        self.current_gate_id = Some(gate_id);
        self.gate_status = ChatroomGateStatus::Pending;
        self.matter_blocked_by_gate = true;
        Ok(())
    }

    pub fn resolve_human_gate(
        &mut self,
        actor: &GovernancePrincipal,
        gate_id: &GovernanceGateId,
        approve: bool,
    ) -> Result<ChatroomGateStatus, GovernancePlaneRejection> {
        let Some(current) = &self.current_gate_id else {
            return Err(GovernancePlaneRejection::GateNotFound(gate_id.clone()));
        };
        if current != gate_id {
            return Err(GovernancePlaneRejection::GateNotFound(gate_id.clone()));
        }
        if let GovernancePrincipal::DelegatedAgent { agent_id, .. } = actor {
            return Err(GovernancePlaneRejection::AgentImpersonationDenied {
                action: "resolve_chatroom_human_gate",
                agent_id: agent_id.clone(),
            });
        }
        if self.gate_status != ChatroomGateStatus::Pending {
            return Err(GovernancePlaneRejection::GateReplayDenied {
                gate_id: gate_id.clone(),
                current_status: self.gate_status,
            });
        }
        self.gate_status = if approve {
            self.matter_blocked_by_gate = false;
            ChatroomGateStatus::Approved
        } else {
            ChatroomGateStatus::Rejected
        };
        Ok(self.gate_status)
    }

    pub fn unblock_matter(
        &mut self,
        actor: &GovernancePrincipal,
    ) -> Result<(), GovernancePlaneRejection> {
        if !self.matter_blocked_by_gate {
            return Err(GovernancePlaneRejection::MatterNotBlocked);
        }
        if let GovernancePrincipal::DelegatedAgent { agent_id, .. } = actor {
            return Err(GovernancePlaneRejection::AgentSelfUnblockDenied(
                agent_id.clone(),
            ));
        }
        self.matter_blocked_by_gate = false;
        Ok(())
    }

    pub fn authorize_tool_execution(&mut self) -> Result<u64, GovernancePlaneRejection> {
        if self.tainted_external_msg && self.gate_status != ChatroomGateStatus::Approved {
            return Err(GovernancePlaneRejection::TaintedChatroomToolBypassDenied(
                self.channel_id.clone(),
            ));
        }
        self.tools_executed = self.tools_executed.saturating_add(1);
        Ok(self.tools_executed)
    }
}

// ===========================================================================
// 4. Dual-Control Separation of Duties (SoD) & Veto Precedence Ledger
// ===========================================================================

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SodProposalRecord {
    pub proposal_id: GovernanceProposalId,
    pub proposer: GovernanceActorId,
    pub approver: Option<GovernanceActorId>,
    pub vetoed: bool,
    pub executed: bool,
}

#[derive(Clone, Debug, Default)]
pub struct DualControlSodLedger {
    proposals: BTreeMap<GovernanceProposalId, SodProposalRecord>,
}

impl DualControlSodLedger {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn proposal(&self, id: &GovernanceProposalId) -> Option<&SodProposalRecord> {
        self.proposals.get(id)
    }

    pub fn propose(
        &mut self,
        proposal_id: GovernanceProposalId,
        proposer: GovernanceActorId,
    ) -> Result<SodProposalRecord, GovernancePlaneRejection> {
        if self.proposals.contains_key(&proposal_id) {
            return Err(GovernancePlaneRejection::ProposalAlreadyExists(proposal_id));
        }
        let rec = SodProposalRecord {
            proposal_id: proposal_id.clone(),
            proposer,
            approver: None,
            vetoed: false,
            executed: false,
        };
        self.proposals.insert(proposal_id, rec.clone());
        Ok(rec)
    }

    pub fn approve(
        &mut self,
        proposal_id: &GovernanceProposalId,
        approver: GovernanceActorId,
    ) -> Result<SodProposalRecord, GovernancePlaneRejection> {
        let rec = self
            .proposals
            .get_mut(proposal_id)
            .ok_or_else(|| GovernancePlaneRejection::ProposalNotFound(proposal_id.clone()))?;
        if rec.vetoed {
            return Err(GovernancePlaneRejection::VetoedProposalDenied(
                proposal_id.clone(),
            ));
        }
        if rec.approver.is_some() {
            return Err(GovernancePlaneRejection::ProposalAlreadyApproved(
                proposal_id.clone(),
            ));
        }
        if rec.proposer == approver {
            return Err(GovernancePlaneRejection::SodSelfApprovalDenied {
                proposal_id: proposal_id.clone(),
                actor_id: approver,
            });
        }
        rec.approver = Some(approver);
        Ok(rec.clone())
    }

    pub fn veto(
        &mut self,
        proposal_id: &GovernanceProposalId,
    ) -> Result<SodProposalRecord, GovernancePlaneRejection> {
        let rec = self
            .proposals
            .get_mut(proposal_id)
            .ok_or_else(|| GovernancePlaneRejection::ProposalNotFound(proposal_id.clone()))?;
        if rec.executed {
            return Err(GovernancePlaneRejection::ProposalAlreadyExecuted(
                proposal_id.clone(),
            ));
        }
        rec.vetoed = true;
        Ok(rec.clone())
    }

    pub fn execute(
        &mut self,
        proposal_id: &GovernanceProposalId,
    ) -> Result<SodProposalRecord, GovernancePlaneRejection> {
        let rec = self
            .proposals
            .get_mut(proposal_id)
            .ok_or_else(|| GovernancePlaneRejection::ProposalNotFound(proposal_id.clone()))?;
        if rec.executed {
            return Err(GovernancePlaneRejection::ProposalAlreadyExecuted(
                proposal_id.clone(),
            ));
        }
        if rec.vetoed {
            return Err(GovernancePlaneRejection::VetoedProposalDenied(
                proposal_id.clone(),
            ));
        }
        let Some(approver) = &rec.approver else {
            return Err(GovernancePlaneRejection::ProposalNotApproved(
                proposal_id.clone(),
            ));
        };
        if approver == &rec.proposer {
            return Err(GovernancePlaneRejection::SodSelfApprovalDenied {
                proposal_id: proposal_id.clone(),
                actor_id: approver.clone(),
            });
        }
        rec.executed = true;
        Ok(rec.clone())
    }
}

// ===========================================================================
// 5. Unified Authoritative Backend Security Governance Coordinator
// ===========================================================================

#[derive(Debug)]
struct CoordinatorState {
    delegation: DelegatedAgentAuthzLedger,
    tenants: MultiTenantIsolationFence,
    chatroom: ChatroomIngressGovernor,
    sod: DualControlSodLedger,
}

/// Thread-safe backend security coordinator uniting OBO delegation, multi-tenant isolation,
/// multi-party chatroom ingress governance, dual-control SoD, and the live `ExecutionCircuitBreaker`.
#[derive(Clone, Debug)]
pub struct BackendSecurityGovernanceCoordinator {
    breaker: Arc<ExecutionCircuitBreaker>,
    state: Arc<Mutex<CoordinatorState>>,
}

impl BackendSecurityGovernanceCoordinator {
    pub fn new(
        breaker: Arc<ExecutionCircuitBreaker>,
        delegation: DelegatedAgentAuthzLedger,
        tenants: MultiTenantIsolationFence,
        chatroom: ChatroomIngressGovernor,
    ) -> Self {
        Self {
            breaker,
            state: Arc::new(Mutex::new(CoordinatorState {
                delegation,
                tenants,
                chatroom,
                sod: DualControlSodLedger::new(),
            })),
        }
    }

    pub fn execution_breaker(&self) -> Arc<ExecutionCircuitBreaker> {
        Arc::clone(&self.breaker)
    }

    /// Ingests an external chatroom event; if a forged/unverified external ingress attempts
    /// to claim a gate receipt, automatically trips the backend `ExecutionCircuitBreaker` fail-closed.
    pub fn ingest_chatroom_event(
        &self,
        request: &ExternalIngressRequest,
    ) -> Result<ChatroomIngressReceipt, GovernancePlaneRejection> {
        let mut guard = self
            .state
            .lock()
            .map_err(|_| GovernancePlaneRejection::LockPoisoned)?;
        match guard.chatroom.ingest_external(request) {
            Ok(receipt) => Ok(receipt),
            Err(err @ GovernancePlaneRejection::ForgedExternalIngressDenied) => {
                let reason =
                    BreakerTripReason::new("auto-trip:forged-external-chatroom-ingress")?;
                let _ = self.breaker.trip(reason);
                Err(err)
            }
            Err(err) => Err(err),
        }
    }

    /// Re-arms a tripped backend `ExecutionCircuitBreaker` ONLY after passing BOTH
    /// verified human RFC 9470 step-up authentication AND verified-human breaker check.
    pub fn rearm_breaker_with_step_up(
        &self,
        actor: &GovernancePrincipal,
    ) -> Result<ExecutionBreakerRearmReceipt, GovernancePlaneRejection> {
        let mut guard = self
            .state
            .lock()
            .map_err(|_| GovernancePlaneRejection::LockPoisoned)?;
        // Enforce RFC 9470 step-up + non-agent actor provenance first.
        let _priv_receipt = guard.delegation.execute_privileged_human(actor)?;
        let breaker_actor = BreakerActor::VerifiedHuman(BreakerActorId::new(
            actor.actor_id().as_str(),
        )?);
        Ok(self.breaker.rearm(&breaker_actor)?)
    }

    /// Executes dual-control SoD verification + RFC 9470 step-up + multi-tenant fence +
    /// OBO delegation + chatroom taint gate + `ExecutionCircuitBreaker` before allowing
    /// a high-impact backend side effect.
    pub fn authorize_governed_side_effect(
        &self,
        tenant_ticket: &TenantExecutionTicket,
        target_tenant: &GovernanceTenantId,
        delegation_ticket: Option<&DelegatedAgentTicket>,
        tool_label: impl Into<String>,
        ifc: &ToolIfcContext,
    ) -> Result<ExecutionBreakerCommitReceipt, GovernancePlaneRejection> {
        let mut guard = self
            .state
            .lock()
            .map_err(|_| GovernancePlaneRejection::LockPoisoned)?;
        guard.tenants.enforce_ticket(tenant_ticket, target_tenant)?;
        if let Some(ticket) = delegation_ticket {
            guard.delegation.execute_delegated(ticket)?;
        }
        guard.chatroom.authorize_tool_execution()?;
        Ok(self.breaker.authorize_side_effect(tool_label, ifc)?)
    }

    pub fn with_ledgers<T>(
        &self,
        f: impl FnOnce(
            &mut DelegatedAgentAuthzLedger,
            &mut MultiTenantIsolationFence,
            &mut ChatroomIngressGovernor,
            &mut DualControlSodLedger,
        ) -> Result<T, GovernancePlaneRejection>,
    ) -> Result<T, GovernancePlaneRejection> {
        let mut guard = self
            .state
            .lock()
            .map_err(|_| GovernancePlaneRejection::LockPoisoned)?;
        let state = &mut *guard;
        f(
            &mut state.delegation,
            &mut state.tenants,
            &mut state.chatroom,
            &mut state.sod,
        )
    }
}
