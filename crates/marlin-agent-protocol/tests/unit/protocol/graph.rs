use std::collections::BTreeMap;

use marlin_agent_protocol::{
    GRAPH_POLICY_PROPOSAL_SCHEMA_ID, GerbilLoopGraphPolicyCompilationRequest,
    GraphLoopExecutionBudget, GraphLoopExecutionRequest, GraphLoopStrategy,
    GraphLoopStrategyRuntime, GraphPolicyProposal, GraphPolicyProposalReceipt,
    GraphPolicyProposalStatus, GraphPolicyProposalValidationReport, LoopEdgeSpec, LoopGraph,
    LoopNodeSpec, compile_gerbil_loop_graph_policy,
};

#[test]
fn graph_policy_proposal_records_native_scheme_strategy_without_runtime_power() {
    let graph = LoopGraph {
        graph_id: "turn-frontier-1".to_string(),
        nodes: vec![
            LoopNodeSpec {
                id: "provider-stream".to_string(),
                executor: "provider.stream".to_string(),
                config: BTreeMap::from([("budget".to_string(), "bounded".to_string())]),
            },
            LoopNodeSpec {
                id: "tool-batch".to_string(),
                executor: "tool.batch".to_string(),
                config: BTreeMap::new(),
            },
        ],
        edges: vec![LoopEdgeSpec {
            from: "provider-stream".to_string(),
            to: "tool-batch".to_string(),
            condition: Some("assistant_has_tool_calls".to_string()),
        }],
    };
    let proposal = GraphPolicyProposal::new(
        GraphLoopStrategy::native_scheme("scheme-loop-ranker", "v1"),
        graph,
        "sha256:input",
        "sha256:output",
    )
    .with_diagnostic("scheme_policy_plane=proposal_only");

    assert!(proposal.has_current_schema());
    assert!(proposal.is_native_policy_plane());
    assert_eq!(
        proposal.strategy.runtime,
        GraphLoopStrategyRuntime::NativeScheme
    );
    assert_eq!(proposal.input_digest.as_str(), "sha256:input");
    assert_eq!(proposal.output_digest.as_str(), "sha256:output");

    let value = serde_json::to_value(&proposal).expect("proposal serializes");
    assert_eq!(value["schema_id"], GRAPH_POLICY_PROPOSAL_SCHEMA_ID);
    assert_eq!(value["strategy"]["strategy_id"], "scheme-loop-ranker");
    assert_eq!(value["strategy"]["runtime"], "NativeScheme");
    assert_eq!(value["proposed_graph"]["graph_id"], "turn-frontier-1");
    assert_eq!(value["diagnostics"][0], "scheme_policy_plane=proposal_only");

    let validation: GraphPolicyProposalValidationReport = proposal.validate();
    assert!(validation.is_accepted());
    assert_eq!(validation.status, GraphPolicyProposalStatus::Accepted);
    assert_eq!(
        validation
            .selected_graph_id
            .as_ref()
            .expect("selected graph")
            .as_str(),
        "turn-frontier-1"
    );

    let accepted = GraphPolicyProposalReceipt::accepted(&proposal);
    assert_eq!(accepted.status, GraphPolicyProposalStatus::Accepted);
    assert_eq!(
        accepted
            .selected_graph_id
            .as_ref()
            .expect("selected graph")
            .as_str(),
        "turn-frontier-1"
    );

    let rejected = GraphPolicyProposalReceipt::rejected(
        &proposal,
        vec!["budget_exceeds_runtime_policy".to_string()],
    );
    assert_eq!(rejected.status, GraphPolicyProposalStatus::Rejected);
    assert!(rejected.selected_graph_id.is_none());
    assert_eq!(rejected.diagnostics[0], "budget_exceeds_runtime_policy");
}

#[test]
fn gerbil_loop_graph_ir_compiles_into_graph_policy_proposal() {
    let compiled_graph = marlin_gerbil_ir::CompiledLoopGraph {
        graph_id: "gerbil-graph".to_string(),
        nodes: vec![
            marlin_gerbil_ir::LoopNodeSpec {
                id: "rank".to_string(),
                executor: "gerbil.rank".to_string(),
                config: BTreeMap::from([("mode".to_string(), "native".to_string())]),
            },
            marlin_gerbil_ir::LoopNodeSpec {
                id: "dispatch".to_string(),
                executor: "kernel.dispatch".to_string(),
                config: BTreeMap::new(),
            },
        ],
        edges: vec![marlin_gerbil_ir::LoopEdgeSpec {
            from: "rank".to_string(),
            to: "dispatch".to_string(),
            condition: Some("always".to_string()),
        }],
    };

    let proposal = compile_gerbil_loop_graph_policy(
        GerbilLoopGraphPolicyCompilationRequest::new(
            GraphLoopStrategy::native_gerbil("gerbil-loop-ranker", "v1"),
            compiled_graph,
            "sha256:gerbil-input",
            "sha256:gerbil-output",
        )
        .with_diagnostic("gerbil_ir=compiled"),
    );

    assert_eq!(
        proposal.strategy.runtime,
        GraphLoopStrategyRuntime::NativeGerbil
    );
    assert_eq!(proposal.proposed_graph.graph_id, "gerbil-graph");
    assert_eq!(proposal.proposed_graph.nodes[0].executor, "gerbil.rank");
    assert_eq!(
        proposal.proposed_graph.edges[0].condition.as_deref(),
        Some("always")
    );
    assert_eq!(proposal.diagnostics, vec!["gerbil_ir=compiled"]);
    assert!(proposal.validate().is_accepted());
}

#[test]
fn graph_loop_execution_request_records_optional_budget() {
    let graph = LoopGraph {
        graph_id: "graph".to_string(),
        nodes: vec![LoopNodeSpec {
            id: "plan".to_string(),
            executor: "provider.stream".to_string(),
            config: BTreeMap::new(),
        }],
        edges: Vec::new(),
    };

    let request_without_budget = GraphLoopExecutionRequest::new("run", graph.clone());
    let value = serde_json::to_value(&request_without_budget).expect("request serializes");
    assert!(value.get("budget").is_none());

    let request = GraphLoopExecutionRequest::new("run", graph)
        .with_budget(GraphLoopExecutionBudget::max_node_executions(1));
    let value = serde_json::to_value(&request).expect("budgeted request serializes");

    assert_eq!(
        request
            .budget
            .as_ref()
            .and_then(|budget| budget.max_node_executions),
        Some(1)
    );
    assert_eq!(value["budget"]["max_node_executions"], 1);
}

#[test]
fn graph_policy_proposal_validation_rejects_invalid_runtime_graph() {
    let graph = LoopGraph {
        graph_id: " ".to_string(),
        nodes: vec![
            LoopNodeSpec {
                id: "provider-stream".to_string(),
                executor: "provider.stream".to_string(),
                config: BTreeMap::new(),
            },
            LoopNodeSpec {
                id: "provider-stream".to_string(),
                executor: " ".to_string(),
                config: BTreeMap::new(),
            },
            LoopNodeSpec {
                id: "tool-batch".to_string(),
                executor: "tool.batch".to_string(),
                config: BTreeMap::new(),
            },
        ],
        edges: vec![
            LoopEdgeSpec {
                from: "provider-stream".to_string(),
                to: "tool-batch".to_string(),
                condition: None,
            },
            LoopEdgeSpec {
                from: "tool-batch".to_string(),
                to: "provider-stream".to_string(),
                condition: Some("retry".to_string()),
            },
            LoopEdgeSpec {
                from: "provider-stream".to_string(),
                to: "missing-node".to_string(),
                condition: None,
            },
        ],
    };
    let mut proposal = GraphPolicyProposal::new(
        GraphLoopStrategy::new("", "", GraphLoopStrategyRuntime::NativeGerbil),
        graph,
        "",
        "sha256:output",
    );
    proposal.schema_id = "marlin.agent.graph_policy_proposal.v0".to_string();

    let validation = proposal.validate();
    assert_eq!(validation.status, GraphPolicyProposalStatus::Rejected);
    assert!(validation.selected_graph_id.is_none());
    assert!(
        validation
            .diagnostics
            .contains(&"graph_policy_proposal.schema_id_mismatch".to_string())
    );
    assert!(
        validation
            .diagnostics
            .contains(&"graph_policy_proposal.strategy_id_empty".to_string())
    );
    assert!(
        validation
            .diagnostics
            .contains(&"graph_policy_proposal.strategy_version_empty".to_string())
    );
    assert!(
        validation
            .diagnostics
            .contains(&"graph_policy_proposal.input_digest_empty".to_string())
    );
    assert!(
        validation
            .diagnostics
            .contains(&"graph_policy_proposal.graph_id_empty".to_string())
    );
    assert!(
        validation
            .diagnostics
            .contains(&"graph_policy_proposal.node_id_duplicate:provider-stream".to_string())
    );
    assert!(
        validation
            .diagnostics
            .contains(&"graph_policy_proposal.node_executor_empty:provider-stream".to_string())
    );
    assert!(
        validation
            .diagnostics
            .contains(&"graph_policy_proposal.edge_to_unknown:missing-node".to_string())
    );
    assert!(
        validation
            .diagnostics
            .contains(&"graph_policy_proposal.graph_cycle_detected".to_string())
    );

    let receipt = GraphPolicyProposalReceipt::validate(&proposal);
    assert_eq!(receipt.status, GraphPolicyProposalStatus::Rejected);
    assert_eq!(receipt.diagnostics, validation.diagnostics);
}
