use futures_executor::block_on;
use marlin_gerbil_ir::{ReleaseGateSpec, ReleaseTopologySpec, ReleaseVisibilitySpec};
use marlin_org_memory::MemoryOrgWorkspace;
use marlin_workspace_protocol::{AgentWorkspace, WorkspaceCtx};
use marlin_workspace_status::{ReleaseGateReceipt, ReleaseGateState, WorkspaceTarget};

#[test]
fn memory_workspace_reports_recorded_release_topology_status() {
    let workspace = MemoryOrgWorkspace::new();
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

    let recorded = workspace
        .record_release_topology(&topology)
        .expect("release topology recorded");
    let status =
        block_on(workspace.status(WorkspaceTarget::Workspace, WorkspaceCtx::new("unit-test")))
            .expect("workspace status");
    let release = status.release.expect("release status");

    assert_eq!(recorded.topology_id, "release:gerbil");
    assert_eq!(release.topology_id, "release:gerbil");
    assert_eq!(release.crate_name, "marlin-gerbil-scheme");
    assert_eq!(release.gates[0].state, ReleaseGateState::Pending);
    assert!(
        release
            .visibility_reports
            .iter()
            .any(|report| report.report_key == "package_asset_audit"
                && report.artifact_paths == ["fixtures/gerbil/build.ss"])
    );
}

#[test]
fn memory_workspace_reports_release_gate_receipts() {
    let workspace = MemoryOrgWorkspace::new();
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
            visibility: Vec::new(),
        }],
    };

    workspace
        .record_release_topology(&topology)
        .expect("release topology recorded");
    assert!(
        workspace
            .record_release_gate_receipt(ReleaseGateReceipt::passed(
                "package-assets",
                vec!["required_artifacts".to_string()],
                vec!["fixtures/gerbil/build.ss".to_string()],
            ))
            .expect("gate receipt recorded")
    );
    let status =
        block_on(workspace.status(WorkspaceTarget::Workspace, WorkspaceCtx::new("unit-test")))
            .expect("workspace status");
    let release = status.release.expect("release status");

    assert_eq!(release.gates[0].state, ReleaseGateState::Passed);
    assert_eq!(
        release.gates[0]
            .last_receipt
            .as_ref()
            .expect("gate receipt")
            .evidence_keys,
        ["required_artifacts"]
    );
}
