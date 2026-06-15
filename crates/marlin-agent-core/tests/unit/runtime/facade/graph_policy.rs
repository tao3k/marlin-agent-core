use marlin_agent_core::{
    AgentTraceSpanRecord, GERBIL_LOOP_GRAPH_POLICY_COMPILATION_SCHEMA_ID,
    GRAPH_POLICY_PROPOSAL_SPAN_NAME, GRAPH_POLICY_PROPOSAL_VISIBILITY_SUBJECT_PREFIX,
    GerbilLoopGraphPolicyCompilationRequest, GraphLoopExecutionBudget, GraphLoopStrategy,
    GraphLoopStrategyId, GraphPolicyProposal, GraphPolicyProposalStatus, HarnessEvidenceKind,
    LoopGraph, LoopNodeSpec, compile_gerbil_loop_graph_policy, compile_graph_policy_proposal,
    graph_policy_proposal_visibility_evidence,
};
use marlin_agent_test_support::graph_native_abi_requirement_fixture;
use std::collections::BTreeMap;

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

    let compilation = compile_graph_policy_proposal("run-proposal", &proposal);

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
    let evidence = graph_policy_proposal_visibility_evidence(&span)
        .expect("core facade should expose proposal visibility evidence projection");
    let detail = evidence
        .detail
        .as_deref()
        .expect("proposal visibility detail");
    assert_eq!(evidence.kind, HarnessEvidenceKind::Visibility);
    assert_eq!(
        evidence.subject,
        format!("{GRAPH_POLICY_PROPOSAL_VISIBILITY_SUBJECT_PREFIX}:scheme-loop-ranker")
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
    let compilation = compile_graph_policy_proposal("core-gerbil-run", &proposal);

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
