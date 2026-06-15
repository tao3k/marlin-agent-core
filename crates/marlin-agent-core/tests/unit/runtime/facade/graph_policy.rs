use marlin_agent_core::{
    AGENT_HARNESS_GRAPH_POLICY_PROPOSAL_VISIBILITY_SUBJECT_PREFIX, AgentHarnessEvidenceKind,
    AgentTraceSpanRecord, GERBIL_LOOP_GRAPH_POLICY_COMPILATION_SCHEMA_ID,
    GRAPH_POLICY_PROPOSAL_SPAN_NAME, GerbilLoopGraphPolicyCompilationRequest,
    GraphLoopExecutionBudget, GraphLoopExecutionStatus, GraphLoopKernel, GraphLoopStrategy,
    GraphLoopStrategyId, GraphNativeAbiReadinessReceipt, GraphNodeExecutionReceipt,
    GraphNodeExecutionStatus, GraphNodeExecutor, GraphNodeInvocation, GraphPolicyProposal,
    GraphPolicyProposalStatus, LoopGraph, LoopNodeSpec, RuntimeContext, RuntimeFuture,
    TokioAgentRuntime, TokioGraphLoopKernel,
    agent_harness_graph_policy_proposal_visibility_evidence, compile_gerbil_loop_graph_policy,
    compile_graph_policy_proposal_with_native_abi_readiness,
};
use marlin_agent_test_support::graph_native_abi_requirement_fixture;
use std::collections::BTreeMap;

struct CoreFacadeGerbilExecutor;

impl GraphNodeExecutor for CoreFacadeGerbilExecutor {
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

#[test]
fn core_facade_compiles_policy_proposal_before_kernel_execution() {
    let proposal = GraphPolicyProposal::new(
        GraphLoopStrategy::native_scheme("scheme-loop-ranker", "v1"),
        LoopGraph {
            graph_id: "graph-1".to_owned(),
            nodes: vec![LoopNodeSpec {
                id: "plan".to_owned(),
                executor: "provider".to_owned(),
                config: BTreeMap::new(),
            }],
            edges: Vec::new(),
        },
        "sha256:input",
        "sha256:output",
    )
    .with_native_abi_requirement(graph_native_abi_requirement_fixture());

    let readiness = ready_graph_native_abi_receipt();
    let compilation = compile_graph_policy_proposal_with_native_abi_readiness(
        "run-proposal",
        &proposal,
        &readiness,
    );

    assert!(compilation.is_accepted());
    assert_eq!(
        compilation.receipt.status,
        GraphPolicyProposalStatus::Accepted
    );
    let request = compilation
        .request
        .expect("accepted proposal should produce an execution request");
    assert_eq!(request.run_id, "run-proposal");
    assert_eq!(request.graph.graph_id, "graph-1");

    let span = AgentTraceSpanRecord::graph_policy_proposal_receipt(&compilation.receipt);
    assert_eq!(span.name.as_str(), GRAPH_POLICY_PROPOSAL_SPAN_NAME);
    assert_eq!(
        span.fields.get("status").map(String::as_str),
        Some("Accepted")
    );
    assert_eq!(
        span.graph_policy_proposal_strategy_id(),
        Some(GraphLoopStrategyId::new("scheme-loop-ranker"))
    );
    assert_eq!(
        span.graph_policy_proposal_status(),
        Some(GraphPolicyProposalStatus::Accepted)
    );
    let evidence = agent_harness_graph_policy_proposal_visibility_evidence(&span)
        .expect("core facade should expose proposal visibility evidence projection");
    let detail = evidence
        .detail
        .as_deref()
        .expect("proposal visibility detail");
    assert_eq!(evidence.kind, AgentHarnessEvidenceKind::Visibility);
    assert_eq!(
        evidence.subject,
        format!(
            "{AGENT_HARNESS_GRAPH_POLICY_PROPOSAL_VISIBILITY_SUBJECT_PREFIX}:scheme-loop-ranker"
        )
    );
    assert!(detail.contains("status=Accepted"));
}

#[test]
fn core_facade_exposes_gerbil_graph_policy_and_budget_contracts() {
    let proposal = compile_gerbil_loop_graph_policy(
        GerbilLoopGraphPolicyCompilationRequest::new(
            GraphLoopStrategy::native_gerbil("core-gerbil-loop-ranker", "v1"),
            marlin_agent_core::gerbil_ir::CompiledLoopGraph {
                graph_id: "core-gerbil-graph".to_owned(),
                nodes: vec![marlin_agent_core::gerbil_ir::LoopNodeSpec {
                    id: "rank".to_owned(),
                    executor: "gerbil.rank".to_owned(),
                    config: BTreeMap::new(),
                }],
                edges: Vec::new(),
            },
            "sha256:core-gerbil-input",
            "sha256:core-gerbil-output",
        )
        .with_native_abi_requirement(graph_native_abi_requirement_fixture())
        .with_diagnostic(GERBIL_LOOP_GRAPH_POLICY_COMPILATION_SCHEMA_ID),
    )
    .expect("core Gerbil loop graph should compile");
    let readiness = ready_graph_native_abi_receipt();
    let compilation = compile_graph_policy_proposal_with_native_abi_readiness(
        "core-gerbil-run",
        &proposal,
        &readiness,
    );

    assert!(compilation.is_accepted());
    assert_eq!(
        proposal.diagnostics,
        vec![GERBIL_LOOP_GRAPH_POLICY_COMPILATION_SCHEMA_ID.to_owned()]
    );
    let request = compilation
        .request
        .expect("accepted Gerbil proposal should produce request")
        .with_budget(GraphLoopExecutionBudget::max_node_executions(1));
    assert_eq!(
        request
            .budget
            .as_ref()
            .and_then(|budget| budget.max_node_executions),
        Some(1)
    );
}

#[tokio::test]
async fn core_facade_executes_gerbil_loop_graph_policy_through_kernel_with_node_receipts() {
    let proposal = compile_gerbil_loop_graph_policy(
        GerbilLoopGraphPolicyCompilationRequest::new(
            GraphLoopStrategy::native_gerbil("core-gerbil-loop-executable", "v1"),
            marlin_agent_core::gerbil_ir::CompiledLoopGraph {
                graph_id: "core-gerbil-executable-graph".to_owned(),
                nodes: vec![
                    marlin_agent_core::gerbil_ir::LoopNodeSpec {
                        id: "rank".to_owned(),
                        executor: "gerbil.rank".to_owned(),
                        config: BTreeMap::new(),
                    },
                    marlin_agent_core::gerbil_ir::LoopNodeSpec {
                        id: "dispatch".to_owned(),
                        executor: "kernel.dispatch".to_owned(),
                        config: BTreeMap::new(),
                    },
                ],
                edges: Vec::new(),
            },
            "sha256:core-gerbil-executable-input",
            "sha256:core-gerbil-executable-output",
        )
        .with_native_abi_requirement(graph_native_abi_requirement_fixture()),
    )
    .expect("core Gerbil executable loop graph should compile");
    let readiness = ready_graph_native_abi_receipt();
    let request = compile_graph_policy_proposal_with_native_abi_readiness(
        "core-gerbil-executable-run",
        &proposal,
        &readiness,
    )
    .request
    .expect("accepted Gerbil proposal should produce request")
    .with_budget(GraphLoopExecutionBudget::max_node_executions(2));
    let (runtime, _events) = TokioAgentRuntime::new(16);
    let kernel = TokioGraphLoopKernel::new(request.run_id.clone(), request.graph.graph_id.clone())
        .with_executor("gerbil.rank", CoreFacadeGerbilExecutor)
        .with_executor("kernel.dispatch", CoreFacadeGerbilExecutor);

    let result = kernel
        .spawn_execution(request, &runtime)
        .join()
        .await
        .expect("core facade Gerbil kernel execution should finish");

    assert_eq!(result.status, GraphLoopExecutionStatus::Completed);
    assert_eq!(result.visited_nodes, vec!["rank", "dispatch"]);
    assert!(result.diagnostics.is_empty());
    assert_eq!(
        result
            .node_receipts
            .iter()
            .map(|receipt| (
                receipt.node_id.as_str(),
                receipt.executor.as_str(),
                receipt.status.clone()
            ))
            .collect::<Vec<_>>(),
        vec![
            ("rank", "gerbil.rank", GraphNodeExecutionStatus::Completed),
            (
                "dispatch",
                "kernel.dispatch",
                GraphNodeExecutionStatus::Completed
            )
        ]
    );
}

#[test]
fn core_facade_rejects_gerbil_loop_graph_policy_when_native_abi_readiness_fails_before_execution() {
    let proposal = compile_gerbil_loop_graph_policy(
        GerbilLoopGraphPolicyCompilationRequest::new(
            GraphLoopStrategy::native_gerbil("core-gerbil-loop-unready", "v1"),
            marlin_agent_core::gerbil_ir::CompiledLoopGraph {
                graph_id: "core-gerbil-unready-graph".to_owned(),
                nodes: vec![marlin_agent_core::gerbil_ir::LoopNodeSpec {
                    id: "rank".to_owned(),
                    executor: "gerbil.rank".to_owned(),
                    config: BTreeMap::new(),
                }],
                edges: Vec::new(),
            },
            "sha256:core-gerbil-unready-input",
            "sha256:core-gerbil-unready-output",
        )
        .with_native_abi_requirement(graph_native_abi_requirement_fixture()),
    )
    .expect("core Gerbil unready loop graph should compile before ABI readiness gate");
    let readiness = GraphNativeAbiReadinessReceipt::evaluate(
        proposal
            .native_abi
            .as_ref()
            .expect("native ABI requirement"),
        ["marlin_graph_loop_rank"],
    );

    let compilation = compile_graph_policy_proposal_with_native_abi_readiness(
        "core-gerbil-unready-run",
        &proposal,
        &readiness,
    );

    assert!(!compilation.is_accepted());
    assert_eq!(
        compilation.receipt.status,
        GraphPolicyProposalStatus::Rejected
    );
    assert!(compilation.request.is_none());
    assert!(
        compilation
            .receipt
            .diagnostics
            .contains(&"graph_policy_proposal.native_abi_readiness_not_ready".to_owned())
    );
    assert!(
        compilation.receipt.diagnostics.contains(
            &"graph_policy_proposal.native_abi_readiness_missing_symbols:marlin_graph_loop_select"
                .to_owned()
        )
    );
}

#[test]
fn core_facade_rejects_invalid_gerbil_loop_graph_shapes_before_execution() {
    let cases = [
        (
            "core-gerbil-missing-edge-target",
            marlin_agent_core::gerbil_ir::CompiledLoopGraph {
                graph_id: "core-gerbil-invalid-edge".to_owned(),
                nodes: vec![marlin_agent_core::gerbil_ir::LoopNodeSpec {
                    id: "rank".to_owned(),
                    executor: "gerbil.rank".to_owned(),
                    config: BTreeMap::new(),
                }],
                edges: vec![marlin_agent_core::gerbil_ir::LoopEdgeSpec {
                    from: "rank".to_owned(),
                    to: "missing".to_owned(),
                    condition: Some("always".to_owned()),
                }],
            },
            marlin_agent_core::gerbil_ir::LoopGraphCompileError::Validation(
                marlin_agent_core::gerbil_ir::LoopGraphValidationError::UnknownEdgeTarget {
                    edge_index: 0,
                    node_id: "missing".to_owned(),
                },
            ),
        ),
        (
            "core-gerbil-cycle",
            marlin_agent_core::gerbil_ir::CompiledLoopGraph {
                graph_id: "core-gerbil-cycle".to_owned(),
                nodes: vec![
                    marlin_agent_core::gerbil_ir::LoopNodeSpec {
                        id: "rank".to_owned(),
                        executor: "gerbil.rank".to_owned(),
                        config: BTreeMap::new(),
                    },
                    marlin_agent_core::gerbil_ir::LoopNodeSpec {
                        id: "dispatch".to_owned(),
                        executor: "kernel.dispatch".to_owned(),
                        config: BTreeMap::new(),
                    },
                ],
                edges: vec![
                    marlin_agent_core::gerbil_ir::LoopEdgeSpec {
                        from: "rank".to_owned(),
                        to: "dispatch".to_owned(),
                        condition: Some("always".to_owned()),
                    },
                    marlin_agent_core::gerbil_ir::LoopEdgeSpec {
                        from: "dispatch".to_owned(),
                        to: "rank".to_owned(),
                        condition: Some("always".to_owned()),
                    },
                ],
            },
            marlin_agent_core::gerbil_ir::LoopGraphCompileError::CycleDetected {
                remaining_node_ids: vec!["rank".to_owned(), "dispatch".to_owned()],
            },
        ),
    ];

    for (strategy_id, graph, expected_error) in cases {
        let error = compile_gerbil_loop_graph_policy(
            GerbilLoopGraphPolicyCompilationRequest::new(
                GraphLoopStrategy::native_gerbil(strategy_id, "v1"),
                graph,
                "sha256:core-gerbil-invalid-input",
                "sha256:core-gerbil-invalid-output",
            )
            .with_native_abi_requirement(graph_native_abi_requirement_fixture())
            .with_diagnostic(GERBIL_LOOP_GRAPH_POLICY_COMPILATION_SCHEMA_ID),
        )
        .expect_err("invalid Gerbil loop graph should fail before proposal execution");

        assert_eq!(error, expected_error);
    }
}

fn ready_graph_native_abi_receipt() -> GraphNativeAbiReadinessReceipt {
    GraphNativeAbiReadinessReceipt::evaluate(
        &graph_native_abi_requirement_fixture(),
        ["marlin_graph_loop_rank", "marlin_graph_loop_select"],
    )
}
