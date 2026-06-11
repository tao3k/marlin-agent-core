use super::support::{
    RICH_LOOP_GRAPH_SOURCE, WORKSPACE_PATCH_INTENT_SOURCE, WORKSPACE_SCHEMA_SOURCE,
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
