use super::support::loop_graph_artifact;
use marlin_gerbil_scheme::{
    GerbilArtifactKind, GerbilCommandCompiler, GerbilCommandSpec, GerbilCompiler, GerbilSource,
};

#[test]
fn command_compiler_reads_typed_artifact_from_stdout() {
    let command = GerbilCommandSpec::new("/bin/sh").arg("-c").arg(
        "printf '%s\n' '{\"artifact\":{\"LoopGraph\":{\"graph_id\":\"from-command\",\"nodes\":[],\"edges\":[]}}}'",
    );
    let compiler = GerbilCommandCompiler::new(command);

    let artifact = compiler
        .compile(
            GerbilSource::new("audit/control-plane", "(module audit/control-plane)"),
            GerbilArtifactKind::LoopGraph,
        )
        .expect("command output should decode to requested artifact kind");

    assert_eq!(artifact, loop_graph_artifact("from-command"));
}

#[test]
fn command_compiler_passes_configured_environment() {
    let command = GerbilCommandSpec::new("/bin/sh").arg("-c").arg(
        "printf '%s\n' '{\"artifact\":{\"LoopGraph\":{\"graph_id\":\"'\"$MARLIN_GERBIL_TEST_GRAPH\"'\",\"nodes\":[],\"edges\":[]}}}'",
    )
    .env("MARLIN_GERBIL_TEST_GRAPH", "from-env");
    let compiler = GerbilCommandCompiler::new(command);

    let artifact = compiler
        .compile(
            GerbilSource::new("audit/control-plane", "(module audit/control-plane)"),
            GerbilArtifactKind::LoopGraph,
        )
        .expect("command should receive configured environment values");

    assert_eq!(artifact, loop_graph_artifact("from-env"));
}

#[test]
fn command_compiler_reports_stdout_diagnostics_when_command_fails() {
    let command = GerbilCommandSpec::new("/bin/sh")
        .arg("-c")
        .arg("printf '%s\n' 'adapter expected LoopGraph'; exit 70");
    let compiler = GerbilCommandCompiler::new(command);

    let error = compiler
        .compile(
            GerbilSource::new("audit/control-plane", "(module audit/control-plane)"),
            GerbilArtifactKind::LoopGraph,
        )
        .unwrap_err();

    assert!(error.contains("gerbil compiler command failed"));
    assert!(error.contains("exit status: 70"));
    assert!(error.contains("adapter expected LoopGraph"));
}

#[test]
fn command_compiler_rejects_mismatched_artifact_kind() {
    let command = GerbilCommandSpec::new("/bin/sh").arg("-c").arg(
        "printf '%s\n' '{\"artifact\":{\"WorkspaceSchema\":{\"schema_id\":\"workspace-record\",\"required_properties\":[\"ID\"],\"todo_states\":[\"TODO\",\"DONE\"]}}}'",
    );
    let compiler = GerbilCommandCompiler::new(command);

    let error = compiler
        .compile(
            GerbilSource::new("audit/control-plane", "(module audit/control-plane)"),
            GerbilArtifactKind::LoopGraph,
        )
        .unwrap_err();

    assert!(error.contains("expected LoopGraph"));
    assert!(error.contains("got WorkspaceSchema"));
}
