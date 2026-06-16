use marlin_agent_kernel::{
    GraphLoopController, GraphLoopEvidencePolicy, GraphLoopExecutionRequest,
    GraphLoopExecutionStatus, GraphLoopRunRequest, LoopGraph, LoopPolicyProfile,
};
use marlin_agent_runtime::TokioAgentRuntime;

use super::{controller, node};

#[tokio::test]
async fn controller_keeps_receipts_and_trace_for_replayable_evidence_policy() {
    let request = GraphLoopRunRequest::new(GraphLoopExecutionRequest::new(
        "run",
        LoopGraph {
            graph_id: "graph".to_owned(),
            nodes: vec![node("plan")],
            edges: Vec::new(),
        },
    ))
    .with_evidence_policy(GraphLoopEvidencePolicy::replayable_runtime());
    let (runtime, mut events) = TokioAgentRuntime::new(8);

    let reports = controller()
        .spawn_loop(request, &runtime)
        .join()
        .await
        .expect("controller task should join");

    assert_eq!(reports.len(), 1);
    let report = &reports[0];
    assert_eq!(report.execution_result.node_receipts.len(), 1);
    let trace = report.trace.as_ref().expect("replayable trace");
    assert_eq!(trace.run_id.as_str(), "run");
    assert_eq!(trace.graph_id.as_str(), "graph");
    assert_eq!(trace.status, GraphLoopExecutionStatus::Completed);
    assert!(
        trace
            .events
            .iter()
            .any(|event| event.topic == "kernel.execution"
                && event.message == "run run started graph graph")
    );
    assert!(trace.diagnostics.is_empty());
    assert_eq!(
        events
            .try_next()
            .expect("parent stream should still receive tee'd event")
            .topic,
        "kernel.execution"
    );
}

#[tokio::test]
async fn controller_projects_policy_profile_evidence_capture() {
    let request = GraphLoopRunRequest::new(GraphLoopExecutionRequest::new(
        "run",
        LoopGraph {
            graph_id: "graph".to_owned(),
            nodes: vec![node("plan")],
            edges: Vec::new(),
        },
    ))
    .with_policy_profile(LoopPolicyProfile::new("profile.evidence"));
    let (runtime, mut events) = TokioAgentRuntime::new(8);

    let reports = controller()
        .spawn_loop(request, &runtime)
        .join()
        .await
        .expect("controller task should join");

    assert_eq!(reports.len(), 1);
    let report = &reports[0];
    assert_eq!(report.execution_result.node_receipts.len(), 1);
    let trace = report
        .trace
        .as_ref()
        .expect("policy-profile projected trace");
    assert_eq!(trace.run_id.as_str(), "run");
    assert_eq!(trace.graph_id.as_str(), "graph");
    assert_eq!(trace.status, GraphLoopExecutionStatus::Completed);
    assert!(
        trace
            .events
            .iter()
            .any(|event| event.topic == "kernel.execution"
                && event.message == "run run started graph graph")
    );
    assert_eq!(
        events
            .try_next()
            .expect("parent stream should still receive tee'd event")
            .topic,
        "kernel.execution"
    );
}
