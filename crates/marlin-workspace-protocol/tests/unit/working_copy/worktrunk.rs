use std::path::PathBuf;

use marlin_workspace_protocol::{
    WorkingCopyBaseRef, WorkingCopyBranchName, WorkingCopyCommandProgram,
    WorkingCopyCommandProjection, WorkingCopyHandle, WorkingCopyId,
    WorkingCopyIsolationOperationKind, WorkingCopyIsolationPlan, WorkingCopyIsolationPlanStep,
    WorkingCopyIsolationProvider, WorkingCopyIsolationReceipt, WorkingCopyIsolationRequest,
    WorkingCopyIsolationStatus, WorkingCopyListRequest, WorkingCopyPullRequestCheckoutRequest,
    WorkingCopyPullRequestNumber, WorkingCopyRemoveRequest, WorkingCopySwitchRequest,
    WorkspaceProjectGitHubRepository,
};

use super::test_git_toplevel_resolver;

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
