use marlin_agent_kernel::{
    GraphLoopExecutionRequest, GraphLoopExecutionStatus, GraphLoopKernel,
    GraphNodeExecutionReceipt, GraphNodeExecutor, GraphNodeInvocation, LoopGraph, LoopNodeSpec,
    TokioGraphLoopKernel,
};
use marlin_agent_runtime::{RuntimeContext, RuntimeFuture, TokioAgentRuntime};
use tokio_stream::StreamExt;

#[derive(Clone, Debug)]
struct CompletingExecutor;

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

#[tokio::test]
async fn kernel_executes_graph_node_through_registered_executor() {
    let graph = LoopGraph {
        graph_id: "graph".to_owned(),
        nodes: vec![LoopNodeSpec {
            id: "node-1".to_owned(),
            executor: "echo".to_owned(),
            config: Default::default(),
        }],
        edges: Vec::new(),
    };
    let request = GraphLoopExecutionRequest::new("run", graph);
    let kernel =
        TokioGraphLoopKernel::new("run", "graph").with_executor("echo", CompletingExecutor);
    let (runtime, mut events) = TokioAgentRuntime::new(8);

    let result = kernel
        .spawn_execution(request, &runtime)
        .join()
        .await
        .expect("kernel task should join");

    assert_eq!(result.status, GraphLoopExecutionStatus::Completed);
    assert_eq!(result.visited_nodes, vec!["node-1"]);
    assert_eq!(
        events.next().await.expect("start event").topic,
        "kernel.execution"
    );
}
