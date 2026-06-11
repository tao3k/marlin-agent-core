//! Graph-loop driver built on the Tokio agent runtime.

use std::{
    collections::BTreeMap,
    future::Future,
    pin::Pin,
    sync::{Arc, RwLock},
};

use marlin_agent_protocol::{
    GraphId, GraphLoopExecutionRequest, GraphLoopExecutionResult, GraphNodeExecutionReceipt,
    GraphNodeExecutionStatus, GraphNodeInvocation, LoopNodeSpec, RunId, RuntimePlanSnapshot,
};
use marlin_agent_runtime::{
    RuntimeContext, RuntimeEvent, RuntimeFuture, RuntimeTask, TokioAgentRuntime,
};

type KernelFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Executor contract for one compiled graph node.
pub trait GraphNodeExecutor: Send + Sync + 'static {
    fn execute_node(
        &self,
        invocation: GraphNodeInvocation,
        context: RuntimeContext,
    ) -> RuntimeFuture<GraphNodeExecutionReceipt>;
}

/// Kernel contract for spawning and observing graph-loop execution.
pub trait GraphLoopKernel: Send + Sync + 'static {
    fn spawn_execution(
        &self,
        request: GraphLoopExecutionRequest,
        runtime: &TokioAgentRuntime,
    ) -> RuntimeTask<GraphLoopExecutionResult>;

    fn snapshot(&self) -> RuntimePlanSnapshot;
}

/// Tokio-backed graph-loop kernel with named node executors.
#[derive(Clone)]
pub struct TokioGraphLoopKernel {
    executors: Arc<BTreeMap<String, Arc<dyn GraphNodeExecutor>>>,
    snapshot: Arc<RwLock<RuntimePlanSnapshot>>,
}

impl TokioGraphLoopKernel {
    pub fn new(run_id: impl Into<String>, graph_id: impl Into<String>) -> Self {
        Self {
            executors: Arc::new(BTreeMap::new()),
            snapshot: Arc::new(RwLock::new(RuntimePlanSnapshot {
                run_id: run_id.into(),
                graph_id: graph_id.into(),
                active_node: None,
            })),
        }
    }

    pub fn with_executor<E>(mut self, name: impl Into<String>, executor: E) -> Self
    where
        E: GraphNodeExecutor,
    {
        Arc::make_mut(&mut self.executors).insert(name.into(), Arc::new(executor));
        self
    }

    async fn execute_graph(
        &self,
        request: GraphLoopExecutionRequest,
        context: RuntimeContext,
    ) -> GraphLoopExecutionResult {
        let run_id = request.run_id;
        let graph_id = request.graph.graph_id;
        self.store_snapshot(&run_id, &graph_id, None);
        emit(
            &context,
            "kernel.execution",
            format!("run {run_id} started graph {graph_id}"),
        )
        .await;

        if context.is_cancelled() {
            return self
                .cancelled_result(&context, &run_id, &graph_id, Vec::new())
                .await;
        }

        self.drive_graph_nodes_from(
            &request.graph.nodes,
            0,
            &run_id,
            &graph_id,
            context,
            Vec::new(),
        )
        .await
    }

    fn drive_graph_nodes_from<'a>(
        &'a self,
        nodes: &'a [LoopNodeSpec],
        next_index: usize,
        run_id: &'a str,
        graph_id: &'a str,
        context: RuntimeContext,
        visited_nodes: Vec<String>,
    ) -> KernelFuture<'a, GraphLoopExecutionResult> {
        Box::pin(async move {
            if next_index >= nodes.len() {
                self.store_snapshot(run_id, graph_id, None);
                emit(
                    &context,
                    "kernel.execution",
                    format!("run {run_id} completed graph {graph_id}"),
                )
                .await;
                return GraphLoopExecutionResult::completed(self.snapshot(), visited_nodes);
            }

            if context.is_cancelled() {
                return self
                    .cancelled_result(&context, run_id, graph_id, visited_nodes)
                    .await;
            }

            let node = &nodes[next_index];
            self.store_snapshot(run_id, graph_id, Some(node.id.clone()));
            emit(
                &context,
                "kernel.node",
                format!("node {} started executor {}", node.id, node.executor),
            )
            .await;

            let Some(executor) = self.executors.get(&node.executor) else {
                let diagnostic = format!(
                    "missing graph node executor `{}` for node {}",
                    node.executor, node.id
                );
                return self
                    .failed_result(&context, run_id, graph_id, visited_nodes, vec![diagnostic])
                    .await;
            };

            let invocation = GraphNodeInvocation::from_loop_node(
                RunId::new(run_id),
                GraphId::new(graph_id),
                node,
            );
            let receipt = executor
                .execute_node(invocation, context.child_context())
                .await;

            match receipt.status {
                GraphNodeExecutionStatus::Completed => {
                    emit(
                        &context,
                        "kernel.node",
                        format!(
                            "node {} completed executor {}",
                            receipt.node_id.as_str(),
                            receipt.executor.as_str()
                        ),
                    )
                    .await;
                    let mut next_visited = visited_nodes;
                    next_visited.push(receipt.node_id.into_string());
                    self.drive_graph_nodes_from(
                        nodes,
                        next_index + 1,
                        run_id,
                        graph_id,
                        context,
                        next_visited,
                    )
                    .await
                }
                GraphNodeExecutionStatus::Failed => {
                    self.failed_result(
                        &context,
                        run_id,
                        graph_id,
                        visited_nodes,
                        receipt.diagnostics,
                    )
                    .await
                }
            }
        })
    }

    async fn cancelled_result(
        &self,
        context: &RuntimeContext,
        run_id: &str,
        graph_id: &str,
        visited_nodes: Vec<String>,
    ) -> GraphLoopExecutionResult {
        self.store_snapshot(run_id, graph_id, None);
        emit(
            context,
            "kernel.execution",
            format!("run {run_id} cancelled graph {graph_id}"),
        )
        .await;
        GraphLoopExecutionResult::cancelled(self.snapshot(), visited_nodes)
    }

    async fn failed_result(
        &self,
        context: &RuntimeContext,
        run_id: &str,
        graph_id: &str,
        visited_nodes: Vec<String>,
        diagnostics: Vec<String>,
    ) -> GraphLoopExecutionResult {
        self.store_snapshot(run_id, graph_id, None);
        emit(
            context,
            "kernel.execution",
            format!("run {run_id} failed graph {graph_id}"),
        )
        .await;
        GraphLoopExecutionResult::failed_with_visited(self.snapshot(), visited_nodes, diagnostics)
    }

    fn store_snapshot(
        &self,
        run_id: &str,
        graph_id: &str,
        active_node: Option<String>,
    ) -> RuntimePlanSnapshot {
        let snapshot = RuntimePlanSnapshot {
            run_id: run_id.to_owned(),
            graph_id: graph_id.to_owned(),
            active_node,
        };
        *self
            .snapshot
            .write()
            .expect("runtime plan snapshot lock should not be poisoned") = snapshot.clone();
        snapshot
    }
}

impl GraphLoopKernel for TokioGraphLoopKernel {
    fn spawn_execution(
        &self,
        request: GraphLoopExecutionRequest,
        runtime: &TokioAgentRuntime,
    ) -> RuntimeTask<GraphLoopExecutionResult> {
        let kernel = self.clone();
        let child_runtime = runtime.child_runtime();
        let context = child_runtime.context();
        child_runtime.spawn(async move { kernel.execute_graph(request, context).await })
    }

    fn snapshot(&self) -> RuntimePlanSnapshot {
        self.snapshot
            .read()
            .expect("runtime plan snapshot lock should not be poisoned")
            .clone()
    }
}

async fn emit(context: &RuntimeContext, topic: impl Into<String>, message: impl Into<String>) {
    let _ = context.emit(RuntimeEvent::new(topic, message)).await;
}
