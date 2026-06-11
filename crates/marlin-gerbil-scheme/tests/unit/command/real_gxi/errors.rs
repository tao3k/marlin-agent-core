use super::{RICH_LOOP_GRAPH_SOURCE, WORKSPACE_SCHEMA_SOURCE, real_gxi_module_compiler};
use marlin_gerbil_scheme::{GerbilArtifactKind, GerbilCompiler, GerbilSource};

#[test]
#[ignore = "requires a local Gerbil gxi executable"]
fn command_compiler_real_gxi_workspace_schema_rejects_loop_graph_source() {
    let Some(compiler) = real_gxi_module_compiler() else {
        return;
    };

    let error = compiler
        .compile(
            GerbilSource::new("audit/control-plane", RICH_LOOP_GRAPH_SOURCE),
            GerbilArtifactKind::WorkspaceSchema,
        )
        .unwrap_err();

    assert!(error.contains("gerbil compiler command failed"));
    assert!(error.contains("expected workspace-schema form"));
    assert!(error.contains("loop-graph"));
}

#[test]
#[ignore = "requires a local Gerbil gxi executable"]
fn command_compiler_real_gxi_loop_graph_rejects_workspace_schema_source() {
    let Some(compiler) = real_gxi_module_compiler() else {
        return;
    };

    let error = compiler
        .compile(
            GerbilSource::new("audit/workspace-schema", WORKSPACE_SCHEMA_SOURCE),
            GerbilArtifactKind::LoopGraph,
        )
        .unwrap_err();

    assert!(error.contains("gerbil compiler command failed"));
    assert!(error.contains("expected loop-graph form"));
    assert!(error.contains("workspace-schema"));
}

#[test]
#[ignore = "requires a local Gerbil gxi executable"]
fn command_compiler_real_gxi_module_entry_rejects_unsupported_expected_kind() {
    let Some(compiler) = real_gxi_module_compiler() else {
        return;
    };

    let error = compiler
        .compile(
            GerbilSource::new("audit/control-plane", RICH_LOOP_GRAPH_SOURCE),
            GerbilArtifactKind::WorkspaceViewPolicy,
        )
        .unwrap_err();

    assert!(error.contains("gerbil compiler command failed"));
    assert!(error.contains("unsupported artifact kind"));
    assert!(error.contains("LoopGraph"));
    assert!(error.contains("WorkspaceSchema"));
    assert!(error.contains("WorkspacePatchIntent"));
    assert!(error.contains("AgentScenarioContract"));
    assert!(error.contains("WorkspaceViewPolicy"));
}

#[test]
#[ignore = "requires a local Gerbil gxi executable"]
fn command_compiler_real_gxi_workspace_patch_intent_rejects_loop_graph_source() {
    let Some(compiler) = real_gxi_module_compiler() else {
        return;
    };

    let error = compiler
        .compile(
            GerbilSource::new("audit/control-plane", RICH_LOOP_GRAPH_SOURCE),
            GerbilArtifactKind::WorkspacePatchIntent,
        )
        .unwrap_err();

    assert!(error.contains("gerbil compiler command failed"));
    assert!(error.contains("expected workspace-patch-intent form"));
    assert!(error.contains("loop-graph"));
}

#[test]
#[ignore = "requires a local Gerbil gxi executable"]
fn command_compiler_real_gxi_agent_scenario_contract_rejects_loop_graph_source() {
    let Some(compiler) = real_gxi_module_compiler() else {
        return;
    };

    let error = compiler
        .compile(
            GerbilSource::new("audit/control-plane", RICH_LOOP_GRAPH_SOURCE),
            GerbilArtifactKind::AgentScenarioContract,
        )
        .unwrap_err();

    assert!(error.contains("gerbil compiler command failed"));
    assert!(error.contains("expected agent-scenario-contract form"));
    assert!(error.contains("loop-graph"));
}
