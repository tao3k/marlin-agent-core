// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use marlin_agent_kernel::{GraphNodeExecutionReceipt, GraphNodeExecutor, GraphNodeInvocation};
use marlin_agent_runtime::{RuntimeContext, RuntimeFuture};

#[derive(Clone, Debug)]
pub(crate) struct CompletingExecutor;

impl GraphNodeExecutor for CompletingExecutor {
    fn execute_node(
        &self,
        invocation: GraphNodeInvocation,
        _context: RuntimeContext,
    ) -> RuntimeFuture<GraphNodeExecutionReceipt> {
        Box::pin(async move {
            GraphNodeExecutionReceipt::completed(invocation.node_id, invocation.executor)
        })
    }
}
