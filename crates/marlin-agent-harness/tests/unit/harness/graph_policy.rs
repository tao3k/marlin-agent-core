use marlin_agent_harness::{AgentHarness, HarnessRuntime};
use marlin_agent_kernel::{
    GraphLoopExecutionStatus, GraphNodeExecutionReceipt, GraphNodeExecutor, GraphNodeInvocation,
    TokioGraphLoopKernel,
};
use marlin_agent_protocol::{AgentScenario, LoopEvidenceKind};
use marlin_agent_runtime::{RuntimeContext, RuntimeFuture};
use marlin_agent_test_support::{
    accepted_gerbil_ir_graph_policy_proposal_fixture, accepted_graph_policy_proposal_fixture,
    assert_accepted_gerbil_ir_graph_policy_proposal_fixture,
    assert_accepted_graph_policy_proposal_fixture, assert_budgeted_graph_policy_execution_request,
    assert_rejected_graph_policy_proposal_fixture, budgeted_graph_policy_execution_request_fixture,
    rejected_graph_policy_proposal_fixture,
};

#[test]
fn harness_consumes_test_support_graph_policy_proposal_compilation() {
    let accepted = accepted_graph_policy_proposal_fixture();
    let rejected = rejected_graph_policy_proposal_fixture();

    assert_accepted_graph_policy_proposal_fixture(&accepted);
    assert_rejected_graph_policy_proposal_fixture(&rejected);
    assert!(accepted.compilation().request.is_some());
    assert!(rejected.compilation().request.is_none());

    let scenario = AgentScenario::new("graph-policy-proposal")
        .expecting_evidence(LoopEvidenceKind::Visibility);
    let mut harness = HarnessRuntime::new(16);

    harness.record_graph_policy_proposal_visibility(&accepted.compilation().receipt);
    harness.record_graph_policy_proposal_visibility(&rejected.compilation().receipt);

    let report = AgentHarness::evaluate(&scenario, &[], harness.evidence());

    assert!(report.is_success());
    assert_eq!(harness.evidence().len(), 2);
    assert!(harness.evidence().iter().all(|evidence| evidence.present));
    assert!(harness.evidence().iter().any(|evidence| {
        evidence
            .detail
            .as_deref()
            .is_some_and(|detail| detail.contains("status=Accepted"))
    }));
    assert!(harness.evidence().iter().any(|evidence| {
        evidence
            .detail
            .as_deref()
            .is_some_and(|detail| detail.contains("status=Rejected"))
    }));
}

#[tokio::test]
async fn harness_execution_report_carries_graph_policy_visibility_evidence() {
    let fixture = accepted_graph_policy_proposal_fixture();
    assert_accepted_graph_policy_proposal_fixture(&fixture);
    let scenario = AgentScenario::new("graph-policy-proposal-execution")
        .expecting_evidence(LoopEvidenceKind::Visibility);
    let request = fixture
        .compilation()
        .request
        .clone()
        .expect("accepted proposal should produce an execution request");
    let kernel = TokioGraphLoopKernel::new(
        fixture.expected_run_id(),
        fixture.proposal().proposed_graph.graph_id.clone(),
    )
    .with_executor("provider.stream", QuietGraphPolicyExecutor);
    let mut harness = HarnessRuntime::new(16);

    harness.record_graph_policy_proposal_visibility(&fixture.compilation().receipt);

    let report = harness.execute_graph(&scenario, &kernel, request).await;
    let evaluated = AgentHarness::evaluate_execution_report(&scenario, &report);

    assert_eq!(report.result.status, GraphLoopExecutionStatus::Completed);
    assert!(report.assertion.is_none());
    assert_eq!(report.evidence, vec![fixture.visibility_evidence()]);
    assert_eq!(
        report.graph_policy_proposal_visibility_evidence().count(),
        1
    );
    assert!(
        report
            .find_graph_policy_proposal_visibility_evidence(
                &fixture.proposal().strategy.strategy_id
            )
            .is_some()
    );
    assert!(report.has_graph_policy_proposal_visibility_status(
        &fixture.proposal().strategy.strategy_id,
        fixture.compilation().receipt.status.clone()
    ));
    assert!(evaluated.is_success());
}

#[tokio::test]
async fn harness_executes_gerbil_ir_graph_policy_with_budget_without_live_llm() {
    let fixture = accepted_gerbil_ir_graph_policy_proposal_fixture();
    assert_accepted_gerbil_ir_graph_policy_proposal_fixture(&fixture);

    let scenario = AgentScenario::new("gerbil-ir-graph-policy-execution")
        .expecting_evidence(LoopEvidenceKind::Visibility);
    let request = budgeted_graph_policy_execution_request_fixture(&fixture, 2);
    assert_budgeted_graph_policy_execution_request(&request, 2);

    let kernel = TokioGraphLoopKernel::new(
        fixture.expected_run_id(),
        fixture.proposal().proposed_graph.graph_id.clone(),
    )
    .with_executor("gerbil.rank", QuietGraphPolicyExecutor)
    .with_executor("kernel.dispatch", QuietGraphPolicyExecutor);
    let mut harness = HarnessRuntime::new(16);
    harness.record_graph_policy_proposal_visibility(&fixture.compilation().receipt);

    let report = harness.execute_graph(&scenario, &kernel, request).await;
    let evaluated = AgentHarness::evaluate_execution_report(&scenario, &report);

    assert_eq!(report.result.status, GraphLoopExecutionStatus::Completed);
    assert_eq!(report.result.visited_nodes, vec!["rank", "dispatch"]);
    assert_eq!(report.evidence, vec![fixture.visibility_evidence()]);
    assert!(report.has_graph_policy_proposal_visibility_status(
        &fixture.proposal().strategy.strategy_id,
        fixture.compilation().receipt.status.clone()
    ));
    assert!(evaluated.is_success());
}

#[derive(Clone, Debug)]
struct QuietGraphPolicyExecutor;

impl GraphNodeExecutor for QuietGraphPolicyExecutor {
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
