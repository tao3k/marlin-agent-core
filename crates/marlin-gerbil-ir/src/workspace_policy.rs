//! Workspace policy `IR` emitted by the `Gerbil` control plane.

use marlin_workspace_patch::WorkspacePatch;
use marlin_workspace_view::WorkspaceViewSpec;
use serde::{Deserialize, Serialize};

/// Schema contract for a class of workspace records.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceSchemaSpec {
    pub schema_id: String,
    pub required_properties: Vec<String>,
    pub todo_states: Vec<String>,
}

/// Named view policy carrying a typed `WorkspaceViewSpec`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceViewPolicySpec {
    pub policy_id: String,
    pub view: WorkspaceViewSpec,
}

/// Validation policy for workspace state transitions.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceValidationPolicySpec {
    pub policy_id: String,
    pub evidence_required_for_done: bool,
    pub block_done_if_open_blockers: bool,
}

/// Memory dispatch policy emitted as data, not runtime mutation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MemoryDispatchPolicySpec {
    pub policy_id: String,
    pub dispatch_when: String,
    pub target: String,
}

/// Typed workspace patch intent emitted as data by the `Gerbil` control plane.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorkspacePatchIntentSpec {
    pub intent_id: String,
    pub patch: WorkspacePatch,
    pub dry_run_first: bool,
}
