// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Authoritative 5-Scope Shared Context Governance, multi-tenant isolation,
//! monotonic ACL-epoch handle revocation, and multi-party CaMeL taint gating.
//!
//! This module implements the Rust backend enforcement plane refined by:
//! - `threat.zero-trust.shared-context-governance` (`SharedContextGovernance.qnt`)
//! - `threat.zero-trust.multi-tenant-isolation` (`MultiTenantIsolation.qnt`)
//! - `dev/quint/threat/MarlinBackendSharedContextOverlay.qnt`

use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    sync::{Arc, RwLock},
};

use thiserror::Error;

macro_rules! ctx_gov_id {
    ($name:ident, $label:literal) => {
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, ContextScopeRejection> {
                let value = value.into();
                if value.is_empty() || value.chars().take(257).count() > 256 {
                    return Err(ContextScopeRejection::InvalidIdentifier($label));
                }
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

ctx_gov_id!(ContextTenantId, "shared context tenant identity");
ctx_gov_id!(ContextPrincipalId, "shared context principal identity");
ctx_gov_id!(ContextChannelId, "shared context channel identity");
ctx_gov_id!(ContextEntryId, "shared context entry identity");
ctx_gov_id!(ContextHandleId, "shared context access handle identity");

/// Monotonic ACL epoch for a shared context channel (`>= 1`).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ContextAclEpoch(u64);

impl ContextAclEpoch {
    pub const INITIAL: Self = Self(1);

    pub fn new(value: u64) -> Result<Self, ContextScopeRejection> {
        if value == 0 {
            return Err(ContextScopeRejection::InvalidAclEpoch(value));
        }
        Ok(Self(value))
    }

    pub fn get(self) -> u64 {
        self.0
    }

    pub fn next(self) -> Result<Self, ContextScopeRejection> {
        self.0
            .checked_add(1)
            .map(Self)
            .ok_or(ContextScopeRejection::AclEpochExhausted)
    }
}

/// Five-tier shared context visibility lattice (ordered narrowest -> broadest).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum SharedContextScope {
    /// Strictly owner-only; peers and cross-tenant actors are always denied.
    Private = 0,
    /// Owner plus explicit same-tenant direct-message allowlist.
    DirectMessage = 1,
    /// Same-tenant active channel members holding a current-epoch handle.
    Channel = 2,
    /// Same-tenant workspace members.
    Workspace = 3,
    /// Explicitly declassified external/federated context.
    External = 4,
}

impl SharedContextScope {
    pub fn rank(self) -> u8 {
        self as u8
    }
}

/// Information-flow sensitivity classification of a context entry.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ContextSensitivity {
    Public,
    Internal,
    Confidential,
}

/// Authenticated principal interacting with shared context.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ContextPrincipal {
    Human {
        id: ContextPrincipalId,
        tenant_id: ContextTenantId,
    },
    DelegatedAgent {
        agent_id: ContextPrincipalId,
        acting_for: ContextPrincipalId,
        tenant_id: ContextTenantId,
    },
}

impl ContextPrincipal {
    pub fn tenant_id(&self) -> &ContextTenantId {
        match self {
            Self::Human { tenant_id, .. } | Self::DelegatedAgent { tenant_id, .. } => tenant_id,
        }
    }

    pub fn principal_id(&self) -> &ContextPrincipalId {
        match self {
            Self::Human { id, .. } => id,
            Self::DelegatedAgent { agent_id, .. } => agent_id,
        }
    }

    pub fn effective_owner_id(&self) -> &ContextPrincipalId {
        match self {
            Self::Human { id, .. } => id,
            Self::DelegatedAgent { acting_for, .. } => acting_for,
        }
    }

    pub fn is_agent(&self) -> bool {
        matches!(self, Self::DelegatedAgent { .. })
    }
}

/// One governed shared-context entry tracked by the Rust authority store.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SharedContextEntry {
    pub entry_id: ContextEntryId,
    pub tenant_id: ContextTenantId,
    pub owner_id: ContextPrincipalId,
    pub channel_id: Option<ContextChannelId>,
    pub dm_participants: BTreeSet<ContextPrincipalId>,
    pub scope: SharedContextScope,
    pub sensitivity: ContextSensitivity,
    pub declassified: bool,
    pub tainted: bool,
    pub content: String,
}

/// Epoch-bound access handle issued to an active channel member.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextAccessHandle {
    pub handle_id: ContextHandleId,
    pub tenant_id: ContextTenantId,
    pub principal_id: ContextPrincipalId,
    pub channel_id: ContextChannelId,
    pub acl_epoch: ContextAclEpoch,
}

/// Typed denial reasons (zero unstructured string errors).
#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum ContextScopeRejection {
    #[error("invalid identifier for {0}")]
    InvalidIdentifier(&'static str),
    #[error("invalid ACL epoch {0}: must be >= 1")]
    InvalidAclEpoch(u64),
    #[error("ACL epoch overflow")]
    AclEpochExhausted,
    #[error("context entry not found: {0}")]
    EntryNotFound(ContextEntryId),
    #[error("duplicate context entry: {0}")]
    DuplicateEntry(ContextEntryId),
    #[error("context channel not found: {0}")]
    ChannelNotFound(ContextChannelId),
    #[error("cross-tenant access denied: actor tenant {actor_tenant} != resource tenant {resource_tenant}")]
    CrossTenantDenied {
        actor_tenant: ContextTenantId,
        resource_tenant: ContextTenantId,
    },
    #[error("private context scope isolation denied read of {entry_id} by {actor_id}")]
    PrivateScopeDenied {
        entry_id: ContextEntryId,
        actor_id: ContextPrincipalId,
    },
    #[error("direct-message scope denied read of {entry_id} by non-participant {actor_id}")]
    DirectMessageParticipantDenied {
        entry_id: ContextEntryId,
        actor_id: ContextPrincipalId,
    },
    #[error("channel access handle required for {0}")]
    ChannelHandleRequired(ContextEntryId),
    #[error("principal {principal_id} is not an active member of channel {channel_id}")]
    NotChannelMember {
        channel_id: ContextChannelId,
        principal_id: ContextPrincipalId,
    },
    #[error("stale access handle epoch {handle_epoch:?} != current channel epoch {current_epoch:?}")]
    StaleHandleEpoch {
        handle_epoch: ContextAclEpoch,
        current_epoch: ContextAclEpoch,
    },
    #[error("access handle tenant/principal/channel binding mismatch")]
    HandleBindingMismatch,
    #[error("delegated agent {agent_id} forbidden from privileged governance action {action}")]
    AgentPrivilegeDenied {
        action: &'static str,
        agent_id: ContextPrincipalId,
    },
    #[error("non-owner {actor_id} cannot promote scope of {entry_id}")]
    NonOwnerPromotionDenied {
        entry_id: ContextEntryId,
        actor_id: ContextPrincipalId,
    },
    #[error("confidential entry {0} requires explicit owner declassification before promotion or egress")]
    DeclassificationRequired(ContextEntryId),
    #[error("invalid scope transition from {from:?} to {to:?}")]
    InvalidScopeTransition {
        from: SharedContextScope,
        to: SharedContextScope,
    },
    #[error("tainted shared context in channel {0} requires verified human approval before tool execution")]
    TaintedContextRequiresHumanGate(ContextChannelId),
    #[error("scope exfiltration blocked: entry {entry_id} ({entry_scope:?}) cannot flow into broader target scope {target_scope:?}")]
    PromptExfiltrationBlocked {
        entry_id: ContextEntryId,
        entry_scope: SharedContextScope,
        target_scope: SharedContextScope,
    },
    #[error("shared context governance lock poisoned")]
    LockPoisoned,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextScopePromotionReceipt {
    pub entry_id: ContextEntryId,
    pub previous_scope: SharedContextScope,
    pub new_scope: SharedContextScope,
    pub declassified: bool,
    pub approved_by: ContextPrincipalId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextReadReceipt {
    pub entry_id: ContextEntryId,
    pub reader_id: ContextPrincipalId,
    pub scope: SharedContextScope,
    pub acl_epoch: Option<ContextAclEpoch>,
    pub tainted: bool,
    pub content: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextToolGateReceipt {
    pub channel_id: ContextChannelId,
    pub actor_id: ContextPrincipalId,
    pub channel_tainted: bool,
    pub human_gate_approved: bool,
    pub tool_calls_in_channel: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GovernedPromptBundleReceipt {
    pub target_scope: SharedContextScope,
    pub included_entry_ids: Vec<ContextEntryId>,
    pub redacted_entry_ids: Vec<ContextEntryId>,
    pub tainted: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ChannelState {
    tenant_id: ContextTenantId,
    acl_epoch: ContextAclEpoch,
    members: BTreeSet<ContextPrincipalId>,
    tainted: bool,
    tool_calls: u64,
}

/// Authoritative deterministic ledger enforcing 5-scope shared context governance.
#[derive(Clone, Debug, Default)]
pub struct SharedContextGovernanceLedger {
    channels: HashMap<ContextChannelId, ChannelState>,
    entries: BTreeMap<ContextEntryId, SharedContextEntry>,
}

impl SharedContextGovernanceLedger {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn create_channel<I>(
        &mut self,
        tenant_id: ContextTenantId,
        channel_id: ContextChannelId,
        members: I,
    ) -> ContextAclEpoch
    where
        I: IntoIterator<Item = ContextPrincipalId>,
    {
        let member_set: BTreeSet<ContextPrincipalId> = members.into_iter().collect();
        self.channels.insert(
            channel_id,
            ChannelState {
                tenant_id,
                acl_epoch: ContextAclEpoch::INITIAL,
                members: member_set,
                tainted: false,
                tool_calls: 0,
            },
        );
        ContextAclEpoch::INITIAL
    }

    pub fn register_entry(
        &mut self,
        entry: SharedContextEntry,
    ) -> Result<(), ContextScopeRejection> {
        if self.entries.contains_key(&entry.entry_id) {
            return Err(ContextScopeRejection::DuplicateEntry(entry.entry_id));
        }
        if let Some(channel_id) = &entry.channel_id {
            let channel = self
                .channels
                .get(channel_id)
                .ok_or_else(|| ContextScopeRejection::ChannelNotFound(channel_id.clone()))?;
            if channel.tenant_id != entry.tenant_id {
                return Err(ContextScopeRejection::CrossTenantDenied {
                    actor_tenant: entry.tenant_id,
                    resource_tenant: channel.tenant_id.clone(),
                });
            }
        }
        self.entries.insert(entry.entry_id.clone(), entry);
        Ok(())
    }

    pub fn channel_acl_epoch(
        &self,
        channel_id: &ContextChannelId,
    ) -> Result<ContextAclEpoch, ContextScopeRejection> {
        self.channels
            .get(channel_id)
            .map(|channel| channel.acl_epoch)
            .ok_or_else(|| ContextScopeRejection::ChannelNotFound(channel_id.clone()))
    }

    pub fn is_channel_member(
        &self,
        channel_id: &ContextChannelId,
        principal_id: &ContextPrincipalId,
    ) -> Result<bool, ContextScopeRejection> {
        self.channels
            .get(channel_id)
            .map(|channel| channel.members.contains(principal_id))
            .ok_or_else(|| ContextScopeRejection::ChannelNotFound(channel_id.clone()))
    }

    pub fn is_channel_tainted(
        &self,
        channel_id: &ContextChannelId,
    ) -> Result<bool, ContextScopeRejection> {
        self.channels
            .get(channel_id)
            .map(|channel| channel.tainted)
            .ok_or_else(|| ContextScopeRejection::ChannelNotFound(channel_id.clone()))
    }

    pub fn entry(&self, entry_id: &ContextEntryId) -> Option<&SharedContextEntry> {
        self.entries.get(entry_id)
    }

    /// Promotes an entry to a broader visibility scope.
    ///
    /// Security invariants:
    /// - Delegated agents are unconditionally denied (`AgentPrivilegeDenied`).
    /// - Cross-tenant actors are denied (`CrossTenantDenied`).
    /// - Only the entry's human owner may promote scope (`NonOwnerPromotionDenied`).
    /// - Confidential entries require explicit human declassification (`DeclassificationRequired`).
    pub fn promote_scope(
        &mut self,
        actor: &ContextPrincipal,
        entry_id: &ContextEntryId,
        target_scope: SharedContextScope,
        explicit_declassify: bool,
    ) -> Result<ContextScopePromotionReceipt, ContextScopeRejection> {
        if let ContextPrincipal::DelegatedAgent { agent_id, .. } = actor {
            return Err(ContextScopeRejection::AgentPrivilegeDenied {
                action: "promote_context_scope",
                agent_id: agent_id.clone(),
            });
        }

        let entry = self
            .entries
            .get_mut(entry_id)
            .ok_or_else(|| ContextScopeRejection::EntryNotFound(entry_id.clone()))?;

        if actor.tenant_id() != &entry.tenant_id {
            return Err(ContextScopeRejection::CrossTenantDenied {
                actor_tenant: actor.tenant_id().clone(),
                resource_tenant: entry.tenant_id.clone(),
            });
        }

        if actor.principal_id() != &entry.owner_id {
            return Err(ContextScopeRejection::NonOwnerPromotionDenied {
                entry_id: entry_id.clone(),
                actor_id: actor.principal_id().clone(),
            });
        }

        if target_scope <= entry.scope {
            return Err(ContextScopeRejection::InvalidScopeTransition {
                from: entry.scope,
                to: target_scope,
            });
        }

        if entry.sensitivity == ContextSensitivity::Confidential
            && !entry.declassified
            && !explicit_declassify
        {
            return Err(ContextScopeRejection::DeclassificationRequired(
                entry_id.clone(),
            ));
        }

        let previous_scope = entry.scope;
        entry.scope = target_scope;
        entry.declassified = entry.declassified || explicit_declassify;

        Ok(ContextScopePromotionReceipt {
            entry_id: entry_id.clone(),
            previous_scope,
            new_scope: target_scope,
            declassified: entry.declassified,
            approved_by: actor.principal_id().clone(),
        })
    }

    /// Mints an epoch-bound channel read handle for an active same-tenant member.
    pub fn mint_channel_handle(
        &self,
        actor: &ContextPrincipal,
        channel_id: &ContextChannelId,
        handle_id: ContextHandleId,
    ) -> Result<ContextAccessHandle, ContextScopeRejection> {
        let channel = self
            .channels
            .get(channel_id)
            .ok_or_else(|| ContextScopeRejection::ChannelNotFound(channel_id.clone()))?;

        if actor.tenant_id() != &channel.tenant_id {
            return Err(ContextScopeRejection::CrossTenantDenied {
                actor_tenant: actor.tenant_id().clone(),
                resource_tenant: channel.tenant_id.clone(),
            });
        }

        let member_id = actor.effective_owner_id();
        if !channel.members.contains(member_id) {
            return Err(ContextScopeRejection::NotChannelMember {
                channel_id: channel_id.clone(),
                principal_id: member_id.clone(),
            });
        }

        Ok(ContextAccessHandle {
            handle_id,
            tenant_id: channel.tenant_id.clone(),
            principal_id: member_id.clone(),
            channel_id: channel_id.clone(),
            acl_epoch: channel.acl_epoch,
        })
    }

    /// Revokes a member from a channel and monotonically bumps `acl_epoch`,
    /// immediately invalidating all pre-revocation handles.
    pub fn revoke_member(
        &mut self,
        channel_id: &ContextChannelId,
        principal_id: &ContextPrincipalId,
    ) -> Result<ContextAclEpoch, ContextScopeRejection> {
        let channel = self
            .channels
            .get_mut(channel_id)
            .ok_or_else(|| ContextScopeRejection::ChannelNotFound(channel_id.clone()))?;

        if !channel.members.remove(principal_id) {
            return Err(ContextScopeRejection::NotChannelMember {
                channel_id: channel_id.clone(),
                principal_id: principal_id.clone(),
            });
        }
        channel.acl_epoch = channel.acl_epoch.next()?;
        Ok(channel.acl_epoch)
    }

    /// Re-admits a member at the current (already advanced) `acl_epoch`.
    /// Pre-revocation handles remain permanently stale because `acl_epoch` never rolls back.
    pub fn rejoin_member(
        &mut self,
        channel_id: &ContextChannelId,
        principal_id: ContextPrincipalId,
    ) -> Result<ContextAclEpoch, ContextScopeRejection> {
        let channel = self
            .channels
            .get_mut(channel_id)
            .ok_or_else(|| ContextScopeRejection::ChannelNotFound(channel_id.clone()))?;
        channel.members.insert(principal_id);
        Ok(channel.acl_epoch)
    }

    /// Marks a shared channel as tainted after untrusted multi-party content ingestion.
    pub fn poison_channel_context(
        &mut self,
        channel_id: &ContextChannelId,
    ) -> Result<(), ContextScopeRejection> {
        let channel = self
            .channels
            .get_mut(channel_id)
            .ok_or_else(|| ContextScopeRejection::ChannelNotFound(channel_id.clone()))?;
        channel.tainted = true;
        Ok(())
    }

    /// Reads an entry subject to tenant isolation, 5-scope visibility rules,
    /// DM participant allowlists, and monotonic channel ACL-epoch validation.
    pub fn read_entry(
        &self,
        actor: &ContextPrincipal,
        entry_id: &ContextEntryId,
        handle: Option<&ContextAccessHandle>,
    ) -> Result<ContextReadReceipt, ContextScopeRejection> {
        let entry = self
            .entries
            .get(entry_id)
            .ok_or_else(|| ContextScopeRejection::EntryNotFound(entry_id.clone()))?;

        if actor.tenant_id() != &entry.tenant_id {
            return Err(ContextScopeRejection::CrossTenantDenied {
                actor_tenant: actor.tenant_id().clone(),
                resource_tenant: entry.tenant_id.clone(),
            });
        }

        let effective_id = actor.effective_owner_id();
        let mut verified_epoch = None;
        let mut channel_tainted = false;

        match entry.scope {
            SharedContextScope::Private => {
                if effective_id != &entry.owner_id {
                    return Err(ContextScopeRejection::PrivateScopeDenied {
                        entry_id: entry_id.clone(),
                        actor_id: effective_id.clone(),
                    });
                }
            }
            SharedContextScope::DirectMessage => {
                if effective_id != &entry.owner_id && !entry.dm_participants.contains(effective_id)
                {
                    return Err(ContextScopeRejection::DirectMessageParticipantDenied {
                        entry_id: entry_id.clone(),
                        actor_id: effective_id.clone(),
                    });
                }
            }
            SharedContextScope::Channel => {
                let channel_id = entry
                    .channel_id
                    .as_ref()
                    .ok_or_else(|| ContextScopeRejection::ChannelHandleRequired(entry_id.clone()))?;
                let channel = self
                    .channels
                    .get(channel_id)
                    .ok_or_else(|| ContextScopeRejection::ChannelNotFound(channel_id.clone()))?;
                channel_tainted = channel.tainted;

                let access_handle = handle
                    .ok_or_else(|| ContextScopeRejection::ChannelHandleRequired(entry_id.clone()))?;
                if &access_handle.tenant_id != actor.tenant_id()
                    || &access_handle.principal_id != effective_id
                    || &access_handle.channel_id != channel_id
                {
                    return Err(ContextScopeRejection::HandleBindingMismatch);
                }
                if !channel.members.contains(effective_id) {
                    return Err(ContextScopeRejection::NotChannelMember {
                        channel_id: channel_id.clone(),
                        principal_id: effective_id.clone(),
                    });
                }
                if access_handle.acl_epoch != channel.acl_epoch {
                    return Err(ContextScopeRejection::StaleHandleEpoch {
                        handle_epoch: access_handle.acl_epoch,
                        current_epoch: channel.acl_epoch,
                    });
                }
                verified_epoch = Some(channel.acl_epoch);
            }
            SharedContextScope::Workspace => {
                // Same-tenant check already passed above.
                if let Some(channel_id) = &entry.channel_id
                    && let Some(channel) = self.channels.get(channel_id)
                {
                    channel_tainted = channel.tainted;
                }
            }
            SharedContextScope::External => {
                if entry.sensitivity == ContextSensitivity::Confidential && !entry.declassified {
                    return Err(ContextScopeRejection::DeclassificationRequired(
                        entry_id.clone(),
                    ));
                }
                if let Some(channel_id) = &entry.channel_id
                    && let Some(channel) = self.channels.get(channel_id)
                {
                    channel_tainted = channel.tainted;
                }
            }
        }

        Ok(ContextReadReceipt {
            entry_id: entry.entry_id.clone(),
            reader_id: actor.principal_id().clone(),
            scope: entry.scope,
            acl_epoch: verified_epoch,
            tainted: entry.tainted || channel_tainted,
            content: entry.content.clone(),
        })
    }

    /// Gates an agent tool invocation sourced from a shared context channel.
    /// If the channel is tainted, a verified human approval gate is mandatory.
    pub fn authorize_agent_tool_call(
        &mut self,
        actor: &ContextPrincipal,
        channel_id: &ContextChannelId,
        human_gate_approved: bool,
    ) -> Result<ContextToolGateReceipt, ContextScopeRejection> {
        let channel = self
            .channels
            .get_mut(channel_id)
            .ok_or_else(|| ContextScopeRejection::ChannelNotFound(channel_id.clone()))?;

        if actor.tenant_id() != &channel.tenant_id {
            return Err(ContextScopeRejection::CrossTenantDenied {
                actor_tenant: actor.tenant_id().clone(),
                resource_tenant: channel.tenant_id.clone(),
            });
        }

        if channel.tainted && !human_gate_approved {
            return Err(ContextScopeRejection::TaintedContextRequiresHumanGate(
                channel_id.clone(),
            ));
        }

        channel.tool_calls = channel.tool_calls.saturating_add(1);
        Ok(ContextToolGateReceipt {
            channel_id: channel_id.clone(),
            actor_id: actor.principal_id().clone(),
            channel_tainted: channel.tainted,
            human_gate_approved,
            tool_calls_in_channel: channel.tool_calls,
        })
    }

    /// Assembles a prompt bundle for `target_scope`, strictly preventing narrower-scope
    /// context (e.g. `Private` or `DirectMessage`) from exfiltrating into broader scopes
    /// (e.g. `Channel`, `Workspace`, or `External`). Unauthorized or narrower entries
    /// are deterministically redacted.
    pub fn assemble_prompt_bundle(
        &self,
        actor: &ContextPrincipal,
        target_scope: SharedContextScope,
        candidate_entry_ids: &[ContextEntryId],
        handle: Option<&ContextAccessHandle>,
    ) -> GovernedPromptBundleReceipt {
        let mut included_entry_ids = Vec::new();
        let mut redacted_entry_ids = Vec::new();
        let mut tainted = false;

        for entry_id in candidate_entry_ids {
            match self.read_entry(actor, entry_id, handle) {
                Ok(receipt) if receipt.scope >= target_scope => {
                    tainted |= receipt.tainted;
                    included_entry_ids.push(receipt.entry_id);
                }
                _ => {
                    redacted_entry_ids.push(entry_id.clone());
                }
            }
        }

        GovernedPromptBundleReceipt {
            target_scope,
            included_entry_ids,
            redacted_entry_ids,
            tainted,
        }
    }

    /// Strict fail-closed check verifying that an entry may legally flow into `target_scope`.
    pub fn verify_scope_flow(
        &self,
        actor: &ContextPrincipal,
        entry_id: &ContextEntryId,
        target_scope: SharedContextScope,
        handle: Option<&ContextAccessHandle>,
    ) -> Result<ContextReadReceipt, ContextScopeRejection> {
        let receipt = self.read_entry(actor, entry_id, handle)?;
        if receipt.scope < target_scope {
            return Err(ContextScopeRejection::PromptExfiltrationBlocked {
                entry_id: entry_id.clone(),
                entry_scope: receipt.scope,
                target_scope,
            });
        }
        Ok(receipt)
    }
}

/// Thread-safe shared handle wrapping `SharedContextGovernanceLedger` for Tokio runtimes.
#[derive(Clone, Debug, Default)]
pub struct SharedContextGovernanceService {
    inner: Arc<RwLock<SharedContextGovernanceLedger>>,
}

impl SharedContextGovernanceService {
    pub fn new(ledger: SharedContextGovernanceLedger) -> Self {
        Self {
            inner: Arc::new(RwLock::new(ledger)),
        }
    }

    pub fn read_with<T>(
        &self,
        f: impl FnOnce(&SharedContextGovernanceLedger) -> Result<T, ContextScopeRejection>,
    ) -> Result<T, ContextScopeRejection> {
        let guard = self
            .inner
            .read()
            .map_err(|_| ContextScopeRejection::LockPoisoned)?;
        f(&guard)
    }

    pub fn write_with<T>(
        &self,
        f: impl FnOnce(&mut SharedContextGovernanceLedger) -> Result<T, ContextScopeRejection>,
    ) -> Result<T, ContextScopeRejection> {
        let mut guard = self
            .inner
            .write()
            .map_err(|_| ContextScopeRejection::LockPoisoned)?;
        f(&mut guard)
    }
}
