use marlin_agent_kernel::{
    GraphLoopStrategy, GraphPolicyProposal, GraphPolicyProposalStatus, LoopGraph, LoopNodeSpec,
    compile_graph_policy_proposal_with_c_abi_readiness,
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
