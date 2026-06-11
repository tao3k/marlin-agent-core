//! Workspace policy `IR` emitted by the `Gerbil` control plane.

use marlin_org_model::{
    OrgContractRegistry, OrgContractResolutionReport, OrgContractValidationReport,
};
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

/// Release topology for a crate or artifact family managed by `Gerbil`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReleaseTopologySpec {
    pub topology_id: String,
    pub crate_name: String,
    pub publish_enabled: bool,
    pub asset_audit_command: String,
    pub package_assets: Vec<String>,
    pub runtime_dependency_chain: Vec<String>,
    pub workflow_dependency_chain: Vec<String>,
    pub gates: Vec<ReleaseGateSpec>,
}

/// Release gate command and artifacts required by a `Gerbil` release topology.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReleaseGateSpec {
    pub gate_id: String,
    pub command: String,
    pub requires_local_gerbil: bool,
    pub required_artifacts: Vec<String>,
}

/// Parser-owned workspace contract facts made available to the `Gerbil` control plane.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilWorkspaceContractFacts {
    pub registry: OrgContractRegistry,
    pub resolutions: OrgContractResolutionReport,
    pub validations: OrgContractValidationReport,
}
