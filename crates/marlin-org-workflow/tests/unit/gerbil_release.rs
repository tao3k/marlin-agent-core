use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

use marlin_gerbil_ir::{ReleaseGateSpec, ReleaseTopologySpec, ReleaseVisibilitySpec};
use marlin_org_store::FileSystemReleaseStatusStore;
use marlin_org_workflow::{GerbilReleaseStatusCommit, GerbilReleaseStatusCommitter};
use marlin_workspace_status::{ReleaseGateReceipt, ReleaseGateState};

#[test]
fn gerbil_release_status_commit_persists_sidecar_status() {
    let root = test_root("release-commit");
    fs::create_dir_all(&root).expect("create temp root");
    let store = FileSystemReleaseStatusStore::new(&root);
    let commit = GerbilReleaseStatusCommit::new(release_topology()).with_gate_receipt(
        ReleaseGateReceipt::passed(
            "package-assets",
            vec!["required_artifacts".to_owned()],
            vec!["fixtures/gerbil/build.ss".to_owned()],
        ),
    );

    let receipt = GerbilReleaseStatusCommitter::commit(&store, &commit)
        .expect("release status commit succeeds");

    assert!(receipt.accepted());
    assert_eq!(receipt.recorded_gate_receipts, 1);
    assert!(receipt.missing_gate_receipts.is_empty());
    assert_eq!(receipt.status.gates[0].state, ReleaseGateState::Passed);
    let persisted = store
        .read_status()
        .expect("status sidecar reads")
        .expect("status sidecar exists");
    assert_eq!(persisted.topology_id, "release:gerbil");
    assert_eq!(persisted.gates[0].state, ReleaseGateState::Passed);
    assert!(persisted.visibility_reports[0].observed);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn gerbil_release_status_commit_reports_missing_gate_receipts() {
    let root = test_root("release-missing-gate");
    fs::create_dir_all(&root).expect("create temp root");
    let store = FileSystemReleaseStatusStore::new(&root);
    let commit = GerbilReleaseStatusCommit::new(release_topology()).with_gate_receipt(
        ReleaseGateReceipt::failed("missing-gate", vec!["not in topology".to_owned()]),
    );

    let receipt = GerbilReleaseStatusCommitter::commit(&store, &commit)
        .expect("release status commit succeeds");

    assert!(!receipt.accepted());
    assert_eq!(receipt.recorded_gate_receipts, 0);
    assert_eq!(receipt.missing_gate_receipts, ["missing-gate"]);
    let persisted = store
        .read_status()
        .expect("status sidecar reads")
        .expect("status sidecar exists");
    assert_eq!(persisted.gates[0].state, ReleaseGateState::Pending);
    let _ = fs::remove_dir_all(root);
}

fn release_topology() -> ReleaseTopologySpec {
    ReleaseTopologySpec {
        topology_id: "release:gerbil".to_owned(),
        crate_name: "marlin-gerbil-scheme".to_owned(),
        publish_enabled: false,
        asset_audit_command: "cargo package -p marlin-gerbil-scheme --list --allow-dirty"
            .to_owned(),
        package_assets: vec!["fixtures/gerbil/build.ss".to_owned()],
        runtime_dependency_chain: vec!["marlin-gerbil-ir".to_owned()],
        workflow_dependency_chain: vec!["marlin-org-workflow".to_owned()],
        gates: vec![ReleaseGateSpec {
            gate_id: "package-assets".to_owned(),
            command: "cargo package -p marlin-gerbil-scheme --list --allow-dirty".to_owned(),
            requires_local_gerbil: false,
            required_artifacts: vec!["fixtures/gerbil/build.ss".to_owned()],
            visibility: vec![ReleaseVisibilitySpec {
                report_key: "package_asset_audit".to_owned(),
                evidence_keys: vec!["required_artifacts".to_owned()],
                artifact_paths: vec!["fixtures/gerbil/build.ss".to_owned()],
            }],
        }],
    }
}

fn test_root(name: &str) -> std::path::PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    std::env::temp_dir().join(format!(
        "marlin-org-workflow-{name}-{}-{suffix}",
        std::process::id()
    ))
}
