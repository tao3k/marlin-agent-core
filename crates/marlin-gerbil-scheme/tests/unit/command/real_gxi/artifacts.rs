use super::{
    AGENT_SCENARIO_CONTRACT_SOURCE, RELEASE_TOPOLOGY_SOURCE, RICH_LOOP_GRAPH_SOURCE,
    WORKSPACE_PATCH_INTENT_SOURCE, WORKSPACE_SCHEMA_SOURCE,
    assert_agent_scenario_contract_artifact, assert_release_topology_artifact,
    assert_rich_loop_graph_artifact, assert_workspace_patch_intent_artifact,
    assert_workspace_schema_artifact, real_gxi_command_adapter_batch_compiler,
    real_gxi_module_compiler,
};
use marlin_gerbil_scheme::{
    GerbilArtifactKind, GerbilCompileRequest, GerbilCompiledArtifact, GerbilCompiler, GerbilSource,
};
use marlin_org_store::FileSystemReleaseStatusStore;
use marlin_workspace_protocol::{ReleaseGateReceipt, ReleaseGateState};
use std::{
    fs,
    sync::OnceLock,
    time::{SystemTime, UNIX_EPOCH},
};

static COMMAND_ADAPTER_BATCH_ARTIFACTS: OnceLock<Option<Vec<GerbilCompiledArtifact>>> =
    OnceLock::new();

fn command_adapter_batch_artifacts() -> Option<&'static [GerbilCompiledArtifact]> {
    COMMAND_ADAPTER_BATCH_ARTIFACTS
        .get_or_init(|| {
            let compiler = real_gxi_command_adapter_batch_compiler()?;
            Some(
                compiler
                    .compile_requests(command_adapter_batch_requests())
                    .expect("real gxi command adapter batch should compile artifacts"),
            )
        })
        .as_deref()
}

fn command_adapter_batch_requests() -> Vec<GerbilCompileRequest> {
    vec![
        GerbilCompileRequest {
            source: GerbilSource::new("audit/control-plane", RICH_LOOP_GRAPH_SOURCE),
            expected: GerbilArtifactKind::LoopGraph,
            contract_facts: None,
        },
        GerbilCompileRequest {
            source: GerbilSource::new("audit/workspace-schema", WORKSPACE_SCHEMA_SOURCE),
            expected: GerbilArtifactKind::WorkspaceSchema,
            contract_facts: None,
        },
        GerbilCompileRequest {
            source: GerbilSource::new(
                "audit/workspace-patch-intent",
                WORKSPACE_PATCH_INTENT_SOURCE,
            ),
            expected: GerbilArtifactKind::WorkspacePatchIntent,
            contract_facts: None,
        },
        GerbilCompileRequest {
            source: GerbilSource::new("audit/agent-scenario", AGENT_SCENARIO_CONTRACT_SOURCE),
            expected: GerbilArtifactKind::AgentScenarioContract,
            contract_facts: None,
        },
        GerbilCompileRequest {
            source: GerbilSource::new("audit/release-topology", RELEASE_TOPOLOGY_SOURCE),
            expected: GerbilArtifactKind::ReleaseTopology,
            contract_facts: None,
        },
    ]
}

#[test]
#[ignore = "requires a local Gerbil gxi executable"]
fn command_compiler_can_call_real_gxi_command_adapter_launcher() {
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
    let report = store
        .read_landing_report()
        .expect("landing report should remain readable")
        .expect("landing report should exist");
    assert!(report.landing_complete);
    assert_eq!(report.topology_id, "release:gerbil");
    assert_eq!(report.passed_gates, 1);
    assert_eq!(report.observed_visibility_reports, 1);

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
