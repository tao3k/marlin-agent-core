use super::{
    RICH_LOOP_GRAPH_SOURCE, assert_agent_scenario_contract_artifact,
    assert_release_topology_artifact, assert_rich_loop_graph_artifact,
    assert_workspace_patch_intent_artifact, assert_workspace_schema_artifact,
    command_adapter_batch_artifacts, real_gxi_module_compiler,
};
use marlin_gerbil_scheme::{GerbilArtifactKind, GerbilCompiler, GerbilSource};
use marlin_org_store::FileSystemReleaseStatusStore;
use marlin_workspace_protocol::{ReleaseGateReceipt, ReleaseGateState, ReleaseLandingReport};
use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

const RELEASE_STATUS_ARTIFACT_DIR_ENV: &str = "MARLIN_RELEASE_STATUS_ARTIFACT_DIR";

#[test]
#[ignore = "requires a local Gerbil gxi executable"]
fn command_compiler_can_call_real_gxi_command_adapter_launcher_with_contract_facts() {
    let Some(artifacts) = command_adapter_batch_artifacts() else {
        return;
    };

    assert_rich_loop_graph_artifact(artifacts[0].clone());
}

#[test]
#[ignore = "requires a local Gerbil gxi executable"]
fn command_compiler_can_call_real_gxi_command_adapter_launcher_workspace_schema() {
    let Some(artifacts) = command_adapter_batch_artifacts() else {
        return;
    };

    assert_workspace_schema_artifact(artifacts[1].clone());
}

#[test]
#[ignore = "requires a local Gerbil gxi executable"]
fn command_compiler_can_call_real_gxi_command_adapter_launcher_workspace_patch_intent() {
    let Some(artifacts) = command_adapter_batch_artifacts() else {
        return;
    };

    assert_workspace_patch_intent_artifact(artifacts[2].clone());
}

#[test]
#[ignore = "requires a local Gerbil gxi executable"]
fn command_compiler_can_call_real_gxi_command_adapter_launcher_agent_scenario_contract() {
    let Some(artifacts) = command_adapter_batch_artifacts() else {
        return;
    };

    assert_agent_scenario_contract_artifact(artifacts[3].clone());
}

#[test]
#[ignore = "requires a local Gerbil gxi executable"]
fn command_compiler_can_call_real_gxi_command_adapter_launcher_release_topology() {
    let Some(artifacts) = command_adapter_batch_artifacts() else {
        return;
    };

    assert_release_topology_artifact(artifacts[4].clone());
}

#[test]
#[ignore = "requires a local Gerbil gxi executable"]
fn command_compiler_can_call_real_gxi_module_entry() {
    let Some(compiler) = real_gxi_module_compiler() else {
        return;
    };

    let artifact = compiler
        .compile(
            GerbilSource::new("audit/control-plane", RICH_LOOP_GRAPH_SOURCE),
            GerbilArtifactKind::LoopGraph,
        )
        .expect(
            "real gxi module entry should compile source text into a typed loop graph artifact",
        );

    assert_rich_loop_graph_artifact(artifact);
}

#[test]
#[ignore = "requires a local Gerbil gxi executable"]
fn command_compiler_can_call_real_gxi_workspace_schema() {
    let Some(artifacts) = command_adapter_batch_artifacts() else {
        return;
    };

    assert_workspace_schema_artifact(artifacts[1].clone());
}

#[test]
#[ignore = "requires a local Gerbil gxi executable"]
fn command_compiler_can_call_real_gxi_workspace_patch_intent() {
    let Some(artifacts) = command_adapter_batch_artifacts() else {
        return;
    };

    assert_workspace_patch_intent_artifact(artifacts[2].clone());
}

#[test]
#[ignore = "requires a local Gerbil gxi executable"]
fn command_compiler_can_call_real_gxi_agent_scenario_contract() {
    let Some(artifacts) = command_adapter_batch_artifacts() else {
        return;
    };

    assert_agent_scenario_contract_artifact(artifacts[3].clone());
}

#[test]
#[ignore = "requires a local Gerbil gxi executable"]
fn command_compiler_can_call_real_gxi_release_topology() {
    let Some(artifacts) = command_adapter_batch_artifacts() else {
        return;
    };

    assert_release_topology_artifact(artifacts[4].clone());
}

#[test]
#[ignore = "requires a local Gerbil gxi executable"]
fn command_compiler_real_gxi_release_topology_persists_landing_status_sidecar() {
    let Some(artifacts) = command_adapter_batch_artifacts() else {
        return;
    };
    let root = test_root("real-gxi-release-topology-status");

    let artifact = artifacts[4].clone();
    let topology = artifact
        .release_topology()
        .expect("real gxi should produce a release topology artifact");
    let store = FileSystemReleaseStatusStore::new(&root);

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
                vec!["fixtures/gerbil/command-adapter.ss".to_owned()],
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

    persist_release_status_artifacts(&store, &report);

    let _ = fs::remove_dir_all(root);
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
}

fn test_root(name: &str) -> std::path::PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    std::env::temp_dir().join(format!(
        "marlin-gerbil-scheme-{name}-{}-{suffix}",
        std::process::id()
    ))
}
