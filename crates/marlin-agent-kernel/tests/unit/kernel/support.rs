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
