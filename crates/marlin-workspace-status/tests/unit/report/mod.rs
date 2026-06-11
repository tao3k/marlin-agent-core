use marlin_gerbil_ir::{ReleaseGateSpec, ReleaseTopologySpec, ReleaseVisibilitySpec};
use marlin_workspace_status::{ReleaseGateState, ReleaseStatus};

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
                && report.artifact_paths == ["fixtures/gerbil/build.ss"])
    );
    assert!(
        status
            .visibility_reports
            .iter()
            .any(|report| report.report_key == "real_gxi_release_gate"
                && report.evidence_keys == ["workspace_patch_intent"])
    );
}
