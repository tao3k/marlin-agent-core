use std::collections::BTreeMap;

use marlin_agent_protocol::{
    GRAPH_POLICY_PROPOSAL_SCHEMA_ID, GraphLoopStrategy, GraphLoopStrategyRuntime, GraphNativeAbiId,
    GraphNativeAbiRequirement, GraphPolicyProposal, GraphPolicyProposalReceipt,
    GraphPolicyProposalStatus, GraphPolicyProposalValidationReport, LoopEdgeSpec, LoopGraph,
    LoopNodeSpec,
};

mod controller;
mod execution;
mod gerbil_policy;
mod loop_event;
mod native_abi;

#[test]
fn graph_policy_proposal_records_native_scheme_strategy_with_abi_requirement() {
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
    .with_native_abi_requirement(native_policy_abi_requirement())
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
    assert_eq!(value["native_abi"]["abi_id"], "marlin.graph-loop.native");
    assert_eq!(value["native_abi"]["version"], 1);
    assert_eq!(
        value["native_abi"]["required_symbols"][0],
        "marlin_graph_loop_rank"
    );
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
        accepted.native_abi.as_ref().expect("native abi").abi_id,
        GraphNativeAbiId::new("marlin.graph-loop.native")
    );
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
    assert!(rejected.native_abi.is_some());
    assert!(rejected.selected_graph_id.is_none());
    assert_eq!(rejected.diagnostics[0], "budget_exceeds_runtime_policy");
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
            .contains(&"graph_policy_proposal.native_abi_missing".to_string())
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

#[test]
fn native_graph_policy_proposal_validation_requires_native_abi_requirement() {
    let proposal = GraphPolicyProposal::new(
        GraphLoopStrategy::native_gerbil("gerbil-loop-ranker", "v1"),
        valid_graph(),
        "sha256:input",
        "sha256:output",
    );

    let validation = proposal.validate();

    assert_eq!(validation.status, GraphPolicyProposalStatus::Rejected);
    assert!(validation.selected_graph_id.is_none());
    assert!(validation.native_abi.is_none());
    assert!(
        validation
            .diagnostics
            .contains(&"graph_policy_proposal.native_abi_missing".to_string())
    );
}

#[test]
fn graph_policy_proposal_validation_rejects_invalid_native_abi_requirement() {
    let proposal = GraphPolicyProposal::new(
        GraphLoopStrategy::native_scheme("scheme-loop-ranker", "v1"),
        valid_graph(),
        "sha256:input",
        "sha256:output",
    )
    .with_native_abi_requirement(
        GraphNativeAbiRequirement::new(" ", 0).with_required_symbols(["rank", "rank", " "]),
    );

    let validation = proposal.validate();

    assert_eq!(validation.status, GraphPolicyProposalStatus::Rejected);
    assert!(validation.selected_graph_id.is_none());
    assert!(
        validation
            .diagnostics
            .contains(&"graph_policy_proposal.native_abi_id_empty".to_string())
    );
    assert!(
        validation
            .diagnostics
            .contains(&"graph_policy_proposal.native_abi_version_zero".to_string())
    );
    assert!(
        validation
            .diagnostics
            .contains(&"graph_policy_proposal.native_abi_symbol_duplicate:rank".to_string())
    );
    assert!(
        validation
            .diagnostics
            .contains(&"graph_policy_proposal.native_abi_symbol_empty".to_string())
    );
}

#[test]
fn static_graph_policy_proposal_validation_rejects_native_abi_requirement() {
    let proposal = GraphPolicyProposal::new(
        GraphLoopStrategy::new(
            "static-loop-ranker",
            "v1",
            GraphLoopStrategyRuntime::StaticPolicy,
        ),
        valid_graph(),
        "sha256:input",
        "sha256:output",
    )
    .with_native_abi_requirement(native_policy_abi_requirement());

    let validation = proposal.validate();

    assert_eq!(validation.status, GraphPolicyProposalStatus::Rejected);
    assert!(
        validation
            .diagnostics
            .contains(&"graph_policy_proposal.native_abi_unexpected".to_string())
    );
}

fn native_policy_abi_requirement() -> GraphNativeAbiRequirement {
    GraphNativeAbiRequirement::new("marlin.graph-loop.native", 1)
        .with_required_symbols(["marlin_graph_loop_rank", "marlin_graph_loop_select"])
}

fn valid_graph() -> LoopGraph {
    LoopGraph {
        graph_id: "valid-graph".to_string(),
        nodes: vec![LoopNodeSpec {
            id: "rank".to_string(),
            executor: "gerbil.rank".to_string(),
            config: BTreeMap::new(),
        }],
        edges: Vec::new(),
    }
}
