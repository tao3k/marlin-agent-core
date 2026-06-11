//! Graph-loop driver built on the Tokio agent runtime.

use std::{
    collections::{BTreeMap, BTreeSet},
    future::Future,
    pin::Pin,
    sync::{Arc, RwLock},
};

use marlin_agent_protocol::{
    GraphId, GraphLoopExecutionRequest, GraphLoopExecutionResult, GraphNodeExecutionReceipt,
    GraphNodeExecutionStatus, GraphNodeInvocation, LoopEdgeSpec, LoopNodeSpec, RunId,
    RuntimePlanSnapshot,
};
use marlin_agent_runtime::{
    RuntimeContext, RuntimeEvent, RuntimeExecutionIdentity, RuntimeFuture, RuntimeTask,
    TokioAgentRuntime, observability,
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
            observability::kernel_execution_event(format!("run {run_id} started graph {graph_id}")),
        )
        .await;

        if context.is_cancelled() {
            return self
                .cancelled_result(&context, &run_id, &graph_id, Vec::new())
                .await;
        }

        let planned_nodes = match resolve_graph_plan(&request.graph.nodes, &request.graph.edges) {
            Ok(planned_nodes) => planned_nodes,
            Err(diagnostics) => {
                return self
                    .failed_result(&context, &run_id, &graph_id, Vec::new(), diagnostics)
                    .await;
            }
        };

        self.drive_graph_nodes_from(&planned_nodes, 0, &run_id, &graph_id, context, Vec::new())
            .await
    }

    fn drive_graph_nodes_from<'a>(
        &'a self,
        nodes: &'a [&'a LoopNodeSpec],
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
                    observability::kernel_execution_event(format!(
                        "run {run_id} completed graph {graph_id}"
                    )),
                )
                .await;
                return GraphLoopExecutionResult::completed(self.snapshot(), visited_nodes);
            }

            if context.is_cancelled() {
                return self
                    .cancelled_result(&context, run_id, graph_id, visited_nodes)
                    .await;
            }

            let node = nodes[next_index];
            self.store_snapshot(run_id, graph_id, Some(node.id.clone()));
            emit(
                &context,
                observability::kernel_node_event(format!(
                    "node {} started executor {}",
                    node.id, node.executor
                )),
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
                        observability::kernel_node_event(format!(
                            "node {} completed executor {}",
                            receipt.node_id.as_str(),
                            receipt.executor.as_str()
                        )),
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
            observability::kernel_execution_event(format!(
                "run {run_id} cancelled graph {graph_id}"
            )),
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
            observability::kernel_execution_event(format!("run {run_id} failed graph {graph_id}")),
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
        let execution =
            RuntimeExecutionIdentity::new(request.run_id.clone(), request.graph.graph_id.clone());
        let context = child_runtime.context().with_execution_identity(execution);
        child_runtime.spawn(async move { kernel.execute_graph(request, context).await })
    }

    fn snapshot(&self) -> RuntimePlanSnapshot {
        self.snapshot
            .read()
            .expect("runtime plan snapshot lock should not be poisoned")
            .clone()
    }
}

async fn emit(context: &RuntimeContext, event: RuntimeEvent) {
    let _ = context.emit(event).await;
}

fn resolve_graph_plan<'a>(
    nodes: &'a [LoopNodeSpec],
    edges: &[LoopEdgeSpec],
) -> Result<Vec<&'a LoopNodeSpec>, Vec<String>> {
    let duplicate_ids = duplicate_node_ids(nodes);
    if !duplicate_ids.is_empty() {
        return Err(vec![format!(
            "graph contains duplicate node ids: {}",
            duplicate_ids.join(", ")
        )]);
    }

    if edges.is_empty() {
        return Ok(nodes.iter().collect());
    }

    let node_by_id = nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect::<BTreeMap<_, _>>();

    let unsupported_conditions = edges
        .iter()
        .filter(|edge| edge.condition.is_some())
        .map(|edge| {
            format!(
                "conditional graph edge {} -> {} is not supported",
                edge.from, edge.to
            )
        })
        .collect::<Vec<_>>();
    if !unsupported_conditions.is_empty() {
        return Err(unsupported_conditions);
    }

    let missing_endpoints = missing_edge_endpoints(edges, &node_by_id);
    if !missing_endpoints.is_empty() {
        return Err(missing_endpoints);
    }

    let node_rank = nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.id.as_str(), index))
        .collect::<BTreeMap<_, _>>();
    let mut incoming = nodes
        .iter()
        .map(|node| (node.id.as_str(), 0usize))
        .collect::<BTreeMap<_, _>>();
    let mut outgoing = nodes
        .iter()
        .map(|node| (node.id.as_str(), Vec::<&str>::new()))
        .collect::<BTreeMap<_, _>>();

    edges.iter().for_each(|edge| {
        *incoming
            .get_mut(edge.to.as_str())
            .expect("edge endpoint should be validated") += 1;
        outgoing
            .get_mut(edge.from.as_str())
            .expect("edge endpoint should be validated")
            .push(edge.to.as_str());
    });

    let mut ready = nodes
        .iter()
        .filter(|node| incoming[node.id.as_str()] == 0)
        .map(|node| node.id.as_str())
        .collect::<Vec<_>>();
    let mut ordered_ids = Vec::with_capacity(nodes.len());

    while let Some(node_id) = pop_next_ready_node(&mut ready, &node_rank) {
        ordered_ids.push(node_id);
        outgoing[node_id].iter().copied().for_each(|successor| {
            let incoming_count = incoming
                .get_mut(successor)
                .expect("edge endpoint should be validated");
            *incoming_count -= 1;
            if *incoming_count == 0 {
                ready.push(successor);
            }
        });
    }

    if ordered_ids.len() != nodes.len() {
        let visited = ordered_ids.iter().copied().collect::<BTreeSet<_>>();
        let pending = nodes
            .iter()
            .filter(|node| !visited.contains(node.id.as_str()))
            .map(|node| node.id.clone())
            .collect::<Vec<_>>();
        return Err(vec![format!(
            "graph edge topology contains a cycle; pending nodes: {}",
            pending.join(", ")
        )]);
    }

    Ok(ordered_ids
        .into_iter()
        .map(|node_id| node_by_id[node_id])
        .collect())
}

fn duplicate_node_ids(nodes: &[LoopNodeSpec]) -> Vec<String> {
    let mut seen = BTreeSet::new();
    nodes
        .iter()
        .filter_map(|node| {
            if seen.insert(node.id.as_str()) {
                None
            } else {
                Some(node.id.clone())
            }
        })
        .collect()
}

fn missing_edge_endpoints(
    edges: &[LoopEdgeSpec],
    node_by_id: &BTreeMap<&str, &LoopNodeSpec>,
) -> Vec<String> {
    edges
        .iter()
        .flat_map(|edge| {
            let mut diagnostics = Vec::new();
            if !node_by_id.contains_key(edge.from.as_str()) {
                diagnostics.push(format!(
                    "graph edge references missing source node `{}`",
                    edge.from
                ));
            }
            if !node_by_id.contains_key(edge.to.as_str()) {
                diagnostics.push(format!(
                    "graph edge references missing target node `{}`",
                    edge.to
                ));
            }
            diagnostics
        })
        .collect()
}

fn pop_next_ready_node<'a>(
    ready: &mut Vec<&'a str>,
    node_rank: &BTreeMap<&'a str, usize>,
) -> Option<&'a str> {
    let next_index = ready
        .iter()
        .enumerate()
        .min_by_key(|(_, node_id)| node_rank[*node_id])
        .map(|(index, _)| index)?;
    Some(ready.remove(next_index))
}
