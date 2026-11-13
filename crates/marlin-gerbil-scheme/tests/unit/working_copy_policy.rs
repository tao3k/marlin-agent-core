// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use marlin_agent_gitops::working_copy::{
    WorkingCopyBaseRef, WorkingCopyBranchName, WorkingCopyHandle,
    WorkingCopyIsolationOperationKind, WorkingCopyIsolationPlan, WorkingCopyIsolationPlanStep,
    WorkingCopyIsolationProvider, WorkingCopyIsolationRequest, WorkingCopyListOptions,
    WorkingCopyPullRequestNumber, WorkingCopyRemovalMode, WorkingCopySwitchMode,
    WorkspaceProjectGitHubRepository,
};
use marlin_gerbil_scheme::{GerbilWorkingCopyPolicyOperation, GerbilWorkingCopyPolicySelection};
use std::path::PathBuf;

#[test]
fn gerbil_working_copy_policy_projects_create_into_typed_git_worktree_request() {
    let request = GerbilWorkingCopyPolicySelection::new(
        "marlin-core",
        WorkingCopyIsolationProvider::GitWorktree,
        "/repo/nested/crate",
        GerbilWorkingCopyPolicyOperation::Create {
            working_copy: WorkingCopyHandle::new("feature-a", "/repo.worktrees/feature-a")
                .with_branch(WorkingCopyBranchName::new("feature/a")),
            base_ref: Some(WorkingCopyBaseRef::new("refs/remotes/origin/main")),
        },
    )
    .into_working_copy_request();

    let WorkingCopyIsolationRequest::Create(create) = request else {
        panic!("expected typed create request");
    };
    assert_eq!(create.project_id.as_str(), "marlin-core");
    assert_eq!(create.provider, WorkingCopyIsolationProvider::GitWorktree);
    assert_eq!(
        create.repository_discovery_path.as_path(),
        PathBuf::from("/repo/nested/crate").as_path()
    );
    assert_eq!(create.working_copy.id.as_str(), "feature-a");
    assert_eq!(
        create.working_copy.path.as_path(),
        PathBuf::from("/repo.worktrees/feature-a").as_path()
    );
    assert_eq!(
        create.base_ref.as_ref().expect("base ref").as_str(),
        "refs/remotes/origin/main"
    );
}

#[test]
fn gerbil_working_copy_policy_projects_worktrunk_switch_into_typed_request() {
    let selection = GerbilWorkingCopyPolicySelection::new(
        "marlin-core",
        WorkingCopyIsolationProvider::Worktrunk,
        "/repo/subdir",
        GerbilWorkingCopyPolicyOperation::Switch {
            working_copy: WorkingCopyHandle::new("feature-a", "/repo.feature-a")
                .with_branch(WorkingCopyBranchName::new("feature/a")),
            mode: WorkingCopySwitchMode::CreateIfMissing {
                base_ref: Some(WorkingCopyBaseRef::new("origin/main")),
            },
        },
    );

    let request = selection.into_working_copy_request();
    let plan = WorkingCopyIsolationPlan::compile(&request).expect("worktrunk switch plan");

    assert_eq!(
        request.operation_kind(),
        WorkingCopyIsolationOperationKind::Switch
    );
    assert!(matches!(
        plan.steps.as_slice(),
        [
            WorkingCopyIsolationPlanStep::PrepareTargetPath { path },
            WorkingCopyIsolationPlanStep::WorktrunkSwitch {
                repository_discovery_path,
                mode: WorkingCopySwitchMode::CreateIfMissing { base_ref: Some(base_ref) },
                ..
            }
        ] if path == &PathBuf::from("/repo.feature-a")
            && repository_discovery_path.as_path() == PathBuf::from("/repo/subdir").as_path()
            && base_ref.as_str() == "origin/main"
    ));
}

#[test]
fn gerbil_working_copy_policy_projects_typed_list_remove_and_pr_requests() {
    let list_request = GerbilWorkingCopyPolicySelection::new(
        "marlin-core",
        WorkingCopyIsolationProvider::Worktrunk,
        "/repo/subdir",
        GerbilWorkingCopyPolicyOperation::List {
            options: WorkingCopyListOptions::new()
                .including_branches()
                .including_remotes(),
        },
    )
    .into_working_copy_request();
    assert!(matches!(list_request, WorkingCopyIsolationRequest::List(_)));
    assert_eq!(
        list_request.operation_kind(),
        WorkingCopyIsolationOperationKind::List
    );

    let remove_request = GerbilWorkingCopyPolicySelection::new(
        "marlin-core",
        WorkingCopyIsolationProvider::Worktrunk,
        "/repo/subdir",
        GerbilWorkingCopyPolicyOperation::Remove {
            working_copy: WorkingCopyHandle::new("feature-a", "/repo.feature-a")
                .with_branch(WorkingCopyBranchName::new("feature/a")),
            mode: WorkingCopyRemovalMode::Force,
        },
    )
    .into_working_copy_request();
    assert!(matches!(
        remove_request,
        WorkingCopyIsolationRequest::Remove(_)
    ));
    assert_eq!(
        remove_request.operation_kind(),
        WorkingCopyIsolationOperationKind::Remove
    );

    let pr_request = GerbilWorkingCopyPolicySelection::new(
        "worktrunk",
        WorkingCopyIsolationProvider::Worktrunk,
        "/repo/subdir",
        GerbilWorkingCopyPolicyOperation::PullRequestCheckout {
            repository: WorkspaceProjectGitHubRepository::new("max-sixty", "worktrunk"),
            pull_request: WorkingCopyPullRequestNumber::new(123),
            working_copy: WorkingCopyHandle::new("pr-123", "/repo.pr-123"),
        },
    )
    .into_working_copy_request();
    assert!(matches!(
        pr_request,
        WorkingCopyIsolationRequest::PullRequestCheckout(_)
    ));
    assert_eq!(
        pr_request.operation_kind(),
        WorkingCopyIsolationOperationKind::PullRequestCheckout
    );
}

#[test]
#[cfg(unix)]
#[ignore = "requires the selected Gerbil package, Git and Worktrunk"]
fn scheme_selects_the_provider_for_a_real_git_worktree() {
    use gerbil_scheme::GerbilRuntime;
    use marlin_agent_environment::{
        ProcessWorkingCopyCommandRunner, WorkingCopyIsolationDriver,
        WorkingCopyProviderExecutableStatus,
    };
    use marlin_agent_gitops::working_copy::{
        WorkingCopyCommandProgram, WorkingCopyIsolationStatus,
    };
    use marlin_gerbil_scheme::{GerbilWorkingCopyPolicy, WorkingCopyProviderAvailability};
    use std::{os::unix::fs::PermissionsExt, path::Path, process::Command};

    fn git(root: &Path, args: &[&str]) {
        let output = Command::new("git")
            .current_dir(root)
            .args(args)
            .output()
            .expect("git");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let directory = tempfile::tempdir().expect("isolated Git fixture");
    let repository = directory.path().join("repository");
    let path = directory.path().join("worktree");
    std::fs::create_dir(&repository).expect("repository directory");
    git(&repository, &["init", "-q", "-b", "main"]);
    git(
        &repository,
        &[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "--allow-empty",
            "-qm",
            "fixture",
        ],
    );
    let nested = repository.join("nested");
    std::fs::create_dir(&nested).unwrap();
    let hooks = repository.join("fixture-hooks");
    std::fs::create_dir(&hooks).unwrap();
    let hook = hooks.join("post-checkout");
    std::fs::write(&hook, b"#!/bin/sh\nprintf 'ran\\n' > unexpected-hook\n").unwrap();
    std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755)).unwrap();
    git(
        &repository,
        &["config", "core.hooksPath", hooks.to_str().unwrap()],
    );
    let tokio = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Tokio");
    let git = tokio
        .block_on(ProcessWorkingCopyCommandRunner::probe_program(
            WorkingCopyCommandProgram::Git,
        ))
        .expect("Git executable probe");
    let available = WorkingCopyProviderAvailability {
        git_worktree: git.status == WorkingCopyProviderExecutableStatus::Available,
        worktrunk: false,
    };
    let working_copy = WorkingCopyHandle::new("test", &path)
        .with_branch(WorkingCopyBranchName::new("test-branch"));
    let runtime = GerbilRuntime::initialize().expect("Gerbil runtime");
    let policy = GerbilWorkingCopyPolicy::load(&runtime).expect("selected Scheme policy");
    let select = |operation| {
        policy
            .select_request(
                "project".into(),
                nested.clone().into(),
                operation,
                available,
                WorkingCopyIsolationProvider::Worktrunk,
            )
            .expect("Scheme decision")
    };
    assert!(
        select(GerbilWorkingCopyPolicyOperation::PullRequestCheckout {
            repository: WorkspaceProjectGitHubRepository::new("fixture", "repository"),
            pull_request: WorkingCopyPullRequestNumber::new(1),
            working_copy: working_copy.clone(),
        })
        .is_none()
    );
    let request = select(GerbilWorkingCopyPolicyOperation::Create {
        working_copy: working_copy.clone(),
        base_ref: Some(WorkingCopyBaseRef::new("HEAD")),
    })
    .expect("Git worktree fallback");
    let remove = select(GerbilWorkingCopyPolicyOperation::Remove {
        working_copy,
        mode: WorkingCopyRemovalMode::default(),
    })
    .expect("same provider for removal");
    let worktrunk = tokio
        .block_on(ProcessWorkingCopyCommandRunner::probe_program(
            WorkingCopyCommandProgram::Worktrunk,
        ))
        .expect("Worktrunk executable probe");
    assert_eq!(
        worktrunk.status,
        WorkingCopyProviderExecutableStatus::Available
    );
    let select_worktrunk = |operation| {
        policy
            .select_request(
                "project".into(),
                nested.clone().into(),
                operation,
                WorkingCopyProviderAvailability {
                    worktrunk: true,
                    ..available
                },
                WorkingCopyIsolationProvider::Worktrunk,
            )
            .expect("Scheme decision")
            .expect("preferred Worktrunk provider")
    };
    let worktrunk_list = select_worktrunk(GerbilWorkingCopyPolicyOperation::List {
        options: WorkingCopyListOptions::new(),
    });

    tokio.block_on(async {
        let driver = WorkingCopyIsolationDriver::new();
        let applied = driver.isolate(request).await.receipt;
        assert_eq!(applied.provider, WorkingCopyIsolationProvider::GitWorktree);
        assert_eq!(
            applied.status,
            WorkingCopyIsolationStatus::Applied,
            "{applied:?}"
        );
        assert!(path.join(".git").is_file());
        assert_eq!(
            applied.command_receipts[0].git_toplevel.as_path(),
            repository.canonicalize().unwrap()
        );
        assert!(!path.join("unexpected-hook").exists());
        let listed = driver.isolate(worktrunk_list).await.receipt;
        assert_eq!(listed.provider, WorkingCopyIsolationProvider::Worktrunk);
        assert_eq!(
            listed.status,
            WorkingCopyIsolationStatus::Applied,
            "{listed:?}"
        );
        let removed = driver.isolate(remove).await.receipt;
        assert_eq!(
            removed.status,
            WorkingCopyIsolationStatus::Applied,
            "{removed:?}"
        );
        assert!(!path.exists());
        assert!(!repository.join("unexpected-hook").exists());
    });

    let placeholder = directory.path().join("unused-worktrunk-path");
    let worktrunk_create = select_worktrunk(GerbilWorkingCopyPolicyOperation::Create {
        working_copy: WorkingCopyHandle::new("worktrunk-test", &placeholder)
            .with_branch(WorkingCopyBranchName::new("agent/fixture")),
        base_ref: Some(WorkingCopyBaseRef::new("HEAD")),
    });
    let driver = WorkingCopyIsolationDriver::new();
    let applied = tokio.block_on(driver.isolate(worktrunk_create)).receipt;
    assert_eq!(applied.provider, WorkingCopyIsolationProvider::Worktrunk);
    assert_eq!(
        applied.status,
        WorkingCopyIsolationStatus::Applied,
        "{applied:?}"
    );
    let actual = applied
        .working_copy
        .expect("provider-reported working copy");
    assert_ne!(actual.path, placeholder);
    assert_eq!(
        actual.branch.as_ref().map(WorkingCopyBranchName::as_str),
        Some("agent/fixture")
    );
    assert!(actual.path.join(".git").is_file());
    assert!(!actual.path.join("unexpected-hook").exists());

    let worktrunk_remove = select_worktrunk(GerbilWorkingCopyPolicyOperation::Remove {
        working_copy: actual.clone(),
        mode: WorkingCopyRemovalMode::default(),
    });
    let removed = tokio.block_on(driver.isolate(worktrunk_remove)).receipt;
    assert_eq!(removed.provider, WorkingCopyIsolationProvider::Worktrunk);
    assert_eq!(
        removed.status,
        WorkingCopyIsolationStatus::Applied,
        "{removed:?}"
    );
    assert_eq!(removed.working_copy.as_ref(), Some(&actual));
    assert!(!actual.path.exists());
}
