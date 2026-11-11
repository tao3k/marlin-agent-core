use marlin_agent_kernel::{
    GraphLoopStrategy, GraphPolicyProposal, GraphPolicyProposalStatus, LoopGraph, LoopNodeSpec,
    compile_graph_policy_proposal_with_c_abi_readiness, compile_static_graph_policy_proposal,
};
use marlin_agent_protocol::{GraphCAbiReadinessReceipt, GraphCAbiRequirement};

#[test]
fn compiles_policy_proposal_after_c_abi_readiness_gate_passes() {
    let proposal = valid_native_policy_proposal();
    let readiness = GraphCAbiReadinessReceipt::evaluate(
        proposal.c_abi.as_ref().expect("native ABI requirement"),
        ["marlin_graph_loop_rank", "marlin_graph_loop_select"],
    );

    let compilation =
        compile_graph_policy_proposal_with_c_abi_readiness("run", &proposal, &readiness);

    assert!(compilation.is_accepted());
    assert_eq!(
        compilation.receipt.status,
        GraphPolicyProposalStatus::Accepted
    );
    assert!(compilation.request.is_some());
}

#[test]
fn rejects_policy_proposal_when_c_abi_readiness_gate_fails() {
    let proposal = valid_native_policy_proposal();
    let readiness = GraphCAbiReadinessReceipt::evaluate(
        proposal.c_abi.as_ref().expect("native ABI requirement"),
        ["marlin_graph_loop_rank"],
    );

    let compilation =
        compile_graph_policy_proposal_with_c_abi_readiness("run", &proposal, &readiness);

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
            .contains(&"graph_policy_proposal.c_abi_readiness_not_ready".to_string())
    );
    assert!(
        compilation.receipt.diagnostics.contains(
            &"graph_policy_proposal.c_abi_readiness_missing_symbols:marlin_graph_loop_select"
                .to_string()
        )
    );
}

#[test]
fn native_proposal_cannot_use_static_compilation() {
    let proposal = valid_native_policy_proposal();
    let compilation = compile_static_graph_policy_proposal("run", &proposal);
    assert!(compilation.request.is_none());
    assert_eq!(
        compilation.receipt.status,
        GraphPolicyProposalStatus::Rejected
    );
    assert!(
        compilation
            .receipt
            .diagnostics
            .contains(&"graph_policy_proposal.c_abi_readiness_required".to_string())
    );
}

#[test]
fn readiness_for_different_symbols_cannot_authorize_a_native_proposal() {
    let proposal = valid_native_policy_proposal();
    let other = GraphCAbiRequirement::new("marlin.graph-loop.native", 1)
        .with_required_symbols(["unrelated_a", "unrelated_b"]);
    let readiness = GraphCAbiReadinessReceipt::evaluate(&other, ["unrelated_a", "unrelated_b"]);
    let compilation =
        compile_graph_policy_proposal_with_c_abi_readiness("run", &proposal, &readiness);
    assert!(compilation.request.is_none());
    assert!(
        compilation
            .receipt
            .diagnostics
            .contains(&"graph_policy_proposal.c_abi_readiness_requirement_mismatch".to_string())
    );
}

#[test]
fn inconsistent_readiness_counts_are_rejected() {
    let proposal = valid_native_policy_proposal();
    let mut readiness = GraphCAbiReadinessReceipt::evaluate(
        proposal.c_abi.as_ref().unwrap(),
        ["marlin_graph_loop_rank", "marlin_graph_loop_select"],
    );
    readiness.matched_symbol_count = 1;
    let compilation =
        compile_graph_policy_proposal_with_c_abi_readiness("run", &proposal, &readiness);
    assert!(compilation.request.is_none());
    assert!(
        compilation
            .receipt
            .diagnostics
            .contains(&"graph_policy_proposal.c_abi_readiness_count_mismatch".to_string())
    );
}

fn valid_native_policy_proposal() -> GraphPolicyProposal {
    GraphPolicyProposal::new(
        GraphLoopStrategy::scheme_c_abi("scheme-loop-ranker", "v1"),
        LoopGraph {
            graph_id: "graph".to_owned(),
            nodes: vec![LoopNodeSpec {
                id: "plan".to_owned(),
                executor: "echo".to_owned(),
                config: Default::default(),
            }],
            edges: Vec::new(),
        },
        "sha256:input",
        "sha256:output",
    )
    .with_c_abi_requirement(policy_c_abi_requirement())
}

fn policy_c_abi_requirement() -> GraphCAbiRequirement {
    GraphCAbiRequirement::new("marlin.graph-loop.native", 1)
        .with_required_symbols(["marlin_graph_loop_rank", "marlin_graph_loop_select"])
}
