//! Planned source edits and diagnostics for `Org` patch application.

use marlin_org_model::OrgNodeId;
use serde::{Deserialize, Serialize};

/// Text edits that can be applied after a caller validates current file bytes.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct OrgPatchPlan {
    pub edits: Vec<OrgTextEdit>,
    pub diagnostics: Vec<OrgPatchDiagnostic>,
}

impl OrgPatchPlan {
    /// Returns true when the plan has no unresolved diagnostics.
    pub fn is_applicable(&self) -> bool {
        self.diagnostics.is_empty()
    }
}

/// Single source edit against an `Org` document.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OrgTextEdit {
    pub document: String,
    pub start_byte: usize,
    pub end_byte: usize,
    pub replacement: String,
    pub reason: String,
}

/// Planner diagnostic for a patch operation that cannot be made source-safe yet.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OrgPatchDiagnostic {
    pub node: Option<OrgNodeId>,
    pub operation: String,
    pub message: String,
}
