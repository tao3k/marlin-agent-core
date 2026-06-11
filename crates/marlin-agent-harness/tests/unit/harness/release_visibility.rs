use marlin_agent_harness::{AgentHarness, HarnessRuntime};
use marlin_agent_protocol::{AgentScenario, LoopEvidenceKind};
use marlin_gerbil_ir::{ReleaseGateSpec, ReleaseTopologySpec, ReleaseVisibilitySpec};

#[test]
fn harness_runtime_records_release_visibility_evidence() {
    let scenario =
        AgentScenario::new("release-visibility").expecting_evidence(LoopEvidenceKind::Visibility);
    let topology = ReleaseTopologySpec {
        topology_id: "release:gerbil".to_owned(),
        crate_name: "marlin-gerbil-scheme".to_owned(),
        publish_enabled: false,
        asset_audit_command: "cargo package -p marlin-gerbil-scheme --list".to_owned(),
        package_assets: vec!["README.md".to_owned()],
        runtime_dependency_chain: vec!["marlin-gerbil-ir".to_owned()],
        workflow_dependency_chain: vec!["marlin-org-workflow".to_owned()],
        gates: vec![ReleaseGateSpec {
            gate_id: "real-gxi".to_owned(),
            command: "cargo test -p marlin-gerbil-scheme command::real_gxi".to_owned(),
            requires_local_gerbil: true,
            required_artifacts: vec!["workspace_schema".to_owned()],
            visibility: vec![ReleaseVisibilitySpec {
                report_key: "real_gxi_release_gate".to_owned(),
                evidence_keys: vec!["workspace_schema".to_owned(), "package_asset".to_owned()],
                artifact_paths: vec!["fixtures/gerbil/command-adapter.ss".to_owned()],
            }],
        }],
    };
    let mut harness = HarnessRuntime::new(16);

    harness.record_release_topology_visibility(&topology);

    let evidence = harness
        .evidence()
        .iter()
        .find(|evidence| evidence.kind == LoopEvidenceKind::Visibility)
        .expect("expected release visibility evidence");
    let detail = evidence.detail.as_deref().expect("visibility detail");
    assert_eq!(
        evidence.subject,
        "release-visibility:release:gerbil:real-gxi:real_gxi_release_gate"
    );
    assert!(detail.contains("topology_id=release:gerbil"));
    assert!(detail.contains("crate_name=marlin-gerbil-scheme"));
    assert!(detail.contains("gate_id=real-gxi"));
    assert!(detail.contains("report_key=real_gxi_release_gate"));
    assert!(detail.contains("evidence_keys=[workspace_schema,package_asset]"));
    assert!(detail.contains("artifact_paths=[fixtures/gerbil/command-adapter.ss]"));

    let report = AgentHarness::evaluate(&scenario, &[], harness.evidence());
    assert!(report.is_success());
}
