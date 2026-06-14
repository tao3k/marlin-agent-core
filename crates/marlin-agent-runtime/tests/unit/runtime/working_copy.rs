use std::sync::Arc;

use marlin_agent_environment::{
    WorkingCopyCreateRequest, WorkingCopyHandle, WorkingCopyIsolationProvider,
    WorkingCopyIsolationReceipt, WorkingCopyIsolationRequest, WorkingCopyIsolationStatus,
};
use marlin_agent_runtime::{
    CancellationToken, RuntimeContext, RuntimeEnvironment, RuntimeFuture, SubAgentRuntime,
    TokioAgentRuntime,
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

    let (environment, receipts) = runtime
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
}

#[derive(Clone, Debug)]
struct WorkingCopyEchoSubAgent;

impl SubAgentRuntime for WorkingCopyEchoSubAgent {
    type Input = ();
    type Output = (RuntimeEnvironment, Vec<WorkingCopyIsolationReceipt>);

    fn run_sub_agent(
        &self,
        _input: Self::Input,
        context: RuntimeContext,
    ) -> RuntimeFuture<Self::Output> {
        let environment = context.environment().clone();
        let receipts = context.working_copy_receipts().to_vec();
        Box::pin(async move { (environment, receipts) })
    }
}
