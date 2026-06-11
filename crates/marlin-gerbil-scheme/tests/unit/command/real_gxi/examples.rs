use super::{assert_workspace_patch_intent_artifact, local_gxi};
use marlin_gerbil_scheme::{
    GERBIL_LOADPATH_ENV, GerbilArtifactKind, GerbilCommandCompiler, GerbilCompileResponse,
    GerbilCompiler, GerbilSource, write_gerbil_runtime_assets,
};
use std::{
    fs,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
#[ignore = "requires a local Gerbil gxi executable"]
fn command_compiler_real_gxi_runs_workspace_patch_intent_example_from_runtime_assets() {
    let Some(gxi) = local_gxi() else {
        return;
    };
    let root = test_root("runtime-example");
    write_gerbil_runtime_assets(&root).expect("write gerbil runtime assets");
    let example = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join("workspace-patch-intent.ss");

    let output = Command::new(gxi)
        .env(GERBIL_LOADPATH_ENV, &root)
        .arg(example)
        .output()
        .expect("run real gxi workspace patch intent example");

    assert!(
        output.status.success(),
        "gxi example failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let response: GerbilCompileResponse =
        serde_json::from_slice(&output.stdout).expect("decode example response");
    assert_workspace_patch_intent_artifact(response.artifact);
    let _ = fs::remove_dir_all(root);
}

#[test]
#[ignore = "requires a local Gerbil gxi executable"]
fn command_compiler_real_gxi_reads_workspace_patch_intent_source_file() {
    if local_gxi().is_none() {
        return;
    };
    let root = test_root("runtime-source-file");
    let compiler = GerbilCommandCompiler::from_default_marlin_runtime_module(&root)
        .expect("write gerbil runtime assets");
    let source_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join("workspace-patch-intent-source.ss");
    let source = fs::read_to_string(&source_path).expect("read gerbil source example");

    let artifact = compiler
        .compile(
            GerbilSource::new(source_path.to_string_lossy(), source),
            GerbilArtifactKind::WorkspacePatchIntent,
        )
        .expect("compile gerbil source file with real gxi");

    assert_workspace_patch_intent_artifact(artifact);
    let _ = fs::remove_dir_all(root);
}

#[test]
#[ignore = "requires a local Gerbil gxi executable"]
fn command_compiler_real_gxi_build_script_compiles_runtime_assets() {
    let Some(gxi) = local_gxi() else {
        return;
    };
    let root = test_root("runtime-build-script");
    write_gerbil_runtime_assets(&root).expect("write gerbil runtime assets");

    let output = Command::new(gxi)
        .env(GERBIL_LOADPATH_ENV, &root)
        .current_dir(&root)
        .arg(root.join("build.ss"))
        .arg("compile")
        .output()
        .expect("run real gxi build script");

    assert!(
        output.status.success(),
        "gxi build script failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
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
