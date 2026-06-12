#![cfg(unix)]

use super::support::test_root;
use std::{
    ffi::OsString,
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Output},
};

#[test]
fn gerbil_aot_repair_cli_reports_system_write_without_writing() {
    let fixture = AotRepairFixture::new("aot-repair-cli-plan");

    let output = fixture.run(false, false);
    assert!(output.status.success(), "{output:?}");

    let output = String::from_utf8(output.stdout).expect("utf-8 repair output");
    assert!(output.contains("probe_status=GscBackendUnavailable"));
    assert!(output.contains("backend_repair=RequiresSystemWrite"));
    assert!(output.contains("requires_system_write=true"));
    assert!(!fixture.backend_gsc.exists());
}

#[test]
fn gerbil_aot_repair_cli_requires_explicit_system_write_before_apply() {
    let fixture = AotRepairFixture::new("aot-repair-cli-apply-refused");

    let output = fixture.run(true, false);

    assert!(!output.status.success(), "{output:?}");
    let error = String::from_utf8(output.stderr).expect("utf-8 repair error");
    assert!(error.contains("without --allow-system-write"), "{error}");
    assert!(!fixture.backend_gsc.exists());
}

#[test]
fn gerbil_aot_repair_cli_applies_system_write_when_authorized() {
    let fixture = AotRepairFixture::new("aot-repair-cli-apply-system");

    let output = fixture.run(true, true);
    assert!(output.status.success(), "{output:?}");

    let output = String::from_utf8(output.stdout).expect("utf-8 repair output");
    assert!(output.contains("backend_repair=RequiresSystemWrite"));
    assert!(output.contains("backend_shim=Created"));
    assert!(fixture.backend_gsc.is_file());
    assert!(
        fs::symlink_metadata(&fixture.backend_gsc)
            .expect("backend shim metadata")
            .file_type()
            .is_symlink()
    );
}

struct AotRepairFixture {
    _root: super::support::TestRoot,
    gxc: PathBuf,
    gsc: PathBuf,
    backend_gsc: PathBuf,
    probe_root: PathBuf,
    allowed_root: PathBuf,
}

impl AotRepairFixture {
    fn new(name: &str) -> Self {
        let root = test_root(name);
        fs::create_dir_all(root.path()).expect("create repair fixture root");
        let gxc = root.path().join("gxc");
        let gsc = root.path().join("gsc");
        let backend_gsc = root
            .path()
            .join("outside")
            .join("v0.18.2")
            .join("bin")
            .join("gsc");
        let probe_root = root.path().join("probe-root");
        let allowed_root = root.path().join("allowed");
        write_fake_gxc(&gxc, &backend_gsc);
        write_executable(&gsc, "#!/bin/sh\nexit 0\n");

        Self {
            _root: root,
            gxc,
            gsc,
            backend_gsc,
            probe_root,
            allowed_root,
        }
    }

    fn run(&self, apply: bool, allow_system_write: bool) -> Output {
        Command::new(env!("CARGO_BIN_EXE_marlin-gerbil-aot-repair"))
            .args(self.args(apply, allow_system_write))
            .output()
            .expect("run marlin-gerbil-aot-repair")
    }

    fn args(&self, apply: bool, allow_system_write: bool) -> Vec<OsString> {
        let mut args = vec![
            OsString::from("--root"),
            self.probe_root.as_os_str().to_os_string(),
            OsString::from("--gxc"),
            self.gxc.as_os_str().to_os_string(),
            OsString::from("--gsc"),
            self.gsc.as_os_str().to_os_string(),
            OsString::from("--allowed-root"),
            self.allowed_root.as_os_str().to_os_string(),
        ];
        if apply {
            args.push(OsString::from("--apply"));
        }
        if allow_system_write {
            args.push(OsString::from("--allow-system-write"));
        }
        args
    }
}

fn write_fake_gxc(path: &Path, backend_gsc: &Path) {
    let backend_gsc = backend_gsc
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    write_executable(
        path,
        &format!(
            "#!/bin/sh\nprintf '%s\\n' \"No such file or directory \\\"{backend_gsc}\\\"\" >&2\nexit 1\n"
        ),
    );
}

fn write_executable(path: &Path, source: &str) {
    fs::write(path, source).expect("write executable fixture");
    let mut permissions = fs::metadata(path)
        .expect("fixture executable metadata")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).expect("mark fixture executable");
}
