use marlin_gerbil_scheme::{
    MARLIN_GERBIL_GXI_ENV, RealGxiGateCommand, run_real_gxi_gate_from_args,
};
use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
};
use tempfile::TempDir;

#[test]
fn real_gxi_gate_command_defaults_to_workspace_gerbil_tree() {
    let workspace = TempDir::new().expect("workspace tempdir");
    let gxi = write_fake_workspace_gxi(workspace.path());

    let command = RealGxiGateCommand::from_args(vec![
        OsString::from("gate"),
        OsString::from("--workspace-root"),
        workspace.path().as_os_str().to_os_string(),
        OsString::from("--cargo"),
        OsString::from("cargo-test"),
        OsString::from("--print-command"),
    ])
    .expect("real gxi gate command");

    assert_eq!(command.workspace_root(), workspace.path());
    assert_eq!(command.cargo(), Path::new("cargo-test"));
    assert_eq!(command.args(), expected_gate_args("command::real_gxi"));
    assert_eq!(
        command.env().get(&OsString::from("GERBIL_HOME")),
        Some(&workspace.path().join(".data/gerbil/build").into_os_string())
    );
    assert_eq!(
        command.env().get(&OsString::from(MARLIN_GERBIL_GXI_ENV)),
        Some(&gxi.into_os_string())
    );
    assert!(!command.describe().contains("MARLIN_REQUIRE_REAL_GXI"));
}

#[test]
fn real_gxi_gate_command_uses_explicit_paths_and_filter() {
    let workspace = TempDir::new().expect("workspace tempdir");
    let gerbil_home = workspace.path().join("gerbil-home");
    let gxi = workspace.path().join("tools").join("gxi");
    fs::create_dir_all(&gerbil_home).expect("gerbil home");
    write_file(&gxi, "#!/bin/sh\n");

    let command = RealGxiGateCommand::from_args(vec![
        OsString::from("gate"),
        OsString::from("--workspace-root"),
        workspace.path().as_os_str().to_os_string(),
        OsString::from("--gerbil-home"),
        gerbil_home.as_os_str().to_os_string(),
        OsString::from("--gxi"),
        gxi.as_os_str().to_os_string(),
        OsString::from("--cargo"),
        OsString::from("cargo-custom"),
        OsString::from("--filter"),
        OsString::from("command::real_gxi::examples"),
    ])
    .expect("real gxi gate command");

    assert_eq!(
        command.args(),
        expected_gate_args("command::real_gxi::examples")
    );
    assert_eq!(
        command.env().get(&OsString::from("GERBIL_HOME")),
        Some(&gerbil_home.into_os_string())
    );
    assert_eq!(
        command.env().get(&OsString::from(MARLIN_GERBIL_GXI_ENV)),
        Some(&gxi.into_os_string())
    );
    assert!(
        command
            .describe()
            .contains("cargo-custom test -p marlin-gerbil-scheme")
    );
}

#[test]
fn real_gxi_gate_reports_missing_gxi() {
    let workspace = TempDir::new().expect("workspace tempdir");

    let error = RealGxiGateCommand::from_args(vec![
        OsString::from("gate"),
        OsString::from("--workspace-root"),
        workspace.path().as_os_str().to_os_string(),
    ])
    .expect_err("missing gxi should fail");

    assert!(error.to_string().contains("missing gxi executable"));
}

#[test]
fn real_gxi_gate_reports_missing_gerbil_home() {
    let workspace = TempDir::new().expect("workspace tempdir");
    let gxi = workspace.path().join("tools").join("gxi");
    let missing_home = workspace.path().join("missing-gerbil-home");
    write_file(&gxi, "#!/bin/sh\n");

    let error = RealGxiGateCommand::from_args(vec![
        OsString::from("gate"),
        OsString::from("--workspace-root"),
        workspace.path().as_os_str().to_os_string(),
        OsString::from("--gxi"),
        gxi.as_os_str().to_os_string(),
        OsString::from("--gerbil-home"),
        missing_home.as_os_str().to_os_string(),
    ])
    .expect_err("missing Gerbil home should fail");

    assert!(error.to_string().contains("missing Gerbil home"));
}

#[test]
#[cfg(unix)]
fn real_gxi_gate_reports_failed_cargo_status() {
    use std::os::unix::fs::PermissionsExt;

    let workspace = TempDir::new().expect("workspace tempdir");
    let gxi = write_fake_workspace_gxi(workspace.path());
    let cargo = workspace.path().join("fake-cargo");
    write_file(&cargo, "#!/bin/sh\nexit 7\n");
    fs::set_permissions(&cargo, fs::Permissions::from_mode(0o755)).expect("fake cargo mode");

    let error = run_real_gxi_gate_from_args(vec![
        OsString::from("gate"),
        OsString::from("--workspace-root"),
        workspace.path().as_os_str().to_os_string(),
        OsString::from("--gxi"),
        gxi.into_os_string(),
        OsString::from("--cargo"),
        cargo.into_os_string(),
    ])
    .expect_err("failed cargo should fail gate");

    assert!(
        error
            .to_string()
            .contains("real gxi gate failed with status")
    );
}

fn expected_gate_args(filter: &str) -> Vec<OsString> {
    [
        "test",
        "-p",
        "marlin-gerbil-scheme",
        "--locked",
        "--test",
        "unit_test",
        filter,
        "--",
        "--ignored",
    ]
    .into_iter()
    .map(OsString::from)
    .collect()
}

fn write_fake_workspace_gxi(workspace: &Path) -> PathBuf {
    let gxi = workspace
        .join(".data")
        .join("gerbil")
        .join("build")
        .join("bin")
        .join("gxi");
    write_file(&gxi, "#!/bin/sh\n");
    gxi
}

fn write_file(path: &Path, contents: &str) {
    fs::create_dir_all(path.parent().expect("parent")).expect("parent dir");
    fs::write(path, contents).expect("write file");
}
