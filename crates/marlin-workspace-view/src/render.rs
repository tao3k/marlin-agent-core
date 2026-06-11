//! Rendered workspace view records.

use marlin_org_model::OrgNodeId;
use serde::{Deserialize, Serialize};

/// Rendered workspace view bounded for agent or UI consumers.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RenderedWorkspaceView {
    pub spec_hash: String,
    pub token_estimate: usize,
    pub nodes: Vec<RenderedViewNode>,
    pub text: String,
}

/// Rendered projection of one workspace node.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RenderedViewNode {
    pub node_id: OrgNodeId,
    pub title: Option<String>,
    pub lines: Vec<String>,
}
