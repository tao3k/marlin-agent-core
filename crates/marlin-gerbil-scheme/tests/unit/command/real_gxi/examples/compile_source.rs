use std::{
    path::{Path, PathBuf},
    process::Command,
};

#[test]
#[ignore = "runs the compile-source binary"]
fn compile_source_binary_requires_native_abi_projection() {
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
        !output.status.success(),
        "compile-source must not project Gerbil source text in Rust\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("waiting on the native ABI typed projection"),
        "compile-source stderr should name the pending native ABI projection\nstderr:\n{stderr}"
    );
    assert!(
        stderr.contains("Rust must not parse Gerbil source text"),
        "compile-source stderr should reject Rust-side Gerbil source parsing\nstderr:\n{stderr}"
    );
    assert!(
        stderr.contains("Scheme types -> native ABI -> Rust types"),
        "compile-source stderr should preserve the architecture path\nstderr:\n{stderr}"
    );
    assert!(
        stderr.contains("GerbilSchemeNativeProjectionRequest"),
        "compile-source stderr should name the Rust request boundary\nstderr:\n{stderr}"
    );
    assert!(
        stderr.contains("GerbilSchemeNativeProjectionReceipt"),
        "compile-source stderr should name the Rust receipt boundary\nstderr:\n{stderr}"
    );
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
