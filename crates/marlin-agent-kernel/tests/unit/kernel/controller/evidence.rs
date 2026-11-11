use marlin_agent_kernel::{
    GraphLoopController, GraphLoopEvidencePolicy, GraphLoopExecutionRequest,
    GraphLoopExecutionStatus, GraphLoopRunRequest, GraphLoopStopPolicy, LoopGraph,
    LoopPolicyProfile,
};
use marlin_agent_runtime::TokioAgentRuntime;

use super::{ContinueOncePlanner, controller, node};

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

#[tokio::test]
async fn replayable_capture_drains_while_large_iteration_executes() {
    let request = GraphLoopRunRequest::new(GraphLoopExecutionRequest::new(
        "run-many",
        LoopGraph {
            graph_id: "graph-many".to_owned(),
            nodes: (0..100).map(|i| node(&format!("node-{i:03}"))).collect(),
            edges: Vec::new(),
        },
    ))
    .with_evidence_policy(GraphLoopEvidencePolicy::replayable_runtime());
    let (runtime, mut events) = TokioAgentRuntime::new(512);
    let reports = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        controller().spawn_loop(request, &runtime).join(),
    )
    .await
    .expect("capture must not deadlock execution")
    .expect("controller task should join");
    let trace = reports[0].trace.as_ref().expect("replayable trace");
    assert_eq!(
        reports[0].execution_result.status,
        GraphLoopExecutionStatus::Completed
    );
    assert_eq!(reports[0].execution_result.node_receipts.len(), 100);
    let mut primary = Vec::new();
    while let Some(event) = events.try_next() {
        primary.push(event);
    }
    assert!(trace.events.len() > 64);
    assert_eq!(trace.events, primary);
}

#[tokio::test]
async fn captured_iterations_preserve_order_with_primary_backpressure() {
    let request = GraphLoopRunRequest::new(GraphLoopExecutionRequest::new(
        "run-multiple",
        LoopGraph {
            graph_id: "graph-multiple".to_owned(),
            nodes: (0..100).map(|i| node(&format!("node-{i:03}"))).collect(),
            edges: Vec::new(),
        },
    ))
    .with_evidence_policy(GraphLoopEvidencePolicy::replayable_runtime())
    .with_stop_policy(GraphLoopStopPolicy::max_iterations(2));
    let (runtime, mut events) = TokioAgentRuntime::new(3);
    let reader = tokio::spawn(async move {
        let mut observed = Vec::new();
        while let Some(event) = events.recv().await {
            observed.push(event);
            tokio::task::yield_now().await;
        }
        observed
    });
    let reports = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        controller()
            .with_continuation_planner(ContinueOncePlanner)
            .spawn_loop(request, &runtime)
            .join(),
    )
    .await
    .expect("concurrent primary and capture readers must make progress")
    .unwrap();
    drop(runtime);
    let observed = tokio::time::timeout(std::time::Duration::from_secs(5), reader)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(reports.len(), 2);
    let first = &reports[0].trace.as_ref().unwrap().events;
    let second = &reports[1].trace.as_ref().unwrap().events;
    assert!(first.len() > 64);
    assert!(
        first
            .iter()
            .all(|event| !event.message.contains("graph-next"))
    );
    assert!(
        second
            .iter()
            .any(|event| event.message.contains("graph-next"))
    );
    assert_eq!(
        first.iter().chain(second).cloned().collect::<Vec<_>>(),
        observed
    );
}
