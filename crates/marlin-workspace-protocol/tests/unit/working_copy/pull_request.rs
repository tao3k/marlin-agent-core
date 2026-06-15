use marlin_workspace_protocol::{
    WorkingCopyCommandProjection, WorkingCopyCommandProjectionError, WorkingCopyHandle,
    WorkingCopyIsolationOperationKind, WorkingCopyIsolationPlan, WorkingCopyIsolationPlanError,
    WorkingCopyIsolationPlanStep, WorkingCopyIsolationProvider, WorkingCopyIsolationReceipt,
    WorkingCopyIsolationRequest, WorkingCopyPullRequestCheckoutRequest,
    WorkingCopyPullRequestNumber, WorkspaceProjectGitHubRepository,
};

use super::test_git_toplevel_resolver;

#[test]
fn plan_allows_pull_request_checkout_without_discovery_path_until_runtime_projection() {
    let request = WorkingCopyIsolationRequest::PullRequestCheckout(
        WorkingCopyPullRequestCheckoutRequest::new(
            "worktrunk",
            WorkingCopyIsolationProvider::Worktrunk,
            WorkspaceProjectGitHubRepository::new("max-sixty", "worktrunk"),
            WorkingCopyPullRequestNumber::new(123),
        )
        .with_working_copy(WorkingCopyHandle::new("pr-123", "/repo.pr-123")),
    );
    let plan = WorkingCopyIsolationPlan::compile(&request).expect("worktrunk pr plan");

    assert!(matches!(
        plan.steps.as_slice(),
        [
            WorkingCopyIsolationPlanStep::PrepareTargetPath { .. },
            WorkingCopyIsolationPlanStep::WorktrunkPullRequestCheckout {
                repository_discovery_path: None,
                ..
            }
        ]
    ));

    let error = WorkingCopyCommandProjection::from_plan(&plan, test_git_toplevel_resolver)
        .expect_err("missing repository discovery path");
    assert_eq!(
        error,
        WorkingCopyCommandProjectionError::MissingRepositoryDiscoveryPath {
            provider: WorkingCopyIsolationProvider::Worktrunk,
            operation: WorkingCopyIsolationOperationKind::PullRequestCheckout
        }
    );
}

#[test]
fn plan_rejects_pull_request_checkout_for_native_git_worktree_provider() {
    let request = WorkingCopyIsolationRequest::PullRequestCheckout(
        WorkingCopyPullRequestCheckoutRequest::new(
            "marlin-core",
            WorkingCopyIsolationProvider::GitWorktree,
            WorkspaceProjectGitHubRepository::new("tao3k", "marlin-agent-core"),
            WorkingCopyPullRequestNumber::new(7),
        )
        .with_working_copy(WorkingCopyHandle::new("pr-7", "/repo.pr-7")),
    );
    let error = WorkingCopyIsolationPlan::compile(&request).expect_err("unsupported provider");

    assert_eq!(
        error,
        WorkingCopyIsolationPlanError::UnsupportedOperation {
            provider: WorkingCopyIsolationProvider::GitWorktree,
            operation: WorkingCopyIsolationOperationKind::PullRequestCheckout
        }
    );

    let receipt = WorkingCopyIsolationReceipt::rejected(&request, error.to_string());
    assert_eq!(receipt.command_receipts, Vec::new());
}
