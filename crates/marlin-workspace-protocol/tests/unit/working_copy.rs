use std::path::PathBuf;

use marlin_workspace_protocol::{
    WorkingCopyBaseRef, WorkingCopyBranchName, WorkingCopyCreateRequest, WorkingCopyHandle,
    WorkingCopyId, WorkingCopyIsolationOperationKind, WorkingCopyIsolationProvider,
    WorkingCopyIsolationReceipt, WorkingCopyIsolationRequest, WorkingCopyIsolationStatus,
    WorkingCopyPullRequestCheckoutRequest, WorkingCopyPullRequestNumber,
    WorkspaceProjectGitHubRepository,
};

#[test]
fn git_worktree_provider_plans_baseline_working_copy_isolation() {
    let handle = WorkingCopyHandle::new("feature-a", "/repo.feature-a")
        .with_branch(WorkingCopyBranchName::new("feature/a"));
    let request = WorkingCopyIsolationRequest::Create(
        WorkingCopyCreateRequest::new(
            "marlin-core",
            WorkingCopyIsolationProvider::GitWorktree,
            "/repo",
            handle,
        )
        .with_base_ref(WorkingCopyBaseRef::new("origin/main")),
    );
    let receipt = WorkingCopyIsolationReceipt::planned(&request);

    assert!(
        request
            .provider()
            .supports(&WorkingCopyIsolationOperationKind::Create)
    );
    assert!(
        !request
            .provider()
            .supports(&WorkingCopyIsolationOperationKind::PullRequestCheckout)
    );
    assert_eq!(receipt.provider, WorkingCopyIsolationProvider::GitWorktree);
    assert_eq!(receipt.operation, WorkingCopyIsolationOperationKind::Create);
    assert_eq!(receipt.status, WorkingCopyIsolationStatus::Planned);
    assert_eq!(receipt.project_id.as_str(), "marlin-core");
    assert_eq!(
        receipt.working_copy.as_ref().map(|copy| copy.id.as_str()),
        Some("feature-a")
    );
    assert_eq!(
        receipt
            .working_copy
            .as_ref()
            .and_then(|copy| copy.branch.as_ref())
            .map(|branch| branch.as_str()),
        Some("feature/a")
    );
}

#[test]
fn worktrunk_provider_can_plan_github_pull_request_checkout() {
    let handle = WorkingCopyHandle {
        id: WorkingCopyId::new("pr-123"),
        path: PathBuf::from("/repo.pr-123"),
        branch: None,
    };
    let request = WorkingCopyIsolationRequest::PullRequestCheckout(
        WorkingCopyPullRequestCheckoutRequest::new(
            "worktrunk",
            WorkingCopyIsolationProvider::Worktrunk,
            WorkspaceProjectGitHubRepository::new("max-sixty", "worktrunk"),
            WorkingCopyPullRequestNumber::new(123),
        )
        .with_working_copy(handle),
    );
    let receipt = WorkingCopyIsolationReceipt::applied(&request);

    assert!(
        request
            .provider()
            .supports(&WorkingCopyIsolationOperationKind::PullRequestCheckout)
    );
    assert!(
        request
            .provider()
            .supports(&WorkingCopyIsolationOperationKind::Merge)
    );
    assert_eq!(receipt.provider, WorkingCopyIsolationProvider::Worktrunk);
    assert_eq!(
        receipt.operation,
        WorkingCopyIsolationOperationKind::PullRequestCheckout
    );
    assert_eq!(receipt.status, WorkingCopyIsolationStatus::Applied);
    assert_eq!(
        receipt
            .working_copy
            .as_ref()
            .map(|copy| copy.path.as_path()),
        Some(PathBuf::from("/repo.pr-123").as_path())
    );
}

#[test]
fn rejected_receipt_preserves_provider_and_reason() {
    let request = WorkingCopyIsolationRequest::Create(WorkingCopyCreateRequest::new(
        "marlin-core",
        WorkingCopyIsolationProvider::GitWorktree,
        "/repo",
        WorkingCopyHandle::new("unsafe", "/tmp/unsafe"),
    ));
    let receipt = WorkingCopyIsolationReceipt::rejected(&request, "target path denied");

    assert_eq!(receipt.status, WorkingCopyIsolationStatus::Rejected);
    assert_eq!(receipt.provider, WorkingCopyIsolationProvider::GitWorktree);
    assert_eq!(receipt.reason.as_deref(), Some("target path denied"));
}
