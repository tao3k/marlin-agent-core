use std::sync::Arc;

use marlin_agent_harness::{AgentHarness, HarnessRuntime, runtime_environment_visibility_evidence};
use marlin_agent_protocol::{AgentScenario, LoopEvidenceKind};
use marlin_agent_test_support::{
    assert_custom_sub_agent_environment, assert_hook_environment_uses_root_home,
    custom_home_runtime_environment_fixture,
};

use super::support::{EnvironmentEchoHook, EnvironmentEchoSubAgent};

#[tokio::test]
async fn harness_runtime_preserves_custom_environment_for_hooks_and_sub_agents() {
    let scenario =
        AgentScenario::new("environment").expecting_evidence(LoopEvidenceKind::Visibility);
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
        .find(|evidence| evidence.kind == LoopEvidenceKind::Visibility)
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
