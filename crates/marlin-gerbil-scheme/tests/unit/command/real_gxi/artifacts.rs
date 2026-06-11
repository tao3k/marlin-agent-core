use super::{
    AGENT_SCENARIO_CONTRACT_SOURCE, RICH_LOOP_GRAPH_SOURCE, WORKSPACE_PATCH_INTENT_SOURCE,
    WORKSPACE_SCHEMA_SOURCE, assert_agent_scenario_contract_artifact,
    assert_rich_loop_graph_artifact, assert_workspace_patch_intent_artifact,
    assert_workspace_schema_artifact, real_gxi_command_adapter_compiler, real_gxi_module_compiler,
};
use marlin_gerbil_scheme::{GerbilArtifactKind, GerbilCompiler, GerbilSource};

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
