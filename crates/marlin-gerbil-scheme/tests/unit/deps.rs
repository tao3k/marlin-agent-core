use marlin_gerbil_scheme::{GerbilDepsConfig, run_gerbil_deps_from_args};
use std::ffi::OsString;
use std::path::PathBuf;
use tempfile::tempdir;

#[test]
fn gerbil_deps_config_requires_homebrew_repair_only_on_macos() {
    let macos = GerbilDepsConfig::for_test("macos", PathBuf::from("/tmp/marlin-home"));
    let linux = GerbilDepsConfig::for_test("linux", PathBuf::from("/tmp/marlin-home"));

    assert!(macos.requires_homebrew_repair());
    assert!(!linux.requires_homebrew_repair());
}

#[test]
fn gerbil_deps_config_describes_platform_specific_plan() {
    let config = GerbilDepsConfig::for_test("linux", PathBuf::from("/tmp/marlin-home"));
    let description = config.describe();

    assert!(description.contains("platform=linux"));
    assert!(description.contains("homebrew_repair=false"));
    assert!(description.contains("gerbil_bin=/tmp/marlin-home/gerbil/bin"));
}

#[test]
fn gerbil_deps_cli_accepts_help_without_host_tools() {
    let result = run_gerbil_deps_from_args([OsString::from("marlin-gerbil-deps"), "--help".into()]);

    assert!(result.is_ok());
}

#[test]
fn gerbil_deps_cli_prints_plan_without_running_bootstrap() {
    let result = run_gerbil_deps_from_args([
        OsString::from("marlin-gerbil-deps"),
        "env".into(),
        "--platform".into(),
        "linux".into(),
        "--gerbil-bin".into(),
        PathBuf::from("/tmp/marlin-gerbil-bin").into_os_string(),
        "--print-plan".into(),
    ]);

    assert!(result.is_ok());
}

#[test]
fn gerbil_deps_cli_skips_homebrew_repair_on_linux_without_tools() {
    let temp = tempdir().expect("tempdir");
    let gerbil_bin = temp.path().join("missing-gerbil-bin");
    let cache_dir = temp.path().join("cache");

    let result = run_gerbil_deps_from_args([
        OsString::from("marlin-gerbil-deps"),
        "repair".into(),
        "--platform".into(),
        "linux".into(),
        "--gerbil-bin".into(),
        gerbil_bin.into_os_string(),
        "--cache-dir".into(),
        cache_dir.into_os_string(),
    ]);

    assert!(result.is_ok());
}
