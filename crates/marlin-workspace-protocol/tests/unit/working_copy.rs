use std::path::PathBuf;

use marlin_workspace_protocol::{
    WorkingCopyBaseRef, WorkingCopyBranchName, WorkingCopyCommandInvocation,
    WorkingCopyCommandProgram, WorkingCopyCommandProjection, WorkingCopyCommandProjectionError,
    WorkingCopyCommandReceipt, WorkingCopyCommandStatus, WorkingCopyCreateRequest,
    WorkingCopyGitTopLevel, WorkingCopyHandle, WorkingCopyId, WorkingCopyIsolationOperationKind,
    WorkingCopyIsolationPlan, WorkingCopyIsolationPlanError, WorkingCopyIsolationPlanStep,
    WorkingCopyIsolationProvider, WorkingCopyIsolationReceipt, WorkingCopyIsolationRequest,
    WorkingCopyIsolationStatus, WorkingCopyListRequest, WorkingCopyPullRequestCheckoutRequest,
    WorkingCopyPullRequestNumber, WorkingCopyRemoveRequest, WorkingCopyRepositoryDiscoveryPath,
    WorkingCopySwitchRequest, WorkspaceProjectGitHubRepository,
};

fn test_git_toplevel_resolver(
    path: &WorkingCopyRepositoryDiscoveryPath,
) -> Result<WorkingCopyGitTopLevel, WorkingCopyCommandProjectionError> {
    let resolved = if path.as_path() == PathBuf::from("/repo/subdir").as_path() {
        PathBuf::from("/repo")
    } else {
        path.as_path().to_path_buf()
    };
    Ok(WorkingCopyGitTopLevel::from_resolved_path(resolved))
}

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
            WorkingCopyIsolationPlanStep::PrepareTargetPath { path },
            WorkingCopyIsolationPlanStep::GitWorktreeCreate {
                repository_discovery_path,
                ..
            }
        ] if path == &PathBuf::from("/repo.feature-a")
            && repository_discovery_path.as_path() == PathBuf::from("/repo").as_path()
    ));

    let projection = WorkingCopyCommandProjection::from_plan(&plan, test_git_toplevel_resolver)
        .expect("git command projection");
    assert_eq!(
        projection.preflight_paths,
        vec![PathBuf::from("/repo.feature-a")]
    );
    assert_eq!(projection.commands.len(), 1);
    assert_eq!(
        projection.commands[0].program,
        WorkingCopyCommandProgram::Git
    );
    assert_eq!(
        projection.commands[0].git_toplevel.as_path(),
        PathBuf::from("/repo").as_path()
    );
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

    let invocation = WorkingCopyCommandInvocation::new(
        WorkingCopyCommandProgram::Git,
        WorkingCopyGitTopLevel::from_resolved_path(PathBuf::from("/repo")),
    )
    .with_args([
        "worktree",
        "add",
        "-b",
        "feature/a",
        "/repo.feature-a",
        "origin/main",
    ])
    .with_expected_working_copy(
        WorkingCopyHandle::new("feature-a", "/repo.feature-a")
            .with_branch(WorkingCopyBranchName::new("feature/a")),
    );
    let receipt = WorkingCopyIsolationReceipt::applied(&request).with_command_receipts([
        WorkingCopyCommandReceipt::succeeded(&invocation, Some(0), "", ""),
    ]);
    assert_eq!(receipt.command_receipts.len(), 1);
    assert_eq!(
        receipt.command_receipts[0].status,
        WorkingCopyCommandStatus::Succeeded
    );
    assert_eq!(
        receipt.command_receipts[0]
            .working_copy
            .as_ref()
            .map(|copy| copy.id.as_str()),
        Some("feature-a")
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
        .with_repository_discovery_path("/repo/subdir")
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
            WorkingCopyIsolationPlanStep::WorktrunkPullRequestCheckout {
                repository_discovery_path: Some(repository_discovery_path),
                ..
            }
        ] if repository_discovery_path.as_path() == PathBuf::from("/repo/subdir").as_path()
    ));

    let projection = WorkingCopyCommandProjection::from_plan(&plan, test_git_toplevel_resolver)
        .expect("worktrunk command projection");
    assert_eq!(projection.commands.len(), 1);
    assert_eq!(
        projection.commands[0].program,
        WorkingCopyCommandProgram::Worktrunk
    );
    assert_eq!(
        projection.commands[0].git_toplevel.as_path(),
        PathBuf::from("/repo").as_path()
    );
    assert_eq!(
        projection.commands[0].args,
        vec![
            "switch".to_string(),
            "--no-cd".to_string(),
            "--no-hooks".to_string(),
            "--format".to_string(),
            "json".to_string(),
            "pr:123".to_string()
        ]
    );
}

#[test]
fn git_worktree_provider_projects_switch_list_and_remove_commands() {
    let switch_request = WorkingCopyIsolationRequest::Switch(WorkingCopySwitchRequest::new(
        "marlin-core",
        WorkingCopyIsolationProvider::GitWorktree,
        "/repo/subdir",
        WorkingCopyHandle::new("feature-a", "/repo.feature-a")
            .with_branch(WorkingCopyBranchName::new("feature/a")),
    ));
    let list_request = WorkingCopyIsolationRequest::List(WorkingCopyListRequest::new(
        "marlin-core",
        WorkingCopyIsolationProvider::GitWorktree,
        "/repo/subdir",
    ));
    let remove_request = WorkingCopyIsolationRequest::Remove(
        WorkingCopyRemoveRequest::new(
            "marlin-core",
            WorkingCopyIsolationProvider::GitWorktree,
            "/repo/subdir",
            WorkingCopyHandle::new("feature-a", "/repo.feature-a")
                .with_branch(WorkingCopyBranchName::new("feature/a")),
        )
        .forcing_removal(),
    );

    let switch_projection = WorkingCopyCommandProjection::from_plan(
        &WorkingCopyIsolationPlan::compile(&switch_request).expect("switch plan"),
        test_git_toplevel_resolver,
    )
    .expect("switch projection");
    let list_projection = WorkingCopyCommandProjection::from_plan(
        &WorkingCopyIsolationPlan::compile(&list_request).expect("list plan"),
        test_git_toplevel_resolver,
    )
    .expect("list projection");
    let remove_projection = WorkingCopyCommandProjection::from_plan(
        &WorkingCopyIsolationPlan::compile(&remove_request).expect("remove plan"),
        test_git_toplevel_resolver,
    )
    .expect("remove projection");

    assert_eq!(
        switch_projection.commands[0].args,
        vec![
            "-C".to_string(),
            "/repo.feature-a".to_string(),
            "switch".to_string(),
            "feature/a".to_string()
        ]
    );
    assert_eq!(
        list_projection.commands[0].args,
        vec![
            "worktree".to_string(),
            "list".to_string(),
            "--porcelain".to_string()
        ]
    );
    assert_eq!(
        remove_projection.commands[0].args,
        vec![
            "worktree".to_string(),
            "remove".to_string(),
            "--force".to_string(),
            "/repo.feature-a".to_string()
        ]
    );
}

#[test]
fn worktrunk_provider_projects_agent_automation_commands() {
    let switch_request = WorkingCopyIsolationRequest::Switch(
        WorkingCopySwitchRequest::new(
            "worktrunk",
            WorkingCopyIsolationProvider::Worktrunk,
            "/repo/subdir",
            WorkingCopyHandle::new("feature-a", "/repo.feature-a")
                .with_branch(WorkingCopyBranchName::new("feature/a")),
        )
        .create_if_missing_from(WorkingCopyBaseRef::new("origin/main")),
    );
    let list_request = WorkingCopyIsolationRequest::List(
        WorkingCopyListRequest::new(
            "worktrunk",
            WorkingCopyIsolationProvider::Worktrunk,
            "/repo/subdir",
        )
        .including_branches()
        .including_remotes(),
    );
    let remove_request = WorkingCopyIsolationRequest::Remove(
        WorkingCopyRemoveRequest::new(
            "worktrunk",
            WorkingCopyIsolationProvider::Worktrunk,
            "/repo/subdir",
            WorkingCopyHandle::new("feature-a", "/repo.feature-a")
                .with_branch(WorkingCopyBranchName::new("feature/a")),
        )
        .forcing_removal(),
    );

    let switch_projection = WorkingCopyCommandProjection::from_plan(
        &WorkingCopyIsolationPlan::compile(&switch_request).expect("switch plan"),
        test_git_toplevel_resolver,
    )
    .expect("switch projection");
    let list_projection = WorkingCopyCommandProjection::from_plan(
        &WorkingCopyIsolationPlan::compile(&list_request).expect("list plan"),
        test_git_toplevel_resolver,
    )
    .expect("list projection");
    let remove_projection = WorkingCopyCommandProjection::from_plan(
        &WorkingCopyIsolationPlan::compile(&remove_request).expect("remove plan"),
        test_git_toplevel_resolver,
    )
    .expect("remove projection");

    assert_eq!(
        switch_projection.commands[0].args,
        vec![
            "switch".to_string(),
            "--no-cd".to_string(),
            "--no-hooks".to_string(),
            "--format".to_string(),
            "json".to_string(),
            "--create".to_string(),
            "--base".to_string(),
            "origin/main".to_string(),
            "feature/a".to_string()
        ]
    );
    assert_eq!(
        list_projection.commands[0].args,
        vec![
            "list".to_string(),
            "--format".to_string(),
            "json".to_string(),
            "--branches".to_string(),
            "--remotes".to_string()
        ]
    );
    assert_eq!(
        remove_projection.commands[0].args,
        vec![
            "remove".to_string(),
            "--foreground".to_string(),
            "--no-hooks".to_string(),
            "--format".to_string(),
            "json".to_string(),
            "--force".to_string(),
            "feature/a".to_string()
        ]
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
