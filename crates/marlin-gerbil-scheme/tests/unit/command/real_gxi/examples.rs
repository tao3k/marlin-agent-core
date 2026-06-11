use super::{assert_workspace_patch_intent_artifact, local_gxi};
use marlin_gerbil_scheme::{
    GERBIL_LOADPATH_ENV, GerbilArtifactKind, GerbilCommandCompiler, GerbilCompileResponse,
    GerbilCompiledArtifact, GerbilCompiler, GerbilSource, write_gerbil_runtime_assets,
};
use std::{
    fs,
    path::{Path, PathBuf},
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
        .env(GERBIL_LOADPATH_ENV, root.as_path())
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
fn command_compiler_real_gxi_protocol_bindings_emit_workspace_patch_intent() {
    let Some(gxi) = local_gxi() else {
        return;
    };
    let root = test_root("runtime-protocol-bindings");
    write_gerbil_runtime_assets(&root).expect("write gerbil runtime assets");
    let example = root.join("protocol-bindings-example.ss");
    fs::write(
        &example,
        r#"(import :marlin/protocol)

(def patch
  (make-marlin-workspace-patch
   "gerbil intent"
   "gerbil"
   (list (make-marlin-set-todo-op "memory.org:1:goal" "Done")
         (make-marlin-set-property-op "memory.org:1:goal" "OWNER" "gerbil")
         (make-marlin-mark-memory-candidate-op "memory.org:1:goal" "long-term"))))

(def intent
  (make-marlin-workspace-patch-intent "intent:memory" patch #t))

(display-marlin-compile-response
 (make-marlin-workspace-patch-intent-artifact intent))
(newline)
"#,
    )
    .expect("write protocol bindings example");

    let output = Command::new(gxi)
        .env(GERBIL_LOADPATH_ENV, root.as_path())
        .arg(example)
        .output()
        .expect("run real gxi protocol bindings example");

    assert!(
        output.status.success(),
        "gxi protocol bindings example failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let response: GerbilCompileResponse =
        serde_json::from_slice(&output.stdout).expect("decode protocol bindings response");
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
fn command_compiler_real_gxi_runs_compile_source_example() {
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
    let mut command = compile_source_example_command(workspace_root);

    let output = command
        .arg("workspace-patch-intent")
        .arg(&source_path)
        .output()
        .expect("run Gerbil compile-source example");

    assert!(
        output.status.success(),
        "compile-source example failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let artifact: GerbilCompiledArtifact =
        serde_json::from_slice(&output.stdout).expect("decode compile-source artifact");
    assert_workspace_patch_intent_artifact(artifact);
}

fn compile_source_example_command(workspace_root: &Path) -> Command {
    if let Some(program) = std::env::var_os("CARGO_BIN_EXE_compile-source") {
        return Command::new(program);
    }

    let target_dir = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| workspace_root.join("target"));
    let compiled_example = target_dir
        .join("debug")
        .join("examples")
        .join(format!("compile-source{}", std::env::consts::EXE_SUFFIX));
    if compiled_example.is_file() {
        return Command::new(compiled_example);
    }

    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let mut command = Command::new(cargo);
    command.current_dir(workspace_root).args([
        "run",
        "--locked",
        "-p",
        "marlin-gerbil-scheme",
        "--example",
        "compile-source",
        "--",
    ]);
    command
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
