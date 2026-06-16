use marlin_agent_core::{
    GraphLoopContinuationAction, GraphLoopContinuationDecision, GraphLoopContinuationInput,
    GraphLoopContinuationPlanner, GraphLoopContinuationReceipt, GraphLoopController,
    GraphLoopExecutionRequest, GraphLoopExecutionStatus, GraphLoopNextAction, GraphLoopRunRequest,
    GraphNodeExecutionReceipt, GraphNodeExecutor, GraphNodeInvocation, LoopGraph, LoopNodeSpec,
    RuntimeContext, RuntimeFuture, TokioAgentRuntime, TokioGraphLoopController,
    TokioGraphLoopKernel,
};

struct CoreFacadeControllerExecutor;

impl GraphNodeExecutor for CoreFacadeControllerExecutor {
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
async fn core_facade_exposes_graph_loop_controller_runner() {
    let (runtime, _events) = TokioAgentRuntime::new(16);
    let controller = TokioGraphLoopController::new(
        TokioGraphLoopKernel::new("run-1", "graph-1")
            .with_executor("core.controller", CoreFacadeControllerExecutor),
    );
    let request = GraphLoopRunRequest::new(GraphLoopExecutionRequest::new("run-1", graph()));

    let reports = controller
        .spawn_loop(request, &runtime)
        .join()
        .await
        .expect("controller task should finish");

    assert_eq!(reports.len(), 1);
    let report = &reports[0];
    assert_eq!(
        report.execution_result.status,
        GraphLoopExecutionStatus::Completed
    );
    assert_eq!(report.execution_result.visited_nodes, vec!["plan"]);
    assert_eq!(report.next_action, GraphLoopNextAction::StopCompleted);
}

#[tokio::test]
async fn core_facade_exposes_graph_loop_continuation_planner_boundary() {
    let (runtime, _events) = TokioAgentRuntime::new(16);
    let controller = TokioGraphLoopController::new(
        TokioGraphLoopKernel::new("run-1", "graph-1")
            .with_executor("core.controller", CoreFacadeControllerExecutor),
    )
    .with_continuation_planner(CoreFacadeStopPlanner);
    let request = GraphLoopRunRequest::new(GraphLoopExecutionRequest::new("run-1", graph()));

    let reports = controller
        .spawn_loop(request, &runtime)
        .join()
        .await
        .expect("controller task should finish");

    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].next_action, GraphLoopNextAction::StopCompleted);
}

fn graph() -> LoopGraph {
    LoopGraph {
        graph_id: "graph-1".to_owned(),
        nodes: vec![LoopNodeSpec {
            id: "plan".to_owned(),
            executor: "core.controller".to_owned(),
            config: Default::default(),
        }],
        edges: Vec::new(),
    }
}

#[derive(Clone, Debug)]
struct CoreFacadeStopPlanner;

impl GraphLoopContinuationPlanner for CoreFacadeStopPlanner {
    fn decide(
        &self,
        input: GraphLoopContinuationInput,
    ) -> RuntimeFuture<GraphLoopContinuationDecision> {
        Box::pin(async move {
            GraphLoopContinuationDecision::new(GraphLoopContinuationReceipt::new(
                input.run_id,
                input.iteration_id,
                GraphLoopContinuationAction::Accept,
            ))
        })
    }
}
