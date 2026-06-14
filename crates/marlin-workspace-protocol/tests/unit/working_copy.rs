use std::path::PathBuf;

use marlin_workspace_protocol::{
    WorkingCopyBaseRef, WorkingCopyBranchName, WorkingCopyCommandProgram,
    WorkingCopyCommandProjection, WorkingCopyCommandProjectionError, WorkingCopyCreateRequest,
    WorkingCopyHandle, WorkingCopyId, WorkingCopyIsolationOperationKind, WorkingCopyIsolationPlan,
    WorkingCopyIsolationPlanError, WorkingCopyIsolationPlanStep, WorkingCopyIsolationProvider,
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

    let plan = WorkingCopyIsolationPlan::compile(&request).expect("git worktree plan");
    assert_eq!(plan.provider, WorkingCopyIsolationProvider::GitWorktree);
    assert_eq!(plan.operation, WorkingCopyIsolationOperationKind::Create);
    assert!(matches!(
        plan.steps.as_slice(),
        [
            WorkingCopyIsolationPlanStep::PrepareTargetPath { .. },
            WorkingCopyIsolationPlanStep::GitWorktreeCreate { .. }
        ]
    ));

    let projection =
        WorkingCopyCommandProjection::from_plan(&plan).expect("git command projection");
    assert_eq!(
        projection.preflight_paths,
        vec![PathBuf::from("/repo.feature-a")]
    );
    assert_eq!(projection.commands.len(), 1);
    assert_eq!(
        projection.commands[0].program,
        WorkingCopyCommandProgram::Git
    );
    assert_eq!(projection.commands[0].cwd, PathBuf::from("/repo"));
    assert_eq!(
        projection.commands[0].args,
        vec![
            "worktree".to_string(),
            "add".to_string(),
            "-b".to_string(),
            "feature/a".to_string(),
            "/repo.feature-a".to_string(),
            "origin/main".to_string()
        ]
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
        .with_repository_root("/repo")
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

    let plan = WorkingCopyIsolationPlan::compile(&request).expect("worktrunk pr plan");
    assert_eq!(plan.provider, WorkingCopyIsolationProvider::Worktrunk);
    assert_eq!(
        plan.operation,
        WorkingCopyIsolationOperationKind::PullRequestCheckout
    );
    assert!(matches!(
        plan.steps.as_slice(),
        [
            WorkingCopyIsolationPlanStep::PrepareTargetPath { .. },
            WorkingCopyIsolationPlanStep::WorktrunkPullRequestCheckout { .. }
        ]
    ));

    let projection =
        WorkingCopyCommandProjection::from_plan(&plan).expect("worktrunk command projection");
    assert_eq!(projection.commands.len(), 1);
    assert_eq!(
        projection.commands[0].program,
        WorkingCopyCommandProgram::Worktrunk
    );
    assert_eq!(projection.commands[0].cwd, PathBuf::from("/repo"));
    assert_eq!(
        projection.commands[0].args,
        vec!["switch".to_string(), "pr:123".to_string()]
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

#[test]
fn command_projection_requires_repository_root_for_worktrunk_pull_request_checkout() {
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
    let error =
        WorkingCopyCommandProjection::from_plan(&plan).expect_err("missing repository root");

    assert_eq!(
        error,
        WorkingCopyCommandProjectionError::MissingRepositoryRoot {
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
}
