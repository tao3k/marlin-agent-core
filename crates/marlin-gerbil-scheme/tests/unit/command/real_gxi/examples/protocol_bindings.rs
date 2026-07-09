use super::support::{local_gxi, test_root, write_protocol_bindings_example};
use marlin_gerbil_scheme::{
    GERBIL_LOADPATH_ENV, gerbil_runtime_loadpath, write_gerbil_runtime_assets,
};
use std::{
    path::Path,
    process::{Command, Output},
};

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

    let output = run_protocol_bindings_example(build_protocol_bindings_example_command(
        &gxi,
        root.path(),
        &example,
    ))
    .expect("run real gxi protocol bindings example");
    assert_protocol_bindings_example_receipt(output);
}

fn build_protocol_bindings_example_command(gxi: &Path, root: &Path, example: &Path) -> Command {
    let mut command = Command::new(gxi);
    command
        .env(GERBIL_LOADPATH_ENV, gerbil_runtime_loadpath(root))
        .arg(example);
    command
}

fn run_protocol_bindings_example(mut command: Command) -> std::io::Result<Output> {
    command.output()
}

fn assert_protocol_bindings_example_receipt(output: Output) {
    assert!(
        output.status.success(),
        "gxi protocol bindings example failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("protocol bindings stdout is UTF-8");
    assert_eq!(stdout, "workspace-patch-intent-artifact\n");
    assert!(!stdout.contains("{\""));
}
