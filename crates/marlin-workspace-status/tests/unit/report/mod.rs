use marlin_agent_protocol::{LoopEvidence, LoopEvidenceKind};
use marlin_gerbil_ir::{ReleaseGateSpec, ReleaseTopologySpec, ReleaseVisibilitySpec};
use marlin_workspace_status::{ReleaseGateReceipt, ReleaseGateState, ReleaseStatus};

#[test]
fn release_status_projects_pending_topology_visibility() {
    let topology = ReleaseTopologySpec {
        topology_id: "release:gerbil".to_string(),
        crate_name: "marlin-gerbil-scheme".to_string(),
        publish_enabled: false,
        asset_audit_command: "cargo package -p marlin-gerbil-scheme --list --allow-dirty"
            .to_string(),
        package_assets: vec![
            "README.md".to_string(),
            "fixtures/gerbil/build.ss".to_string(),
        ],
        runtime_dependency_chain: vec![
            "marlin-gerbil-ir".to_string(),
            "marlin-workspace-patch".to_string(),
        ],
        workflow_dependency_chain: vec![
            "marlin-org-workflow".to_string(),
            "marlin-org-store".to_string(),
        ],
        gates: vec![
            ReleaseGateSpec {
                gate_id: "package-assets".to_string(),
                command: "cargo package -p marlin-gerbil-scheme --list --allow-dirty"
                    .to_string(),
                requires_local_gerbil: false,
                required_artifacts: vec!["fixtures/gerbil/build.ss".to_string()],
                visibility: vec![ReleaseVisibilitySpec {
                    report_key: "package_asset_audit".to_string(),
                    evidence_keys: vec!["required_artifacts".to_string()],
                    artifact_paths: vec!["fixtures/gerbil/build.ss".to_string()],
                }],
            },
            ReleaseGateSpec {
                gate_id: "real-gxi".to_string(),
                command: "cargo test -p marlin-gerbil-scheme --test unit_test command::real_gxi -- --ignored".to_string(),
                requires_local_gerbil: true,
                required_artifacts: vec!["workspace_patch_intent".to_string()],
                visibility: vec![ReleaseVisibilitySpec {
                    report_key: "real_gxi_release_gate".to_string(),
                    evidence_keys: vec!["workspace_patch_intent".to_string()],
                    artifact_paths: vec!["fixtures/gerbil/command-adapter.ss".to_string()],
                }],
            },
        ],
    };

    let status = ReleaseStatus::pending_from_topology(&topology);

    assert_eq!(status.topology_id, "release:gerbil");
    assert_eq!(status.crate_name, "marlin-gerbil-scheme");
    assert!(!status.publish_enabled);
    assert!(
        status
            .package_assets
            .iter()
            .any(|asset| asset == "fixtures/gerbil/build.ss")
    );
    assert_eq!(status.gates[0].state, ReleaseGateState::Pending);
    assert_eq!(status.gates[1].state, ReleaseGateState::RequiresLocalGerbil);
    assert_eq!(status.visibility_reports.len(), 2);
    assert!(
        status
            .visibility_reports
            .iter()
            .any(|report| report.report_key == "package_asset_audit"
                && report.artifact_paths == ["fixtures/gerbil/build.ss"]
                && !report.observed)
    );
    assert!(
        status
            .visibility_reports
            .iter()
            .any(|report| report.report_key == "real_gxi_release_gate"
                && report.evidence_keys == ["workspace_patch_intent"]
                && !report.observed)
    );
}

#[test]
fn release_status_records_gate_receipts() {
    let topology = ReleaseTopologySpec {
        topology_id: "release:gerbil".to_string(),
        crate_name: "marlin-gerbil-scheme".to_string(),
        publish_enabled: false,
        asset_audit_command: "cargo package -p marlin-gerbil-scheme --list --allow-dirty"
            .to_string(),
        package_assets: vec!["fixtures/gerbil/build.ss".to_string()],
        runtime_dependency_chain: vec!["marlin-gerbil-ir".to_string()],
        workflow_dependency_chain: vec!["marlin-org-workflow".to_string()],
        gates: vec![ReleaseGateSpec {
            gate_id: "package-assets".to_string(),
            command: "cargo package -p marlin-gerbil-scheme --list --allow-dirty".to_string(),
            requires_local_gerbil: false,
            required_artifacts: vec!["fixtures/gerbil/build.ss".to_string()],
            visibility: vec![ReleaseVisibilitySpec {
                report_key: "package_asset_audit".to_string(),
                evidence_keys: vec!["required_artifacts".to_string()],
                artifact_paths: vec!["fixtures/gerbil/build.ss".to_string()],
            }],
        }],
    };
    let mut status = ReleaseStatus::pending_from_topology(&topology);

    let recorded = status.record_gate_receipt(ReleaseGateReceipt::passed(
        "package-assets",
        vec!["required_artifacts".to_string()],
        vec!["fixtures/gerbil/build.ss".to_string()],
    ));

    assert!(recorded);
    assert_eq!(status.gates[0].state, ReleaseGateState::Passed);
    assert_eq!(
        status.gates[0]
            .last_receipt
            .as_ref()
            .expect("gate receipt")
            .artifact_paths,
        ["fixtures/gerbil/build.ss"]
    );
    assert!(status.visibility_reports[0].observed);
    assert!(!status.record_gate_receipt(ReleaseGateReceipt::failed(
        "missing-gate",
        vec!["not in topology".to_string()],
    )));
}

#[test]
fn release_status_marks_visibility_evidence_as_passed_gate() {
    let topology = ReleaseTopologySpec {
        topology_id: "release:gerbil".to_string(),
        crate_name: "marlin-gerbil-scheme".to_string(),
        publish_enabled: false,
        asset_audit_command: "cargo package -p marlin-gerbil-scheme --list --allow-dirty"
            .to_string(),
        package_assets: vec!["fixtures/gerbil/command-adapter.ss".to_string()],
        runtime_dependency_chain: vec!["marlin-gerbil-ir".to_string()],
        workflow_dependency_chain: vec!["marlin-org-workflow".to_string()],
        gates: vec![ReleaseGateSpec {
            gate_id: "real-gxi".to_string(),
            command:
                "cargo test -p marlin-gerbil-scheme --test unit_test command::real_gxi -- --ignored"
                    .to_string(),
            requires_local_gerbil: true,
            required_artifacts: vec!["workspace_patch_intent".to_string()],
            visibility: vec![ReleaseVisibilitySpec {
                report_key: "real_gxi_release_gate".to_string(),
                evidence_keys: vec!["workspace_patch_intent".to_string()],
                artifact_paths: vec!["fixtures/gerbil/command-adapter.ss".to_string()],
            }],
        }],
    };
    let evidence = vec![LoopEvidence::present(
        LoopEvidenceKind::Visibility,
        "release-visibility:release:gerbil:real-gxi:real_gxi_release_gate",
    )];

    let status = ReleaseStatus::from_topology_and_evidence(&topology, &evidence);

    assert_eq!(status.gates[0].state, ReleaseGateState::Passed);
    let receipt = status.gates[0]
        .last_receipt
        .as_ref()
        .expect("visibility evidence receipt");
    assert_eq!(receipt.evidence_keys, ["workspace_patch_intent"]);
    assert_eq!(
        receipt.artifact_paths,
        ["fixtures/gerbil/command-adapter.ss"]
    );
    assert!(status.visibility_reports[0].observed);
}
