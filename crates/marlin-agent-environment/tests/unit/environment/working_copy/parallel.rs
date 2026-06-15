use super::{ScriptedGitRepositoryResolver, ScriptedWorkingCopyRunner};
use marlin_agent_environment::{WorkingCopyCommandOutput, WorkingCopyIsolationDriver};
use marlin_workspace_protocol::{
    WorkingCopyBaseRef, WorkingCopyBranchName, WorkingCopyCommandProgram,
    WorkingCopyFanoutBenchmarkReceipt, WorkingCopyFinalizeBranchRequest, WorkingCopyHandle,
    WorkingCopyIsolationProvider, WorkingCopyIsolationRequest, WorkingCopyIsolationStatus,
    WorkingCopySwitchRequest,
};
use std::{path::PathBuf, time::Instant};

#[tokio::test]
async fn working_copy_driver_executes_parallel_worktrunk_fanout_with_bounded_receipt() {
    let runner = ScriptedWorkingCopyRunner::new([
        WorkingCopyCommandOutput::succeeded(r#"{"branch":"agent/a"}"#),
        WorkingCopyCommandOutput::succeeded(r#"{"branch":"agent/b"}"#),
        WorkingCopyCommandOutput::succeeded(r#"{"branch":"agent/c"}"#),
    ]);
    let git_resolver = ScriptedGitRepositoryResolver::new("/repo");
    let driver = WorkingCopyIsolationDriver::with_runner_and_git_resolver(
        runner.clone(),
        git_resolver.clone(),
    );
    let requests = ["a", "b", "c"].into_iter().map(|suffix| {
        WorkingCopyIsolationRequest::Switch(
            WorkingCopySwitchRequest::new(
                "worktrunk",
                WorkingCopyIsolationProvider::Worktrunk,
                "/repo/subdir",
                WorkingCopyHandle::new(format!("agent-{suffix}"), format!("/repo.agent-{suffix}"))
                    .with_branch(WorkingCopyBranchName::new(format!("agent/{suffix}"))),
            )
            .create_if_missing_from(WorkingCopyBaseRef::new("HEAD")),
        )
    });

    let result = driver
        .isolate_parallel(
            "worktrunk",
            WorkingCopyIsolationProvider::Worktrunk,
            2,
            requests,
        )
        .await;

    assert!(result.receipt.is_success());
    assert_eq!(result.receipt.requested, 3);
    assert_eq!(result.receipt.applied, 3);
    assert_eq!(result.receipt.max_parallelism, 2);
    assert!(result.receipt.has_consistent_scope());
    assert_eq!(runner.calls().len(), 3);
    assert_eq!(
        git_resolver.calls(),
        vec![
            PathBuf::from("/repo/subdir"),
            PathBuf::from("/repo/subdir"),
            PathBuf::from("/repo/subdir")
        ]
    );
}

#[tokio::test]
async fn working_copy_driver_executes_finalize_branch_projection() {
    let runner = ScriptedWorkingCopyRunner::new([WorkingCopyCommandOutput::succeeded("")]);
    let git_resolver = ScriptedGitRepositoryResolver::new("/repo");
    let driver = WorkingCopyIsolationDriver::with_runner_and_git_resolver(
        runner.clone(),
        git_resolver.clone(),
    );
    let request = WorkingCopyIsolationRequest::FinalizeBranch(
        WorkingCopyFinalizeBranchRequest::new(
            "worktrunk",
            WorkingCopyIsolationProvider::Worktrunk,
            WorkingCopyHandle::new("agent-a", "/repo.agent-a"),
            WorkingCopyBranchName::new("agent/a"),
        )
        .with_repository_discovery_path("/repo/subdir"),
    );

    let result = driver.isolate(request).await;

    assert_eq!(result.receipt.status, WorkingCopyIsolationStatus::Applied);
    assert_eq!(runner.calls().len(), 1);
    assert_eq!(runner.calls()[0].program, WorkingCopyCommandProgram::Git);
    assert_eq!(
        runner.calls()[0].args,
        vec![
            "-C".to_string(),
            "/repo.agent-a".to_string(),
            "switch".to_string(),
            "-c".to_string(),
            "agent/a".to_string()
        ]
    );
}

#[tokio::test]
async fn working_copy_parallel_fanout_emits_benchmark_receipt() {
    let runner = ScriptedWorkingCopyRunner::new([
        WorkingCopyCommandOutput::succeeded(r#"{"branch":"agent/a"}"#),
        WorkingCopyCommandOutput::succeeded(r#"{"branch":"agent/b"}"#),
    ]);
    let git_resolver = ScriptedGitRepositoryResolver::new("/repo");
    let driver = WorkingCopyIsolationDriver::with_runner_and_git_resolver(runner, git_resolver);
    let requests = ["a", "b"].into_iter().map(|suffix| {
        WorkingCopyIsolationRequest::Switch(WorkingCopySwitchRequest::new(
            "worktrunk",
            WorkingCopyIsolationProvider::Worktrunk,
            "/repo/subdir",
            WorkingCopyHandle::new(format!("agent-{suffix}"), format!("/repo.agent-{suffix}"))
                .with_branch(WorkingCopyBranchName::new(format!("agent/{suffix}"))),
        ))
    });

    let started = Instant::now();
    let result = driver
        .isolate_parallel(
            "worktrunk",
            WorkingCopyIsolationProvider::Worktrunk,
            2,
            requests,
        )
        .await;
    let benchmark = WorkingCopyFanoutBenchmarkReceipt::from_parallel_receipt(
        &result.receipt,
        started.elapsed().as_micros(),
    );

    assert_eq!(benchmark.requested, 2);
    assert_eq!(benchmark.max_parallelism, 2);
    assert_eq!(benchmark.applied, 2);
    assert_eq!(benchmark.rejected, 0);
}
