//! Core graph-loop request, receipt, snapshot, and identifier contracts.

use super::execution_budget::GraphLoopExecutionBudget;
use super::native_abi::{GraphNativeAbiRequirement, validate_graph_native_abi_requirement};
use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

/// Schema identifier for graph-loop policy proposals emitted by strategy planes.
pub const GRAPH_POLICY_PROPOSAL_SCHEMA_ID: &str = "marlin.agent.graph_policy_proposal.v1";

/// Runtime-ready loop graph produced from typed control-plane `IR`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LoopGraph {
    pub graph_id: String,
    pub nodes: Vec<LoopNodeSpec>,
    pub edges: Vec<LoopEdgeSpec>,
}

/// Protocol-owned node specification inside a runtime loop graph.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LoopNodeSpec {
    pub id: String,
    pub executor: String,
    pub config: BTreeMap<String, String>,
}

/// Protocol-owned directed edge specification between runtime graph nodes.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LoopEdgeSpec {
    pub from: String,
    pub to: String,
    pub condition: Option<String>,
}

/// Stable identifier for a graph-loop strategy implementation.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GraphLoopStrategyId(String);

impl GraphLoopStrategyId {
    /// Creates a graph-loop strategy identifier.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns the strategy identifier as a string slice.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Consumes the strategy identifier and returns its string.
    pub fn into_string(self) -> String {
        self.0
    }
}

impl From<String> for GraphLoopStrategyId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for GraphLoopStrategyId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// Stable version tag for a graph-loop strategy implementation.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GraphLoopStrategyVersion(String);

impl GraphLoopStrategyVersion {
    /// Creates a graph-loop strategy version.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns the strategy version as a string slice.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl From<String> for GraphLoopStrategyVersion {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for GraphLoopStrategyVersion {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// Stable digest for graph-loop strategy input or output payloads.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GraphPolicyDigest(String);

impl GraphPolicyDigest {
    /// Creates a graph policy digest.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns the digest as a string slice.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl From<String> for GraphPolicyDigest {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for GraphPolicyDigest {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// Execution plane used to compute a graph-loop policy proposal.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GraphLoopStrategyRuntime {
    /// Static Rust/TOML policy path used in deterministic hot-path execution.
    StaticPolicy,
    /// Native Scheme strategy reached through a typed C ABI adapter.
    NativeScheme,
    /// Native Gerbil strategy reached through a typed C ABI adapter.
    NativeGerbil,
}

impl GraphLoopStrategyRuntime {
    /// Returns true when this runtime represents an external native policy plane.
    pub fn is_native_policy_plane(&self) -> bool {
        matches!(self, Self::NativeScheme | Self::NativeGerbil)
    }
}

/// Strategy identity attached to a graph-loop policy proposal.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GraphLoopStrategy {
    pub strategy_id: GraphLoopStrategyId,
    pub version: GraphLoopStrategyVersion,
    pub runtime: GraphLoopStrategyRuntime,
}

impl GraphLoopStrategy {
    /// Creates a graph-loop strategy descriptor.
    pub fn new(
        strategy_id: impl Into<GraphLoopStrategyId>,
        version: impl Into<GraphLoopStrategyVersion>,
        runtime: GraphLoopStrategyRuntime,
    ) -> Self {
        Self {
            strategy_id: strategy_id.into(),
            version: version.into(),
            runtime,
        }
    }

    /// Creates a native Scheme strategy descriptor.
    pub fn native_scheme(
        strategy_id: impl Into<GraphLoopStrategyId>,
        version: impl Into<GraphLoopStrategyVersion>,
    ) -> Self {
        Self::new(strategy_id, version, GraphLoopStrategyRuntime::NativeScheme)
    }

    /// Creates a native Gerbil strategy descriptor.
    pub fn native_gerbil(
        strategy_id: impl Into<GraphLoopStrategyId>,
        version: impl Into<GraphLoopStrategyVersion>,
    ) -> Self {
        Self::new(strategy_id, version, GraphLoopStrategyRuntime::NativeGerbil)
    }

    /// Returns true when this strategy is computed outside the Rust hot path.
    pub fn is_native_policy_plane(&self) -> bool {
        self.runtime.is_native_policy_plane()
    }
}

/// Proposed graph-loop policy returned by a strategy plane before Rust validation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GraphPolicyProposal {
    pub schema_id: String,
    pub strategy: GraphLoopStrategy,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub native_abi: Option<GraphNativeAbiRequirement>,
    pub proposed_graph: LoopGraph,
    pub input_digest: GraphPolicyDigest,
    pub output_digest: GraphPolicyDigest,
    pub diagnostics: Vec<String>,
}

impl GraphPolicyProposal {
    /// Creates a graph-loop policy proposal using the current schema identifier.
    pub fn new(
        strategy: GraphLoopStrategy,
        proposed_graph: LoopGraph,
        input_digest: impl Into<GraphPolicyDigest>,
        output_digest: impl Into<GraphPolicyDigest>,
    ) -> Self {
        Self {
            schema_id: GRAPH_POLICY_PROPOSAL_SCHEMA_ID.to_string(),
            strategy,
            native_abi: None,
            proposed_graph,
            input_digest: input_digest.into(),
            output_digest: output_digest.into(),
            diagnostics: Vec::new(),
        }
    }

    /// Returns true when the proposal uses the current schema identifier.
    pub fn has_current_schema(&self) -> bool {
        self.schema_id == GRAPH_POLICY_PROPOSAL_SCHEMA_ID
    }

    /// Returns true when the proposal came from a native strategy plane.
    pub fn is_native_policy_plane(&self) -> bool {
        self.strategy.is_native_policy_plane()
    }

    /// Attaches the native ABI requirement proven by the strategy plane adapter.
    pub fn with_native_abi_requirement(mut self, native_abi: GraphNativeAbiRequirement) -> Self {
        self.native_abi = Some(native_abi);
        self
    }

    /// Adds one strategy diagnostic to the proposal.
    pub fn with_diagnostic(mut self, diagnostic: impl Into<String>) -> Self {
        self.diagnostics.push(diagnostic.into());
        self
    }

    /// Validates the proposal at the Rust protocol boundary before execution.
    pub fn validate(&self) -> GraphPolicyProposalValidationReport {
        validate_graph_policy_proposal(self)
    }
}

/// Validation status for a graph-loop policy proposal.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GraphPolicyProposalStatus {
    /// Rust accepted the proposal and may compile the proposed graph.
    Accepted,
    /// Rust rejected the proposal before runtime execution.
    Rejected,
}

/// Rust-side validation report for a graph-loop policy proposal.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GraphPolicyProposalValidationReport {
    pub schema_id: String,
    pub strategy_id: GraphLoopStrategyId,
    pub native_abi: Option<GraphNativeAbiRequirement>,
    pub status: GraphPolicyProposalStatus,
    pub selected_graph_id: Option<GraphId>,
    pub diagnostics: Vec<String>,
}

impl GraphPolicyProposalValidationReport {
    /// Returns true when Rust accepted the proposal for compilation.
    pub fn is_accepted(&self) -> bool {
        self.status == GraphPolicyProposalStatus::Accepted
    }
}

/// Rust-side validation receipt for a graph-loop policy proposal.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GraphPolicyProposalReceipt {
    pub schema_id: String,
    pub strategy_id: GraphLoopStrategyId,
    pub native_abi: Option<GraphNativeAbiRequirement>,
    pub status: GraphPolicyProposalStatus,
    pub selected_graph_id: Option<GraphId>,
    pub diagnostics: Vec<String>,
}

impl GraphPolicyProposalReceipt {
    /// Records that Rust accepted a graph-loop policy proposal.
    pub fn accepted(proposal: &GraphPolicyProposal) -> Self {
        Self::from_validation(&GraphPolicyProposalValidationReport::accepted(proposal))
    }

    /// Records that Rust rejected a graph-loop policy proposal.
    pub fn rejected(proposal: &GraphPolicyProposal, diagnostics: Vec<String>) -> Self {
        Self::from_validation(&GraphPolicyProposalValidationReport::rejected(
            proposal,
            diagnostics,
        ))
    }

    /// Converts a Rust validation report into the receipt persisted by callers.
    pub fn from_validation(report: &GraphPolicyProposalValidationReport) -> Self {
        Self {
            schema_id: report.schema_id.clone(),
            strategy_id: report.strategy_id.clone(),
            native_abi: report.native_abi.clone(),
            status: report.status.clone(),
            selected_graph_id: report.selected_graph_id.clone(),
            diagnostics: report.diagnostics.clone(),
        }
    }

    /// Validates and records a graph-loop policy proposal.
    pub fn validate(proposal: &GraphPolicyProposal) -> Self {
        Self::from_validation(&proposal.validate())
    }
}

impl GraphPolicyProposalValidationReport {
    fn accepted(proposal: &GraphPolicyProposal) -> Self {
        Self {
            schema_id: proposal.schema_id.clone(),
            strategy_id: proposal.strategy.strategy_id.clone(),
            native_abi: proposal.native_abi.clone(),
            status: GraphPolicyProposalStatus::Accepted,
            selected_graph_id: Some(GraphId::new(proposal.proposed_graph.graph_id.clone())),
            diagnostics: Vec::new(),
        }
    }

    fn rejected(proposal: &GraphPolicyProposal, diagnostics: Vec<String>) -> Self {
        Self {
            schema_id: proposal.schema_id.clone(),
            strategy_id: proposal.strategy.strategy_id.clone(),
            native_abi: proposal.native_abi.clone(),
            status: GraphPolicyProposalStatus::Rejected,
            selected_graph_id: None,
            diagnostics,
        }
    }
}

/// Validates a graph-loop policy proposal before Rust compiles or executes it.
pub fn validate_graph_policy_proposal(
    proposal: &GraphPolicyProposal,
) -> GraphPolicyProposalValidationReport {
    let mut diagnostics = Vec::new();
    if !proposal.has_current_schema() {
        diagnostics.push("graph_policy_proposal.schema_id_mismatch".to_string());
    }
    if proposal.strategy.strategy_id.as_str().trim().is_empty() {
        diagnostics.push("graph_policy_proposal.strategy_id_empty".to_string());
    }
    if proposal.strategy.version.as_str().trim().is_empty() {
        diagnostics.push("graph_policy_proposal.strategy_version_empty".to_string());
    }
    if proposal.input_digest.as_str().trim().is_empty() {
        diagnostics.push("graph_policy_proposal.input_digest_empty".to_string());
    }
    if proposal.output_digest.as_str().trim().is_empty() {
        diagnostics.push("graph_policy_proposal.output_digest_empty".to_string());
    }
    validate_graph_policy_proposal_native_abi(proposal, &mut diagnostics);
    validate_loop_graph_shape(&proposal.proposed_graph, &mut diagnostics);

    if diagnostics.is_empty() {
        GraphPolicyProposalValidationReport::accepted(proposal)
    } else {
        GraphPolicyProposalValidationReport::rejected(proposal, diagnostics)
    }
}

fn validate_graph_policy_proposal_native_abi(
    proposal: &GraphPolicyProposal,
    diagnostics: &mut Vec<String>,
) {
    if proposal.strategy.is_native_policy_plane() {
        if let Some(native_abi) = &proposal.native_abi {
            validate_graph_native_abi_requirement(native_abi, diagnostics);
        } else {
            diagnostics.push("graph_policy_proposal.native_abi_missing".to_string());
        }
    } else if proposal.native_abi.is_some() {
        diagnostics.push("graph_policy_proposal.native_abi_unexpected".to_string());
    }
}

fn validate_loop_graph_shape(graph: &LoopGraph, diagnostics: &mut Vec<String>) {
    if graph.graph_id.trim().is_empty() {
        diagnostics.push("graph_policy_proposal.graph_id_empty".to_string());
    }
    if graph.nodes.is_empty() {
        diagnostics.push("graph_policy_proposal.nodes_empty".to_string());
    }

    let mut node_ids = BTreeSet::new();
    for node in &graph.nodes {
        let node_id = node.id.trim();
        if node_id.is_empty() {
            diagnostics.push("graph_policy_proposal.node_id_empty".to_string());
            continue;
        }
        if !node_ids.insert(node_id.to_string()) {
            diagnostics.push(format!("graph_policy_proposal.node_id_duplicate:{node_id}"));
        }
        if node.executor.trim().is_empty() {
            diagnostics.push(format!(
                "graph_policy_proposal.node_executor_empty:{node_id}"
            ));
        }
    }

    for edge in &graph.edges {
        let from = edge.from.trim();
        let to = edge.to.trim();
        if from.is_empty() {
            diagnostics.push("graph_policy_proposal.edge_from_empty".to_string());
        } else if !node_ids.contains(from) {
            diagnostics.push(format!("graph_policy_proposal.edge_from_unknown:{from}"));
        }
        if to.is_empty() {
            diagnostics.push("graph_policy_proposal.edge_to_empty".to_string());
        } else if !node_ids.contains(to) {
            diagnostics.push(format!("graph_policy_proposal.edge_to_unknown:{to}"));
        }
    }

    if graph_has_cycle(&node_ids, &graph.edges) {
        diagnostics.push("graph_policy_proposal.graph_cycle_detected".to_string());
    }
}

fn graph_has_cycle(node_ids: &BTreeSet<String>, edges: &[LoopEdgeSpec]) -> bool {
    let mut adjacency = BTreeMap::<String, Vec<String>>::new();
    for node_id in node_ids {
        adjacency.entry(node_id.clone()).or_default();
    }
    for edge in edges {
        if node_ids.contains(edge.from.trim()) && node_ids.contains(edge.to.trim()) {
            adjacency
                .entry(edge.from.trim().to_string())
                .or_default()
                .push(edge.to.trim().to_string());
        }
    }

    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    node_ids.iter().any(|node_id| {
        graph_visit_has_cycle(node_id.as_str(), &adjacency, &mut visiting, &mut visited)
    })
}

fn graph_visit_has_cycle(
    node_id: &str,
    adjacency: &BTreeMap<String, Vec<String>>,
    visiting: &mut BTreeSet<String>,
    visited: &mut BTreeSet<String>,
) -> bool {
    if visited.contains(node_id) {
        return false;
    }
    if !visiting.insert(node_id.to_string()) {
        return true;
    }
    if let Some(next_nodes) = adjacency.get(node_id) {
        for next_node in next_nodes {
            if graph_visit_has_cycle(next_node, adjacency, visiting, visited) {
                return true;
            }
        }
    }
    visiting.remove(node_id);
    visited.insert(node_id.to_string());
    false
}

/// Stable view of a running graph loop for status and recovery.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimePlanSnapshot {
    pub run_id: String,
    pub graph_id: String,
    pub active_node: Option<String>,
}

/// Request to execute a compiled graph loop on a Tokio-backed runtime.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GraphLoopExecutionRequest {
    pub run_id: String,
    pub graph: LoopGraph,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget: Option<GraphLoopExecutionBudget>,
}

impl GraphLoopExecutionRequest {
    pub fn new(run_id: impl Into<String>, graph: LoopGraph) -> Self {
        Self {
            run_id: run_id.into(),
            graph,
            budget: None,
        }
    }

    /// Attaches an execution budget to the graph-loop request.
    pub fn with_budget(mut self, budget: GraphLoopExecutionBudget) -> Self {
        self.budget = Some(budget);
        self
    }
}

/// Terminal status for a graph-loop execution.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GraphLoopExecutionStatus {
    Completed,
    Cancelled,
    Failed,
}

/// Receipt returned when a graph-loop execution reaches a terminal status.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GraphLoopExecutionResult {
    pub status: GraphLoopExecutionStatus,
    pub snapshot: RuntimePlanSnapshot,
    pub visited_nodes: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub node_receipts: Vec<GraphNodeExecutionReceipt>,
    pub diagnostics: Vec<String>,
}

impl GraphLoopExecutionResult {
    pub fn completed(snapshot: RuntimePlanSnapshot, visited_nodes: Vec<String>) -> Self {
        Self {
            status: GraphLoopExecutionStatus::Completed,
            snapshot,
            visited_nodes,
            node_receipts: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    pub fn cancelled(snapshot: RuntimePlanSnapshot, visited_nodes: Vec<String>) -> Self {
        Self {
            status: GraphLoopExecutionStatus::Cancelled,
            snapshot,
            visited_nodes,
            node_receipts: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    pub fn failed(snapshot: RuntimePlanSnapshot, diagnostics: Vec<String>) -> Self {
        Self::failed_with_visited(snapshot, Vec::new(), diagnostics)
    }

    pub fn failed_with_visited(
        snapshot: RuntimePlanSnapshot,
        visited_nodes: Vec<String>,
        diagnostics: Vec<String>,
    ) -> Self {
        Self {
            status: GraphLoopExecutionStatus::Failed,
            snapshot,
            visited_nodes,
            node_receipts: Vec::new(),
            diagnostics,
        }
    }

    pub fn with_diagnostics(mut self, diagnostics: Vec<String>) -> Self {
        self.diagnostics = diagnostics;
        self
    }

    pub fn with_node_receipts(mut self, node_receipts: Vec<GraphNodeExecutionReceipt>) -> Self {
        self.node_receipts = node_receipts;
        self
    }
}

/// Stable graph-loop run identifier.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct RunId(String);

impl RunId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

impl From<String> for RunId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for RunId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// Stable compiled graph identifier.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct GraphId(String);

impl GraphId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

impl From<String> for GraphId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for GraphId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// Stable graph node identifier.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct NodeId(String);

impl NodeId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

impl From<String> for NodeId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for NodeId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// Named executor slot selected by a compiled graph node.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct ExecutorName(String);

impl ExecutorName {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

impl From<String> for ExecutorName {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for ExecutorName {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// Invocation passed from the graph-loop kernel to a node executor.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GraphNodeInvocation {
    pub run_id: RunId,
    pub graph_id: GraphId,
    pub node_id: NodeId,
    pub executor: ExecutorName,
    pub config: BTreeMap<String, String>,
}

impl GraphNodeInvocation {
    pub fn from_loop_node(run_id: RunId, graph_id: GraphId, node: &LoopNodeSpec) -> Self {
        Self {
            run_id,
            graph_id,
            node_id: NodeId::new(node.id.clone()),
            executor: ExecutorName::new(node.executor.clone()),
            config: node.config.clone(),
        }
    }
}

/// Terminal status for one graph node executor invocation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GraphNodeExecutionStatus {
    Completed,
    Failed,
}

/// Receipt returned by a graph node executor.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GraphNodeExecutionReceipt {
    pub status: GraphNodeExecutionStatus,
    pub node_id: NodeId,
    pub executor: ExecutorName,
    pub diagnostics: Vec<String>,
}

impl GraphNodeExecutionReceipt {
    pub fn completed(node_id: impl Into<NodeId>, executor: impl Into<ExecutorName>) -> Self {
        Self {
            status: GraphNodeExecutionStatus::Completed,
            node_id: node_id.into(),
            executor: executor.into(),
            diagnostics: Vec::new(),
        }
    }

    pub fn failed(
        node_id: impl Into<NodeId>,
        executor: impl Into<ExecutorName>,
        diagnostics: Vec<String>,
    ) -> Self {
        Self {
            status: GraphNodeExecutionStatus::Failed,
            node_id: node_id.into(),
            executor: executor.into(),
            diagnostics,
        }
    }
}
