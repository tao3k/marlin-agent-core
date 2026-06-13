use super::support::{assert_workspace_patch_intent_artifact, local_gxi, test_root};
use marlin_gerbil_scheme::{
    GerbilArtifactKind, GerbilCommandCompiler, GerbilCompiledArtifact, GerbilCompiler, GerbilSource,
};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

#[test]
#[ignore = "requires a local Gerbil gxi executable"]
fn command_compiler_real_gxi_reads_workspace_patch_intent_source_file() {
    if local_gxi().is_none() {
        return;
    };
    let root = test_root("runtime-source-file");
    let compiler = GerbilCommandCompiler::from_default_marlin_runtime_module(root.path())
        .expect("write gerbil runtime assets");
    let source_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
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
}

#[test]
#[ignore = "requires a local Gerbil gxi executable"]
fn command_compiler_real_gxi_runs_compile_source_binary() {
    if local_gxi().is_none() {
        return;
    };
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .and_then(|path| path.parent())
        .expect("resolve workspace root from crate manifest dir");
    let source_path = manifest_dir
        .join("examples")
        .join("workspace-patch-intent-source.ss");
    let mut command = compile_source_command(workspace_root);

    let output = command
        .arg("workspace-patch-intent")
        .arg(&source_path)
        .output()
        .expect("run Gerbil compile-source binary");

    assert!(
        output.status.success(),
        "compile-source binary failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let artifact: GerbilCompiledArtifact =
        serde_json::from_slice(&output.stdout).expect("decode compile-source artifact");
    assert_workspace_patch_intent_artifact(artifact);
}

fn compile_source_command(workspace_root: &Path) -> Command {
    if let Some(program) = option_env!("CARGO_BIN_EXE_marlin-gerbil-compile-source") {
        return Command::new(program);
    }

    if let Some(program) = std::env::var_os("CARGO_BIN_EXE_marlin-gerbil-compile-source") {
        return Command::new(program);
    }

    let target_dir = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| workspace_root.join("target"));
    let compiled_binary = target_dir.join("debug").join(format!(
        "marlin-gerbil-compile-source{}",
        std::env::consts::EXE_SUFFIX
    ));
    if compiled_binary.is_file() {
        return Command::new(compiled_binary);
    }

    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let mut command = Command::new(cargo);
    command.current_dir(workspace_root).args([
        "run",
        "--locked",
        "-p",
        "marlin-gerbil-scheme",
        "--bin",
        "marlin-gerbil-compile-source",
        "--",
    ]);
    command
}
