use std::sync::Arc;

use marlin_agent_environment::{
    WorkingCopyCreateRequest, WorkingCopyHandle, WorkingCopyIsolationProvider,
    WorkingCopyIsolationReceipt, WorkingCopyIsolationRequest, WorkingCopyIsolationStatus,
};
use marlin_agent_runtime::{
    CancellationToken, RuntimeContext, RuntimeEnvironment, RuntimeFuture, SubAgentRuntime,
    TokioAgentRuntime, WorkingCopyActiveBinding, WorkingCopySubAgentFanoutItem,
};

#[tokio::test]
async fn sub_agent_can_run_with_working_copy_environment_receipt() {
    let parent_environment = RuntimeEnvironment::default().with_cwd("/repo");
    let child_environment = RuntimeEnvironment::default().with_cwd("/repo.feature-a");
    let request = WorkingCopyIsolationRequest::Create(WorkingCopyCreateRequest::new(
        "marlin-core",
        WorkingCopyIsolationProvider::GitWorktree,
        "/repo",
        WorkingCopyHandle::new("feature-a", "/repo.feature-a"),
    ));
    let receipt = WorkingCopyIsolationReceipt::applied(&request);
    let (runtime, _events) =
        TokioAgentRuntime::with_environment(4, CancellationToken::new(), parent_environment);

    let (environment, receipts, binding) = runtime
        .spawn_sub_agent_with_working_copy_environment(
            Arc::new(WorkingCopyEchoSubAgent),
            (),
            child_environment.clone(),
            receipt,
        )
        .join()
        .await
        .expect("sub-agent task should finish");

    assert_eq!(environment, child_environment);
    assert_eq!(receipts.len(), 1);
    assert_eq!(receipts[0].status, WorkingCopyIsolationStatus::Applied);
    assert_eq!(
        receipts[0]
            .working_copy
            .as_ref()
            .map(|copy| copy.id.as_str()),
        Some("feature-a")
    );
    assert_eq!(
        binding.map(|binding| binding.working_copy.id.as_str().to_owned()),
        Some("feature-a".to_string())
    );
}

#[tokio::test]
async fn sub_agents_can_run_with_bounded_working_copy_fanout() {
    let parent_environment = RuntimeEnvironment::default().with_cwd("/repo");
    let child_environment_a = RuntimeEnvironment::default().with_cwd("/repo.feature-a");
    let child_environment_b = RuntimeEnvironment::default().with_cwd("/repo.feature-b");
    let request_a = WorkingCopyIsolationRequest::Create(WorkingCopyCreateRequest::new(
        "marlin-core",
        WorkingCopyIsolationProvider::GitWorktree,
        "/repo",
        WorkingCopyHandle::new("feature-a", "/repo.feature-a"),
    ));
    let request_b = WorkingCopyIsolationRequest::Create(WorkingCopyCreateRequest::new(
        "marlin-core",
        WorkingCopyIsolationProvider::GitWorktree,
        "/repo",
        WorkingCopyHandle::new("feature-b", "/repo.feature-b"),
    ));
    let fanout = vec![
        WorkingCopySubAgentFanoutItem::new(
            "first".to_string(),
            child_environment_a.clone(),
            WorkingCopyIsolationReceipt::applied(&request_a),
        ),
        WorkingCopySubAgentFanoutItem::new(
            "second".to_string(),
            child_environment_b.clone(),
            WorkingCopyIsolationReceipt::applied(&request_b),
        ),
    ];
    let (runtime, _events) =
        TokioAgentRuntime::with_environment(4, CancellationToken::new(), parent_environment);

    let outputs = runtime
        .spawn_sub_agents_with_working_copy_environments(
            Arc::new(WorkingCopyFanoutEchoSubAgent),
            fanout,
            2,
        )
        .join()
        .await
        .expect("sub-agent fanout task should finish");

    assert_eq!(outputs.len(), 2);
    assert_eq!(outputs[0].0, "first");
    assert_eq!(outputs[0].1, child_environment_a);
    assert_eq!(outputs[0].2.len(), 1);
    assert_eq!(outputs[0].2[0].status, WorkingCopyIsolationStatus::Applied);
    assert_eq!(
        outputs[0].2[0]
            .working_copy
            .as_ref()
            .map(|copy| copy.id.as_str()),
        Some("feature-a")
    );
    assert_eq!(
        outputs[0]
            .3
            .as_ref()
            .map(|binding| binding.working_copy.id.as_str()),
        Some("feature-a")
    );
    assert_eq!(outputs[1].0, "second");
    assert_eq!(outputs[1].1, child_environment_b);
    assert_eq!(outputs[1].2.len(), 1);
    assert_eq!(outputs[1].2[0].status, WorkingCopyIsolationStatus::Applied);
    assert_eq!(
        outputs[1].2[0]
            .working_copy
            .as_ref()
            .map(|copy| copy.id.as_str()),
        Some("feature-b")
    );
    assert_eq!(
        outputs[1]
            .3
            .as_ref()
            .map(|binding| binding.working_copy.id.as_str()),
        Some("feature-b")
    );
}

#[derive(Clone, Debug)]
struct WorkingCopyEchoSubAgent;

impl SubAgentRuntime for WorkingCopyEchoSubAgent {
    type Input = ();
    type Output = (
        RuntimeEnvironment,
        Vec<WorkingCopyIsolationReceipt>,
        Option<WorkingCopyActiveBinding>,
    );

    fn run_sub_agent(
        &self,
        _input: Self::Input,
        context: RuntimeContext,
    ) -> RuntimeFuture<Self::Output> {
        let environment = context.environment().clone();
        let receipts = context.working_copy_receipts().to_vec();
        let binding = context.active_working_copy().cloned();
        Box::pin(async move { (environment, receipts, binding) })
    }
}

#[derive(Clone, Debug)]
struct WorkingCopyFanoutEchoSubAgent;

impl SubAgentRuntime for WorkingCopyFanoutEchoSubAgent {
    type Input = String;
    type Output = (
        String,
        RuntimeEnvironment,
        Vec<WorkingCopyIsolationReceipt>,
        Option<WorkingCopyActiveBinding>,
    );

    fn run_sub_agent(
        &self,
        input: Self::Input,
        context: RuntimeContext,
    ) -> RuntimeFuture<Self::Output> {
        let environment = context.environment().clone();
        let receipts = context.working_copy_receipts().to_vec();
        let binding = context.active_working_copy().cloned();
        Box::pin(async move { (input, environment, receipts, binding) })
    }
}
