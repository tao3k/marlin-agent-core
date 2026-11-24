// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Structural Org `:ID:` region leases and async governance coordination.
//!
//! Multiple Tokio Agent tasks run concurrently and submit typed proposals
//! keyed by `(source_id, target_anchor_id)` rather than raw byte offsets.
//! One coordinator owner arbitrates active `[start_byte, end_byte)` region
//! leases in `O(active_leases_for_source)` interval checks:
//! - Disjoint `:ID:` sections acquire leases simultaneously with zero blocking;
//! - Overlapping or identical `:ID:` sections are rejected with a typed receipt;
//! - Commits advance `OrgSourceGeneration` monotonically, consume single-use
//!   governance nonces, and automatically rebase sibling `:ID:` spans when
//!   earlier commits shift downstream UTF-8 byte offsets while preserving the
//!   target section's subtree digest.

use std::collections::{HashMap, HashSet};

use marlin_agent_protocol::{
    GraphQueryScoreBasisPoints, ProjectRuntimeAgentId, ProjectRuntimeProjectId,
    ProjectRuntimeSessionId, ProjectRuntimeWorktreeId,
};

use crate::ContextTaskId;

/// Maximum concurrent active leases per Org source cut.
pub const MAX_ACTIVE_SOURCE_LEASES: usize = 64;

/// Maximum retained spent nonces per coordinator epoch before rotation.
pub const MAX_RETAINED_NONCES: usize = 4096;

macro_rules! gov_id {
    ($name:ident, $label:literal) => {
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, OrgGovernanceError> {
                let value = value.into();
                if value.is_empty() || value.chars().take(257).count() > 256 {
                    return Err(OrgGovernanceError::InvalidIdentifier($label));
                }
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

gov_id!(GovernedSourceId, "governed Org source identity");
gov_id!(GovernedAnchorPropertyId, "Org property :ID: anchor");
gov_id!(
    GovernedQualifiedAnchorId,
    "qualified OrgElement sha256 identity"
);
gov_id!(GovernedSubtreeDigest, "Org :ID: subtree digest");
gov_id!(GovernedLeaseId, "structural region lease identity");
gov_id!(GovernedEffectNonce, "single-use governance effect nonce");
gov_id!(
    GovernedArchComponentId,
    "GitOps architecture component identity"
);

/// Monotonic generation of a shared GitOps Context Org source (`1..=65534`).
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct GovernedGeneration(u16);

impl GovernedGeneration {
    pub fn new(value: u16) -> Result<Self, OrgGovernanceError> {
        if value == 0 || value == u16::MAX {
            return Err(OrgGovernanceError::InvalidGeneration(value));
        }
        Ok(Self(value))
    }

    pub fn get(self) -> u16 {
        self.0
    }

    pub fn next(self) -> Result<Self, OrgGovernanceError> {
        let next = self
            .0
            .checked_add(1)
            .ok_or(OrgGovernanceError::GenerationExhausted)?;
        Self::new(next)
    }
}

/// Half-open UTF-8 byte interval `[start_byte, end_byte)` of an Org container,
/// paired with `drawer_end_byte` separating the parent-governed `:PROPERTIES:`
/// header from the Agent-editable section body.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GovernedSectionSpan {
    pub container_start: usize,
    pub drawer_end: usize,
    pub body_end: usize,
    pub container_end: usize,
}

impl GovernedSectionSpan {
    pub fn new(
        container_start: usize,
        drawer_end: usize,
        body_end: usize,
        container_end: usize,
    ) -> Result<Self, OrgGovernanceError> {
        if !(container_start <= drawer_end && drawer_end <= body_end && body_end <= container_end) {
            return Err(OrgGovernanceError::InvalidSpan {
                container_start,
                drawer_end,
                body_end,
                container_end,
            });
        }
        Ok(Self {
            container_start,
            drawer_end,
            body_end,
            container_end,
        })
    }

    /// Two governed spans are structurally disjoint when their half-open
    /// container intervals do not overlap.
    pub fn is_disjoint_from(&self, other: &Self) -> bool {
        self.container_end <= other.container_start || other.container_end <= self.container_start
    }

    /// Shift all coordinates by `delta` bytes when an upstream disjoint edit
    /// changes prefix length without touching this section's bytes.
    pub fn shifted_by(&self, delta: isize) -> Result<Self, OrgGovernanceError> {
        let shift = |coord: usize| -> Result<usize, OrgGovernanceError> {
            coord
                .checked_add_signed(delta)
                .ok_or(OrgGovernanceError::InvalidSpan {
                    container_start: self.container_start,
                    drawer_end: self.drawer_end,
                    body_end: self.body_end,
                    container_end: self.container_end,
                })
        };
        Self::new(
            shift(self.container_start)?,
            shift(self.drawer_end)?,
            shift(self.body_end)?,
            shift(self.container_end)?,
        )
    }
}

/// Snapshot of one governed `:ID:` section projected by Lab's Scheme Orgize boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GovernedAnchorDescriptor {
    pub qualified_id: GovernedQualifiedAnchorId,
    pub property_id: GovernedAnchorPropertyId,
    pub project_id: ProjectRuntimeProjectId,
    pub worktree_id: Option<ProjectRuntimeWorktreeId>,
    pub component_id: Option<GovernedArchComponentId>,
    pub owner_agent_id: Option<ProjectRuntimeAgentId>,
    pub reviewer_agent_id: Option<ProjectRuntimeAgentId>,
    pub task_id: Option<ContextTaskId>,
    pub span: GovernedSectionSpan,
    pub subtree_digest: GovernedSubtreeDigest,
    pub affinity_score: GraphQueryScoreBasisPoints,
}

impl GovernedAnchorDescriptor {
    /// True iff `agent` owns this anchor, is bound to `task`, and does not
    /// violate Separation-of-Duties by acting as its own reviewer.
    pub fn is_write_eligible_for(
        &self,
        agent: &ProjectRuntimeAgentId,
        task: &ContextTaskId,
    ) -> bool {
        let owns = self.owner_agent_id.as_ref() == Some(agent);
        let matches_task = self.task_id.as_ref() == Some(task);
        let sod_clean = self.reviewer_agent_id.as_ref() != Some(agent);
        owns && matches_task && sod_clean
    }
}

/// Registered shared Org source snapshot held by the single-owner coordinator.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GovernedOrgSnapshot {
    pub source_id: GovernedSourceId,
    pub generation: GovernedGeneration,
    pub source: String,
    pub anchors: HashMap<GovernedAnchorPropertyId, GovernedAnchorDescriptor>,
}

/// Active structural region lease granted to one Agent/Task on one `:ID:`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActiveRegionLease {
    pub lease_id: GovernedLeaseId,
    pub source_id: GovernedSourceId,
    pub acquired_at_generation: GovernedGeneration,
    pub target_property_id: GovernedAnchorPropertyId,
    pub agent_id: ProjectRuntimeAgentId,
    pub task_id: ContextTaskId,
    pub session_id: ProjectRuntimeSessionId,
    pub expected_subtree_digest: GovernedSubtreeDigest,
    pub span: GovernedSectionSpan,
}

/// Request from an async Agent worker to acquire a structural write lease.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AcquireRegionLeaseRequest {
    pub lease_id: GovernedLeaseId,
    pub source_id: GovernedSourceId,
    pub target_property_id: GovernedAnchorPropertyId,
    pub agent_id: ProjectRuntimeAgentId,
    pub task_id: ContextTaskId,
    pub session_id: ProjectRuntimeSessionId,
}

/// Typed proposal submitted by an async Agent worker to commit its section body.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubtreeCommitProposal {
    pub lease_id: GovernedLeaseId,
    pub nonce: GovernedEffectNonce,
    pub source_id: GovernedSourceId,
    pub base_generation: GovernedGeneration,
    pub target_property_id: GovernedAnchorPropertyId,
    pub agent_id: ProjectRuntimeAgentId,
    pub task_id: ContextTaskId,
    pub session_id: ProjectRuntimeSessionId,
    pub expected_subtree_digest: GovernedSubtreeDigest,
    pub replacement_body: String,
}

/// Typed rejection reasons across lease acquisition and subtree commit mediation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GovernanceRejectionReason {
    UnknownSource,
    UnknownAnchor,
    LeaseCapacityReached,
    DuplicateLeaseId,
    OverlappingRegionLease {
        conflicting_lease: GovernedLeaseId,
        conflicting_anchor: GovernedAnchorPropertyId,
        conflicting_agent: ProjectRuntimeAgentId,
    },
    SeparationOfDutiesViolation,
    WriteIneligibleForAgentOrTask,
    LeaseNotFoundOrMismatch,
    NonceAlreadyConsumed,
    GenerationRollback,
    SubtreeDigestMismatch,
    HeaderOrStructuralEscape,
}

/// Typed receipt returned for a lease request (never unstructured log strings).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RegionLeaseReceipt {
    Granted(ActiveRegionLease),
    Denied {
        source_id: GovernedSourceId,
        target_property_id: GovernedAnchorPropertyId,
        reason: GovernanceRejectionReason,
    },
}

/// Typed receipt returned when committing a governed subtree edit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubtreeCommitReceipt {
    pub accepted: bool,
    pub rejection: Option<GovernanceRejectionReason>,
    pub source_id: GovernedSourceId,
    pub committed_generation: Option<GovernedGeneration>,
    pub target_property_id: GovernedAnchorPropertyId,
    pub agent_id: ProjectRuntimeAgentId,
    pub before_subtree_digest: GovernedSubtreeDigest,
    pub after_subtree_digest: Option<GovernedSubtreeDigest>,
    pub rebased: bool,
}

#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum OrgGovernanceError {
    #[error("invalid {0}")]
    InvalidIdentifier(&'static str),
    #[error("invalid Org source generation {0}")]
    InvalidGeneration(u16),
    #[error("Org source generation space exhausted")]
    GenerationExhausted,
    #[error(
        "invalid governed section byte span [{container_start}, {drawer_end}, {body_end}, {container_end})"
    )]
    InvalidSpan {
        container_start: usize,
        drawer_end: usize,
        body_end: usize,
        container_end: usize,
    },
    #[error("UTF-8 slice boundary mismatch in governed Org source")]
    Utf8BoundaryMismatch,
    #[error("spent governance nonce table exhausted; rotate epoch")]
    NonceCapacityExhausted,
}

/// Pluggable subtree digest & body-safety verifier.
/// In production Deck runtime this delegates to Lab's Scheme `funs-subtree-mediation`;
/// pure Rust unit/async tests may supply a deterministic SHA-256 / structural checker.
pub trait SubtreeSemanticVerifier {
    fn compute_subtree_digest(
        &self,
        property_id: &GovernedAnchorPropertyId,
        header: &str,
        body: &str,
    ) -> GovernedSubtreeDigest;

    /// Returns `Some(normalized_body)` iff `replacement_body` introduces no rogue
    /// headlines or `:PROPERTIES:` drawers that escape the governed section.
    fn normalize_and_check_body(&self, header: &str, replacement_body: &str) -> Option<String>;
}

/// Single-owner state machine mediating concurrent multi-agent Org reads,
/// disjoint region leases, and conflict-free subtree commits.
pub struct OrgGovernanceLedger<V> {
    verifier: V,
    sources: HashMap<GovernedSourceId, GovernedOrgSnapshot>,
    leases: HashMap<GovernedLeaseId, ActiveRegionLease>,
    consumed_nonces: HashSet<GovernedEffectNonce>,
}

impl<V: SubtreeSemanticVerifier> OrgGovernanceLedger<V> {
    pub fn new(verifier: V) -> Self {
        Self {
            verifier,
            sources: HashMap::new(),
            leases: HashMap::new(),
            consumed_nonces: HashSet::new(),
        }
    }

    pub fn register_snapshot(
        &mut self,
        snapshot: GovernedOrgSnapshot,
    ) -> Result<(), OrgGovernanceError> {
        for desc in snapshot.anchors.values() {
            if desc.span.container_end > snapshot.source.len()
                || !snapshot.source.is_char_boundary(desc.span.container_start)
                || !snapshot.source.is_char_boundary(desc.span.drawer_end)
                || !snapshot.source.is_char_boundary(desc.span.body_end)
                || !snapshot.source.is_char_boundary(desc.span.container_end)
            {
                return Err(OrgGovernanceError::Utf8BoundaryMismatch);
            }
        }
        self.sources.insert(snapshot.source_id.clone(), snapshot);
        Ok(())
    }

    pub fn snapshot(&self, source_id: &GovernedSourceId) -> Option<&GovernedOrgSnapshot> {
        self.sources.get(source_id)
    }

    pub fn active_lease_count(&self, source_id: &GovernedSourceId) -> usize {
        self.leases
            .values()
            .filter(|lease| &lease.source_id == source_id)
            .count()
    }

    /// Acquire a structural `[container_start, container_end)` lease.
    /// Succeeds concurrently for any number of Agents working on disjoint `:ID:`s.
    pub fn acquire_lease(&mut self, req: AcquireRegionLeaseRequest) -> RegionLeaseReceipt {
        if self.leases.contains_key(&req.lease_id) {
            return RegionLeaseReceipt::Denied {
                source_id: req.source_id,
                target_property_id: req.target_property_id,
                reason: GovernanceRejectionReason::DuplicateLeaseId,
            };
        }
        let Some(snapshot) = self.sources.get(&req.source_id) else {
            return RegionLeaseReceipt::Denied {
                source_id: req.source_id,
                target_property_id: req.target_property_id,
                reason: GovernanceRejectionReason::UnknownSource,
            };
        };
        let Some(anchor) = snapshot.anchors.get(&req.target_property_id) else {
            return RegionLeaseReceipt::Denied {
                source_id: req.source_id,
                target_property_id: req.target_property_id,
                reason: GovernanceRejectionReason::UnknownAnchor,
            };
        };
        if anchor.reviewer_agent_id.as_ref() == Some(&req.agent_id)
            && anchor.owner_agent_id.as_ref() == Some(&req.agent_id)
        {
            return RegionLeaseReceipt::Denied {
                source_id: req.source_id,
                target_property_id: req.target_property_id,
                reason: GovernanceRejectionReason::SeparationOfDutiesViolation,
            };
        }
        if !anchor.is_write_eligible_for(&req.agent_id, &req.task_id) {
            return RegionLeaseReceipt::Denied {
                source_id: req.source_id,
                target_property_id: req.target_property_id,
                reason: GovernanceRejectionReason::WriteIneligibleForAgentOrTask,
            };
        }
        let mut source_lease_count = 0usize;
        for active in self.leases.values() {
            if active.source_id == req.source_id {
                source_lease_count += 1;
                if !active.span.is_disjoint_from(&anchor.span) {
                    return RegionLeaseReceipt::Denied {
                        source_id: req.source_id,
                        target_property_id: req.target_property_id,
                        reason: GovernanceRejectionReason::OverlappingRegionLease {
                            conflicting_lease: active.lease_id.clone(),
                            conflicting_anchor: active.target_property_id.clone(),
                            conflicting_agent: active.agent_id.clone(),
                        },
                    };
                }
            }
        }
        if source_lease_count >= MAX_ACTIVE_SOURCE_LEASES {
            return RegionLeaseReceipt::Denied {
                source_id: req.source_id,
                target_property_id: req.target_property_id,
                reason: GovernanceRejectionReason::LeaseCapacityReached,
            };
        }
        let granted = ActiveRegionLease {
            lease_id: req.lease_id.clone(),
            source_id: req.source_id,
            acquired_at_generation: snapshot.generation,
            target_property_id: req.target_property_id,
            agent_id: req.agent_id,
            task_id: req.task_id,
            session_id: req.session_id,
            expected_subtree_digest: anchor.subtree_digest.clone(),
            span: anchor.span,
        };
        self.leases.insert(req.lease_id, granted.clone());
        RegionLeaseReceipt::Granted(granted)
    }

    pub fn release_lease(&mut self, lease_id: &GovernedLeaseId) -> bool {
        self.leases.remove(lease_id).is_some()
    }

    /// Commit a section body replacement under strict nonce + subtree-digest mediation,
    /// automatically rebasing all downstream anchor & active-lease byte spans!
    pub fn commit_subtree(
        &mut self,
        proposal: SubtreeCommitProposal,
    ) -> Result<SubtreeCommitReceipt, OrgGovernanceError> {
        let reject = |reason: GovernanceRejectionReason| SubtreeCommitReceipt {
            accepted: false,
            rejection: Some(reason),
            source_id: proposal.source_id.clone(),
            committed_generation: None,
            target_property_id: proposal.target_property_id.clone(),
            agent_id: proposal.agent_id.clone(),
            before_subtree_digest: proposal.expected_subtree_digest.clone(),
            after_subtree_digest: None,
            rebased: false,
        };

        if self.consumed_nonces.contains(&proposal.nonce) {
            return Ok(reject(GovernanceRejectionReason::NonceAlreadyConsumed));
        }
        if self.consumed_nonces.len() >= MAX_RETAINED_NONCES {
            return Err(OrgGovernanceError::NonceCapacityExhausted);
        }

        let Some(lease) = self.leases.get(&proposal.lease_id) else {
            return Ok(reject(GovernanceRejectionReason::LeaseNotFoundOrMismatch));
        };
        if lease.source_id != proposal.source_id
            || lease.target_property_id != proposal.target_property_id
            || lease.agent_id != proposal.agent_id
            || lease.task_id != proposal.task_id
            || lease.session_id != proposal.session_id
        {
            return Ok(reject(GovernanceRejectionReason::LeaseNotFoundOrMismatch));
        }

        let Some(snapshot) = self.sources.get(&proposal.source_id) else {
            return Ok(reject(GovernanceRejectionReason::UnknownSource));
        };
        if snapshot.generation < proposal.base_generation {
            return Ok(reject(GovernanceRejectionReason::GenerationRollback));
        }
        let Some(anchor) = snapshot.anchors.get(&proposal.target_property_id) else {
            return Ok(reject(GovernanceRejectionReason::UnknownAnchor));
        };
        if !anchor.is_write_eligible_for(&proposal.agent_id, &proposal.task_id) {
            return Ok(reject(
                GovernanceRejectionReason::WriteIneligibleForAgentOrTask,
            ));
        }
        // Subtree-level CAS: unaffected by earlier commits to disjoint sections!
        if anchor.subtree_digest != proposal.expected_subtree_digest {
            return Ok(reject(GovernanceRejectionReason::SubtreeDigestMismatch));
        }

        let span = anchor.span;
        let header = &snapshot.source[span.container_start..span.drawer_end];
        let Some(safe_body) = self
            .verifier
            .normalize_and_check_body(header, &proposal.replacement_body)
        else {
            return Ok(reject(GovernanceRejectionReason::HeaderOrStructuralEscape));
        };

        let next_gen = snapshot.generation.next()?;
        let old_body_len = span.body_end - span.drawer_end;
        let new_body_len = safe_body.len();
        let byte_delta = (new_body_len as isize) - (old_body_len as isize);

        let mut next_source = String::with_capacity(
            snapshot
                .source
                .len()
                .checked_add_signed(byte_delta)
                .ok_or(OrgGovernanceError::Utf8BoundaryMismatch)?,
        );
        next_source.push_str(&snapshot.source[..span.drawer_end]);
        next_source.push_str(&safe_body);
        next_source.push_str(&snapshot.source[span.body_end..]);

        let after_digest =
            self.verifier
                .compute_subtree_digest(&proposal.target_property_id, header, &safe_body);
        let rebased = snapshot.generation > proposal.base_generation;
        let before_digest = anchor.subtree_digest.clone();

        // Advance snapshot and rebase all anchor spans deterministically in O(anchors).
        let mut next_anchors = HashMap::with_capacity(snapshot.anchors.len());
        for (prop_id, mut desc) in snapshot.anchors.clone() {
            if prop_id == proposal.target_property_id {
                desc.span = GovernedSectionSpan::new(
                    span.container_start,
                    span.drawer_end,
                    span.drawer_end + new_body_len,
                    (span.container_end as isize + byte_delta) as usize,
                )?;
                desc.subtree_digest = after_digest.clone();
            } else if desc.span.container_start >= span.container_end {
                // Strictly downstream sibling section: shift all coordinates by `byte_delta`.
                desc.span = desc.span.shifted_by(byte_delta)?;
            } else if desc.span.container_start <= span.container_start
                && desc.span.container_end >= span.container_end
            {
                // Enclosing ancestor container: only its `container_end` grows/shrinks.
                desc.span = GovernedSectionSpan::new(
                    desc.span.container_start,
                    desc.span.drawer_end,
                    desc.span.body_end,
                    (desc.span.container_end as isize + byte_delta) as usize,
                )?;
            }
            next_anchors.insert(prop_id, desc);
        }

        // Rebase active concurrent leases on downstream disjoint sections in O(leases),
        // then release the completed lease and record the spent nonce.
        self.leases.remove(&proposal.lease_id);
        for active in self.leases.values_mut() {
            if active.source_id == proposal.source_id
                && active.span.container_start >= span.container_end
            {
                active.span = active.span.shifted_by(byte_delta)?;
            }
        }
        self.consumed_nonces.insert(proposal.nonce);
        self.sources.insert(
            proposal.source_id.clone(),
            GovernedOrgSnapshot {
                source_id: proposal.source_id.clone(),
                generation: next_gen,
                source: next_source,
                anchors: next_anchors,
            },
        );

        Ok(SubtreeCommitReceipt {
            accepted: true,
            rejection: None,
            source_id: proposal.source_id,
            committed_generation: Some(next_gen),
            target_property_id: proposal.target_property_id,
            agent_id: proposal.agent_id,
            before_subtree_digest: before_digest,
            after_subtree_digest: Some(after_digest),
            rebased,
        })
    }
}
