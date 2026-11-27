// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Governed Context capsules (Two-Zone KV-Prefix digest, L0..L3 progressive
//! disclosure floor under active defeaters, MINJA/MemoryGraft quarantine counts,
//! Nono Lethal-Trifecta sandbox admission, Cedar handoff validity) and Scoped
//! Detour 4-exit re-anchoring contracts for Session-free `TaskActor`s.

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::ids::AgentGraphValidationError;
use crate::task_actor::{FencingEpoch, TaskActorId};

macro_rules! governed_id {
    ($name:ident) => {
        #[doc = concat!("Typed semantic identifier for `", stringify!($name), "`.")]
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(String);

        impl $name {
            /// Builds a non-empty semantic identifier free of control characters.
            pub fn new(value: impl Into<String>) -> Result<Self, AgentGraphValidationError> {
                let value = value.into();
                if value.trim().is_empty() || value.chars().any(char::is_control) {
                    return Err(AgentGraphValidationError::empty_named(stringify!($name)));
                }
                Ok(Self(value))
            }

            /// Returns the identifier as a stable string slice.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(self.as_str())
            }
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.serialize_str(self.as_str())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;
                Self::new(value).map_err(serde::de::Error::custom)
            }
        }
    };
}

governed_id!(KvPrefixDigest);
governed_id!(DetourNodeId);

/// Progressive disclosure depth (`L0 < L1 < L2 < L3`) computed by `ai-agentic-context`.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum DisclosureDepth {
    /// Minimal headline / ID index only.
    L0,
    /// Summary / active obligations.
    L1,
    /// Structured evidence & linked sections.
    L2,
    /// Full audit / defeater disclosure floor required whenever an active defeater exists.
    L3,
}

/// Active Scoped Detour operating mode (mirrors Design 165 `durable.detour-state`).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum DetourMode {
    /// Normal trunk mainline execution.
    Mainline,
    /// Blocker detour that halts trunk settlement and disables auto-archive (`MarlinDetourBlockerProfile.`).
    BlockerActive,
    /// Bounded side-quest detour that isolates scratch state without blocking sibling observation.
    SideQuestActive,
}

/// The 4 typed re-anchoring exits of a Scoped Detour.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum DetourExitKind {
    /// Return cleanly to the originating mainline anchor without mutating trunk cut.
    ReturnToOrigin,
    /// Absorb validated detour evidence into the trunk and advance `context_cut`.
    AbsorbAndContinue,
    /// Promote the detour into an independent sibling `TaskActor` and return parent to mainline.
    SpawnSiblingTask,
    /// Discard detour scratchpad and return parent to mainline without trunk mutation.
    AbortDetour,
}

/// Current Scoped Detour state held by a `TaskActor`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum DetourState {
    /// Executing on the main trunk.
    Mainline,
    /// Executing inside an active Scoped Detour anchored at `(origin_epoch, detour_node_id)`.
    Active {
        /// Whether this detour is a settlement-blocking `BlockerActive` or `SideQuestActive`.
        mode: DetourMode,
        /// Actor fencing epoch when the detour was entered.
        origin_epoch: FencingEpoch,
        /// Stable anchor node identifier for the detour.
        detour_node_id: DetourNodeId,
    },
}

impl DetourState {
    /// Returns the active `DetourMode` (`Mainline`, `BlockerActive`, or `SideQuestActive`).
    pub fn mode(&self) -> DetourMode {
        match self {
            Self::Mainline => DetourMode::Mainline,
            Self::Active { mode, .. } => *mode,
        }
    }

    /// Returns true iff a settlement-blocking `BlockerActive` detour is currently open.
    pub fn is_blocker_active(&self) -> bool {
        matches!(
            self,
            Self::Active {
                mode: DetourMode::BlockerActive,
                ..
            }
        )
    }
}

/// Governed Context capsule projected from Lab `ai-agentic-context` + `GraphEngine`
/// + Design 165 profile convergence.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GovernedContextCapsule {
    /// Owning Session-free `TaskActor` identifier.
    pub task_id: TaskActorId,
    /// Monotonic shared Org/Context cut (strictly >= 1).
    pub context_cut: u64,
    /// Frozen Two-Zone KV-Prefix digest computed by `ai-agentic-context`.
    pub kv_prefix_digest: KvPrefixDigest,
    /// Effective progressive disclosure depth (`L0..L3`).
    pub disclosure_depth: DisclosureDepth,
    /// Whether any active defeater exists on the obligation graph.
    pub has_active_defeater: bool,
    /// Count of MINJA/MemoryGraft memory candidates excluded by quarantine.
    pub quarantined_memory_count: u32,
    /// Result of Slice 52 Lethal-Trifecta (`untrusted + private + unendorsed egress`) Nono sandbox gate.
    pub sandbox_admitted: bool,
    /// Whether the inert Cedar policy handoff descriptor validated cleanly.
    pub cedar_handoff_valid: bool,
    /// Active Design 165 detour mode (`Mainline`, `BlockerActive`, or `SideQuestActive`).
    pub detour_mode: DetourMode,
    /// Whether Task-as-Tab auto-archive is enabled (must be `false` under `BlockerActive`).
    pub auto_archive_enabled: bool,
}

/// Witness proving a `GovernedContextCapsule` passed Marlin's fail-closed governance gate.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AdmittedGovernedContext {
    capsule: GovernedContextCapsule,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AdmittedGovernedContextWire {
    capsule: GovernedContextCapsule,
}

impl<'de> Deserialize<'de> for AdmittedGovernedContext {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = AdmittedGovernedContextWire::deserialize(deserializer)?;
        admit_governed_context_capsule(wire.capsule).map_err(serde::de::Error::custom)
    }
}

impl AdmittedGovernedContext {
    /// Borrows the admitted governed context capsule.
    pub fn capsule(&self) -> &GovernedContextCapsule {
        &self.capsule
    }

    /// Consumes the witness and returns the underlying capsule.
    pub fn into_capsule(self) -> GovernedContextCapsule {
        self.capsule
    }

    /// Returns a cloned witness with `context_cut` advanced by one (used by `AbsorbAndContinue`).
    pub fn with_next_cut(&self) -> Self {
        let mut next = self.capsule.clone();
        next.context_cut = next.context_cut.saturating_add(1);
        Self { capsule: next }
    }
}

/// Typed fail-closed rejection reasons for `admit_governed_context_capsule`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GovernedContextRejection {
    /// Shared context cut must be strictly >= 1.
    InvalidContextCut,
    /// An active defeater requires at least `DisclosureDepth::L3` (anti-suppression floor).
    ActiveDefeaterRequiresL3Disclosure,
    /// Slice 52 Lethal-Trifecta Nono sandbox gate denied execution.
    SandboxLethalTrifectaDenied,
    /// Cedar authorization handoff descriptor is invalid.
    InvalidCedarHandoff,
    /// `BlockerActive` detour must disable auto-archive (`MarlinDetourBlockerProfile.`).
    BlockerDetourMustDisableAutoArchive,
}

impl fmt::Display for GovernedContextRejection {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidContextCut => formatter.write_str("invalid-context-cut"),
            Self::ActiveDefeaterRequiresL3Disclosure => {
                formatter.write_str("active-defeater-requires-l3-disclosure")
            }
            Self::SandboxLethalTrifectaDenied => {
                formatter.write_str("sandbox-lethal-trifecta-denied")
            }
            Self::InvalidCedarHandoff => formatter.write_str("invalid-cedar-policy-handoff"),
            Self::BlockerDetourMustDisableAutoArchive => {
                formatter.write_str("blocker-detour-must-disable-auto-archive")
            }
        }
    }
}

impl std::error::Error for GovernedContextRejection {}

/// Fail-closed Rust admission gate verifying all `ai-agentic-context` + Design 165 invariants.
pub fn admit_governed_context_capsule(
    capsule: GovernedContextCapsule,
) -> Result<AdmittedGovernedContext, GovernedContextRejection> {
    if capsule.context_cut == 0 {
        return Err(GovernedContextRejection::InvalidContextCut);
    }
    if capsule.has_active_defeater && capsule.disclosure_depth < DisclosureDepth::L3 {
        return Err(GovernedContextRejection::ActiveDefeaterRequiresL3Disclosure);
    }
    if !capsule.sandbox_admitted {
        return Err(GovernedContextRejection::SandboxLethalTrifectaDenied);
    }
    if !capsule.cedar_handoff_valid {
        return Err(GovernedContextRejection::InvalidCedarHandoff);
    }
    if capsule.detour_mode == DetourMode::BlockerActive && capsule.auto_archive_enabled {
        return Err(GovernedContextRejection::BlockerDetourMustDisableAutoArchive);
    }
    Ok(AdmittedGovernedContext { capsule })
}
