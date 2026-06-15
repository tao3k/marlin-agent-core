mod basic;
mod continuation;
mod evidence;

use marlin_agent_kernel::{
    GraphLoopContinuationInput, GraphLoopContinuationPlanner, GraphLoopExecutionStatus,
    GraphLoopNextAction, LoopEdgeSpec, LoopGraph, LoopNodeSpec, TokioGraphLoopController,
    TokioGraphLoopKernel,
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
    fn next_action(&self, input: GraphLoopContinuationInput) -> RuntimeFuture<GraphLoopNextAction> {
        Box::pin(async move {
            if input.iteration == 0 {
                GraphLoopNextAction::ContinueWithGraph(LoopGraph {
                    graph_id: "graph-next".to_owned(),
                    nodes: vec![node("review")],
                    edges: Vec::new(),
                })
            } else {
                GraphLoopNextAction::StopCompleted
            }
        })
    }
}

#[derive(Clone, Debug)]
struct RepairFailurePlanner;

impl GraphLoopContinuationPlanner for RepairFailurePlanner {
    fn next_action(&self, input: GraphLoopContinuationInput) -> RuntimeFuture<GraphLoopNextAction> {
        Box::pin(async move {
            if input.execution_result.status == GraphLoopExecutionStatus::Failed {
                GraphLoopNextAction::ContinueWithGraph(LoopGraph {
                    graph_id: "graph-repair".to_owned(),
                    nodes: vec![node("repair")],
                    edges: Vec::new(),
                })
            } else {
                GraphLoopNextAction::StopCompleted
            }
        })
    }
}
