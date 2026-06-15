use marlin_workspace_protocol::{
    WorkingCopyActiveBinding, WorkingCopyBaseRef, WorkingCopyBranchName, WorkingCopyCreateRequest,
    WorkingCopyFanoutBenchmarkReceipt, WorkingCopyHandle, WorkingCopyIsolationProvider,
    WorkingCopyIsolationReceipt, WorkingCopyIsolationRequest, WorkingCopyIsolationStatus,
    WorkingCopyParallelIsolationReceipt, WorkingCopyRetentionActionKind,
    WorkingCopyRetentionPolicy, WorkingCopySwitchRequest,
};

#[test]
fn parallel_isolation_receipt_summarizes_agent_worktree_fanout() {
    let agent_a_request = WorkingCopyIsolationRequest::Switch(
        WorkingCopySwitchRequest::new(
            "worktrunk",
            WorkingCopyIsolationProvider::Worktrunk,
            "/repo",
            WorkingCopyHandle::new("agent-a", "/repo.agent-a")
                .with_branch(WorkingCopyBranchName::new("agent/a")),
        )
        .create_if_missing_from(WorkingCopyBaseRef::new("HEAD")),
    );
    let agent_b_request = WorkingCopyIsolationRequest::Switch(
        WorkingCopySwitchRequest::new(
            "worktrunk",
            WorkingCopyIsolationProvider::Worktrunk,
            "/repo",
            WorkingCopyHandle::new("agent-b", "/repo.agent-b")
                .with_branch(WorkingCopyBranchName::new("agent/b")),
        )
        .create_if_missing_from(WorkingCopyBaseRef::new("HEAD")),
    );
    let agent_c_request = WorkingCopyIsolationRequest::Switch(
        WorkingCopySwitchRequest::new(
            "worktrunk",
            WorkingCopyIsolationProvider::Worktrunk,
            "/repo",
            WorkingCopyHandle::new("agent-c", "/repo.agent-c")
                .with_branch(WorkingCopyBranchName::new("agent/c")),
        )
        .create_if_missing_from(WorkingCopyBaseRef::new("HEAD")),
    );

    let success = WorkingCopyParallelIsolationReceipt::from_receipts(
        "worktrunk",
        WorkingCopyIsolationProvider::Worktrunk,
        2,
        [
            WorkingCopyIsolationReceipt::applied(&agent_a_request),
            WorkingCopyIsolationReceipt::applied(&agent_b_request),
        ],
    );

    assert!(success.is_success());
    assert!(success.has_consistent_scope());
    assert_eq!(success.project_id.as_str(), "worktrunk");
    assert_eq!(success.provider, WorkingCopyIsolationProvider::Worktrunk);
    assert_eq!(success.max_parallelism, 2);
    assert_eq!(success.requested, 2);
    assert_eq!(success.applied, 2);
    assert_eq!(success.rejected, 0);
    assert_eq!(
        success.receipts[1]
            .working_copy
            .as_ref()
            .map(|copy| copy.id.as_str()),
        Some("agent-b")
    );

    let partial = WorkingCopyParallelIsolationReceipt::from_receipts(
        "worktrunk",
        WorkingCopyIsolationProvider::Worktrunk,
        2,
        [
            WorkingCopyIsolationReceipt::applied(&agent_a_request),
            WorkingCopyIsolationReceipt::applied(&agent_b_request),
            WorkingCopyIsolationReceipt::rejected(&agent_c_request, "parallel capacity exhausted"),
        ],
    );

    assert!(!partial.is_success());
    assert!(partial.has_consistent_scope());
    assert_eq!(partial.requested, 3);
    assert_eq!(partial.applied, 2);
    assert_eq!(partial.rejected, 1);
    assert_eq!(
        partial.receipts[2].reason.as_deref(),
        Some("parallel capacity exhausted")
    );

    let off_scope_request = WorkingCopyIsolationRequest::Create(WorkingCopyCreateRequest::new(
        "marlin-core",
        WorkingCopyIsolationProvider::GitWorktree,
        "/repo",
        WorkingCopyHandle::new("agent-d", "/repo.agent-d"),
    ));
    let mixed_scope = WorkingCopyParallelIsolationReceipt::from_receipts(
        "worktrunk",
        WorkingCopyIsolationProvider::Worktrunk,
        2,
        [
            WorkingCopyIsolationReceipt::applied(&agent_a_request),
            WorkingCopyIsolationReceipt::applied(&off_scope_request),
        ],
    );

    assert_eq!(mixed_scope.applied, 2);
    assert!(!mixed_scope.has_consistent_scope());
    assert!(!mixed_scope.is_success());
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
fn active_binding_projects_only_applied_working_copy_receipts() {
    let request = WorkingCopyIsolationRequest::Create(WorkingCopyCreateRequest::new(
        "marlin-core",
        WorkingCopyIsolationProvider::GitWorktree,
        "/repo",
        WorkingCopyHandle::new("feature-a", "/repo.feature-a")
            .with_branch(WorkingCopyBranchName::new("feature/a")),
    ));
    let applied = WorkingCopyIsolationReceipt::applied(&request);
    let rejected = WorkingCopyIsolationReceipt::rejected(&request, "denied");

    let binding = WorkingCopyActiveBinding::from_receipt(&applied)
        .expect("applied working-copy receipt should bind runtime");

    assert_eq!(binding.project_id.as_str(), "marlin-core");
    assert_eq!(binding.provider, WorkingCopyIsolationProvider::GitWorktree);
    assert_eq!(binding.working_copy.id.as_str(), "feature-a");
    assert!(WorkingCopyActiveBinding::from_receipt(&rejected).is_none());
}

#[test]
fn retention_policy_snapshots_and_removes_old_working_copies() {
    let receipt = WorkingCopyRetentionPolicy::new(2).plan_sweep(
        "worktrunk",
        WorkingCopyIsolationProvider::Worktrunk,
        [
            WorkingCopyHandle::new("agent-a", "/repo.agent-a"),
            WorkingCopyHandle::new("agent-b", "/repo.agent-b"),
            WorkingCopyHandle::new("agent-c", "/repo.agent-c"),
        ],
        "/repo/.marlin/snapshots",
    );

    assert_eq!(receipt.max_retained, 2);
    assert_eq!(receipt.retained, 2);
    assert_eq!(receipt.snapshotted, 1);
    assert_eq!(receipt.removed, 1);
    assert_eq!(
        receipt.actions[0].action,
        WorkingCopyRetentionActionKind::Keep
    );
    assert_eq!(
        receipt.actions[2].action,
        WorkingCopyRetentionActionKind::SnapshotAndRemove
    );
    assert_eq!(
        receipt.actions[2]
            .snapshot_path
            .as_ref()
            .map(|path| path.display().to_string()),
        Some("/repo/.marlin/snapshots/agent-c.patch".to_string())
    );
}

#[test]
fn fanout_benchmark_receipt_preserves_parallel_metrics() {
    let request = WorkingCopyIsolationRequest::Switch(
        WorkingCopySwitchRequest::new(
            "worktrunk",
            WorkingCopyIsolationProvider::Worktrunk,
            "/repo",
            WorkingCopyHandle::new("agent-a", "/repo.agent-a")
                .with_branch(WorkingCopyBranchName::new("agent/a")),
        )
        .create_if_missing_from(WorkingCopyBaseRef::new("HEAD")),
    );
    let fanout = WorkingCopyParallelIsolationReceipt::from_receipts(
        "worktrunk",
        WorkingCopyIsolationProvider::Worktrunk,
        4,
        [WorkingCopyIsolationReceipt::applied(&request)],
    );
    let benchmark = WorkingCopyFanoutBenchmarkReceipt::from_parallel_receipt(&fanout, 125);

    assert_eq!(benchmark.requested, 1);
    assert_eq!(benchmark.max_parallelism, 4);
    assert_eq!(benchmark.applied, 1);
    assert_eq!(benchmark.rejected, 0);
    assert_eq!(benchmark.elapsed_micros, 125);
}
