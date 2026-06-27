//! Runtime repair case receipts shared by live and no-live harness runs.

use marlin_agent_protocol::LoopProgramId;
use serde::{Deserialize, Serialize};

/// Stable schema id for live runtime repair case receipts.
pub const RUNTIME_REPAIR_LIVE_CASE_RECEIPT_SCHEMA_ID: &str =
    "marlin.runtime-repair.live-case-receipt.v1";
/// Stable schema id for no-live runtime repair denial receipts.
pub const RUNTIME_REPAIR_NO_LIVE_CASE_RECEIPT_SCHEMA_ID: &str =
    "marlin.runtime-repair.no-live-case-receipt.v1";

macro_rules! define_runtime_repair_string_id {
    ($type_name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $type_name(String);

        impl $type_name {
            #[must_use]
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl From<&str> for $type_name {
            fn from(value: &str) -> Self {
                Self::new(value)
            }
        }

        impl From<String> for $type_name {
            fn from(value: String) -> Self {
                Self::new(value)
            }
        }

        impl std::fmt::Display for $type_name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str(self.as_str())
            }
        }
    };
}

define_runtime_repair_string_id!(
    RuntimeRepairSchemaId,
    "Stable schema id for one runtime repair receipt."
);
define_runtime_repair_string_id!(
    RuntimeRepairCaseId,
    "Stable id for one runtime repair case."
);
define_runtime_repair_string_id!(
    RuntimeRepairProfileRef,
    "Stable Scheme/POO profile reference used by one runtime repair case."
);
define_runtime_repair_string_id!(
    RuntimeRepairModelCompletionId,
    "Provider completion id observed during one runtime repair case."
);
define_runtime_repair_string_id!(
    RuntimeRepairModelId,
    "Provider model id observed during one runtime repair case."
);
define_runtime_repair_string_id!(
    RuntimeRepairContentDigest,
    "Stable digest of a runtime repair content artifact."
);
define_runtime_repair_string_id!(
    RuntimeRepairDenialReason,
    "Stable denial reason for a no-live runtime repair receipt."
);

/// Count observed in a runtime repair receipt.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RuntimeRepairCount(usize);

impl RuntimeRepairCount {
    #[must_use]
    pub fn new(value: usize) -> Self {
        Self(value)
    }

    #[must_use]
    pub fn get(self) -> usize {
        self.0
    }
}

/// Millisecond duration observed in a runtime repair receipt.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RuntimeRepairDurationMillis(u64);

impl RuntimeRepairDurationMillis {
    #[must_use]
    pub fn new(value: u64) -> Self {
        Self(value)
    }

    #[must_use]
    pub fn get(self) -> u64 {
        self.0
    }
}

/// Digest and byte count for repaired content without embedding source text.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeRepairContentSummary {
    pub digest: RuntimeRepairContentDigest,
    pub byte_count: RuntimeRepairCount,
}

impl RuntimeRepairContentSummary {
    #[must_use]
    pub fn from_bytes(bytes: &[u8]) -> Self {
        Self {
            digest: RuntimeRepairContentDigest::new(stable_bytes_digest(bytes)),
            byte_count: RuntimeRepairCount::new(bytes.len()),
        }
    }

    #[must_use]
    pub fn from_text(text: &str) -> Self {
        Self::from_bytes(text.as_bytes())
    }
}

/// Live LLM gate status observed by a runtime repair case.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RuntimeRepairLiveGateStatus {
    Disabled,
    MissingProvider,
    MissingModel,
    MissingProviderKey,
    Enabled,
}

/// Runtime handoff status summarized for repair receipts.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RuntimeRepairHandoffStatus {
    Empty,
    Completed,
    Deferred,
    Denied,
}

/// Receipt for a live single-file runtime repair case.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeRepairLiveCaseReceipt {
    pub schema_id: RuntimeRepairSchemaId,
    pub case_id: RuntimeRepairCaseId,
    pub profile_ref: RuntimeRepairProfileRef,
    pub program_id: LoopProgramId,
    pub model_completion_id: RuntimeRepairModelCompletionId,
    pub model: RuntimeRepairModelId,
    pub elapsed_ms: RuntimeRepairDurationMillis,
    pub action_count: RuntimeRepairCount,
    pub tool_projection_count: RuntimeRepairCount,
    pub patch_tool_success: bool,
    pub graph_rewrite_projected: bool,
    pub verification_success: bool,
    pub repaired_content: RuntimeRepairContentSummary,
}

impl RuntimeRepairLiveCaseReceipt {
    #[must_use]
    pub fn new(request: RuntimeRepairLiveCaseReceiptRequest) -> Self {
        Self {
            schema_id: RuntimeRepairSchemaId::new(RUNTIME_REPAIR_LIVE_CASE_RECEIPT_SCHEMA_ID),
            case_id: request.case_id,
            profile_ref: request.profile_ref,
            program_id: request.program_id,
            model_completion_id: request.model_completion_id,
            model: request.model,
            elapsed_ms: request.elapsed_ms,
            action_count: request.action_count,
            tool_projection_count: request.tool_projection_count,
            patch_tool_success: request.patch_tool_success,
            graph_rewrite_projected: request.graph_rewrite_projected,
            verification_success: request.verification_success,
            repaired_content: request.repaired_content,
        }
    }
}

/// Named request for constructing a live runtime repair receipt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeRepairLiveCaseReceiptRequest {
    pub case_id: RuntimeRepairCaseId,
    pub profile_ref: RuntimeRepairProfileRef,
    pub program_id: LoopProgramId,
    pub model_completion_id: RuntimeRepairModelCompletionId,
    pub model: RuntimeRepairModelId,
    pub elapsed_ms: RuntimeRepairDurationMillis,
    pub action_count: RuntimeRepairCount,
    pub tool_projection_count: RuntimeRepairCount,
    pub patch_tool_success: bool,
    pub graph_rewrite_projected: bool,
    pub verification_success: bool,
    pub repaired_content: RuntimeRepairContentSummary,
}

/// Receipt emitted when a live runtime repair case is denied by the no-live gate.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeRepairNoLiveCaseReceipt {
    pub schema_id: RuntimeRepairSchemaId,
    pub case_id: RuntimeRepairCaseId,
    pub profile_ref: RuntimeRepairProfileRef,
    pub program_id: LoopProgramId,
    pub gate_status: RuntimeRepairLiveGateStatus,
    pub denial_reason: RuntimeRepairDenialReason,
    pub live_llm_allowed: bool,
    pub action_count: RuntimeRepairCount,
    pub model_handoff_status: RuntimeRepairHandoffStatus,
}

impl RuntimeRepairNoLiveCaseReceipt {
    #[must_use]
    pub fn new(request: RuntimeRepairNoLiveCaseReceiptRequest) -> Self {
        Self {
            schema_id: RuntimeRepairSchemaId::new(RUNTIME_REPAIR_NO_LIVE_CASE_RECEIPT_SCHEMA_ID),
            case_id: request.case_id,
            profile_ref: request.profile_ref,
            program_id: request.program_id,
            gate_status: request.gate_status,
            denial_reason: request.denial_reason,
            live_llm_allowed: request.live_llm_allowed,
            action_count: request.action_count,
            model_handoff_status: request.model_handoff_status,
        }
    }
}

/// Named request for constructing a no-live runtime repair receipt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeRepairNoLiveCaseReceiptRequest {
    pub case_id: RuntimeRepairCaseId,
    pub profile_ref: RuntimeRepairProfileRef,
    pub program_id: LoopProgramId,
    pub gate_status: RuntimeRepairLiveGateStatus,
    pub denial_reason: RuntimeRepairDenialReason,
    pub live_llm_allowed: bool,
    pub action_count: RuntimeRepairCount,
    pub model_handoff_status: RuntimeRepairHandoffStatus,
}

fn stable_bytes_digest(bytes: &[u8]) -> String {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x00000100000001b3;

    let mut value = FNV_OFFSET;
    for byte in bytes {
        value ^= u64::from(*byte);
        value = value.wrapping_mul(FNV_PRIME);
    }
    format!("fnv1a64:{value:016x}")
}
