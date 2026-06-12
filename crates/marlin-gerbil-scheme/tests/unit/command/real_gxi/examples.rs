use super::{assert_workspace_patch_intent_artifact, local_gxi};
use marlin_agent_runtime::{
    AsyncManagedChildProcess, ManagedChildProcessSpec, RuntimeContext, TokioAgentRuntime,
    observability,
};
use marlin_gerbil_scheme::{
    GERBIL_LOADPATH_ENV, GerbilArtifactKind, GerbilCommandCompiler, GerbilCompileResponse,
    GerbilCompiledArtifact, GerbilCompiler, GerbilSource, gerbil_runtime_loadpath,
    write_gerbil_runtime_assets,
};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
use tempfile::{Builder, TempDir};
use tokio::process::Command as AsyncCommand;

#[test]
#[ignore = "requires a local Gerbil gxi executable"]
fn command_compiler_real_gxi_runs_workspace_patch_intent_example_from_runtime_assets() {
    let Some(gxi) = local_gxi() else {
        return;
    };
    let root = test_root("runtime-example");
    write_gerbil_runtime_assets(root.path()).expect("write gerbil runtime assets");
    let example = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join("workspace-patch-intent.ss");

    let output = Command::new(gxi)
        .env(GERBIL_LOADPATH_ENV, gerbil_runtime_loadpath(root.path()))
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
}

#[test]
#[ignore = "requires a local Gerbil gxi executable"]
fn command_compiler_real_gxi_protocol_bindings_emit_workspace_patch_intent() {
    let Some(gxi) = local_gxi() else {
        return;
    };
    let root = test_root("runtime-protocol-bindings");
    write_gerbil_runtime_assets(root.path()).expect("write gerbil runtime assets");
    let example = root.path().join("protocol-bindings-example.ss");
    write_protocol_bindings_example(&example);

    let output = Command::new(gxi)
        .env(GERBIL_LOADPATH_ENV, gerbil_runtime_loadpath(root.path()))
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
}

#[tokio::test]
#[ignore = "requires a local Gerbil gxi executable"]
async fn command_compiler_real_gxi_protocol_bindings_have_runtime_process_visibility() {
    let Some(gxi) = local_gxi() else {
        return;
    };
    let root = test_root("runtime-protocol-bindings-visibility");
    write_gerbil_runtime_assets(root.path()).expect("write gerbil runtime assets");
    let example = root.path().join("protocol-bindings-example.ss");
    write_protocol_bindings_example(&example);
    let (runtime, _events) = TokioAgentRuntime::new(4);
    let context = runtime.context();
    let mut command = AsyncCommand::new(gxi);
    command
        .env(GERBIL_LOADPATH_ENV, gerbil_runtime_loadpath(root.path()))
        .arg(&example)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let child = AsyncManagedChildProcess::spawn_with_spec(
        &context,
        command,
        ManagedChildProcessSpec::new(
            observability::RuntimeProcessKind::Tool,
            "gerbil:protocol-bindings-example",
        )
        .with_started_at_ms(10),
    )
    .await
    .expect("runtime-visible real gxi protocol bindings example should spawn");
    let pid = child.pid();
    assert_eq!(
        context
            .process_registry()
            .get(pid)
            .map(|process| process.status),
        Some(observability::RuntimeProcessStatus::Running)
    );

    let output = child
        .wait_with_output_observed_at(20)
        .await
        .expect("runtime-visible real gxi protocol bindings example should finish");

    assert!(
        output.status.success(),
        "gxi protocol bindings example failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let response: GerbilCompileResponse =
        serde_json::from_slice(&output.stdout).expect("decode protocol bindings response");
    assert_workspace_patch_intent_artifact(response.artifact);
    assert_runtime_process_registry_empty(&context);
}

#[test]
#[ignore = "requires a local Gerbil gxi executable"]
fn command_compiler_real_gxi_reads_workspace_patch_intent_source_file() {
    if local_gxi().is_none() {
        return;
    };
    let root = test_root("runtime-source-file");
    let compiler = GerbilCommandCompiler::from_default_marlin_runtime_module(root.path())
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
}

#[tokio::test]
#[ignore = "requires a local Gerbil gxi executable"]
async fn command_compiler_real_gxi_reads_source_file_with_runtime_process_visibility() {
    if local_gxi().is_none() {
        return;
    };
    let root = test_root("runtime-source-file-visibility");
    let compiler = GerbilCommandCompiler::from_default_marlin_runtime_module(root.path())
        .expect("write gerbil runtime assets");
    let source_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join("workspace-patch-intent-source.ss");
    let source = fs::read_to_string(&source_path).expect("read gerbil source example");
    let (runtime, _events) = TokioAgentRuntime::new(4);
    let context = runtime.context();

    let artifact = compiler
        .compile_with_runtime(
            &context,
            GerbilSource::new(source_path.to_string_lossy(), source),
            GerbilArtifactKind::WorkspacePatchIntent,
        )
        .await
        .expect("compile gerbil source file with runtime-visible real gxi");

    assert_workspace_patch_intent_artifact(artifact);
    assert_runtime_process_registry_empty(&context);
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

#[test]
#[ignore = "requires a local Gerbil gxi executable"]
fn command_compiler_real_gxi_build_script_compiles_runtime_assets() {
    let Some(gxi) = local_gxi() else {
        return;
    };
    let root = test_root("runtime-build-script");
    write_gerbil_runtime_assets(root.path()).expect("write gerbil runtime assets");

    let output = Command::new(gxi)
        .env(GERBIL_LOADPATH_ENV, gerbil_runtime_loadpath(root.path()))
        .current_dir(root.path())
        .arg(root.path().join("build.ss"))
        .arg("compile")
        .output()
        .expect("run real gxi build script");

    assert!(
        output.status.success(),
        "gxi build script failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn test_root(name: &str) -> TempDir {
    Builder::new()
        .prefix(&format!("marlin-gerbil-scheme-{name}-"))
        .tempdir()
        .unwrap_or_else(|error| panic!("creates {name} test root: {error}"))
}

fn write_protocol_bindings_example(example: &Path) {
    fs::write(
        example,
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
}

fn assert_runtime_process_registry_empty(context: &RuntimeContext) {
    let registry = context.process_registry();
    assert!(registry.active_processes().is_empty());
    assert!(registry.cleanup_candidates().is_empty());
}
