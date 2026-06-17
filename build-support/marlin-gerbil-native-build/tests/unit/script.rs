use std::ffi::OsStr;

use marlin_gerbil_native_build::rustc_wrapper_is_clippy;

#[test]
fn detects_clippy_driver_from_rustc_workspace_wrapper() {
    assert!(rustc_wrapper_is_clippy(OsStr::new(
        "/nix/store/toolchain/bin/clippy-driver"
    )));
    assert!(rustc_wrapper_is_clippy(OsStr::new(
        "/Users/dev/.rustup/toolchains/stable/bin/clippy-driver"
    )));
}

#[test]
fn does_not_treat_plain_rustc_wrapper_as_clippy() {
    assert!(!rustc_wrapper_is_clippy(OsStr::new(
        "/nix/store/toolchain/bin/rustc"
    )));
    assert!(!rustc_wrapper_is_clippy(OsStr::new(
        "/nix/store/toolchain/bin/sccache"
    )));
    assert!(!rustc_wrapper_is_clippy(OsStr::new(
        "/tmp/clippy-driver-wrapper/rustc"
    )));
}
