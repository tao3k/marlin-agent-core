//! Workspace Scheme package and compiler selection for the C ABI link units.

use std::{
    env,
    path::{Path, PathBuf},
};

const MARLIN_GERBIL_GXI_ENV: &str = "MARLIN_GERBIL_GXI";
const MARLIN_GERBIL_GXPKG_ENV: &str = "MARLIN_GERBIL_GXPKG";
const MARLIN_GERBIL_GXC_ENV: &str = "MARLIN_GERBIL_GXC";
const MARLIN_GERBIL_GSC_ENV: &str = "MARLIN_GERBIL_GSC";

/// Returns the workspace Scheme package root.
pub fn gerbil_package_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scheme")
}

/// Returns the configured `gxi` executable path or PATH program name.
fn default_gerbil_gxi_program() -> PathBuf {
    env::var_os(MARLIN_GERBIL_GXI_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("gxi"))
}

/// Returns the configured `gxpkg` executable path or PATH program name.
pub fn default_gerbil_gxpkg_program() -> PathBuf {
    env::var_os(MARLIN_GERBIL_GXPKG_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("gxpkg"))
}

/// Returns the configured `gxc` executable path or PATH program name.
pub fn default_gerbil_gxc_program() -> PathBuf {
    env::var_os(MARLIN_GERBIL_GXC_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("gxc"))
}

/// Returns the configured `Gerbil` Gambit compiler path or PATH program name.
pub fn default_gerbil_gsc_program() -> PathBuf {
    if let Some(program) = env::var_os(MARLIN_GERBIL_GSC_ENV) {
        return PathBuf::from(program);
    }

    gerbil_scheme::GerbilToolchain::new(
        default_gerbil_gxi_program(),
        default_gerbil_gxc_program(),
        PathBuf::from("gsc"),
    )
    .resolved_gsc()
}

/// Resolves a configured Gerbil executable through PATH when it is a program name.
pub fn resolve_gerbil_executable(program: impl AsRef<Path>) -> Option<PathBuf> {
    gerbil_scheme::resolve_gerbil_executable(program)
}
