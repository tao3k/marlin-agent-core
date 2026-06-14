use super::{ScriptedGitRepositoryResolver, ScriptedWorkingCopyRunner};
use marlin_agent_environment::{WorkingCopyCommandOutput, WorkingCopyIsolationDriver};
use marlin_workspace_protocol::{
    WorkingCopyBaseRef, WorkingCopyBranchName, WorkingCopyCommandProgram, WorkingCopyCreateRequest,
    WorkingCopyHandle, WorkingCopyId, WorkingCopyIsolationProvider, WorkingCopyIsolationRequest,
    WorkingCopyIsolationStatus, WorkingCopyListRequest, WorkingCopyPullRequestCheckoutRequest,
    WorkingCopyPullRequestNumber, WorkingCopyRemoveRequest, WorkingCopySwitchRequest,
    WorkspaceProjectGitHubRepository,
};
use std::path::PathBuf;

#[tokio::test]
async fn working_copy_driver_executes_git_worktree_create_projection() {
    let runner = ScriptedWorkingCopyRunner::new([WorkingCopyCommandOutput::succeeded("")]);
    let git_resolver = ScriptedGitRepositoryResolver::new("/repo");
    let driver = WorkingCopyIsolationDriver::with_runner_and_git_resolver(
        runner.clone(),
        git_resolver.clone(),
    );
    let request = WorkingCopyIsolationRequest::Create(
        WorkingCopyCreateRequest::new(
            "marlin-core",
            WorkingCopyIsolationProvider::GitWorktree,
            "/repo/subdir",
            WorkingCopyHandle::new("feature-a", "/repo.feature-a")
                .with_branch(WorkingCopyBranchName::new("feature/a")),
        )
        .with_base_ref(WorkingCopyBaseRef::new("origin/main")),
    );

    let result = driver.isolate(request).await;

    assert_eq!(result.receipt.status, WorkingCopyIsolationStatus::Applied);
    assert_eq!(result.receipt.command_receipts.len(), 1);
    assert_eq!(result.receipt.command_receipts[0].status_code, Some(0));
    assert_eq!(git_resolver.calls(), vec![PathBuf::from("/repo/subdir")]);
    assert_eq!(runner.calls().len(), 1);
    assert_eq!(runner.calls()[0].program, WorkingCopyCommandProgram::Git);
    assert_eq!(
        runner.calls()[0].git_toplevel.as_path(),
        PathBuf::from("/repo").as_path()
    );
    assert_eq!(
        runner.calls()[0].args,
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

#[tokio::test]
async fn working_copy_driver_executes_worktrunk_pull_request_checkout_projection() {
    let runner = ScriptedWorkingCopyRunner::new([WorkingCopyCommandOutput::succeeded("")]);
    let git_resolver = ScriptedGitRepositoryResolver::new("/repo");
    let driver = WorkingCopyIsolationDriver::with_runner_and_git_resolver(
        runner.clone(),
        git_resolver.clone(),
    );
    let request = WorkingCopyIsolationRequest::PullRequestCheckout(
        WorkingCopyPullRequestCheckoutRequest::new(
            "worktrunk",
            WorkingCopyIsolationProvider::Worktrunk,
            WorkspaceProjectGitHubRepository::new("max-sixty", "worktrunk"),
            WorkingCopyPullRequestNumber::new(123),
        )
        .with_repository_discovery_path("/repo/subdir")
        .with_working_copy(WorkingCopyHandle {
            id: WorkingCopyId::new("pr-123"),
            path: "/repo.pr-123".into(),
            branch: None,
        }),
    );

    let result = driver.isolate(request).await;

    assert_eq!(result.receipt.status, WorkingCopyIsolationStatus::Applied);
    assert_eq!(git_resolver.calls(), vec![PathBuf::from("/repo/subdir")]);
    assert_eq!(runner.calls().len(), 1);
    assert_eq!(
        runner.calls()[0].program,
        WorkingCopyCommandProgram::Worktrunk
    );
    assert_eq!(
        runner.calls()[0].git_toplevel.as_path(),
        PathBuf::from("/repo").as_path()
    );
    assert_eq!(
        runner.calls()[0].args,
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

#[tokio::test]
async fn working_copy_driver_executes_worktrunk_switch_list_and_remove_projection() {
    let runner = ScriptedWorkingCopyRunner::new([
        WorkingCopyCommandOutput::succeeded(r#"{"branch":"feature/a"}"#),
        WorkingCopyCommandOutput::succeeded(r#"[]"#),
        WorkingCopyCommandOutput::succeeded(r#"{"removed":true}"#),
    ]);
    let git_resolver = ScriptedGitRepositoryResolver::new("/repo");
    let driver = WorkingCopyIsolationDriver::with_runner_and_git_resolver(
        runner.clone(),
        git_resolver.clone(),
    );
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
        .including_branches(),
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

    let switch_result = driver.isolate(switch_request).await;
    let list_result = driver.isolate(list_request).await;
    let remove_result = driver.isolate(remove_request).await;

    assert_eq!(
        switch_result.receipt.status,
        WorkingCopyIsolationStatus::Applied
    );
    assert_eq!(
        list_result.receipt.status,
        WorkingCopyIsolationStatus::Applied
    );
    assert_eq!(
        remove_result.receipt.status,
        WorkingCopyIsolationStatus::Applied
    );
    assert_eq!(
        git_resolver.calls(),
        vec![
            PathBuf::from("/repo/subdir"),
            PathBuf::from("/repo/subdir"),
            PathBuf::from("/repo/subdir")
        ]
    );
    let calls = runner.calls();
    assert_eq!(
        calls[0].args,
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
        calls[1].args,
        vec![
            "list".to_string(),
            "--format".to_string(),
            "json".to_string(),
            "--branches".to_string()
        ]
    );
    assert_eq!(
        calls[2].args,
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

#[tokio::test]
async fn working_copy_driver_rejects_failed_provider_command_with_receipt() {
    let runner = ScriptedWorkingCopyRunner::new([WorkingCopyCommandOutput::failed(
        Some(128),
        "path already exists",
    )]);
    let git_resolver = ScriptedGitRepositoryResolver::new("/repo");
    let driver = WorkingCopyIsolationDriver::with_runner_and_git_resolver(runner, git_resolver);
    let request = WorkingCopyIsolationRequest::Create(WorkingCopyCreateRequest::new(
        "marlin-core",
        WorkingCopyIsolationProvider::GitWorktree,
        "/repo",
        WorkingCopyHandle::new("feature-a", "/repo.feature-a"),
    ));

    let result = driver.isolate(request).await;

    assert_eq!(result.receipt.status, WorkingCopyIsolationStatus::Rejected);
    assert_eq!(
        result.receipt.reason.as_deref(),
        Some("working-copy provider command failed")
    );
    assert_eq!(result.receipt.command_receipts.len(), 1);
    assert_eq!(result.receipt.command_receipts[0].status_code, Some(128));
    assert_eq!(
        result.receipt.command_receipts[0].stderr.as_deref(),
        Some("path already exists")
    );
}

#[tokio::test]
async fn working_copy_driver_rejects_pull_request_checkout_without_discovery_path() {
    let runner = ScriptedWorkingCopyRunner::new([WorkingCopyCommandOutput::succeeded("")]);
    let git_resolver = ScriptedGitRepositoryResolver::new("/repo");
    let driver = WorkingCopyIsolationDriver::with_runner_and_git_resolver(runner, git_resolver);
    let request = WorkingCopyIsolationRequest::PullRequestCheckout(
        WorkingCopyPullRequestCheckoutRequest::new(
            "worktrunk",
            WorkingCopyIsolationProvider::Worktrunk,
            WorkspaceProjectGitHubRepository::new("max-sixty", "worktrunk"),
            WorkingCopyPullRequestNumber::new(123),
        )
        .with_working_copy(WorkingCopyHandle::new("pr-123", "/repo.pr-123")),
    );

    let result = driver.isolate(request).await;

    assert_eq!(result.receipt.status, WorkingCopyIsolationStatus::Rejected);
    assert_eq!(
        result.receipt.reason.as_deref(),
        Some(
            "working-copy command projection requires repository discovery path for Worktrunk PullRequestCheckout"
        )
    );
    assert!(result.receipt.command_receipts.is_empty());
}
