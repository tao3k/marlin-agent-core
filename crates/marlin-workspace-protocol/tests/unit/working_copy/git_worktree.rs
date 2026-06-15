use std::path::PathBuf;

use marlin_workspace_protocol::{
    WorkingCopyBaseRef, WorkingCopyBranchName, WorkingCopyCommandInvocation,
    WorkingCopyCommandProgram, WorkingCopyCommandProjection, WorkingCopyCommandReceipt,
    WorkingCopyCommandStatus, WorkingCopyCreateRequest, WorkingCopyFinalizeBranchRequest,
    WorkingCopyGitTopLevel, WorkingCopyHandle, WorkingCopyIsolationOperationKind,
    WorkingCopyIsolationPlan, WorkingCopyIsolationPlanStep, WorkingCopyIsolationProvider,
    WorkingCopyIsolationReceipt, WorkingCopyIsolationRequest, WorkingCopyIsolationStatus,
    WorkingCopyListRequest, WorkingCopyRemoveRequest, WorkingCopySwitchRequest,
};

use super::test_git_toplevel_resolver;

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
fn git_core_projects_finalize_branch_inside_active_working_copy() {
    let request = WorkingCopyIsolationRequest::FinalizeBranch(
        WorkingCopyFinalizeBranchRequest::new(
            "worktrunk",
            WorkingCopyIsolationProvider::Worktrunk,
            WorkingCopyHandle::new("agent-a", "/repo.agent-a"),
            WorkingCopyBranchName::new("agent/a"),
        )
        .with_repository_discovery_path("/repo/subdir"),
    );

    let plan = WorkingCopyIsolationPlan::compile(&request).expect("finalize branch plan");
    let projection = WorkingCopyCommandProjection::from_plan(&plan, test_git_toplevel_resolver)
        .expect("finalize branch projection");

    assert_eq!(
        projection.operation,
        WorkingCopyIsolationOperationKind::FinalizeBranch
    );
    assert_eq!(
        projection.commands[0].program,
        WorkingCopyCommandProgram::Git
    );
    assert_eq!(
        projection.commands[0].args,
        vec![
            "-C".to_string(),
            "/repo.agent-a".to_string(),
            "switch".to_string(),
            "-c".to_string(),
            "agent/a".to_string()
        ]
    );
    assert_eq!(
        projection.commands[0]
            .expected_working_copy
            .as_ref()
            .and_then(|copy| copy.branch.as_ref())
            .map(|branch| branch.as_str()),
        Some("agent/a")
    );
}
