use std::{collections::BTreeMap, path::PathBuf};

use marlin_agent_harness::{AgentHarness, HarnessRuntime};
use marlin_agent_kernel::{
    GraphLoopExecutionStatus, GraphNodeExecutionReceipt, GraphNodeExecutor, GraphNodeInvocation,
    TokioGraphLoopKernel, compile_graph_policy_proposal,
};
use marlin_agent_protocol::{
    GERBIL_LOOP_GRAPH_POLICY_COMPILATION_SCHEMA_ID, GerbilLoopGraphPolicyCompilationRequest,
    GraphLoopExecutionBudget, GraphLoopExecutionRequest, GraphLoopStrategy, LoopEdgeSpec,
    LoopEvidence, LoopEvidenceKind, LoopGraph, LoopNodeSpec, compile_gerbil_loop_graph_policy,
};
use marlin_agent_runtime::{RuntimeContext, RuntimeFuture, observability};
use marlin_gerbil_ir::CompiledLoopGraph;
use marlin_gerbil_scheme::{
    GerbilArtifactKind, GerbilCompiledArtifact, GerbilCompiler, GerbilRuntimeBinding, GerbilSource,
    MARLIN_GERBIL_GXI_ENV, default_gerbil_gxi_program,
};
use tempfile::Builder;

const MARLIN_REQUIRE_REAL_GXI_ENV: &str = "MARLIN_REQUIRE_REAL_GXI";

const HARNESS_LOOP_GRAPH_SOURCE: &str = r#"(loop-graph gerbil-harness-loop
  (node rank gerbil-rank (config policy native))
  (node dispatch kernel-dispatch (config mode deterministic))
  (edge rank dispatch always))"#;

const HARNESS_COMPLEX_POLICY_GRAPH_SOURCE: &str = r#"(loop-graph gerbil-complex-policy
  (node rank gerbil-rank (config policy native complexity complex))
  (node budget-check policy-budget (config max-steps "4" gate strict))
  (node dispatch kernel-dispatch (config mode deterministic))
  (node audit policy-audit (config receipt visibility))
  (edge rank budget-check always)
  (edge budget-check dispatch always)
  (edge dispatch audit always))"#;

const HARNESS_SCENARIO_CONTRACT_SOURCE: &str = r#"(agent-scenario-contract gerbil-harness-scenario
  (description "real gxi into harness")
  (step run
    (event-topic kernel.execution)
    (span-name harness.execution))
  (evidence Runtime))"#;

#[tokio::test]
#[ignore = "requires a local Gerbil gxi executable"]
async fn harness_executes_real_gxi_loop_graph_against_real_gxi_scenario_contract() {
    let Some(gxi) = local_gxi() else {
        return;
    };
    let root = Builder::new()
        .prefix("marlin-agent-harness-real-gxi-")
        .tempdir()
        .expect("real gxi harness tempdir");
    let binding =
        GerbilRuntimeBinding::new(gxi, root.path()).expect("write Gerbil runtime binding assets");

    let graph_artifact = compile_artifact(
        &binding,
        "audit/harness-loop-graph",
        HARNESS_LOOP_GRAPH_SOURCE,
        GerbilArtifactKind::LoopGraph,
    );
    let contract_artifact = compile_artifact(
        &binding,
        "audit/harness-scenario",
        HARNESS_SCENARIO_CONTRACT_SOURCE,
        GerbilArtifactKind::AgentScenarioContract,
    );

    let GerbilCompiledArtifact::LoopGraph(compiled_graph) = graph_artifact else {
        panic!("expected real gxi loop graph artifact");
    };
    let GerbilCompiledArtifact::AgentScenarioContract(contract) = contract_artifact else {
        panic!("expected real gxi agent scenario contract artifact");
    };
    assert!(contract.is_supported_schema());

    let graph = compiled_loop_graph_into_protocol(compiled_graph);
    assert_eq!(graph.graph_id, "gerbil-harness-loop");
    let request = GraphLoopExecutionRequest::new("real-gxi-harness-run", graph.clone())
        .with_budget(GraphLoopExecutionBudget::max_node_executions(2));
    let kernel = TokioGraphLoopKernel::new("real-gxi-harness-run", graph.graph_id.clone())
        .with_executor("gerbil-rank", RealGxiHarnessExecutor)
        .with_executor("kernel-dispatch", RealGxiHarnessExecutor);
    let mut harness = HarnessRuntime::new(64);
    harness.record_evidence(
        LoopEvidence::present(LoopEvidenceKind::Runtime, "real-gxi-harness")
            .with_detail("source=gerbil artifact=loop_graph+agent_scenario_contract"),
    );

    let report = harness
        .execute_graph(&contract.scenario, &kernel, request)
        .await;
    let evaluated = AgentHarness::evaluate_contract_execution_report(&contract, &report);

    assert_eq!(
        report.result.status,
        GraphLoopExecutionStatus::Completed,
        "real gxi complex policy diagnostics: {:?}",
        report.result.diagnostics
    );
    assert_eq!(report.result.visited_nodes, vec!["rank", "dispatch"]);
    assert!(report.has_event_topic(&observability::kernel_execution_topic()));
    assert!(report.has_span(&observability::harness_execution_span_name()));
    assert!(report.assertion.is_none());
    assert!(
        evaluated.is_success(),
        "real gxi harness contract failed: {:?}",
        evaluated.diagnostics
    );
}

#[tokio::test]
#[ignore = "requires a local Gerbil gxi executable"]
async fn harness_compiles_real_gxi_complex_graph_policy_into_rust_receipt() {
    let Some(gxi) = local_gxi() else {
        return;
    };
    let root = Builder::new()
        .prefix("marlin-agent-harness-real-gxi-policy-")
        .tempdir()
        .expect("real gxi graph policy tempdir");
    let binding =
        GerbilRuntimeBinding::new(gxi, root.path()).expect("write Gerbil runtime binding assets");

    let graph_artifact = compile_artifact(
        &binding,
        "audit/harness-complex-policy-graph",
        HARNESS_COMPLEX_POLICY_GRAPH_SOURCE,
        GerbilArtifactKind::LoopGraph,
    );
    let GerbilCompiledArtifact::LoopGraph(compiled_graph) = graph_artifact else {
        panic!("expected real gxi complex policy loop graph artifact");
    };

    let proposal = compile_gerbil_loop_graph_policy(
        GerbilLoopGraphPolicyCompilationRequest::new(
            GraphLoopStrategy::native_gerbil("real-gxi-complex-policy", "v1"),
            compiled_graph,
            "sha256:real-gxi-complex-policy-input",
            "sha256:real-gxi-complex-policy-output",
        )
        .with_diagnostic(GERBIL_LOOP_GRAPH_POLICY_COMPILATION_SCHEMA_ID)
        .with_diagnostic("source=real-gxi complex scheme policy"),
    );
    let compilation = compile_graph_policy_proposal("real-gxi-complex-policy-run", &proposal);
    assert!(
        compilation.is_accepted(),
        "real gxi graph policy should compile: {:?}",
        compilation.receipt.diagnostics
    );
    assert_eq!(proposal.proposed_graph.graph_id, "gerbil-complex-policy");
    assert_eq!(proposal.proposed_graph.nodes.len(), 4);
    assert_eq!(proposal.proposed_graph.edges.len(), 3);

    let request = compilation
        .request
        .clone()
        .expect("accepted complex policy should produce request")
        .with_budget(GraphLoopExecutionBudget::max_node_executions(4));
    let graph = proposal.proposed_graph.clone();
    let kernel = TokioGraphLoopKernel::new("real-gxi-complex-policy-run", graph.graph_id.clone())
        .with_executor("gerbil-rank", RealGxiHarnessExecutor)
        .with_executor("policy-budget", RealGxiHarnessExecutor)
        .with_executor("kernel-dispatch", RealGxiHarnessExecutor)
        .with_executor("policy-audit", RealGxiHarnessExecutor);
    let mut harness = HarnessRuntime::new(64);
    harness.record_graph_policy_proposal_visibility(&compilation.receipt);
    harness.record_evidence(
        LoopEvidence::present(LoopEvidenceKind::Runtime, "real-gxi-complex-policy")
            .with_detail("source=gerbil artifact=complex_loop_graph graph_policy=rust_receipt"),
    );

    let scenario = HARNESS_SCENARIO_CONTRACT_SOURCE;
    let contract_artifact = compile_artifact(
        &binding,
        "audit/harness-complex-policy-scenario",
        scenario,
        GerbilArtifactKind::AgentScenarioContract,
    );
    let GerbilCompiledArtifact::AgentScenarioContract(contract) = contract_artifact else {
        panic!("expected real gxi agent scenario contract artifact");
    };
    let report = harness
        .execute_graph(&contract.scenario, &kernel, request)
        .await;
    let evaluated = AgentHarness::evaluate_contract_execution_report(&contract, &report);

    assert_eq!(
        report.result.status,
        GraphLoopExecutionStatus::Completed,
        "real gxi complex policy diagnostics: {:?}",
        report.result.diagnostics
    );
    assert_eq!(
        report.result.visited_nodes,
        vec!["rank", "budget-check", "dispatch", "audit"]
    );
    assert!(report.has_event_topic(&observability::kernel_execution_topic()));
    assert!(report.has_span(&observability::harness_execution_span_name()));
    assert!(report.has_graph_policy_proposal_visibility_status(
        &proposal.strategy.strategy_id,
        compilation.receipt.status.clone()
    ));
    assert!(report.assertion.is_none());
    assert!(
        evaluated.is_success(),
        "real gxi complex policy contract failed: {:?}",
        evaluated.diagnostics
    );
}

fn compile_artifact(
    binding: &GerbilRuntimeBinding,
    source_name: &str,
    source: &str,
    kind: GerbilArtifactKind,
) -> GerbilCompiledArtifact {
    binding
        .compiler()
        .compile(GerbilSource::new(source_name, source), kind)
        .unwrap_or_else(|error| panic!("real gxi should compile {source_name}: {error}"))
}

fn compiled_loop_graph_into_protocol(graph: CompiledLoopGraph) -> LoopGraph {
    LoopGraph {
        graph_id: graph.graph_id,
        nodes: graph
            .nodes
            .into_iter()
            .map(|node| LoopNodeSpec {
                id: node.id,
                executor: node.executor,
                config: BTreeMap::from_iter(node.config),
            })
            .collect(),
        edges: graph
            .edges
            .into_iter()
            .map(|edge| LoopEdgeSpec {
                from: edge.from,
                to: edge.to,
                condition: edge.condition,
            })
            .collect(),
    }
}

fn local_gxi() -> Option<PathBuf> {
    let gxi = default_gerbil_gxi_program();
    if !gxi.exists() {
        let message = format!(
            "skipping real gxi harness test because {} is missing",
            gxi.display()
        );
        if std::env::var_os(MARLIN_REQUIRE_REAL_GXI_ENV).is_some() {
            panic!("{message}; unset {MARLIN_REQUIRE_REAL_GXI_ENV} or set {MARLIN_GERBIL_GXI_ENV}");
        }
        eprintln!("{message}");
        return None;
    }

    Some(gxi)
}

#[derive(Clone, Debug)]
struct RealGxiHarnessExecutor;

impl GraphNodeExecutor for RealGxiHarnessExecutor {
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
