//! Typed loop policy IR produced by Gerbil and consumed by Rust runtime plans.

use serde::{Deserialize, Serialize};

/// Current schema version for `ResolvedLoopPolicyPack`.
pub const RESOLVED_LOOP_POLICY_PACK_SCHEMA_VERSION: u32 = 1;

/// Current schema version for `LoopProgram`.
pub const LOOP_PROGRAM_SCHEMA_VERSION: u32 = 1;

macro_rules! u32_id {
    ($name:ident) => {
        #[derive(
            Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Hash, Serialize, Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(u32);

        impl $name {
            pub const fn new(value: u32) -> Self {
                Self(value)
            }

            pub const fn get(self) -> u32 {
                self.0
            }
        }

        impl From<u32> for $name {
            fn from(value: u32) -> Self {
                Self::new(value)
            }
        }
    };
}

macro_rules! string_id {
    ($name:ident) => {
        #[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self::new(value)
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self::new(value)
            }
        }
    };
}

u32_id!(LoopPolicyNodeId);
u32_id!(LoopPolicyExecutorId);
u32_id!(LoopPolicyConditionId);
u32_id!(LoopPolicyRouteBucketId);
u32_id!(LoopPolicyRouteTargetId);
u32_id!(LoopPolicyResourceClassId);
u32_id!(LoopPolicyGraphTemplateId);
u32_id!(LoopPolicyDeltaTemplateId);
u32_id!(LoopPolicyGateId);
u32_id!(LoopPolicyReasonCode);
u32_id!(LoopPolicyAgentProfileId);
u32_id!(LoopPolicySlotId);
u32_id!(LoopPolicySourceLocationId);

string_id!(LoopPolicyRoleId);
string_id!(LoopPolicyMixinId);
string_id!(LoopPolicyDiagnosticCode);
string_id!(LoopPolicySourcePath);
string_id!(LoopPolicyExplanation);
string_id!(LoopProgramId);
string_id!(LoopMechanismPolicyId);
string_id!(LoopProgramStateId);
string_id!(LoopProgramTransitionId);

/// Monotonic policy version produced by the Gerbil semantic compiler.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LoopPolicyEpoch(u64);

impl LoopPolicyEpoch {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u64 {
        self.0
    }
}

impl From<u64> for LoopPolicyEpoch {
    fn from(value: u64) -> Self {
        Self::new(value)
    }
}

/// Provider-neutral loop program produced by Gerbil/POO Flow and executed by Rust.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LoopProgram {
    pub schema_version: u32,
    pub program_id: LoopProgramId,
    pub policy_epoch: LoopPolicyEpoch,
    pub policy_digest: LoopPolicyDigest,
    pub mechanism_policies: Box<[LoopMechanismPolicyId]>,
    pub initial_state: LoopProgramStateId,
    pub transitions: Box<[LoopProgramTransition]>,
}

impl LoopProgram {
    pub fn new(input: LoopProgramInput) -> Self {
        Self {
            schema_version: LOOP_PROGRAM_SCHEMA_VERSION,
            program_id: input.program_id,
            policy_epoch: input.policy_epoch,
            policy_digest: input.policy_digest,
            mechanism_policies: input.mechanism_policies,
            initial_state: input.initial_state,
            transitions: input.transitions,
        }
    }

    pub fn has_current_schema(&self) -> bool {
        self.schema_version == LOOP_PROGRAM_SCHEMA_VERSION
    }

    pub fn uses_policy(&self, policy_id: &LoopMechanismPolicyId) -> bool {
        self.mechanism_policies
            .iter()
            .any(|candidate| candidate == policy_id)
    }
}

/// Named construction surface for a provider-neutral loop program.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LoopProgramInput {
    pub program_id: LoopProgramId,
    pub policy_epoch: LoopPolicyEpoch,
    pub policy_digest: LoopPolicyDigest,
    pub mechanism_policies: Box<[LoopMechanismPolicyId]>,
    pub initial_state: LoopProgramStateId,
    pub transitions: Box<[LoopProgramTransition]>,
}

/// One defunctionalized transition in a provider-neutral loop program.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LoopProgramTransition {
    pub transition_id: LoopProgramTransitionId,
    pub from: LoopProgramStateId,
    pub event: LoopProgramEventKind,
    pub action: LoopProgramActionKind,
    pub to: LoopProgramStateId,
}

/// Runtime event classes consumed by the generic loop machine.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoopProgramEventKind {
    Start,
    ModelEvent,
    ToolRequest,
    ToolReceipt,
    MemoryReceipt,
    PlacementReceipt,
    RuntimeReceipt,
    HumanDecision,
    VerificationReceipt,
    BudgetExceeded,
    StopSignal,
    Error,
}

/// Runtime action classes emitted by the generic loop machine.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoopProgramActionKind {
    Continue,
    InvokeModel,
    DispatchTools,
    ReadMemory,
    WriteMemory,
    RequestPlacement,
    RuntimeHandoff,
    RewriteGraph,
    ForkSession,
    DelegateAgent,
    Verify,
    HumanGate,
    CompactContext,
    EmitReceipt,
    Stop,
}

/// Stable digest of the resolved policy pack.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LoopPolicyDigest([u8; 32]);

impl LoopPolicyDigest {
    pub const fn from_bytes(value: [u8; 32]) -> Self {
        Self(value)
    }

    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Gerbil-resolved loop policy pack split into hot runtime data and cold audit data.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ResolvedLoopPolicyPack {
    pub schema_version: u32,
    pub policy_epoch: LoopPolicyEpoch,
    pub policy_digest: LoopPolicyDigest,
    pub hot: HotLoopPolicyPack,
    pub audit: AuditLoopPolicyPack,
}

impl ResolvedLoopPolicyPack {
    pub fn new(
        policy_epoch: impl Into<LoopPolicyEpoch>,
        policy_digest: LoopPolicyDigest,
        hot: HotLoopPolicyPack,
        audit: AuditLoopPolicyPack,
    ) -> Self {
        Self {
            schema_version: RESOLVED_LOOP_POLICY_PACK_SCHEMA_VERSION,
            policy_epoch: policy_epoch.into(),
            policy_digest,
            hot,
            audit,
        }
    }

    pub fn has_current_schema(&self) -> bool {
        self.schema_version == RESOLVED_LOOP_POLICY_PACK_SCHEMA_VERSION
    }
}

/// Runtime-hot loop policy data. Keep strings, source locations, and explanations out of this pack.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct HotLoopPolicyPack {
    pub capability_mask: u64,
    pub human_gate_mask: u64,
    pub budget_caps: BudgetCaps,
    pub graph_nodes: Box<[CompiledLoopNode]>,
    pub graph_edges: Box<[CompiledLoopEdge]>,
    pub route_index: CompiledRouteIndex,
    pub resource_classes: Box<[ResourceClass]>,
    pub continuation_table: Box<[ContinuationOp]>,
    pub maker_profiles: Box<[LoopPolicyAgentProfileId]>,
    pub checker_profiles: Box<[LoopPolicyAgentProfileId]>,
}

/// Runtime budget caps after Gerbil slot forcing, merge algebra, and freeze.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct BudgetCaps {
    pub max_attempts: u16,
    pub max_cost_units: u64,
    pub max_wall_time_ms: u64,
}

/// Compiled node entry in a bounded loop DAG template.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CompiledLoopNode {
    pub node_id: LoopPolicyNodeId,
    pub executor_id: LoopPolicyExecutorId,
    pub capability_mask: u64,
    pub resource_class_id: LoopPolicyResourceClassId,
}

/// Compiled edge entry in a bounded loop DAG template.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CompiledLoopEdge {
    pub from: LoopPolicyNodeId,
    pub to: LoopPolicyNodeId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition_id: Option<LoopPolicyConditionId>,
}

/// Compact route index for hot-path admission and route selection.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct CompiledRouteIndex {
    pub buckets: Box<[CompiledRouteBucket]>,
}

/// One compiled route bucket, typically produced from trie, DFA, or first-token bucketing.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CompiledRouteBucket {
    pub bucket_id: LoopPolicyRouteBucketId,
    pub scope_mask: u64,
    pub target_id: LoopPolicyRouteTargetId,
}

/// Resource class used by Rust scheduling and collision checks.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ResourceClass {
    pub resource_class_id: LoopPolicyResourceClassId,
    pub exclusive: bool,
}

/// Defunctionalized continuation operation. Scheme closures do not cross the runtime boundary.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum ContinuationOp {
    StopCompleted,
    StopFailed,
    Retry {
        graph_template: LoopPolicyGraphTemplateId,
        max_attempts: u16,
    },
    Rewrite {
        delta_template: LoopPolicyDeltaTemplateId,
    },
    Defer {
        gate_id: LoopPolicyGateId,
    },
    Escalate {
        reason_code: LoopPolicyReasonCode,
    },
}

/// Audit-only policy data retained for provenance, explanation, and replay.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct AuditLoopPolicyPack {
    #[serde(default)]
    pub policy_mixins: Box<[LoopPolicyMixinId]>,
    pub provenance: Box<[SlotProvenance]>,
    pub linearization: Box<[LoopPolicyRoleId]>,
    pub diagnostics: Box<[LoopPolicyDiagnostic]>,
    pub source_locations: Box<[SourceLocation]>,
    pub explanation_strings: Box<[LoopPolicyExplanation]>,
    pub forced_slots: Box<[ForcedSlot]>,
    pub merge_receipts: Box<[SlotMergeReceipt]>,
}

impl AuditLoopPolicyPack {
    pub fn uses_mixin(&self, mixin_id: &LoopPolicyMixinId) -> bool {
        self.policy_mixins
            .iter()
            .any(|candidate| candidate == mixin_id)
    }

    pub fn has_applied_merge(&self, merge: SlotMergeAlgebra) -> bool {
        self.merge_receipts
            .iter()
            .any(|receipt| receipt.merge == merge && receipt.status == SlotMergeStatus::Applied)
    }

    pub fn has_conflict_merge(&self, merge: SlotMergeAlgebra) -> bool {
        self.merge_receipts
            .iter()
            .any(|receipt| receipt.merge == merge && receipt.status == SlotMergeStatus::Conflict)
    }

    pub fn covers_slot_merge_algebras(
        &self,
        required: impl IntoIterator<Item = SlotMergeAlgebra>,
    ) -> bool {
        required
            .into_iter()
            .all(|merge| self.has_applied_merge(merge.clone()) || self.has_conflict_merge(merge))
    }
}

/// Provenance for one resolved policy slot.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SlotProvenance {
    pub slot_id: LoopPolicySlotId,
    pub winner_role: LoopPolicyRoleId,
    pub source_role_order: Box<[LoopPolicyRoleId]>,
    pub merge: SlotMergeAlgebra,
}

/// Slot merge algebra selected by policy compiler before runtime execution.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SlotMergeAlgebra {
    Intersection,
    Union,
    Min,
    OrderedAppend,
    ConflictError,
    Override,
}

/// Diagnostic emitted by the Gerbil semantic compiler.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LoopPolicyDiagnostic {
    pub code: LoopPolicyDiagnosticCode,
    pub severity: LoopPolicyDiagnosticSeverity,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_location_id: Option<LoopPolicySourceLocationId>,
}

/// Severity for diagnostics emitted while resolving a loop policy pack.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoopPolicyDiagnosticSeverity {
    Info,
    Warning,
    Error,
}

/// Source location retained only in the audit pack.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SourceLocation {
    pub source_location_id: LoopPolicySourceLocationId,
    pub path: LoopPolicySourcePath,
    pub line: u32,
    pub column: u32,
}

/// Forced slot classification recorded when POO lazy slots are materialized before handoff.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ForcedSlot {
    pub slot_id: LoopPolicySlotId,
    pub hotness: SlotHotness,
}

/// Runtime hotness classification for a forced POO slot.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SlotHotness {
    Hot,
    Warm,
    AuditOnly,
}

/// Receipt for one slot merge operation performed by Gerbil before freezing the pack.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SlotMergeReceipt {
    pub slot_id: LoopPolicySlotId,
    pub merge: SlotMergeAlgebra,
    pub status: SlotMergeStatus,
}

/// Status for one slot merge receipt.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SlotMergeStatus {
    Applied,
    Conflict,
}
