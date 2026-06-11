use super::{
    AGENT_SCENARIO_CONTRACT_SOURCE, RELEASE_TOPOLOGY_SOURCE, RICH_LOOP_GRAPH_SOURCE,
    WORKSPACE_PATCH_INTENT_SOURCE, WORKSPACE_SCHEMA_SOURCE,
    assert_agent_scenario_contract_artifact, assert_release_topology_artifact,
    assert_rich_loop_graph_artifact, assert_workspace_patch_intent_artifact,
    assert_workspace_schema_artifact, real_gxi_command_adapter_compiler, real_gxi_module_compiler,
};
use marlin_gerbil_scheme::{GerbilArtifactKind, GerbilCompiler, GerbilSource};
use marlin_org_store::FileSystemReleaseStatusStore;
use marlin_workspace_protocol::{ReleaseGateReceipt, ReleaseGateState};
use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
#[ignore = "requires a local Gerbil gxi executable"]
fn command_compiler_can_call_real_gxi_command_adapter_launcher() {
    let Some(compiler) = real_gxi_command_adapter_compiler() else {
        return;
    };

    let artifact = compiler
        .compile(
            GerbilSource::new("audit/control-plane", RICH_LOOP_GRAPH_SOURCE),
            GerbilArtifactKind::LoopGraph,
        )
        .expect(
            "real gxi command adapter launcher should compile source text into a loop graph artifact",
        );

    assert_rich_loop_graph_artifact(artifact);
}

#[test]
#[ignore = "requires a local Gerbil gxi executable"]
fn command_compiler_can_call_real_gxi_command_adapter_launcher_workspace_schema() {
    let Some(compiler) = real_gxi_command_adapter_compiler() else {
        return;
    };

    let artifact = compiler
        .compile(
            GerbilSource::new("audit/workspace-schema", WORKSPACE_SCHEMA_SOURCE),
            GerbilArtifactKind::WorkspaceSchema,
        )
        .expect(
            "real gxi command adapter launcher should compile source text into a workspace schema artifact",
        );

    assert_workspace_schema_artifact(artifact);
}

#[test]
#[ignore = "requires a local Gerbil gxi executable"]
fn command_compiler_can_call_real_gxi_command_adapter_launcher_workspace_patch_intent() {
    let Some(compiler) = real_gxi_command_adapter_compiler() else {
        return;
    };

    let artifact = compiler
        .compile(
            GerbilSource::new("audit/workspace-patch-intent", WORKSPACE_PATCH_INTENT_SOURCE),
            GerbilArtifactKind::WorkspacePatchIntent,
        )
        .expect(
            "real gxi command adapter launcher should compile source text into a workspace patch intent artifact",
        );

    assert_workspace_patch_intent_artifact(artifact);
}

#[test]
#[ignore = "requires a local Gerbil gxi executable"]
fn command_compiler_can_call_real_gxi_command_adapter_launcher_agent_scenario_contract() {
    let Some(compiler) = real_gxi_command_adapter_compiler() else {
        return;
    };

    let artifact = compiler
        .compile(
            GerbilSource::new("audit/agent-scenario", AGENT_SCENARIO_CONTRACT_SOURCE),
            GerbilArtifactKind::AgentScenarioContract,
        )
        .expect(
            "real gxi command adapter launcher should compile source text into an agent scenario contract artifact",
        );

    assert_agent_scenario_contract_artifact(artifact);
}

#[test]
#[ignore = "requires a local Gerbil gxi executable"]
fn command_compiler_can_call_real_gxi_command_adapter_launcher_release_topology() {
    let Some(compiler) = real_gxi_command_adapter_compiler() else {
        return;
    };

    let artifact = compiler
        .compile(
            GerbilSource::new("audit/release-topology", RELEASE_TOPOLOGY_SOURCE),
            GerbilArtifactKind::ReleaseTopology,
        )
        .expect(
            "real gxi command adapter launcher should compile source text into a release topology artifact",
        );

    assert_release_topology_artifact(artifact);
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
    let Some(compiler) = real_gxi_module_compiler() else {
        return;
    };

    let artifact = compiler
        .compile(
            GerbilSource::new("audit/workspace-schema", WORKSPACE_SCHEMA_SOURCE),
            GerbilArtifactKind::WorkspaceSchema,
        )
        .expect(
            "real gxi module entry should compile source text into a workspace schema artifact",
        );

    assert_workspace_schema_artifact(artifact);
}

#[test]
#[ignore = "requires a local Gerbil gxi executable"]
fn command_compiler_can_call_real_gxi_workspace_patch_intent() {
    let Some(compiler) = real_gxi_module_compiler() else {
        return;
    };

    let artifact = compiler
        .compile(
            GerbilSource::new("audit/workspace-patch-intent", WORKSPACE_PATCH_INTENT_SOURCE),
            GerbilArtifactKind::WorkspacePatchIntent,
        )
        .expect(
            "real gxi module entry should compile source text into a workspace patch intent artifact",
        );

    assert_workspace_patch_intent_artifact(artifact);
}

#[test]
#[ignore = "requires a local Gerbil gxi executable"]
fn command_compiler_can_call_real_gxi_agent_scenario_contract() {
    let Some(compiler) = real_gxi_module_compiler() else {
        return;
    };

    let artifact = compiler
        .compile(
            GerbilSource::new("audit/agent-scenario", AGENT_SCENARIO_CONTRACT_SOURCE),
            GerbilArtifactKind::AgentScenarioContract,
        )
        .expect(
            "real gxi module entry should compile source text into an agent scenario contract artifact",
        );

    assert_agent_scenario_contract_artifact(artifact);
}

#[test]
#[ignore = "requires a local Gerbil gxi executable"]
fn command_compiler_can_call_real_gxi_release_topology() {
    let Some(compiler) = real_gxi_module_compiler() else {
        return;
    };

    let artifact = compiler
        .compile(
            GerbilSource::new("audit/release-topology", RELEASE_TOPOLOGY_SOURCE),
            GerbilArtifactKind::ReleaseTopology,
        )
        .expect(
            "real gxi module entry should compile source text into a release topology artifact",
        );

    assert_release_topology_artifact(artifact);
}

#[test]
#[ignore = "requires a local Gerbil gxi executable"]
fn command_compiler_real_gxi_release_topology_persists_landing_status_sidecar() {
    let Some(compiler) = real_gxi_module_compiler() else {
        return;
    };
    let root = test_root("real-gxi-release-topology-status");

    let artifact = compiler
        .compile(
            GerbilSource::new("audit/release-topology-status", RELEASE_TOPOLOGY_SOURCE),
            GerbilArtifactKind::ReleaseTopology,
        )
        .expect("real gxi should compile release topology for landing status");
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

    let _ = fs::remove_dir_all(root);
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
