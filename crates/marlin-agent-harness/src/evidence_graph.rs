//! Harness-owned evidence graph projections for scenario and runtime reports.

use marlin_agent_protocol::{
    GraphLoopExecutionResult, GraphLoopExecutionStatus, GraphNodeExecutionStatus,
    GraphPolicyProposalReceipt, GraphPolicyProposalStatus,
};
use serde::{Deserialize, Serialize};

use crate::{HarnessEvidence, HarnessEvidenceKind};

/// Schema identifier for evidence graphs emitted by the harness boundary.
pub const HARNESS_EVIDENCE_GRAPH_SCHEMA_ID: &str = "marlin.agent.harness_evidence_graph.v1";

/// Node category inside a harness evidence graph.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum HarnessEvidenceGraphNodeKind {
    HumanIntent,
    TypeInvariant,
    TestBehavior,
    ProofResult,
    Counterexample,
    ReviewJudgment,
    ExecutionReceipt,
    EvidenceFact,
}

/// Relationship between two harness evidence graph nodes.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum HarnessEvidenceGraphEdgeKind {
    Requires,
    Supports,
    Checks,
    Proves,
    Refutes,
    Reviews,
    Observes,
}

/// One typed node in a harness evidence graph.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct HarnessEvidenceGraphNode {
    pub id: String,
    pub kind: HarnessEvidenceGraphNodeKind,
    pub subject: String,
    pub present: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_evidence_kind: Option<HarnessEvidenceKind>,
}

impl HarnessEvidenceGraphNode {
    /// Creates a present graph node.
    pub fn present(
        id: impl Into<String>,
        kind: HarnessEvidenceGraphNodeKind,
        subject: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            kind,
            subject: subject.into(),
            present: true,
            detail: None,
            source_evidence_kind: None,
        }
    }

    /// Creates a missing graph node.
    pub fn missing(
        id: impl Into<String>,
        kind: HarnessEvidenceGraphNodeKind,
        subject: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            kind,
            subject: subject.into(),
            present: false,
            detail: None,
            source_evidence_kind: None,
        }
    }

    /// Attaches a human-readable detail or receipt summary.
    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// Records the harness evidence kind this node was projected from.
    pub fn with_source_evidence_kind(mut self, kind: HarnessEvidenceKind) -> Self {
        self.source_evidence_kind = Some(kind);
        self
    }

    /// Projects one harness evidence fact into a harness graph node.
    pub fn from_harness_evidence(index: usize, evidence: &HarnessEvidence) -> Self {
        let node = if evidence.present {
            Self::present(
                format!("evidence:{index}"),
                harness_evidence_graph_node_kind(&evidence.kind),
                evidence.subject.clone(),
            )
        } else {
            Self::missing(
                format!("evidence:{index}"),
                harness_evidence_graph_node_kind(&evidence.kind),
                evidence.subject.clone(),
            )
        };

        let node = node.with_source_evidence_kind(evidence.kind.clone());
        if let Some(detail) = evidence.detail.clone() {
            node.with_detail(detail)
        } else {
            node
        }
    }
}

/// One typed relationship in a harness evidence graph.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct HarnessEvidenceGraphEdge {
    pub from: String,
    pub to: String,
    pub kind: HarnessEvidenceGraphEdgeKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl HarnessEvidenceGraphEdge {
    /// Creates a typed edge between two evidence graph nodes.
    pub fn new(
        from: impl Into<String>,
        to: impl Into<String>,
        kind: HarnessEvidenceGraphEdgeKind,
    ) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            kind,
            detail: None,
        }
    }

    /// Attaches a short reason for this evidence relationship.
    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }
}

/// Compact receipt for evidence graph shape and risk-bearing node counts.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct HarnessEvidenceGraphSummary {
    pub nodes: usize,
    pub edges: usize,
    pub missing_nodes: usize,
    pub counterexamples: usize,
    pub review_judgments: usize,
}

/// Evolvable evidence graph for one harness scenario or graph-loop run.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct HarnessEvidenceGraph {
    #[serde(default = "default_harness_evidence_graph_schema_id")]
    pub schema_id: String,
    pub graph_id: String,
    pub nodes: Vec<HarnessEvidenceGraphNode>,
    pub edges: Vec<HarnessEvidenceGraphEdge>,
}

impl HarnessEvidenceGraph {
    /// Creates an empty evidence graph.
    pub fn new(graph_id: impl Into<String>) -> Self {
        Self {
            schema_id: HARNESS_EVIDENCE_GRAPH_SCHEMA_ID.to_owned(),
            graph_id: graph_id.into(),
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }

    /// Projects a list of harness evidence facts into typed graph nodes.
    pub fn from_harness_evidence(
        graph_id: impl Into<String>,
        evidence: &[HarnessEvidence],
    ) -> Self {
        let nodes = evidence
            .iter()
            .enumerate()
            .map(|(index, evidence)| {
                HarnessEvidenceGraphNode::from_harness_evidence(index, evidence)
            })
            .collect();

        Self {
            schema_id: HARNESS_EVIDENCE_GRAPH_SCHEMA_ID.to_owned(),
            graph_id: graph_id.into(),
            nodes,
            edges: Vec::new(),
        }
    }

    /// Projects a graph-policy proposal validation receipt into a harness evidence graph.
    pub fn from_graph_policy_proposal_receipt(
        graph_id: impl Into<String>,
        receipt: &GraphPolicyProposalReceipt,
    ) -> Self {
        Self::new(graph_id).with_graph_policy_proposal_receipt(receipt)
    }

    /// Adds graph-policy proposal validation evidence to this graph.
    pub fn with_graph_policy_proposal_receipt(
        mut self,
        receipt: &GraphPolicyProposalReceipt,
    ) -> Self {
        let strategy_id = receipt.strategy_id.as_str();
        let invariant_id = format!("invariant:graph-policy-proposal:{strategy_id}");
        let review_id = format!("review:graph-policy-proposal:{strategy_id}");
        let selected_graph_id = receipt
            .selected_graph_id
            .as_ref()
            .map(|graph_id| graph_id.as_str())
            .unwrap_or("none");
        let status = graph_policy_proposal_status_label(&receipt.status);
        let detail = format!(
            "schema_id={} status={} selected_graph_id={} native_abi={} diagnostic_count={}",
            receipt.schema_id,
            status,
            selected_graph_id,
            receipt.native_abi.is_some(),
            receipt.diagnostics.len(),
        );

        self.nodes.push(
            HarnessEvidenceGraphNode::present(
                invariant_id.clone(),
                HarnessEvidenceGraphNodeKind::TypeInvariant,
                format!("graph-policy-proposal:{strategy_id}"),
            )
            .with_detail("proposal schema, native ABI, graph shape, and digest invariants"),
        );
        self.nodes.push(
            HarnessEvidenceGraphNode::present(
                review_id.clone(),
                HarnessEvidenceGraphNodeKind::ReviewJudgment,
                format!("rust-validation:{strategy_id}"),
            )
            .with_detail(detail),
        );
        self.edges.push(
            HarnessEvidenceGraphEdge::new(
                invariant_id.clone(),
                review_id.clone(),
                HarnessEvidenceGraphEdgeKind::Checks,
            )
            .with_detail("Rust validates the proposed graph-loop policy before execution"),
        );

        match receipt.status {
            GraphPolicyProposalStatus::Accepted => {
                let proof_id = format!("proof:graph-policy-proposal:{strategy_id}");
                self.nodes.push(
                    HarnessEvidenceGraphNode::present(
                        proof_id.clone(),
                        HarnessEvidenceGraphNodeKind::ProofResult,
                        format!("graph-policy-accepted:{strategy_id}"),
                    )
                    .with_detail(format!(
                        "selected_graph_id={selected_graph_id} diagnostic_count=0"
                    )),
                );
                self.edges.push(
                    HarnessEvidenceGraphEdge::new(
                        review_id,
                        proof_id,
                        HarnessEvidenceGraphEdgeKind::Proves,
                    )
                    .with_detail("accepted validation proves the proposal is executable"),
                );
            }
            GraphPolicyProposalStatus::Rejected => {
                let counterexample_id =
                    format!("counterexample:graph-policy-proposal:{strategy_id}");
                self.nodes.push(
                    HarnessEvidenceGraphNode::present(
                        counterexample_id.clone(),
                        HarnessEvidenceGraphNodeKind::Counterexample,
                        format!("graph-policy-rejected:{strategy_id}"),
                    )
                    .with_detail(format!("diagnostics={}", receipt.diagnostics.join(","))),
                );
                self.edges.push(
                    HarnessEvidenceGraphEdge::new(
                        counterexample_id,
                        invariant_id,
                        HarnessEvidenceGraphEdgeKind::Refutes,
                    )
                    .with_detail("rejected validation is a counterexample to proposal readiness"),
                );
            }
        }

        self
    }

    /// Adds graph-loop execution result evidence to this graph.
    pub fn with_graph_execution_result(mut self, result: &GraphLoopExecutionResult) -> Self {
        let run_id = result.snapshot.run_id.as_str();
        let graph_id = result.snapshot.graph_id.as_str();
        let execution_id = format!("execution:graph-loop:{run_id}");
        let status = graph_loop_execution_status_label(&result.status);

        self.nodes.push(
            HarnessEvidenceGraphNode::present(
                execution_id.clone(),
                HarnessEvidenceGraphNodeKind::ExecutionReceipt,
                format!("graph-loop:{run_id}:{graph_id}"),
            )
            .with_detail(format!(
                "status={} visited_nodes={} node_receipts={} diagnostic_count={}",
                status,
                result.visited_nodes.len(),
                result.node_receipts.len(),
                result.diagnostics.len(),
            )),
        );

        for (index, receipt) in result.node_receipts.iter().enumerate() {
            let node_receipt_id = format!(
                "execution:graph-loop:{run_id}:node:{index}:{}",
                receipt.node_id.as_str()
            );
            let node_status = graph_node_execution_status_label(&receipt.status);
            self.nodes.push(
                HarnessEvidenceGraphNode::present(
                    node_receipt_id.clone(),
                    HarnessEvidenceGraphNodeKind::ExecutionReceipt,
                    format!("graph-loop-node:{run_id}:{}", receipt.node_id.as_str()),
                )
                .with_detail(format!(
                    "node_id={} executor={} status={} diagnostic_count={}",
                    receipt.node_id.as_str(),
                    receipt.executor.as_str(),
                    node_status,
                    receipt.diagnostics.len()
                )),
            );
            self.edges.push(
                HarnessEvidenceGraphEdge::new(
                    execution_id.clone(),
                    node_receipt_id,
                    HarnessEvidenceGraphEdgeKind::Observes,
                )
                .with_detail("graph-loop execution observes this node receipt"),
            );
        }

        match result.status {
            GraphLoopExecutionStatus::Completed => {
                let proof_id = format!("proof:graph-loop-completed:{run_id}");
                self.nodes.push(
                    HarnessEvidenceGraphNode::present(
                        proof_id.clone(),
                        HarnessEvidenceGraphNodeKind::ProofResult,
                        format!("graph-loop-completed:{run_id}"),
                    )
                    .with_detail(format!("visited_nodes={}", result.visited_nodes.join(","))),
                );
                self.edges.push(
                    HarnessEvidenceGraphEdge::new(
                        execution_id,
                        proof_id,
                        HarnessEvidenceGraphEdgeKind::Observes,
                    )
                    .with_detail("completed execution observes the accepted runtime behavior"),
                );
            }
            GraphLoopExecutionStatus::Cancelled | GraphLoopExecutionStatus::Failed => {
                let counterexample_id = format!("counterexample:graph-loop:{run_id}");
                self.nodes.push(
                    HarnessEvidenceGraphNode::present(
                        counterexample_id.clone(),
                        HarnessEvidenceGraphNodeKind::Counterexample,
                        format!("graph-loop-{status}:{run_id}"),
                    )
                    .with_detail(format!("diagnostics={}", result.diagnostics.join(","))),
                );
                self.edges.push(
                    HarnessEvidenceGraphEdge::new(
                        counterexample_id,
                        execution_id,
                        HarnessEvidenceGraphEdgeKind::Refutes,
                    )
                    .with_detail("failed or cancelled execution is a counterexample to readiness"),
                );
            }
        }

        self
    }

    /// Adds one typed evidence graph node.
    pub fn with_node(mut self, node: HarnessEvidenceGraphNode) -> Self {
        self.nodes.push(node);
        self
    }

    /// Adds one typed evidence graph edge.
    pub fn with_edge(mut self, edge: HarnessEvidenceGraphEdge) -> Self {
        self.edges.push(edge);
        self
    }

    /// Returns true when the graph contains at least one node with the requested kind.
    pub fn has_node_kind(&self, kind: HarnessEvidenceGraphNodeKind) -> bool {
        self.nodes.iter().any(|node| node.kind == kind)
    }

    /// Builds a compact graph-shape receipt.
    pub fn summary(&self) -> HarnessEvidenceGraphSummary {
        HarnessEvidenceGraphSummary {
            nodes: self.nodes.len(),
            edges: self.edges.len(),
            missing_nodes: self.nodes.iter().filter(|node| !node.present).count(),
            counterexamples: self
                .nodes
                .iter()
                .filter(|node| node.kind == HarnessEvidenceGraphNodeKind::Counterexample)
                .count(),
            review_judgments: self
                .nodes
                .iter()
                .filter(|node| node.kind == HarnessEvidenceGraphNodeKind::ReviewJudgment)
                .count(),
        }
    }
}

fn graph_policy_proposal_status_label(status: &GraphPolicyProposalStatus) -> &'static str {
    match status {
        GraphPolicyProposalStatus::Accepted => "Accepted",
        GraphPolicyProposalStatus::Rejected => "Rejected",
    }
}

fn graph_loop_execution_status_label(status: &GraphLoopExecutionStatus) -> &'static str {
    match status {
        GraphLoopExecutionStatus::Completed => "Completed",
        GraphLoopExecutionStatus::Cancelled => "Cancelled",
        GraphLoopExecutionStatus::Failed => "Failed",
    }
}

fn graph_node_execution_status_label(status: &GraphNodeExecutionStatus) -> &'static str {
    match status {
        GraphNodeExecutionStatus::Completed => "Completed",
        GraphNodeExecutionStatus::Failed => "Failed",
    }
}

fn harness_evidence_graph_node_kind(kind: &HarnessEvidenceKind) -> HarnessEvidenceGraphNodeKind {
    match kind {
        HarnessEvidenceKind::RunLog
        | HarnessEvidenceKind::Workflow
        | HarnessEvidenceKind::Runtime => HarnessEvidenceGraphNodeKind::ExecutionReceipt,
        HarnessEvidenceKind::Content
        | HarnessEvidenceKind::Safety
        | HarnessEvidenceKind::Budget
        | HarnessEvidenceKind::Registry
        | HarnessEvidenceKind::Provider
        | HarnessEvidenceKind::Tool
        | HarnessEvidenceKind::SubAgent
        | HarnessEvidenceKind::Visibility
        | HarnessEvidenceKind::Performance
        | HarnessEvidenceKind::Stability => HarnessEvidenceGraphNodeKind::EvidenceFact,
    }
}

fn default_harness_evidence_graph_schema_id() -> String {
    HARNESS_EVIDENCE_GRAPH_SCHEMA_ID.to_owned()
}
