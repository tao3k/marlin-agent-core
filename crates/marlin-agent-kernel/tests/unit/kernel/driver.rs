use marlin_agent_kernel::{
    GraphLoopExecutionRequest, GraphLoopExecutionResult, GraphLoopExecutionStatus, GraphLoopKernel,
    LoopEdgeSpec, LoopGraph, LoopNodeSpec, TokioGraphLoopKernel,
};
use marlin_agent_runtime::TokioAgentRuntime;
use tokio_stream::StreamExt;

use super::support::CompletingExecutor;

#[tokio::test]
async fn executes_graph_node_through_registered_executor() {
    let graph = LoopGraph {
        graph_id: "graph".to_owned(),
        nodes: vec![node("node-1")],
        edges: Vec::new(),
    };
    let request = GraphLoopExecutionRequest::new("run", graph);
    let kernel = echo_kernel();
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

#[tokio::test]
async fn executes_graph_nodes_in_edge_topology_order() {
    let graph = LoopGraph {
        graph_id: "graph".to_owned(),
        nodes: vec![node("review"), node("apply"), node("plan")],
        edges: vec![edge("plan", "apply"), edge("apply", "review")],
    };

    let result = run_echo_graph(graph).await;

    assert_eq!(result.status, GraphLoopExecutionStatus::Completed);
    assert_eq!(result.visited_nodes, vec!["plan", "apply", "review"]);
}

#[tokio::test]
async fn rejects_edges_that_reference_missing_nodes() {
    let graph = LoopGraph {
        graph_id: "graph".to_owned(),
        nodes: vec![node("plan")],
        edges: vec![edge("plan", "apply")],
    };

    let result = run_echo_graph(graph).await;

    assert_eq!(result.status, GraphLoopExecutionStatus::Failed);
    assert_eq!(result.visited_nodes, Vec::<String>::new());
    assert_eq!(
        result.diagnostics,
        vec!["graph edge references missing target node `apply`"]
    );
}

#[tokio::test]
async fn rejects_duplicate_node_ids() {
    let graph = LoopGraph {
        graph_id: "graph".to_owned(),
        nodes: vec![node("plan"), node("plan")],
        edges: Vec::new(),
    };

    let result = run_echo_graph(graph).await;

    assert_eq!(result.status, GraphLoopExecutionStatus::Failed);
    assert_eq!(
        result.diagnostics,
        vec!["graph contains duplicate node ids: plan"]
    );
}

#[tokio::test]
async fn rejects_conditional_edges_until_condition_evaluation_exists() {
    let graph = LoopGraph {
        graph_id: "graph".to_owned(),
        nodes: vec![node("plan"), node("apply")],
        edges: vec![LoopEdgeSpec {
            from: "plan".to_owned(),
            to: "apply".to_owned(),
            condition: Some("approved".to_owned()),
        }],
    };

    let result = run_echo_graph(graph).await;

    assert_eq!(result.status, GraphLoopExecutionStatus::Failed);
    assert_eq!(
        result.diagnostics,
        vec!["conditional graph edge plan -> apply is not supported"]
    );
}

#[tokio::test]
async fn rejects_cyclic_edge_topologies() {
    let graph = LoopGraph {
        graph_id: "graph".to_owned(),
        nodes: vec![node("plan"), node("apply")],
        edges: vec![edge("plan", "apply"), edge("apply", "plan")],
    };

    let result = run_echo_graph(graph).await;

    assert_eq!(result.status, GraphLoopExecutionStatus::Failed);
    assert_eq!(
        result.diagnostics,
        vec!["graph edge topology contains a cycle; pending nodes: plan, apply"]
    );
}

async fn run_echo_graph(graph: LoopGraph) -> GraphLoopExecutionResult {
    let request = GraphLoopExecutionRequest::new("run", graph);
    let (runtime, _events) = TokioAgentRuntime::new(8);
    echo_kernel()
        .spawn_execution(request, &runtime)
        .join()
        .await
        .expect("kernel task should join")
}

fn echo_kernel() -> TokioGraphLoopKernel {
    TokioGraphLoopKernel::new("run", "graph").with_executor("echo", CompletingExecutor)
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
