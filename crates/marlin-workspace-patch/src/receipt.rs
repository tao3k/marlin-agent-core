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
    #[serde(default)]
    pub execution: WorkspacePatchExecutionReceipt,
    pub memory_dispatch: Vec<MemoryDispatchReceipt>,
}

/// Execution proof for the patch receipt boundary.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkspacePatchExecutionReceipt {
    pub mode: WorkspacePatchExecutionMode,
    pub policy: WorkspacePatchPolicyDecision,
}

impl WorkspacePatchExecutionReceipt {
    pub fn dry_run_accepted(reason: impl Into<String>) -> Self {
        Self::accepted(WorkspacePatchExecutionMode::DryRun, reason)
    }

    pub fn commit_accepted(reason: impl Into<String>) -> Self {
        Self::accepted(WorkspacePatchExecutionMode::Commit, reason)
    }

    pub fn accepted(mode: WorkspacePatchExecutionMode, reason: impl Into<String>) -> Self {
        Self {
            mode,
            policy: WorkspacePatchPolicyDecision::accepted(reason),
        }
    }

    pub fn rejected(mode: WorkspacePatchExecutionMode, reason: impl Into<String>) -> Self {
        Self {
            mode,
            policy: WorkspacePatchPolicyDecision::rejected(reason),
        }
    }
}

impl Default for WorkspacePatchExecutionReceipt {
    fn default() -> Self {
        Self::rejected(
            WorkspacePatchExecutionMode::DryRun,
            "execution metadata absent",
        )
    }
}

/// Whether a receipt proves a dry-run or durable commit boundary.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum WorkspacePatchExecutionMode {
    #[default]
    DryRun,
    Commit,
}

/// Policy decision that allowed or blocked execution.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkspacePatchPolicyDecision {
    pub accepted: bool,
    pub reason: Option<String>,
}

impl WorkspacePatchPolicyDecision {
    pub fn accepted(reason: impl Into<String>) -> Self {
        Self {
            accepted: true,
            reason: Some(reason.into()),
        }
    }

    pub fn rejected(reason: impl Into<String>) -> Self {
        Self {
            accepted: false,
            reason: Some(reason.into()),
        }
    }
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
