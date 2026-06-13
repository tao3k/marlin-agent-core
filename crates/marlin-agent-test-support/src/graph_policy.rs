//! No-LLM fixtures for graph policy proposal compilation.

use std::collections::BTreeMap;

use marlin_agent_kernel::{GraphPolicyProposalCompilation, compile_graph_policy_proposal};
use marlin_agent_protocol::{
    AgentTraceSpanRecord, GERBIL_LOOP_GRAPH_POLICY_COMPILATION_SCHEMA_ID,
    GRAPH_POLICY_PROPOSAL_SPAN_NAME, GRAPH_POLICY_PROPOSAL_VISIBILITY_SUBJECT_PREFIX,
    GerbilLoopGraphPolicyCompilationRequest, GraphLoopExecutionBudget, GraphLoopExecutionRequest,
    GraphLoopStrategy, GraphLoopStrategyRuntime, GraphPolicyProposal, GraphPolicyProposalStatus,
    LoopEvidence, LoopEvidenceKind, LoopGraph, LoopNodeSpec, compile_gerbil_loop_graph_policy,
};

const ACCEPTED_RUN_ID: &str = "test-support/graph-policy/accepted";
const GERBIL_IR_RUN_ID: &str = "test-support/graph-policy/gerbil-ir";
const REJECTED_RUN_ID: &str = "test-support/graph-policy/rejected";

/// Deterministic graph policy proposal fixture compiled through the Rust gate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeterministicGraphPolicyProposalFixture {
    proposal: GraphPolicyProposal,
    compilation: GraphPolicyProposalCompilation,
    expected_run_id: String,
}

impl DeterministicGraphPolicyProposalFixture {
    /// Native strategy proposal returned by the fixture.
    pub fn proposal(&self) -> &GraphPolicyProposal {
        &self.proposal
    }

    /// Rust compilation result for the proposal.
    pub fn compilation(&self) -> &GraphPolicyProposalCompilation {
        &self.compilation
    }

    /// Run id expected when the accepted proposal produces an execution request.
    pub fn expected_run_id(&self) -> &str {
        self.expected_run_id.as_str()
    }

    /// Trace span exposing the Rust validation and compilation receipt.
    pub fn trace_span(&self) -> AgentTraceSpanRecord {
        AgentTraceSpanRecord::graph_policy_proposal_receipt(&self.compilation.receipt)
    }

    /// Visibility evidence projected from the Rust proposal compilation receipt span.
    pub fn visibility_evidence(&self) -> LoopEvidence {
        self.trace_span()
            .graph_policy_proposal_visibility_evidence()
            .expect("graph policy proposal trace span should project visibility evidence")
    }

    /// Stable span name expected for graph policy proposal visibility.
    pub fn expected_trace_span_name(&self) -> &str {
        GRAPH_POLICY_PROPOSAL_SPAN_NAME
    }
}

/// Accepted fixture proving native strategy proposals compile through Rust.
pub fn accepted_graph_policy_proposal_fixture() -> DeterministicGraphPolicyProposalFixture {
    let proposal = GraphPolicyProposal::new(
        GraphLoopStrategy::native_scheme("test-support-scheme-loop-ranker", "v1"),
        LoopGraph {
            graph_id: "test-support-graph".to_owned(),
            nodes: vec![LoopNodeSpec {
                id: "provider-stream".to_owned(),
                executor: "provider.stream".to_owned(),
                config: BTreeMap::from([("budget".to_owned(), "bounded".to_owned())]),
            }],
            edges: Vec::new(),
        },
        "sha256:test-support-input",
        "sha256:test-support-output",
    );
    let compilation = compile_graph_policy_proposal(ACCEPTED_RUN_ID, &proposal);

    DeterministicGraphPolicyProposalFixture {
        proposal,
        compilation,
        expected_run_id: ACCEPTED_RUN_ID.to_owned(),
    }
}

/// Accepted fixture proving Gerbil loop graph `IR` compiles through the Rust gate.
pub fn accepted_gerbil_ir_graph_policy_proposal_fixture() -> DeterministicGraphPolicyProposalFixture
{
    let proposal = compile_gerbil_loop_graph_policy(
        GerbilLoopGraphPolicyCompilationRequest::new(
            GraphLoopStrategy::native_gerbil("test-support-gerbil-ir-loop-ranker", "v1"),
            gerbil_ir_loop_graph(),
            "sha256:test-support-gerbil-ir-input",
            "sha256:test-support-gerbil-ir-output",
        )
        .with_diagnostic(GERBIL_LOOP_GRAPH_POLICY_COMPILATION_SCHEMA_ID),
    );
    let compilation = compile_graph_policy_proposal(GERBIL_IR_RUN_ID, &proposal);

    DeterministicGraphPolicyProposalFixture {
        proposal,
        compilation,
        expected_run_id: GERBIL_IR_RUN_ID.to_owned(),
    }
}

fn gerbil_ir_loop_graph() -> marlin_gerbil_ir::CompiledLoopGraph {
    marlin_gerbil_ir::CompiledLoopGraph {
        graph_id: "test-support-gerbil-ir-graph".to_owned(),
        nodes: vec![
            marlin_gerbil_ir::LoopNodeSpec {
                id: "rank".to_owned(),
                executor: "gerbil.rank".to_owned(),
                config: BTreeMap::from([("policy".to_owned(), "native".to_owned())]),
            },
            marlin_gerbil_ir::LoopNodeSpec {
                id: "dispatch".to_owned(),
                executor: "kernel.dispatch".to_owned(),
                config: BTreeMap::new(),
            },
        ],
        edges: vec![marlin_gerbil_ir::LoopEdgeSpec {
            from: "rank".to_owned(),
            to: "dispatch".to_owned(),
            condition: Some("always".to_owned()),
        }],
    }
}

/// Rejected fixture proving invalid native strategy proposals stop before execution.
pub fn rejected_graph_policy_proposal_fixture() -> DeterministicGraphPolicyProposalFixture {
    let proposal = GraphPolicyProposal::new(
        GraphLoopStrategy::new(
            "test-support-gerbil-loop-ranker",
            "v1",
            GraphLoopStrategyRuntime::NativeGerbil,
        ),
        LoopGraph {
            graph_id: "test-support-rejected-graph".to_owned(),
            nodes: Vec::new(),
            edges: Vec::new(),
        },
        "sha256:test-support-input",
        "sha256:test-support-output",
    );
    let compilation = compile_graph_policy_proposal(REJECTED_RUN_ID, &proposal);

    DeterministicGraphPolicyProposalFixture {
        proposal,
        compilation,
        expected_run_id: REJECTED_RUN_ID.to_owned(),
    }
}

/// Assert the accepted proposal fixture stays on the Rust compilation path.
pub fn assert_accepted_graph_policy_proposal_fixture(
    fixture: &DeterministicGraphPolicyProposalFixture,
) {
    assert_accepted_proposal_runtime(fixture);
    assert_accepted_compilation_receipt(fixture);
    assert_accepted_execution_request(fixture);
    assert_accepted_trace_span(fixture);
    assert_accepted_visibility_evidence(fixture);
}

/// Assert the accepted Gerbil `IR` proposal fixture stays on the Rust compilation path.
pub fn assert_accepted_gerbil_ir_graph_policy_proposal_fixture(
    fixture: &DeterministicGraphPolicyProposalFixture,
) {
    assert!(fixture.proposal.is_native_policy_plane());
    assert_eq!(
        fixture.proposal.strategy.runtime,
        GraphLoopStrategyRuntime::NativeGerbil
    );
    assert_eq!(
        fixture.proposal.diagnostics,
        vec![GERBIL_LOOP_GRAPH_POLICY_COMPILATION_SCHEMA_ID.to_owned()]
    );
    assert_accepted_compilation_receipt(fixture);
    assert_accepted_execution_request(fixture);
    assert_accepted_trace_span(fixture);
    assert_accepted_visibility_evidence(fixture);

    let request = fixture
        .compilation
        .request
        .as_ref()
        .expect("accepted Gerbil IR proposal should produce an execution request");
    assert_eq!(request.graph.nodes[0].executor, "gerbil.rank");
    assert_eq!(request.graph.edges[0].condition.as_deref(), Some("always"));
}

fn assert_accepted_proposal_runtime(fixture: &DeterministicGraphPolicyProposalFixture) {
    assert!(fixture.proposal.is_native_policy_plane());
    assert_eq!(
        fixture.proposal.strategy.runtime,
        GraphLoopStrategyRuntime::NativeScheme
    );
}

fn assert_accepted_compilation_receipt(fixture: &DeterministicGraphPolicyProposalFixture) {
    assert!(fixture.compilation.is_accepted());
    assert_eq!(
        fixture.compilation.receipt.status,
        GraphPolicyProposalStatus::Accepted
    );
    assert!(fixture.compilation.receipt.diagnostics.is_empty());
}

fn assert_accepted_execution_request(fixture: &DeterministicGraphPolicyProposalFixture) {
    let request = fixture
        .compilation
        .request
        .as_ref()
        .expect("accepted proposal should produce an execution request");
    assert_eq!(request.run_id, fixture.expected_run_id);
    assert_eq!(
        request.graph.graph_id,
        fixture.proposal.proposed_graph.graph_id
    );
}

fn assert_accepted_trace_span(fixture: &DeterministicGraphPolicyProposalFixture) {
    let span = fixture.trace_span();
    assert_eq!(span.name.as_str(), fixture.expected_trace_span_name());
    assert_eq!(
        span.fields.get("status").map(String::as_str),
        Some("Accepted")
    );
    assert_eq!(
        span.fields.get("selected_graph_id").map(String::as_str),
        Some(fixture.proposal.proposed_graph.graph_id.as_str())
    );
}

fn assert_accepted_visibility_evidence(fixture: &DeterministicGraphPolicyProposalFixture) {
    let evidence = fixture.visibility_evidence();
    let detail = visibility_detail(&evidence, "accepted");

    assert_graph_policy_visibility_evidence_shape(&evidence, fixture);
    assert!(detail.contains("status=Accepted"));
    assert!(detail.contains("diagnostic_count=0"));
    assert!(detail.contains(&format!(
        "selected_graph_id={}",
        fixture.proposal.proposed_graph.graph_id
    )));
}

/// Assert the rejected proposal fixture does not produce an execution request.
pub fn assert_rejected_graph_policy_proposal_fixture(
    fixture: &DeterministicGraphPolicyProposalFixture,
) {
    assert_rejected_proposal_runtime(fixture);
    assert_rejected_compilation_receipt(fixture);
    assert_rejected_trace_span(fixture);
    assert_rejected_visibility_evidence(fixture);
}

fn assert_rejected_proposal_runtime(fixture: &DeterministicGraphPolicyProposalFixture) {
    assert!(fixture.proposal.is_native_policy_plane());
    assert_eq!(
        fixture.proposal.strategy.runtime,
        GraphLoopStrategyRuntime::NativeGerbil
    );
}

fn assert_rejected_compilation_receipt(fixture: &DeterministicGraphPolicyProposalFixture) {
    assert!(!fixture.compilation.is_accepted());
    assert_eq!(
        fixture.compilation.receipt.status,
        GraphPolicyProposalStatus::Rejected
    );
    assert!(fixture.compilation.request.is_none());
    assert!(
        fixture
            .compilation
            .receipt
            .diagnostics
            .contains(&"graph_policy_proposal.nodes_empty".to_owned())
    );
}

fn assert_rejected_trace_span(fixture: &DeterministicGraphPolicyProposalFixture) {
    let span = fixture.trace_span();
    assert_eq!(span.name.as_str(), fixture.expected_trace_span_name());
    assert_eq!(
        span.fields.get("status").map(String::as_str),
        Some("Rejected")
    );
    assert_eq!(
        span.fields.get("diagnostic_count").map(String::as_str),
        Some("1")
    );
    assert!(!span.fields.contains_key("selected_graph_id"));
}

fn assert_rejected_visibility_evidence(fixture: &DeterministicGraphPolicyProposalFixture) {
    let evidence = fixture.visibility_evidence();
    let detail = visibility_detail(&evidence, "rejected");

    assert_graph_policy_visibility_evidence_shape(&evidence, fixture);
    assert!(detail.contains("status=Rejected"));
    assert!(detail.contains("diagnostic_count=1"));
    assert!(!detail.contains("selected_graph_id="));
}

fn assert_graph_policy_visibility_evidence_shape(
    evidence: &LoopEvidence,
    fixture: &DeterministicGraphPolicyProposalFixture,
) {
    assert_eq!(evidence.kind, LoopEvidenceKind::Visibility);
    assert_eq!(evidence.subject, expected_visibility_subject(fixture));
    assert!(evidence.present);
}

fn expected_visibility_subject(fixture: &DeterministicGraphPolicyProposalFixture) -> String {
    format!(
        "{}:{}",
        GRAPH_POLICY_PROPOSAL_VISIBILITY_SUBJECT_PREFIX,
        fixture.proposal.strategy.strategy_id.as_str()
    )
}

fn visibility_detail<'a>(evidence: &'a LoopEvidence, label: &str) -> &'a str {
    evidence
        .detail
        .as_deref()
        .unwrap_or_else(|| panic!("{label} visibility evidence detail"))
}

/// Builds a budgeted execution request from an accepted graph-policy fixture.
pub fn budgeted_graph_policy_execution_request_fixture(
    fixture: &DeterministicGraphPolicyProposalFixture,
    max_node_executions: u64,
) -> GraphLoopExecutionRequest {
    fixture
        .compilation
        .request
        .clone()
        .expect("accepted proposal should produce an execution request")
        .with_budget(GraphLoopExecutionBudget::max_node_executions(
            max_node_executions,
        ))
}

/// Assert a budgeted execution request records the expected runtime budget.
pub fn assert_budgeted_graph_policy_execution_request(
    request: &GraphLoopExecutionRequest,
    max_node_executions: u64,
) {
    assert_eq!(
        request
            .budget
            .as_ref()
            .and_then(|budget| budget.max_node_executions),
        Some(max_node_executions)
    );
}
