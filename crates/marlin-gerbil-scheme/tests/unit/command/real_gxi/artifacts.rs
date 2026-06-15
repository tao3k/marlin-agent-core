use super::assert_release_topology_artifact;
use marlin_gerbil_scheme::GerbilCompiledArtifact;
use marlin_org_store::FileSystemReleaseStatusStore;
use marlin_workspace_protocol::{
    ReleaseGateReceipt, ReleaseGateState, ReleaseLandingReport, ReleaseStatus,
};
use serde_json::json;
use std::{fs, path::Path};
use tempfile::{Builder, TempDir};

const RELEASE_STATUS_ARTIFACT_DIR_ENV: &str = "MARLIN_RELEASE_STATUS_ARTIFACT_DIR";
const TYPED_ADAPTER_ASSET_PATH: &str = "gerbil/src/marlin/adapter.ss";

#[test]
fn release_topology_fixture_persists_landing_status_sidecar() {
    let root = test_root("release-topology-status");
    let artifact = release_topology_artifact();
    assert_release_topology_artifact(artifact.clone());

    let topology = artifact
        .release_topology()
        .expect("fixture should produce a release topology artifact");
    let store = FileSystemReleaseStatusStore::new(root.path());

    let pending = store
        .record_release_topology(topology)
        .expect("release topology should be persisted as a landing status sidecar");
    assert_eq!(pending.topology_id, "release:gerbil");
    assert_eq!(
        pending.gates[0].state,
        ReleaseGateState::RequiresLocalGerbil
    );

    assert!(
        store
            .record_release_gate_receipt(ReleaseGateReceipt::passed(
                "real-gxi",
                vec![
                    "workspace_schema".to_owned(),
                    "workspace_patch_intent".to_owned()
                ],
                vec![TYPED_ADAPTER_ASSET_PATH.to_owned()],
            ))
            .expect("real gxi gate receipt should update sidecar")
    );
    let status = store
        .read_status()
        .expect("release status should remain readable")
        .expect("release status sidecar should exist");

    assert_eq!(status.crate_name, "marlin-gerbil-scheme");
    assert_eq!(status.gates[0].state, ReleaseGateState::Passed);
    assert!(status.visibility_reports[0].observed);
    let report = store
        .read_landing_report()
        .expect("landing report should remain readable")
        .expect("landing report should exist");
    assert!(report.landing_complete);
    assert_eq!(report.topology_id, "release:gerbil");
    assert_eq!(report.passed_gates, 1);
    assert_eq!(report.observed_visibility_reports, 1);
    assert_eq!(
        report.observed_evidence_keys,
        ["workspace_patch_intent", "workspace_schema"]
    );
    assert_eq!(report.observed_artifact_paths, [TYPED_ADAPTER_ASSET_PATH]);
    assert!(report.missing_artifact_paths.is_empty());

    persist_release_status_artifacts(&store, &report);
}

fn release_topology_artifact() -> GerbilCompiledArtifact {
    serde_json::from_value(json!({
        "ReleaseTopology": {
            "topology_id": "release:gerbil",
            "crate_name": "marlin-gerbil-scheme",
            "publish_enabled": false,
            "asset_audit_command": "cargo package -p marlin-gerbil-scheme --allow-dirty --no-verify --list",
            "package_assets": ["README.md", "gerbil"],
            "runtime_dependency_chain": ["marlin-gerbil-ir", "marlin-workspace-patch"],
            "workflow_dependency_chain": ["marlin-org-workflow", "marlin-org-store"],
            "gates": [{
                "gate_id": "real-gxi",
                "command": "cargo test -p marlin-gerbil-scheme --test unit_test command::real_gxi -- --ignored",
                "requires_local_gerbil": true,
                "required_artifacts": ["workspace_schema", "workspace_patch_intent"],
                "visibility": [{
                    "report_key": "real_gxi_release_gate",
                    "evidence_keys": ["workspace_schema", "workspace_patch_intent"],
                    "artifact_paths": [TYPED_ADAPTER_ASSET_PATH]
                }]
            }]
        }
    }))
    .expect("release topology fixture should decode")
}

fn persist_release_status_artifacts(
    store: &FileSystemReleaseStatusStore,
    report: &ReleaseLandingReport,
) {
    let Some(artifact_dir) = std::env::var_os(RELEASE_STATUS_ARTIFACT_DIR_ENV) else {
        return;
    };
    let artifact_dir = std::path::PathBuf::from(artifact_dir);
    fs::create_dir_all(&artifact_dir).expect("release status artifact dir should be created");
    fs::copy(store.path(), artifact_dir.join("release-status.json"))
        .expect("release status sidecar should be copied to artifact dir");
    let report_json =
        serde_json::to_string_pretty(report).expect("release landing report should encode as json");
    fs::write(
        artifact_dir.join("release-landing-report.json"),
        report_json,
    )
    .expect("release landing report should be written to artifact dir");
    assert_release_status_artifacts(&artifact_dir);
}

fn assert_release_status_artifacts(artifact_dir: &Path) {
    let status_path = artifact_dir.join("release-status.json");
    let report_path = artifact_dir.join("release-landing-report.json");
    let status: ReleaseStatus = serde_json::from_str(
        &fs::read_to_string(status_path).expect("release status artifact should be readable"),
    )
    .expect("release status artifact should match ReleaseStatus schema");
    let report: ReleaseLandingReport = serde_json::from_str(
        &fs::read_to_string(report_path)
            .expect("release landing report artifact should be readable"),
    )
    .expect("release landing report artifact should match ReleaseLandingReport schema");

    assert_eq!(status.topology_id, report.topology_id);
    assert_eq!(status.crate_name, report.crate_name);
    assert!(report.landing_complete);
    assert_eq!(report.passed_gates, status.gates.len());
    assert_eq!(
        report.observed_visibility_reports,
        status.visibility_reports.len()
    );
    assert_eq!(
        report.observed_evidence_keys,
        ["workspace_patch_intent", "workspace_schema"]
    );
    assert_eq!(report.observed_artifact_paths, [TYPED_ADAPTER_ASSET_PATH]);
    assert!(report.missing_artifact_paths.is_empty());
    assert!(report.missing_visibility_reports.is_empty());
}

fn test_root(name: &str) -> TempDir {
    Builder::new()
        .prefix(&format!("marlin-gerbil-scheme-{name}-"))
        .tempdir()
        .unwrap_or_else(|error| panic!("creates {name} test root: {error}"))
}
