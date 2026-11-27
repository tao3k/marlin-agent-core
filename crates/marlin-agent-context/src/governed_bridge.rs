// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Production bridge uniting `OrgGovernanceLedger` (`governance.rs`) with
//! `marlin-agent-graph`'s `GovernedContextCapsule`, `TaskActorCursor`,
//! `DetourState`, and Two-Zone KV-Prefix `ModelGatewayMessage` prompt assembly.

use marlin_agent_graph::{
    AdmittedGovernedContext, DetourState, DisclosureDepth, GovernedContextCapsule,
    GovernedContextRejection, KvPrefixDigest, TaskActorCursor, TaskActorScopeMode,
    admit_governed_context_capsule,
};
use marlin_agent_protocol::{ModelGatewayMessage, system_gateway_message, user_gateway_message};
use sha2::{Digest, Sha256};

use crate::governance::{
    OrgGovernanceError, OrgGovernanceLedger, SubtreeCommitProposal, SubtreeCommitReceipt,
    SubtreeSemanticVerifier,
};
use crate::shared_scope::{
    ContextAccessHandle, ContextEntryId, ContextPrincipal, GovernedPromptBundleReceipt,
    SharedContextGovernanceLedger, SharedContextScope,
};

/// Fail-closed rejection when mediating a governed Org commit or Two-Zone prompt
/// against a Session-free `TaskActor`'s governance envelope.
#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum GovernedContextBridgeError {
    /// Detached actor is forbidden from committing mutations to the shared project Org trunk.
    #[error("detached actor {task_id} cannot mutate shared project Org trunk")]
    DetachedActorCannotMutateProjectOrg { task_id: String },
    /// Actor is already in an immutable terminal state.
    #[error("terminal actor {task_id} cannot commit Org mutations")]
    TerminalActorCannotMutateOrg { task_id: String },
    /// An open `BlockerActive` Scoped Detour halts trunk mutations until re-anchored.
    #[error("blocker-active Scoped Detour halts Org trunk mutation for {task_id}")]
    BlockerDetourBlocksOrgCommit { task_id: String },
    /// Actor cursor and governed context capsule refer to different `TaskActorId`s.
    #[error("task actor mismatch: cursor={cursor_task_id}, capsule={capsule_task_id}")]
    TaskActorMismatch {
        cursor_task_id: String,
        capsule_task_id: String,
    },
    /// Underlying `OrgGovernanceLedger` error.
    #[error("org governance error: {0}")]
    OrgGovernance(#[from] OrgGovernanceError),
    /// Resulting capsule failed fail-closed `ai-agentic-context` admission.
    #[error("governed context capsule rejected: {0}")]
    CapsuleRejected(#[from] GovernedContextRejection),
    /// Prompt envelope's `kv_prefix_digest` does not match the admitted capsule's frozen digest.
    #[error("two-zone KV-Prefix digest mismatch: envelope={envelope}, admitted={admitted}")]
    KvPrefixDigestMismatch { envelope: String, admitted: String },
    /// Prompt envelope attempted to lower disclosure below the admitted capsule's floor.
    #[error("prompt disclosure {requested:?} is below admitted floor {admitted:?}")]
    DisclosureFloorViolation {
        requested: DisclosureDepth,
        admitted: DisclosureDepth,
    },
    /// Prompt dynamic tail must be non-empty.
    #[error("two-zone prompt dynamic tail must be non-empty")]
    EmptyDynamicTail,
}

/// Receipt emitted after mediating a `SubtreeCommitProposal` against both
/// `TaskActorCursor`/`DetourState` and `OrgGovernanceLedger`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GovernedOrgCommitReceipt {
    /// Underlying structural Org subtree commit receipt.
    pub org_receipt: SubtreeCommitReceipt,
    /// Updated `AdmittedGovernedContext` with monotonically incremented `context_cut`
    /// and refreshed Two-Zone `kv_prefix_digest` whenever `org_receipt.accepted == true`.
    pub next_context: Option<AdmittedGovernedContext>,
}

/// Computes a deterministic SHA-256 `KvPrefixDigest` over a frozen prompt prefix string.
pub fn compute_kv_prefix_digest(frozen_prefix: &str) -> KvPrefixDigest {
    let mut hasher = Sha256::new();
    hasher.update(frozen_prefix.as_bytes());
    let hex = format!("sha256:{:x}", hasher.finalize());
    KvPrefixDigest::new(hex).expect("hex sha256 digest is always non-empty")
}

/// Mediates an `OrgGovernanceLedger::commit_subtree` call through Session-free
/// `TaskActor` & Design 165 Scoped Detour invariants, advancing `context_cut`
/// monotonically and refreshing `kv_prefix_digest` upon accepted commit.
pub fn commit_governed_org_subtree<V: SubtreeSemanticVerifier>(
    ledger: &mut OrgGovernanceLedger<V>,
    proposal: SubtreeCommitProposal,
    cursor: &TaskActorCursor,
    detour_state: &DetourState,
    current_context: &AdmittedGovernedContext,
) -> Result<GovernedOrgCommitReceipt, GovernedContextBridgeError> {
    if cursor.task_id != current_context.capsule().task_id {
        return Err(GovernedContextBridgeError::TaskActorMismatch {
            cursor_task_id: cursor.task_id.as_str().to_owned(),
            capsule_task_id: current_context.capsule().task_id.as_str().to_owned(),
        });
    }
    if cursor.scope_mode == TaskActorScopeMode::Detached {
        return Err(
            GovernedContextBridgeError::DetachedActorCannotMutateProjectOrg {
                task_id: cursor.task_id.as_str().to_owned(),
            },
        );
    }
    if cursor.status.is_terminal() {
        return Err(GovernedContextBridgeError::TerminalActorCannotMutateOrg {
            task_id: cursor.task_id.as_str().to_owned(),
        });
    }
    if detour_state.is_blocker_active() {
        return Err(GovernedContextBridgeError::BlockerDetourBlocksOrgCommit {
            task_id: cursor.task_id.as_str().to_owned(),
        });
    }

    let source_id = proposal.source_id.clone();
    let org_receipt = ledger.commit_subtree(proposal)?;
    if !org_receipt.accepted {
        return Ok(GovernedOrgCommitReceipt {
            org_receipt,
            next_context: None,
        });
    }

    let committed_source = ledger
        .snapshot(&source_id)
        .map_or("", |snap| snap.source.as_str());
    let next_digest = compute_kv_prefix_digest(&format!(
        "source={}:gen={}:body={}",
        source_id.as_str(),
        org_receipt.committed_generation.map_or(0, |g| g.get()),
        committed_source
    ));
    let mut next_capsule = GovernedContextCapsule {
        kv_prefix_digest: next_digest,
        ..current_context.capsule().clone()
    };
    next_capsule.context_cut = next_capsule.context_cut.saturating_add(1);
    let next_context = admit_governed_context_capsule(next_capsule)?;
    Ok(GovernedOrgCommitReceipt {
        org_receipt,
        next_context: Some(next_context),
    })
}

/// Two-Zone KV-Prefix prompt envelope assembled from `ai-agentic-context`:
/// - Zone 1 (`frozen_prefix`): immutable structural Org/governance context whose
///   exact SHA-256 digest matches `AdmittedGovernedContext::capsule().kv_prefix_digest`.
/// - Zone 2 (`dynamic_tail`): per-turn volatile user/task step prompt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TwoZonePromptEnvelope {
    /// Immutable Zone-1 structural prefix (stable across turns for KV-Cache reuse).
    pub frozen_prefix: String,
    /// Exact digest of `frozen_prefix` verified against `AdmittedGovernedContext`.
    pub kv_prefix_digest: KvPrefixDigest,
    /// Effective progressive disclosure depth (`L0..L3`) of `frozen_prefix`.
    pub disclosure_depth: DisclosureDepth,
    /// Dynamic Zone-2 turn prompt appended after the frozen prefix.
    pub dynamic_tail: String,
}

impl TwoZonePromptEnvelope {
    /// Constructs a `TwoZonePromptEnvelope` computing its exact SHA-256 `kv_prefix_digest`.
    pub fn from_frozen_and_tail(
        frozen_prefix: impl Into<String>,
        disclosure_depth: DisclosureDepth,
        dynamic_tail: impl Into<String>,
    ) -> Self {
        let frozen_prefix = frozen_prefix.into();
        let kv_prefix_digest = compute_kv_prefix_digest(&frozen_prefix);
        Self {
            frozen_prefix,
            kv_prefix_digest,
            disclosure_depth,
            dynamic_tail: dynamic_tail.into(),
        }
    }
}

/// Assembles production `ModelGatewayMessage`s for `ModelStreamingGateway` under
/// strict `AdmittedGovernedContext` invariants:
/// 1. Zone-1 `frozen_prefix` digest MUST match `admitted.capsule().kv_prefix_digest`.
/// 2. Envelope `disclosure_depth` MUST NOT fall below `admitted.capsule().disclosure_depth`
///    (preserving the L3 active-defeater anti-suppression floor).
/// 3. Emits Zone-1 as the immutable leading `System` message followed by prior
///    history and Zone-2 `User` tail so downstream LLM providers retain KV-Prefix cache hits.
pub fn assemble_two_zone_gateway_messages(
    envelope: &TwoZonePromptEnvelope,
    admitted: &AdmittedGovernedContext,
    history: &[ModelGatewayMessage],
) -> Result<Vec<ModelGatewayMessage>, GovernedContextBridgeError> {
    let cap = admitted.capsule();
    if envelope.kv_prefix_digest != cap.kv_prefix_digest {
        return Err(GovernedContextBridgeError::KvPrefixDigestMismatch {
            envelope: envelope.kv_prefix_digest.as_str().to_owned(),
            admitted: cap.kv_prefix_digest.as_str().to_owned(),
        });
    }
    if envelope.disclosure_depth < cap.disclosure_depth {
        return Err(GovernedContextBridgeError::DisclosureFloorViolation {
            requested: envelope.disclosure_depth,
            admitted: cap.disclosure_depth,
        });
    }
    if envelope.dynamic_tail.trim().is_empty() {
        return Err(GovernedContextBridgeError::EmptyDynamicTail);
    }

    let mut messages = Vec::with_capacity(history.len() + 2);
    let frozen_header = format!(
        "[governed-context cut={} depth={:?} kv_prefix={} quarantined={}]\n{}",
        cap.context_cut,
        cap.disclosure_depth,
        cap.kv_prefix_digest.as_str(),
        cap.quarantined_memory_count,
        envelope.frozen_prefix
    );
    messages.push(system_gateway_message(frozen_header));
    messages.extend(history.iter().cloned());
    messages.push(user_gateway_message(envelope.dynamic_tail.clone()));
    Ok(messages)
}

/// Receipt produced when assembling Two-Zone KV-Prefix gateway messages under
/// 5-Scope Shared Context Governance anti-exfiltration filtering.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScopedTwoZoneGatewayMessagesReceipt {
    pub messages: Vec<ModelGatewayMessage>,
    pub bundle_receipt: GovernedPromptBundleReceipt,
}

/// Unifies 5-Scope Shared Context Governance (`SharedContextGovernanceLedger`) with
/// Two-Zone KV-Prefix prompt assembly (`assemble_two_zone_gateway_messages`):
/// - Automatically redacts any candidate entry that belongs to another tenant, has a revoked
///   channel handle epoch, or carries a narrower scope (e.g. `Private` / `DirectMessage`)
///   than `target_scope` so narrow secrets can never exfiltrate into broader shared prompts.
/// - Appends ONLY admitted shared context content into Zone-2 dynamic tail (preserving Zone-1
///   KV-Prefix immutability and cache hits) and surfaces multi-party taint provenance.
pub fn assemble_scoped_two_zone_gateway_messages(
    envelope: &TwoZonePromptEnvelope,
    admitted: &AdmittedGovernedContext,
    history: &[ModelGatewayMessage],
    shared_ledger: &SharedContextGovernanceLedger,
    actor: &ContextPrincipal,
    target_scope: SharedContextScope,
    candidate_entry_ids: &[ContextEntryId],
    channel_handle: Option<&ContextAccessHandle>,
) -> Result<ScopedTwoZoneGatewayMessagesReceipt, GovernedContextBridgeError> {
    let bundle_receipt = shared_ledger.assemble_prompt_bundle(
        actor,
        target_scope,
        candidate_entry_ids,
        channel_handle,
    );

    let mut scoped_tail = envelope.dynamic_tail.clone();
    if !bundle_receipt.included_entry_ids.is_empty() {
        scoped_tail.push_str(&format!(
            "\n\n[shared-context scope={:?} tainted={} included={} redacted={}]",
            bundle_receipt.target_scope,
            bundle_receipt.tainted,
            bundle_receipt.included_entry_ids.len(),
            bundle_receipt.redacted_entry_ids.len()
        ));
        for id in &bundle_receipt.included_entry_ids {
            if let Ok(read_receipt) = shared_ledger.read_entry(actor, id, channel_handle) {
                scoped_tail.push_str(&format!(
                    "\n- [{}] {}",
                    read_receipt.entry_id.as_str(),
                    read_receipt.content
                ));
            }
        }
    }

    let scoped_envelope = TwoZonePromptEnvelope {
        frozen_prefix: envelope.frozen_prefix.clone(),
        kv_prefix_digest: envelope.kv_prefix_digest.clone(),
        disclosure_depth: envelope.disclosure_depth,
        dynamic_tail: scoped_tail,
    };
    let messages = assemble_two_zone_gateway_messages(&scoped_envelope, admitted, history)?;
    Ok(ScopedTwoZoneGatewayMessagesReceipt {
        messages,
        bundle_receipt,
    })
}
