//! Graph-loop driver built on the Tokio agent runtime.

use std::{
    collections::{BTreeMap, BTreeSet},
    future::Future,
    pin::Pin,
    sync::{Arc, RwLock},
};

use marlin_agent_protocol::{
    GraphId, GraphLoopExecutionBudget, GraphLoopExecutionRequest, GraphLoopExecutionResult,
    GraphNativeAbiReadinessReceipt, GraphNativeAbiReadinessStatus, GraphNodeExecutionReceipt,
    GraphNodeExecutionStatus, GraphNodeInvocation, GraphPolicyProposal, GraphPolicyProposalReceipt,
    GraphPolicyProposalStatus, LoopEdgeSpec, LoopNodeSpec, RunId, RuntimePlanSnapshot,
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

/// Kernel-side compilation result for a graph policy proposal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphPolicyProposalCompilation {
    pub receipt: GraphPolicyProposalReceipt,
    pub request: Option<GraphLoopExecutionRequest>,
}

impl GraphPolicyProposalCompilation {
    /// Returns true when Rust accepted the proposal and produced an execution request.
    pub fn is_accepted(&self) -> bool {
        self.receipt.status == GraphPolicyProposalStatus::Accepted && self.request.is_some()
    }
}

/// Validates a graph policy proposal and compiles it into an execution request.
pub fn compile_graph_policy_proposal(
    run_id: impl Into<String>,
    proposal: &GraphPolicyProposal,
) -> GraphPolicyProposalCompilation {
    let receipt = GraphPolicyProposalReceipt::validate(proposal);
    let request = (receipt.status == GraphPolicyProposalStatus::Accepted)
        .then(|| GraphLoopExecutionRequest::new(run_id, proposal.proposed_graph.clone()));
    GraphPolicyProposalCompilation { receipt, request }
}

/// Validates a graph policy proposal against a native ABI readiness receipt before execution.
pub fn compile_graph_policy_proposal_with_native_abi_readiness(
    run_id: impl Into<String>,
    proposal: &GraphPolicyProposal,
    readiness: &GraphNativeAbiReadinessReceipt,
) -> GraphPolicyProposalCompilation {
    let mut receipt = GraphPolicyProposalReceipt::validate(proposal);
    if receipt.status == GraphPolicyProposalStatus::Accepted {
        let diagnostics = native_abi_readiness_diagnostics(proposal, readiness);
        if !diagnostics.is_empty() {
            receipt = GraphPolicyProposalReceipt::rejected(proposal, diagnostics);
        }
    }

    let request = (receipt.status == GraphPolicyProposalStatus::Accepted)
        .then(|| GraphLoopExecutionRequest::new(run_id, proposal.proposed_graph.clone()));
    GraphPolicyProposalCompilation { receipt, request }
}

fn native_abi_readiness_diagnostics(
    proposal: &GraphPolicyProposal,
    readiness: &GraphNativeAbiReadinessReceipt,
) -> Vec<String> {
    let mut diagnostics = Vec::new();
    let Some(requirement) = proposal.native_abi.as_ref() else {
        diagnostics.push("graph_policy_proposal.native_abi_readiness_unexpected".to_string());
        return diagnostics;
    };

    if requirement.abi_id != readiness.abi_id {
        diagnostics.push(format!(
            "graph_policy_proposal.native_abi_readiness_abi_id_mismatch:{}:{}",
            requirement.abi_id.as_str(),
            readiness.abi_id.as_str()
        ));
    }
    if requirement.version != readiness.version {
        diagnostics.push(format!(
            "graph_policy_proposal.native_abi_readiness_version_mismatch:{}:{}",
            requirement.version, readiness.version
        ));
    }
    if readiness.status != GraphNativeAbiReadinessStatus::Ready {
        diagnostics.push("graph_policy_proposal.native_abi_readiness_not_ready".to_string());
    }
    if !readiness.missing_symbols.is_empty() {
        diagnostics.push(format!(
            "graph_policy_proposal.native_abi_readiness_missing_symbols:{}",
            readiness
                .missing_symbols
                .iter()
                .map(|symbol| symbol.as_str())
                .collect::<Vec<_>>()
                .join(",")
        ));
    }

    diagnostics
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

        if let Some(diagnostic) =
            execution_budget_diagnostic(request.budget.as_ref(), planned_nodes.len())
        {
            return self
                .failed_result(&context, &run_id, &graph_id, Vec::new(), vec![diagnostic])
                .await;
        }

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
        node_receipts: Vec<GraphNodeExecutionReceipt>,
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
                let visited_nodes = completed_node_ids(&node_receipts);
                return GraphLoopExecutionResult::completed(self.snapshot(), visited_nodes)
                    .with_node_receipts(node_receipts);
            }

            if context.is_cancelled() {
                return self
                    .cancelled_result(&context, run_id, graph_id, node_receipts)
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
                    .failed_result(&context, run_id, graph_id, node_receipts, vec![diagnostic])
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
                    let mut next_node_receipts = node_receipts;
                    next_node_receipts.push(receipt);
                    self.drive_graph_nodes_from(
                        nodes,
                        next_index + 1,
                        run_id,
                        graph_id,
                        context,
                        next_node_receipts,
                    )
                    .await
                }
                GraphNodeExecutionStatus::Failed => {
                    let diagnostics = receipt.diagnostics.clone();
                    let mut failed_node_receipts = node_receipts;
                    failed_node_receipts.push(receipt);
                    self.failed_result(
                        &context,
                        run_id,
                        graph_id,
                        failed_node_receipts,
                        diagnostics,
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
        node_receipts: Vec<GraphNodeExecutionReceipt>,
    ) -> GraphLoopExecutionResult {
        self.store_snapshot(run_id, graph_id, None);
        emit(
            context,
            observability::kernel_execution_event(format!(
                "run {run_id} cancelled graph {graph_id}"
            )),
        )
        .await;
        let visited_nodes = completed_node_ids(&node_receipts);
        GraphLoopExecutionResult::cancelled(self.snapshot(), visited_nodes)
            .with_node_receipts(node_receipts)
    }

    async fn failed_result(
        &self,
        context: &RuntimeContext,
        run_id: &str,
        graph_id: &str,
        node_receipts: Vec<GraphNodeExecutionReceipt>,
        diagnostics: Vec<String>,
    ) -> GraphLoopExecutionResult {
        self.store_snapshot(run_id, graph_id, None);
        emit(
            context,
            observability::kernel_execution_event(format!("run {run_id} failed graph {graph_id}")),
        )
        .await;
        let visited_nodes = completed_node_ids(&node_receipts);
        GraphLoopExecutionResult::failed_with_visited(self.snapshot(), visited_nodes, diagnostics)
            .with_node_receipts(node_receipts)
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

fn completed_node_ids(node_receipts: &[GraphNodeExecutionReceipt]) -> Vec<String> {
    node_receipts
        .iter()
        .filter(|receipt| receipt.status == GraphNodeExecutionStatus::Completed)
        .map(|receipt| receipt.node_id.as_str().to_owned())
        .collect()
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

    let missing_endpoints = missing_edge_endpoints(edges, &node_by_id);
    if !missing_endpoints.is_empty() {
        return Err(missing_endpoints);
    }

    let active_edges = active_graph_edges(edges)?;
    let executable_ids = executable_node_ids(nodes, edges, &active_edges);

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

    active_edges.iter().for_each(|edge| {
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
        .filter(|node_id| executable_ids.contains(*node_id))
        .map(|node_id| node_by_id[node_id])
        .collect())
}

fn execution_budget_diagnostic(
    budget: Option<&GraphLoopExecutionBudget>,
    planned_node_count: usize,
) -> Option<String> {
    let max_node_executions = budget?.max_node_executions?;
    let planned_node_count = planned_node_count as u64;
    (planned_node_count > max_node_executions).then(|| {
        format!(
            "graph execution budget exceeded: planned node executions {planned_node_count} > max {max_node_executions}"
        )
    })
}

fn active_graph_edges(edges: &[LoopEdgeSpec]) -> Result<Vec<&LoopEdgeSpec>, Vec<String>> {
    let evaluations = edges.iter().map(active_graph_edge).collect::<Vec<_>>();
    let diagnostics = evaluations
        .iter()
        .filter_map(|evaluation| evaluation.as_ref().err().cloned())
        .collect::<Vec<_>>();
    if diagnostics.is_empty() {
        Ok(evaluations
            .into_iter()
            .filter_map(Result::ok)
            .flatten()
            .collect())
    } else {
        Err(diagnostics)
    }
}

fn active_graph_edge(edge: &LoopEdgeSpec) -> Result<Option<&LoopEdgeSpec>, String> {
    graph_edge_condition_is_active(edge).map(|is_active| is_active.then_some(edge))
}

fn graph_edge_condition_is_active(edge: &LoopEdgeSpec) -> Result<bool, String> {
    let Some(condition) = edge
        .condition
        .as_deref()
        .map(str::trim)
        .filter(|condition| !condition.is_empty())
    else {
        return Ok(true);
    };

    if condition.eq_ignore_ascii_case("always") || condition.eq_ignore_ascii_case("true") {
        return Ok(true);
    }
    if condition.eq_ignore_ascii_case("never") || condition.eq_ignore_ascii_case("false") {
        return Ok(false);
    }
    Err(format!(
        "unsupported graph edge condition `{condition}` on {} -> {}",
        edge.from, edge.to
    ))
}

fn executable_node_ids(
    nodes: &[LoopNodeSpec],
    edges: &[LoopEdgeSpec],
    active_edges: &[&LoopEdgeSpec],
) -> BTreeSet<String> {
    let incoming = original_incoming_counts(nodes, edges);
    let executable = root_node_ids(&incoming);
    let outgoing = active_outgoing_edges(active_edges);
    reachable_node_ids(executable, &outgoing)
}

fn original_incoming_counts<'a>(
    nodes: &'a [LoopNodeSpec],
    edges: &[LoopEdgeSpec],
) -> BTreeMap<&'a str, usize> {
    edges.iter().fold(
        nodes
            .iter()
            .map(|node| (node.id.as_str(), 0usize))
            .collect::<BTreeMap<_, _>>(),
        |mut incoming, edge| {
            if let Some(incoming_count) = incoming.get_mut(edge.to.as_str()) {
                *incoming_count += 1;
            }
            incoming
        },
    )
}

fn root_node_ids(incoming: &BTreeMap<&str, usize>) -> BTreeSet<String> {
    incoming
        .iter()
        .filter(|(_, incoming_count)| **incoming_count == 0)
        .map(|(node_id, _)| (*node_id).to_owned())
        .collect()
}

fn active_outgoing_edges<'a>(active_edges: &[&'a LoopEdgeSpec]) -> BTreeMap<&'a str, Vec<&'a str>> {
    active_edges
        .iter()
        .fold(BTreeMap::<&str, Vec<&str>>::new(), |mut outgoing, edge| {
            outgoing
                .entry(edge.from.as_str())
                .or_default()
                .push(edge.to.as_str());
            outgoing
        })
}

fn reachable_node_ids(
    mut executable: BTreeSet<String>,
    outgoing: &BTreeMap<&str, Vec<&str>>,
) -> BTreeSet<String> {
    let mut frontier = executable.iter().cloned().collect::<Vec<_>>();
    while let Some(node_id) = frontier.pop() {
        push_new_successors(node_id.as_str(), outgoing, &mut executable, &mut frontier);
    }
    executable
}

fn push_new_successors(
    node_id: &str,
    outgoing: &BTreeMap<&str, Vec<&str>>,
    executable: &mut BTreeSet<String>,
    frontier: &mut Vec<String>,
) {
    if let Some(successors) = outgoing.get(node_id) {
        frontier.extend(
            successors
                .iter()
                .filter_map(|successor| pushable_successor(successor, executable)),
        );
    }
}

fn pushable_successor(successor: &str, executable: &mut BTreeSet<String>) -> Option<String> {
    executable
        .insert(successor.to_owned())
        .then(|| successor.to_owned())
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
