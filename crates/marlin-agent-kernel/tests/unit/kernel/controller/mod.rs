mod basic;
mod continuation;
mod evidence;

use marlin_agent_kernel::{
    GraphLoopContinuationAction, GraphLoopContinuationDecision, GraphLoopContinuationInput,
    GraphLoopContinuationPlanner, GraphLoopContinuationReceipt, GraphLoopExecutionStatus,
    LoopEdgeSpec, LoopGraph, LoopNodeSpec, TokioGraphLoopController, TokioGraphLoopKernel,
};
use marlin_agent_runtime::RuntimeFuture;

use super::support::CompletingExecutor;

fn controller() -> TokioGraphLoopController {
    TokioGraphLoopController::new(
        TokioGraphLoopKernel::new("run", "graph").with_executor("echo", CompletingExecutor),
    )
}

fn node(id: &str) -> LoopNodeSpec {
    LoopNodeSpec {
        id: id.to_owned(),
        executor: "echo".to_owned(),
        config: Default::default(),
    }
}

fn edge(from: &str, to: &str) -> LoopEdgeSpec {
    LoopEdgeSpec {
        from: from.to_owned(),
        to: to.to_owned(),
        condition: None,
    }
}

#[derive(Clone, Debug)]
struct ContinueOncePlanner;

impl GraphLoopContinuationPlanner for ContinueOncePlanner {
    fn decide(
        &self,
        input: GraphLoopContinuationInput,
    ) -> RuntimeFuture<GraphLoopContinuationDecision> {
        Box::pin(async move {
            if input.iteration_id.get() == 0 {
                GraphLoopContinuationDecision::new(GraphLoopContinuationReceipt::new(
                    input.run_id,
                    input.iteration_id,
                    GraphLoopContinuationAction::Rewrite {
                        graph: LoopGraph {
                            graph_id: "graph-next".to_owned(),
                            nodes: vec![node("review")],
                            edges: Vec::new(),
                        },
                        reason: "test.continue_once".to_owned(),
                    },
                ))
            } else {
                GraphLoopContinuationDecision::new(GraphLoopContinuationReceipt::new(
                    input.run_id,
                    input.iteration_id,
                    GraphLoopContinuationAction::Accept,
                ))
            }
        })
    }
}

#[derive(Clone, Debug)]
struct RepairFailurePlanner;

impl GraphLoopContinuationPlanner for RepairFailurePlanner {
    fn decide(
        &self,
        input: GraphLoopContinuationInput,
    ) -> RuntimeFuture<GraphLoopContinuationDecision> {
        Box::pin(async move {
            if input.execution_result.status == GraphLoopExecutionStatus::Failed {
                GraphLoopContinuationDecision::new(GraphLoopContinuationReceipt::new(
                    input.run_id,
                    input.iteration_id,
                    GraphLoopContinuationAction::Rewrite {
                        graph: LoopGraph {
                            graph_id: "graph-repair".to_owned(),
                            nodes: vec![node("repair")],
                            edges: Vec::new(),
                        },
                        reason: "test.repair_failure".to_owned(),
                    },
                ))
            } else {
                GraphLoopContinuationDecision::new(GraphLoopContinuationReceipt::new(
                    input.run_id,
                    input.iteration_id,
                    GraphLoopContinuationAction::Accept,
                ))
            }
        })
    }
}
