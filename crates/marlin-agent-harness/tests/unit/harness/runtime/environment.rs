use std::sync::Arc;

use marlin_agent_harness::{AgentHarness, HarnessRuntime, runtime_environment_visibility_evidence};
use marlin_agent_protocol::{AgentScenario, LoopEvidenceKind, RuntimeHome};
use marlin_agent_runtime::RuntimeEnvironment;

use super::support::{EnvironmentEchoHook, EnvironmentEchoSubAgent};

#[tokio::test]
async fn harness_runtime_preserves_custom_environment_for_hooks_and_sub_agents() {
    let scenario =
        AgentScenario::new("environment").expecting_evidence(LoopEvidenceKind::Visibility);
    let parent_environment = RuntimeEnvironment::default()
        .with_home(RuntimeHome::custom("/tmp/marlin-home").with_profile("main"))
        .with_cwd("/tmp/workspace");
    let child_environment = RuntimeEnvironment::default()
        .with_home(RuntimeHome::custom("/tmp/marlin-home/sub/reviewer").with_profile("reviewer"))
        .with_cwd("/tmp/workspace/sub");
    let mut harness = HarnessRuntime::with_environment(4, parent_environment.clone());
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
            child_environment.clone(),
        )
        .join()
        .await
        .expect("sub-agent task should finish");

    assert_eq!(harness.environment(), &parent_environment);
    assert_eq!(hook_environment, parent_environment);
    assert_eq!(sub_agent_environment, child_environment);

    let evidence = harness
        .evidence()
        .iter()
        .find(|evidence| evidence.kind == LoopEvidenceKind::Visibility)
        .expect("expected runtime environment visibility evidence");
    assert_eq!(
        evidence,
        &runtime_environment_visibility_evidence(&parent_environment)
    );
    assert_eq!(evidence.subject, "runtime-environment");
    assert_eq!(
        evidence.detail.as_deref(),
        Some("home=true cwd=true config_layers=0 writable_roots=0 network_access=false")
    );

    let report = AgentHarness::evaluate(&scenario, &[], harness.evidence());
    assert!(report.is_success());
}
