// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Target model for workspace status requests.

use marlin_context_org_model::OrgNodeId;
use serde::{Deserialize, Serialize};

/// Target entity for a status report.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum WorkspaceTarget {
    Goal(OrgNodeId),
    Sdd(OrgNodeId),
    Checklist(OrgNodeId),
    Workspace,
}
