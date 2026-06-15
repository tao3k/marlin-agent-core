use marlin_agent_core::{
    GraphLoopStrategy, GraphLoopStrategyRuntime, GraphPolicyProposal, GraphPolicyProposalReceipt,
    GraphPolicyProposalStatus, LoopGraph, run_marlin_cli_from_args,
};
use std::fs;
use tempfile::tempdir;

#[test]
fn debug_cli_graph_validate_returns_rejected_receipt_for_invalid_proposal() {
    let dir = tempdir().expect("tempdir");
    let input = dir.path().join("proposal.json");
    let proposal = GraphPolicyProposal::new(
        GraphLoopStrategy::new("static-debug", "v1", GraphLoopStrategyRuntime::StaticPolicy),
        LoopGraph {
            graph_id: "graph".to_owned(),
            nodes: Vec::new(),
            edges: Vec::new(),
        },
        "debug:input",
        "debug:output",
    );
    fs::write(
        &input,
        serde_json::to_string(&proposal).expect("proposal JSON"),
    )
    .expect("write proposal");

    let result = run_marlin_cli_from_args([
        "graph",
        "validate",
        "--input",
        input.to_str().expect("utf8 path"),
    ]);

    assert_eq!(result.status, 0);
    let report: GraphPolicyProposalReceipt =
        serde_json::from_str(&result.stdout).expect("validation receipt");
    assert_eq!(report.status, GraphPolicyProposalStatus::Rejected);
    assert!(
        report
            .diagnostics
            .contains(&"graph_policy_proposal.nodes_empty".to_owned())
    );
}
