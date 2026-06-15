use std::{path::PathBuf, sync::Arc};

use marlin_agent_harness::{
    AgentHarness, HarnessEvidenceKind, HarnessRuntime, HarnessScenario,
    runtime_environment_visibility_evidence, working_copy_isolation_visibility_evidence,
};
use marlin_agent_runtime::{
    WorkingCopyCommandInvocation, WorkingCopyCommandProgram, WorkingCopyCommandReceipt,
    WorkingCopyCreateRequest, WorkingCopyGitTopLevel, WorkingCopyHandle,
    WorkingCopyIsolationProvider, WorkingCopyIsolationReceipt, WorkingCopyIsolationRequest,
    WorkingCopyIsolationStatus,
};
use marlin_agent_test_support::{
    assert_custom_sub_agent_environment, assert_hook_environment_uses_root_home,
    custom_home_runtime_environment_fixture,
};

use super::support::{EnvironmentEchoHook, EnvironmentEchoSubAgent};

#[tokio::test]
async fn harness_runtime_preserves_custom_environment_for_hooks_and_sub_agents() {
    let scenario =
        HarnessScenario::new("environment").expecting_evidence(HarnessEvidenceKind::Visibility);
    let fixture = custom_home_runtime_environment_fixture();
    let mut harness = HarnessRuntime::with_environment(4, fixture.root_environment().clone());
    harness.record_environment_visibility();

    let hook_environment = harness
        .runtime()
        .spawn_hook(Arc::new(EnvironmentEchoHook), "pre-tool".to_owned())
        .join()
        .await
        .expect("hook task should finish");
    let sub_agent_environment = harness
        .runtime()
        .spawn_sub_agent_with_environment(
            Arc::new(EnvironmentEchoSubAgent),
            (),
            fixture.sub_agent_environment().clone(),
        )
        .join()
        .await
        .expect("sub-agent task should finish");

    assert_eq!(harness.environment(), fixture.root_environment());
    assert_hook_environment_uses_root_home(&fixture, &hook_environment);
    assert_custom_sub_agent_environment(&fixture, &sub_agent_environment);

    let evidence = harness
        .evidence()
        .iter()
        .find(|evidence| evidence.kind == HarnessEvidenceKind::Visibility)
        .expect("expected runtime environment visibility evidence");
    assert_eq!(
        evidence,
        &runtime_environment_visibility_evidence(fixture.root_environment())
    );
    assert_eq!(evidence.subject, "runtime-environment");
    assert_eq!(
        evidence.detail.as_deref(),
        Some("home=true cwd=true config_layers=2 writable_roots=0 network_access=false")
    );

    let report = AgentHarness::evaluate(&scenario, &[], harness.evidence());
    assert!(report.is_success());
}

#[test]
fn harness_runtime_records_working_copy_isolation_visibility() {
    let scenario =
        HarnessScenario::new("working-copy").expecting_evidence(HarnessEvidenceKind::Visibility);
    let request = WorkingCopyIsolationRequest::Create(WorkingCopyCreateRequest::new(
        "marlin-core",
        WorkingCopyIsolationProvider::GitWorktree,
        "/repo",
        WorkingCopyHandle::new("feature-a", "/repo.feature-a"),
    ));
    let invocation = WorkingCopyCommandInvocation::new(
        WorkingCopyCommandProgram::Git,
        WorkingCopyGitTopLevel::from_resolved_path(PathBuf::from("/repo")),
    )
    .with_args(["worktree", "add", "/repo.feature-a"])
    .with_expected_working_copy(WorkingCopyHandle::new("feature-a", "/repo.feature-a"));
    let passed = WorkingCopyIsolationReceipt::applied(&request).with_command_receipts([
        WorkingCopyCommandReceipt::succeeded(&invocation, Some(0), "", ""),
    ]);
    let failed =
        WorkingCopyIsolationReceipt::rejected(&request, "working-copy provider command failed")
            .with_command_receipts([WorkingCopyCommandReceipt::failed(
                &invocation,
                Some(128),
                "",
                "path already exists",
            )]);
    let mut harness = HarnessRuntime::new(16);

    harness.record_working_copy_isolation_visibility(&passed);
    harness.record_working_copy_isolation_visibility(&failed);

    assert_eq!(passed.status, WorkingCopyIsolationStatus::Applied);
    assert_eq!(
        harness.evidence()[0],
        working_copy_isolation_visibility_evidence(&passed)
    );
    assert!(
        harness.evidence()[0]
            .detail
            .as_deref()
            .expect("passed detail")
            .contains("status=Applied")
    );
    assert!(
        harness.evidence()[1]
            .detail
            .as_deref()
            .expect("failed detail")
            .contains("failed_command_count=1")
    );
    assert!(
        harness.evidence()[1]
            .detail
            .as_deref()
            .expect("failed detail")
            .contains("reason=working-copy provider command failed")
    );

    let report = AgentHarness::evaluate(&scenario, &[], harness.evidence());
    assert!(report.is_success());
}
