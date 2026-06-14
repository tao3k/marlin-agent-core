use marlin_gerbil_scheme::{GerbilWorkingCopyPolicyOperation, GerbilWorkingCopyPolicySelection};
use marlin_workspace_protocol::{
    WorkingCopyBaseRef, WorkingCopyBranchName, WorkingCopyHandle,
    WorkingCopyIsolationOperationKind, WorkingCopyIsolationPlan, WorkingCopyIsolationPlanStep,
    WorkingCopyIsolationProvider, WorkingCopyIsolationRequest, WorkingCopyListOptions,
    WorkingCopyPullRequestNumber, WorkingCopyRemovalMode, WorkingCopySwitchMode,
    WorkspaceProjectGitHubRepository,
};
use std::path::PathBuf;

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
