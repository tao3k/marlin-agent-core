//! Receipts produced by validated workspace patches.

use marlin_org_model::{OrgNodeId, OrgNodeSourceTokens, OrgSourceSpan};
use serde::{Deserialize, Serialize};

use crate::WorkspaceValidationReport;

/// Stable identifier for a workspace patch receipt.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct PatchId(String);

impl PatchId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

/// Receipt proving which nodes changed and how validation finished.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkspacePatchReceipt {
    pub patch_id: PatchId,
    pub affected_nodes: Vec<OrgNodeId>,
    pub affected_sources: Vec<AffectedNodeSource>,
    pub before_hash: String,
    pub after_hash: String,
    pub validation: WorkspaceValidationReport,
    pub memory_dispatch: Vec<MemoryDispatchReceipt>,
}

/// Source provenance for an affected workspace node.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AffectedNodeSource {
    pub node: OrgNodeId,
    pub source: OrgSourceSpan,
    pub tokens: OrgNodeSourceTokens,
}

/// Receipt for downstream memory dispatch attempts.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MemoryDispatchReceipt {
    pub target: String,
    pub accepted: bool,
    pub reason: Option<String>,
}
