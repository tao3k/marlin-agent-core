use marlin_agent_harness::{
    AgentHarness, HarnessEvidenceKind, HarnessRuntime, HarnessScenario, ReleaseGateExecutionStatus,
    native_abi_readiness_release_gate_execution_receipt, release_gate_execution_receipt,
    release_topology_execution_receipts,
};
use marlin_agent_protocol::{GraphNativeAbiReadinessReceipt, GraphNativeAbiRequirement};
use marlin_gerbil_ir::{ReleaseGateSpec, ReleaseTopologySpec, ReleaseVisibilitySpec};

#[test]
fn harness_runtime_records_release_visibility_evidence() {
    let scenario = HarnessScenario::new("release-visibility")
        .expecting_evidence(HarnessEvidenceKind::Visibility);
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
    let gate = &topology.gates[0];
    let receipt =
        release_gate_execution_receipt(&topology, gate, ReleaseGateExecutionStatus::Passed);
    assert_eq!(receipt.topology_id, "release:gerbil");
    assert_eq!(receipt.crate_name, "marlin-gerbil-scheme");
    assert_eq!(receipt.gate_id, "real-gxi");
    assert_eq!(receipt.status, ReleaseGateExecutionStatus::Passed);
    assert_eq!(receipt.required_artifacts, vec!["workspace_schema"]);
    assert_eq!(
        receipt.evidence_keys,
        vec!["workspace_schema", "package_asset"]
    );
    assert_eq!(
        receipt.artifact_paths,
        vec!["fixtures/gerbil/command-adapter.ss"]
    );
    assert_eq!(receipt.visibility_evidence.len(), 1);

    let receipts =
        release_topology_execution_receipts(&topology, ReleaseGateExecutionStatus::Expected);
    assert_eq!(receipts.len(), 1);
    assert_eq!(receipts[0].status, ReleaseGateExecutionStatus::Expected);

    let mut harness = HarnessRuntime::new(16);

    harness.record_release_topology_visibility(&topology);

    let evidence = harness
        .evidence()
        .iter()
        .find(|evidence| evidence.kind == HarnessEvidenceKind::Visibility)
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

#[test]
fn harness_release_gate_receipt_projects_native_abi_readiness() {
    let topology = ReleaseTopologySpec {
        topology_id: "release:native".to_owned(),
        crate_name: "marlin-deck-runtime-native".to_owned(),
        publish_enabled: false,
        asset_audit_command: "cargo test -p marlin-deck-runtime-native --features linked-native"
            .to_owned(),
        package_assets: vec!["README.md".to_owned()],
        runtime_dependency_chain: vec!["marlin-gerbil-scheme".to_owned()],
        workflow_dependency_chain: Vec::new(),
        gates: vec![ReleaseGateSpec {
            gate_id: "native-abi-readiness".to_owned(),
            command: "cargo test -p marlin-deck-runtime-native --features linked-native".to_owned(),
            requires_local_gerbil: true,
            required_artifacts: vec!["deck-runtime-native-link-unit".to_owned()],
            visibility: Vec::new(),
        }],
    };
    let gate = &topology.gates[0];
    let requirement = GraphNativeAbiRequirement::new("marlin.deck-runtime.native", 1)
        .with_required_symbols([
            "marlin_deck_runtime_initialize",
            "marlin_deck_runtime_select_model_route",
        ]);
    let ready = GraphNativeAbiReadinessReceipt::evaluate(
        &requirement,
        [
            "marlin_deck_runtime_initialize",
            "marlin_deck_runtime_select_model_route",
        ],
    );
    let missing =
        GraphNativeAbiReadinessReceipt::evaluate(&requirement, ["marlin_deck_runtime_initialize"]);

    let passed = native_abi_readiness_release_gate_execution_receipt(&topology, gate, &ready);
    let failed = native_abi_readiness_release_gate_execution_receipt(&topology, gate, &missing);

    assert_eq!(passed.status, ReleaseGateExecutionStatus::Passed);
    assert!(passed.diagnostics.is_empty());
    assert!(
        passed
            .evidence_keys
            .contains(&"native_abi_readiness".to_owned())
    );
    assert_eq!(failed.status, ReleaseGateExecutionStatus::Failed);
    assert!(
        failed
            .diagnostics
            .contains(&"native_abi_readiness.missing_symbols".to_owned())
    );
    assert!(
        failed
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("marlin_deck_runtime_select_model_route"))
    );
}
