//! Rendered workspace view records.

use marlin_org_model::{
    OrgContractDiagnostic, OrgContractResolution, OrgContractTemplate, OrgContractValidationReport,
    OrgNodeId,
};
use serde::{Deserialize, Serialize};

/// Rendered workspace view bounded for agent or UI consumers.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RenderedWorkspaceView {
    pub spec_hash: String,
    pub token_estimate: usize,
    pub nodes: Vec<RenderedViewNode>,
    pub contract_facts: Option<RenderedContractFacts>,
    pub text: String,
}

/// Contract facts selected for a rendered workspace view.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct RenderedContractFacts {
    pub resolutions: Vec<OrgContractResolution>,
    pub diagnostics: Vec<OrgContractDiagnostic>,
    pub templates: Vec<OrgContractTemplate>,
    pub validations: OrgContractValidationReport,
}

/// Rendered projection of one workspace node.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RenderedViewNode {
    pub node_id: OrgNodeId,
    pub title: Option<String>,
    pub lines: Vec<String>,
}
