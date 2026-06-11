use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

use marlin_agent_core::gerbil_ir::{ReleaseGateSpec, ReleaseTopologySpec, ReleaseVisibilitySpec};
use marlin_agent_core::{
    FileSystemReleaseStatusStore, LoopEvidenceKind, ReleaseGateExecutionStatus, ReleaseGateState,
    ReleaseLandingReport, commit_release_gate_execution_receipts,
    gerbil_release_status_commit_from_execution_receipts, record_release_gate_execution_receipt,
    release_gate_execution_receipt, release_gate_state_from_execution, release_gate_status_receipt,
    release_gate_visibility_evidence,
};

#[test]
fn core_facade_exposes_release_visibility_contract() {
    let topology = ReleaseTopologySpec {
        topology_id: "release:core".to_owned(),
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
                evidence_keys: vec!["workspace_schema".to_owned()],
                artifact_paths: vec!["fixtures/gerbil/command-adapter.ss".to_owned()],
            }],
        }],
    };
    let gate = &topology.gates[0];

    let evidence = release_gate_visibility_evidence(&topology, gate);
    let receipt =
        release_gate_execution_receipt(&topology, gate, ReleaseGateExecutionStatus::Passed);
    let status_receipt = release_gate_status_receipt(&receipt);

    assert_eq!(evidence.len(), 1);
    assert_eq!(evidence[0].kind, LoopEvidenceKind::Visibility);
    assert_eq!(receipt.status, ReleaseGateExecutionStatus::Passed);
    assert_eq!(receipt.visibility_evidence, evidence);
    assert_eq!(
        receipt.artifact_paths,
        vec!["fixtures/gerbil/command-adapter.ss"]
    );
    assert_eq!(status_receipt.state, ReleaseGateState::Passed);
    assert_eq!(status_receipt.evidence_keys, vec!["workspace_schema"]);
    assert_eq!(
        status_receipt.artifact_paths,
        vec!["fixtures/gerbil/command-adapter.ss"]
    );
}

#[test]
fn core_release_bridge_marks_expected_local_gerbil_gate() {
    let topology = ReleaseTopologySpec {
        topology_id: "release:core".to_owned(),
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
            visibility: Vec::new(),
        }],
    };
    let receipt = release_gate_execution_receipt(
        &topology,
        &topology.gates[0],
        ReleaseGateExecutionStatus::Expected,
    );

    assert_eq!(
        release_gate_state_from_execution(&receipt),
        ReleaseGateState::RequiresLocalGerbil
    );
    assert_eq!(
        release_gate_status_receipt(&receipt).state,
        ReleaseGateState::RequiresLocalGerbil
    );
}

#[test]
fn core_release_bridge_records_execution_receipt_in_status_store() {
    let root = core_release_test_root("status-store");
    let store = FileSystemReleaseStatusStore::new(&root);
    let topology = release_status_topology();

    store
        .record_release_topology(&topology)
        .expect("topology status persisted");
    let receipt = release_gate_execution_receipt(
        &topology,
        &topology.gates[0],
        ReleaseGateExecutionStatus::Passed,
    );

    assert!(
        record_release_gate_execution_receipt(&store, &receipt)
            .expect("execution receipt persisted")
    );

    let status = store
        .read_status()
        .expect("status readable")
        .expect("status present");
    assert_eq!(status.gates[0].state, ReleaseGateState::Passed);
    assert!(status.visibility_reports[0].observed);
    let report: ReleaseLandingReport = store
        .read_landing_report()
        .expect("landing report readable")
        .expect("landing report present");
    assert!(report.landing_complete);
    assert_eq!(report.passed_gates, 1);
    assert_eq!(report.observed_visibility_reports, 1);
    assert_eq!(report.observed_evidence_keys, ["required_artifacts"]);
    assert_eq!(
        report.observed_artifact_paths,
        ["fixtures/gerbil/command-adapter.ss"]
    );
    assert!(report.missing_artifact_paths.is_empty());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn core_release_bridge_builds_workflow_commit_from_execution_receipts() {
    let root = core_release_test_root("workflow-commit");
    let store = FileSystemReleaseStatusStore::new(&root);
    let topology = release_status_topology();
    let receipt = release_gate_execution_receipt(
        &topology,
        &topology.gates[0],
        ReleaseGateExecutionStatus::Passed,
    );

    let workflow_commit = gerbil_release_status_commit_from_execution_receipts(
        topology.clone(),
        std::slice::from_ref(&receipt),
    );
    assert_eq!(workflow_commit.gate_receipts.len(), 1);
    assert_eq!(
        workflow_commit.gate_receipts[0].state,
        ReleaseGateState::Passed
    );

    let commit_receipt = commit_release_gate_execution_receipts(&store, topology, &[receipt])
        .expect("workflow commit should persist release status");
    assert!(commit_receipt.accepted());
    assert_eq!(commit_receipt.recorded_gate_receipts, 1);
    assert!(commit_receipt.status.visibility_reports[0].observed);

    let _ = fs::remove_dir_all(root);
}

fn release_status_topology() -> ReleaseTopologySpec {
    ReleaseTopologySpec {
        topology_id: "release:core".to_owned(),
        crate_name: "marlin-gerbil-scheme".to_owned(),
        publish_enabled: false,
        asset_audit_command: "cargo package -p marlin-gerbil-scheme --list".to_owned(),
        package_assets: vec!["fixtures/gerbil/command-adapter.ss".to_owned()],
        runtime_dependency_chain: vec!["marlin-gerbil-ir".to_owned()],
        workflow_dependency_chain: vec!["marlin-org-workflow".to_owned()],
        gates: vec![ReleaseGateSpec {
            gate_id: "package-assets".to_owned(),
            command: "cargo package -p marlin-gerbil-scheme --list".to_owned(),
            requires_local_gerbil: false,
            required_artifacts: vec!["fixtures/gerbil/command-adapter.ss".to_owned()],
            visibility: vec![ReleaseVisibilitySpec {
                report_key: "package_asset_audit".to_owned(),
                evidence_keys: vec!["required_artifacts".to_owned()],
                artifact_paths: vec!["fixtures/gerbil/command-adapter.ss".to_owned()],
            }],
        }],
    }
}

fn core_release_test_root(name: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time should be monotonic")
        .as_nanos();
    std::env::temp_dir().join(format!("marlin-agent-core-{name}-{nanos}"))
}
