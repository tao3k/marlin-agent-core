use marlin_agent_core::{
    GraphLoopExecutionRequest, GraphLoopExecutionStatus, GraphLoopGovernancePolicy,
    GraphLoopRunRequest, GraphLoopStopPolicy, LoopGovernanceVerifierDecision, LoopRunReceipt,
    run_marlin_cli_from_args,
};
use std::fs;
use tempfile::tempdir;

use crate::runtime::facade::debug_cli::fixtures::single_node_graph;

#[test]
fn debug_cli_loop_run_materializes_nono_governance_session_receipt() {
    let dir = tempdir().expect("tempdir");
    let input = dir.path().join("govern-loop.json");
    let request = GraphLoopRunRequest::new(GraphLoopExecutionRequest::new(
        "marlin-govern-loop-run",
        single_node_graph(),
    ))
    .with_governance_policy(GraphLoopGovernancePolicy::nono("nono-profile"))
    .with_stop_policy(GraphLoopStopPolicy::max_iterations(1));
    fs::write(
        &input,
        serde_json::to_string(&request).expect("graph loop request JSON"),
    )
    .expect("write graph loop request");

    let run = run_marlin_cli_from_args([
        "loop",
        "run",
        "--input",
        input.to_str().expect("utf8 path"),
        "--no-store",
    ]);

    assert_eq!(run.status, 0, "{}", run.stderr);
    let receipt: LoopRunReceipt = serde_json::from_str(&run.stdout).expect("loop run receipt");
    let governance = receipt
        .governance_receipt
        .as_ref()
        .expect("governance receipt");
    assert_eq!(governance.run_id.as_str(), "marlin-govern-loop-run");
    assert!(governance.state.read_before_run);
    assert!(governance.state.write_receipt_on_pass);
    assert_eq!(governance.sandbox.backend, "nono");
    assert_eq!(governance.sandbox.profile_ref, "nono-profile");
    assert_eq!(
        governance.sandbox.filesystem_scope.as_deref(),
        Some("runtime")
    );
    assert!(!governance.sandbox.network_access);
    assert_eq!(governance.sandbox.runtime_owner, "marlin-agent-core");
    assert_eq!(governance.sandbox.materialized_by, "debug_cli.govern_loop");
    assert_eq!(
        governance.session.child_session_id,
        "govern-loop:marlin-govern-loop-run"
    );
    assert_eq!(
        governance.session.requested_namespaces,
        vec!["system", "workspace", "tools"]
    );
    assert_eq!(
        governance.session.granted_namespaces,
        vec!["system", "workspace", "tools"]
    );
    assert!(governance.session.denied_namespaces.is_empty());
    assert_eq!(
        governance.verifier.decision,
        LoopGovernanceVerifierDecision::Passed
    );
    assert_eq!(
        governance.verifier.terminal_status,
        Some(GraphLoopExecutionStatus::Completed)
    );
    assert!(!governance.verifier.retryable);
    assert!(!governance.verifier.human_audit_required);
}
