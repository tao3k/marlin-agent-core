use super::support::{
    RICH_LOOP_GRAPH_SOURCE, assert_rich_loop_graph_artifact, gerbil_fixture_root, local_gxi,
    real_gxi_module_compiler,
};
use marlin_gerbil_scheme::{
    GerbilArtifactKind, GerbilCommandCompiler, GerbilCommandSpec, GerbilCompiler, GerbilSource,
};

#[test]
#[ignore = "requires a local Gerbil gxi executable"]
fn command_compiler_can_call_real_gxi_fixture() {
    let Some(gxi) = local_gxi() else {
        return;
    };

    let fixture_root = gerbil_fixture_root();
    let fixture = fixture_root.join("fixed-loop-graph-adapter.ss");
    let compiler = GerbilCommandCompiler::new(
        GerbilCommandSpec::new(gxi)
            .env("GERBIL_LOADPATH", fixture_root.as_os_str().to_os_string())
            .arg(fixture),
    );

    let artifact = compiler
        .compile(
            GerbilSource::new("audit/control-plane", RICH_LOOP_GRAPH_SOURCE),
            GerbilArtifactKind::LoopGraph,
        )
        .expect("real gxi fixture should compile source text into a typed loop graph artifact");

    assert_rich_loop_graph_artifact(artifact);
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
fn command_compiler_real_gxi_module_entry_rejects_unsupported_expected_kind() {
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
    assert!(error.contains("unsupported artifact kind"));
    assert!(error.contains("LoopGraph"));
    assert!(error.contains("WorkspaceSchema"));
}
