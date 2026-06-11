//! Target model for workspace status requests.

use marlin_org_model::OrgNodeId;
use serde::{Deserialize, Serialize};

/// Target entity for a status report.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum WorkspaceTarget {
    Goal(OrgNodeId),
    Sdd(OrgNodeId),
    Checklist(OrgNodeId),
    Workspace,
}
