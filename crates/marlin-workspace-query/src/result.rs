// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Query result records returned by workspace backends.

use marlin_context_org_model::{OrgNode, OrgNodeId};
use serde::{Deserialize, Serialize};

/// Query result set with truncation metadata.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceQueryResult {
    pub matches: Vec<QueryMatch>,
    pub truncated: bool,
}

impl WorkspaceQueryResult {
    pub fn empty() -> Self {
        Self {
            matches: Vec::new(),
            truncated: false,
        }
    }
}

/// Single matched workspace node.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct QueryMatch {
    pub node_id: OrgNodeId,
    pub node: Option<OrgNode>,
    pub score: Option<u32>,
    pub reason: Option<String>,
}
