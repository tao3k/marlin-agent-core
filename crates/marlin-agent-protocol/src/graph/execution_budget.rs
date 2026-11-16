// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Execution budget contract for graph-loop runtime requests.

use serde::{Deserialize, Serialize};

/// Execution-time budget enforced by the Rust graph-loop kernel.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct GraphLoopExecutionBudget {
    pub max_node_executions: Option<u64>,
}

impl GraphLoopExecutionBudget {
    /// Creates a budget that caps the number of nodes the kernel may execute.
    pub fn max_node_executions(max_node_executions: u64) -> Self {
        Self {
            max_node_executions: Some(max_node_executions),
        }
    }
}
