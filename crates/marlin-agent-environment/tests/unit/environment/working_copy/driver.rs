// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use super::{ScriptedGitRepositoryResolver, ScriptedWorkingCopyRunner};
use marlin_agent_environment::{WorkingCopyCommandOutput, WorkingCopyIsolationDriver};
use marlin_agent_gitops::working_copy::{
    WorkingCopyActiveBinding, WorkingCopyBaseRef, WorkingCopyBranchName, WorkingCopyCommandProgram,
    WorkingCopyCreateRequest, WorkingCopyFinalizeBranchRequest, WorkingCopyHandle, WorkingCopyId,
    WorkingCopyIsolationProvider, WorkingCopyIsolationRequest, WorkingCopyIsolationStatus,
    WorkingCopyListRequest, WorkingCopyPullRequestCheckoutRequest, WorkingCopyPullRequestNumber,
    WorkingCopyRemoveRequest, WorkingCopySwitchRequest, WorkspaceProjectGitHubRepository,
};
use std::{
    path::{Path, PathBuf},
    process::Command,
};

#[tokio::test]
async fn git_branch_changes_require_a_worktree_of_the_selected_repository() {
    let temp = tempfile::tempdir().unwrap();
    let parent = temp.path().canonicalize().unwrap();
    let repo = parent.join("project");
    let foreign = parent.join("foreign");
    for path in [&repo, &foreign] {
        std::fs::create_dir(path).unwrap();
        git(path, &["init", "-q", "-b", "main"]);
        git(
            path,
            &[
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@example.test",
                "commit",
                "-qm",
                "fixture",
                "--allow-empty",
            ],
        );
        git(path, &["branch", "switch-target"]);
    }
    let linked = parent.join("linked");
    git(
        &repo,
        &[
            "worktree",
            "add",
            "--detach",
            linked.to_str().unwrap(),
            "HEAD",
        ],
    );
    let driver = WorkingCopyIsolationDriver::new();
    let provider = WorkingCopyIsolationProvider::GitWorktree;
    let switch = |target: &Path| {
        WorkingCopyIsolationRequest::Switch(WorkingCopySwitchRequest::new(
            "project",
            provider.clone(),
            repo.as_path(),
            WorkingCopyHandle::new("copy", target).with_branch("switch-target"),
        ))
    };
    let finalize = |target: &Path| {
        WorkingCopyIsolationRequest::FinalizeBranch(
            WorkingCopyFinalizeBranchRequest::new(
                "project",
                provider.clone(),
                WorkingCopyHandle::new("copy", target),
                "finalized",
            )
            .with_repository_discovery_path(repo.as_path()),
        )
    };
    let remove = |target: &Path| {
        WorkingCopyIsolationRequest::Remove(WorkingCopyRemoveRequest::new(
            "project",
            provider.clone(),
            repo.as_path(),
            WorkingCopyHandle::new("copy", target),
        ))
    };
    let nested = linked.join("nested");
    std::fs::create_dir(&nested).unwrap();
    for request in [
        switch(&foreign),
        finalize(&foreign),
        switch(&nested),
        remove(&foreign),
        remove(&nested),
    ] {
        let receipt = driver.isolate(request).await.receipt;
        assert_eq!(receipt.status, WorkingCopyIsolationStatus::Rejected);
        assert!(receipt.command_receipts.is_empty());
        assert!(
            receipt
                .reason
                .unwrap()
                .contains("not a worktree of the selected Git repository")
        );
    }
    assert_eq!(git(&foreign, &["branch", "--show-current"]).trim(), "main");
    assert!(foreign.join(".git").exists());
    assert!(linked.join(".git").is_file());
    assert!(git(&foreign, &["branch", "--list", "finalized"]).is_empty());
    assert!(git(&linked, &["branch", "--show-current"]).is_empty());

    for (request, branch) in [
        (switch(Path::new("../linked")), "switch-target"),
        (finalize(&linked), "finalized"),
    ] {
        let receipt = driver.isolate(request).await.receipt;
        assert_eq!(
            receipt.status,
            WorkingCopyIsolationStatus::Applied,
            "{receipt:?}"
        );
        let copy = receipt.working_copy.unwrap();
        assert_eq!(copy.path, linked);
        assert_eq!(copy.branch.unwrap().as_str(), branch);
        assert_eq!(git(&linked, &["branch", "--show-current"]).trim(), branch);
    }
    let removed = driver.isolate(remove(Path::new("../linked"))).await.receipt;
    assert_eq!(removed.status, WorkingCopyIsolationStatus::Applied);
    assert_eq!(removed.working_copy.unwrap().path, linked);
    assert!(!linked.exists());
}

fn git(path: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(path)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

#[tokio::test]
#[ignore = "requires Worktrunk on PATH"]
async fn worktrunk_reports_the_registered_worktree_and_removes_it_by_path() {
    let temp = tempfile::tempdir().unwrap();
    let parent = temp.path().canonicalize().unwrap();
    let repository = parent.join("project");
    std::fs::create_dir(&repository).unwrap();
    git(&repository, &["init", "-q", "-b", "main"]);
    git(
        &repository,
        &[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.test",
            "commit",
            "-qm",
            "fixture",
            "--allow-empty",
        ],
    );

    let driver = WorkingCopyIsolationDriver::new();
    let provider = WorkingCopyIsolationProvider::Worktrunk;
    let placeholder = WorkingCopyHandle::new("agent", parent.join("unused")).with_branch("agent/a");
    let request = WorkingCopyIsolationRequest::Create(
        WorkingCopyCreateRequest::new(
            "project",
            provider.clone(),
            repository.as_path(),
            placeholder.clone(),
        )
        .with_base_ref("HEAD"),
    );
    let receipt = driver.isolate(request).await.receipt;
    assert_eq!(
        receipt.status,
        WorkingCopyIsolationStatus::Applied,
        "{receipt:?}"
    );
    let active = WorkingCopyActiveBinding::from_receipt(&receipt).unwrap();
    assert_ne!(active.working_copy.path, placeholder.path);
    assert_eq!(active.working_copy.branch, placeholder.branch);
    assert_eq!(
        git(&active.working_copy.path, &["branch", "--show-current"]).trim(),
        "agent/a"
    );
    assert_eq!(
        receipt.command_receipts[0].working_copy.as_ref(),
        Some(&active.working_copy)
    );
    let registered_path = format!("worktree {}", active.working_copy.path.display());
    assert!(
        git(&repository, &["worktree", "list", "--porcelain"])
            .lines()
            .any(|line| line == registered_path)
    );

    let remove = |working_copy| {
        WorkingCopyIsolationRequest::Remove(WorkingCopyRemoveRequest::new(
            "project",
            provider.clone(),
            repository.as_path(),
            working_copy,
        ))
    };
    let rejected = driver.isolate(remove(placeholder)).await.receipt;
    assert_eq!(
        rejected.status,
        WorkingCopyIsolationStatus::Rejected,
        "{rejected:?}"
    );
    assert!(rejected.command_receipts.is_empty());
    assert!(active.working_copy.path.is_dir());

    let removed = driver
        .isolate(remove(active.working_copy.clone()))
        .await
        .receipt;
    assert_eq!(
        removed.status,
        WorkingCopyIsolationStatus::Applied,
        "{removed:?}"
    );
    assert_eq!(removed.working_copy.as_ref(), Some(&active.working_copy));
    assert!(WorkingCopyActiveBinding::from_receipt(&removed).is_none());
    assert!(!active.working_copy.path.exists());
    assert!(
        !git(&repository, &["worktree", "list", "--porcelain"])
            .lines()
            .any(|line| line == registered_path)
    );
}

#[tokio::test]
async fn pull_request_checkout_refuses_a_different_fetch_repository() {
    let worktree = tempfile::tempdir().unwrap();
    let path = worktree.path();
    git(path, &["init", "-q"]);
    let url = "https://github.com/expected/project.git";
    git(path, &["remote", "add", "origin", url]);
    git(
        path,
        &[
            "config",
            "url.https://github.com/other/project.git.insteadOf",
            url,
        ],
    );
    let request = WorkingCopyIsolationRequest::PullRequestCheckout(
        WorkingCopyPullRequestCheckoutRequest::new(
            "project",
            WorkingCopyIsolationProvider::Worktrunk,
            WorkspaceProjectGitHubRepository::new("expected", "project"),
            WorkingCopyPullRequestNumber::new(123),
        )
        .with_repository_discovery_path(path),
    );
    let receipt = WorkingCopyIsolationDriver::new()
        .isolate(request)
        .await
        .receipt;

    assert_eq!(receipt.status, WorkingCopyIsolationStatus::Rejected);
    assert!(receipt.command_receipts.is_empty());
    assert_eq!(
        receipt.reason.as_deref(),
        Some(
            "Worktrunk GitHub checkout could not verify the requested repository expected/project"
        ),
    );
}

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
            "-c".to_string(),
            if cfg!(windows) {
                "core.hooksPath=NUL"
            } else {
                "core.hooksPath=/dev/null"
            }
            .to_string(),
            "worktree".to_string(),
            "add".to_string(),
            "-b".to_string(),
            "feature/a".to_string(),
            "--".to_string(),
            "/repo.feature-a".to_string(),
            "origin/main".to_string()
        ]
    );
}

#[tokio::test]
async fn working_copy_driver_executes_worktrunk_pull_request_checkout_projection() {
    let runner = ScriptedWorkingCopyRunner::new([WorkingCopyCommandOutput::succeeded(
        r#"{"action":"created","branch":"fix/123","path":"/copies/fix-123"}"#,
    )]);
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
    assert_eq!(
        result.receipt.working_copy,
        Some(WorkingCopyHandle::new("pr-123", "/copies/fix-123").with_branch("fix/123")),
    );
    assert_eq!(git_resolver.calls(), vec![PathBuf::from("/repo/subdir")]);
    assert_eq!(runner.calls().len(), 1);
    assert_eq!(
        runner.calls()[0].program,
        WorkingCopyCommandProgram::Worktrunk
    );
    assert_eq!(
        runner.calls()[0].github_repository,
        Some(WorkspaceProjectGitHubRepository::new(
            "max-sixty",
            "worktrunk"
        )),
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
        WorkingCopyCommandOutput::succeeded(
            r#"{"action":"created","branch":"feature/a","path":"/copies/feature-a"}"#,
        ),
        WorkingCopyCommandOutput::succeeded(r#"[]"#),
        WorkingCopyCommandOutput::succeeded(
            r#"[{"kind":"worktree","branch":"feature/a","path":"/copies/feature-a"}]"#,
        ),
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
    let switch_result = driver.isolate(switch_request).await;
    let expected_copy =
        WorkingCopyHandle::new("feature-a", "/copies/feature-a").with_branch("feature/a");
    assert_eq!(
        switch_result.receipt.working_copy.as_ref(),
        Some(&expected_copy)
    );
    assert_eq!(
        switch_result.receipt.command_receipts[0]
            .working_copy
            .as_ref(),
        Some(&expected_copy),
    );
    let active = WorkingCopyActiveBinding::from_receipt(&switch_result.receipt)
        .expect("switch should bind the provider-reported working copy");
    let remove_request = WorkingCopyIsolationRequest::Remove(
        WorkingCopyRemoveRequest::new(
            "worktrunk",
            WorkingCopyIsolationProvider::Worktrunk,
            "/repo/subdir",
            active.working_copy,
        )
        .forcing_removal(),
    );

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
    assert_eq!(remove_result.receipt.working_copy, Some(expected_copy));
    assert!(WorkingCopyActiveBinding::from_receipt(&remove_result.receipt).is_none());
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
            "--".to_string(),
            "/copies/feature-a".to_string()
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
    assert!(result.receipt.command_receipts[0].working_copy.is_none());
    assert_eq!(
        result.receipt.command_receipts[0].stderr.as_deref(),
        Some("path already exists")
    );
}

#[tokio::test]
async fn working_copy_driver_rejects_success_without_a_worktrunk_path() {
    let stdout = r#"{"action":"existing","branch":"feature/a"}"#;
    let runner = ScriptedWorkingCopyRunner::new([WorkingCopyCommandOutput::succeeded(stdout)]);
    let git_resolver = ScriptedGitRepositoryResolver::new("/repo");
    let driver = WorkingCopyIsolationDriver::with_runner_and_git_resolver(runner, git_resolver);
    let request = WorkingCopyIsolationRequest::Switch(WorkingCopySwitchRequest::new(
        "worktrunk",
        WorkingCopyIsolationProvider::Worktrunk,
        "/repo",
        WorkingCopyHandle::new("feature-a", "/expected").with_branch("feature/a"),
    ));

    let result = driver.isolate(request).await.receipt;

    assert_eq!(result.status, WorkingCopyIsolationStatus::Rejected);
    assert!(
        result
            .reason
            .as_deref()
            .is_some_and(|reason| reason.contains("missing field `path`"))
    );
    assert_eq!(result.command_receipts[0].stdout.as_deref(), Some(stdout));
    assert!(result.command_receipts[0].working_copy.is_none());
    assert!(WorkingCopyActiveBinding::from_receipt(&result).is_none());
}

#[tokio::test]
async fn working_copy_driver_rejects_a_worktrunk_receipt_for_a_different_removal_path() {
    let stdout = r#"[{"kind":"worktree","branch":"feature/a","path":"/copies/different"}]"#;
    let runner = ScriptedWorkingCopyRunner::new([WorkingCopyCommandOutput::succeeded(stdout)]);
    let git_resolver = ScriptedGitRepositoryResolver::new("/repo");
    let driver = WorkingCopyIsolationDriver::with_runner_and_git_resolver(runner, git_resolver);
    let request = WorkingCopyIsolationRequest::Remove(WorkingCopyRemoveRequest::new(
        "worktrunk",
        WorkingCopyIsolationProvider::Worktrunk,
        "/repo",
        WorkingCopyHandle::new("feature-a", "/copies/expected").with_branch("feature/a"),
    ));

    let receipt = driver.isolate(request).await.receipt;

    assert_eq!(receipt.status, WorkingCopyIsolationStatus::Rejected);
    assert_eq!(
        receipt.reason.as_deref(),
        Some("Worktrunk reported removing a different worktree")
    );
    assert_eq!(receipt.command_receipts[0].stdout.as_deref(), Some(stdout));
    assert!(receipt.command_receipts[0].working_copy.is_none());
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
