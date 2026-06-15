use super::support::{local_gxi, test_root, write_protocol_bindings_example};
use marlin_gerbil_scheme::{
    GERBIL_LOADPATH_ENV, gerbil_runtime_loadpath, write_gerbil_runtime_assets,
};
use std::process::Command;

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
    let stdout = String::from_utf8(output.stdout).expect("protocol bindings stdout is UTF-8");
    assert_eq!(stdout, "workspace-patch-intent-artifact\n");
    assert!(!stdout.contains("{\""));
}
