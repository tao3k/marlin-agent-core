//! Crate-shipped `Gerbil` runtime assets for the `marlin` adapter loadpath.

use std::{
    env, fs, io,
    path::{Component, Path, PathBuf},
};

/// Environment variable that overrides the `gxi` executable path.
pub const MARLIN_GERBIL_GXI_ENV: &str = "MARLIN_GERBIL_GXI";
/// Environment variable that overrides the `gxc` executable path.
pub const MARLIN_GERBIL_GXC_ENV: &str = "MARLIN_GERBIL_GXC";
/// Environment variable that identifies the paired `Gerbil` Gambit compiler.
pub const MARLIN_GERBIL_GSC_ENV: &str = "MARLIN_GERBIL_GSC";

/// Gerbil loadpath environment variable consumed by `gxi`.
pub const GERBIL_LOADPATH_ENV: &str = "GERBIL_LOADPATH";

/// Module entry point for the crate-shipped Marlin command adapter.
pub const GERBIL_ADAPTER_MODULE: &str = ":marlin/adapter";
/// Object-system package dependency used by the crate-shipped runtime package.
pub const GERBIL_POO_DEPENDENCY: &str = "git.cons.io/mighty-gerbils/gerbil-poo";
/// Gerbil package name provided by the `gerbil-poo` dependency.
pub const GERBIL_POO_PACKAGE_NAME: &str = "clan/poo";
/// Object prototype module provided by the `gerbil-poo` dependency.
pub const GERBIL_POO_OBJECT_MODULE: &str = ":clan/poo/object";
/// Meta-object protocol module provided by the `gerbil-poo` dependency.
pub const GERBIL_POO_MOP_MODULE: &str = ":clan/poo/mop";
/// Prototype composition module provided by the `gerbil-poo` dependency.
pub const GERBIL_POO_PROTO_MODULE: &str = ":clan/poo/proto";
/// Source directory inside the crate-shipped `Gerbil` runtime package.
pub const GERBIL_PACKAGE_SOURCE_PATH: &str = "src";
/// Directory that contains the crate-shipped `Gerbil` runtime package.
pub const GERBIL_PACKAGE_ROOT_PATH: &str = "gerbil";
/// Package-owned build script for crate-shipped `Gerbil` runtime assets.
pub const GERBIL_PACKAGE_BUILD_SCRIPT: &str = "build.ss";
/// Runtime source asset that can be written into a `gxi` loadpath root.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GerbilRuntimeAsset {
    pub path: &'static str,
    pub source: &'static str,
}

include!(concat!(env!("OUT_DIR"), "/gerbil_runtime_assets.rs"));

/// Returns the crate-owned `Gerbil` runtime asset manifest.
pub fn gerbil_runtime_assets() -> &'static [GerbilRuntimeAsset] {
    GERBIL_RUNTIME_ASSETS
}

/// Finds one crate-owned `Gerbil` runtime asset by package-relative path.
pub fn gerbil_runtime_asset(path: &str) -> Option<&'static GerbilRuntimeAsset> {
    GERBIL_RUNTIME_ASSETS
        .iter()
        .find(|asset| asset.path == path)
}

/// Returns the package source path that must be exposed through `GERBIL_LOADPATH`.
pub fn gerbil_runtime_loadpath(root: impl AsRef<Path>) -> PathBuf {
    root.as_ref().join(GERBIL_PACKAGE_SOURCE_PATH)
}

/// Returns the crate-owned `Gerbil` package root.
pub fn gerbil_package_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(GERBIL_PACKAGE_ROOT_PATH)
}

/// Returns the crate-owned package build script.
pub fn gerbil_package_build_script() -> PathBuf {
    gerbil_package_root().join(GERBIL_PACKAGE_BUILD_SCRIPT)
}

/// Returns the configured `gxi` executable path or PATH program name.
pub fn default_gerbil_gxi_program() -> PathBuf {
    env::var_os(MARLIN_GERBIL_GXI_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("gxi"))
}

/// Returns the configured `gxc` executable path or PATH program name.
pub fn default_gerbil_gxc_program() -> PathBuf {
    env::var_os(MARLIN_GERBIL_GXC_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("gxc"))
}

/// Returns the configured `Gerbil` Gambit compiler path or PATH program name.
pub fn default_gerbil_gsc_program() -> PathBuf {
    env::var_os(MARLIN_GERBIL_GSC_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("gsc"))
}

/// Resolves a configured Gerbil executable through PATH when it is a program name.
pub fn resolve_gerbil_executable(program: impl AsRef<Path>) -> Option<PathBuf> {
    let program = program.as_ref();
    if should_check_gerbil_program_directly(program) {
        return program.is_file().then(|| program.to_path_buf());
    }

    env::var_os("PATH").and_then(|paths| {
        env::split_paths(&paths)
            .map(|dir| dir.join(program))
            .find(|candidate| candidate.is_file())
    })
}

fn should_check_gerbil_program_directly(program: &Path) -> bool {
    if program.has_root() {
        return true;
    }
    let mut components = program.components();
    let Some(first) = components.next() else {
        return false;
    };
    matches!(
        first,
        Component::CurDir | Component::ParentDir | Component::Prefix(_)
    ) || components.next().is_some()
}

/// Writes the crate-owned `Gerbil` runtime assets under a loadpath root.
pub fn write_gerbil_runtime_assets(root: impl AsRef<Path>) -> io::Result<Vec<PathBuf>> {
    let root = root.as_ref();
    let mut written = Vec::with_capacity(GERBIL_RUNTIME_ASSETS.len());
    for asset in GERBIL_RUNTIME_ASSETS {
        let path = root.join(asset.path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, asset.source)?;
        written.push(path);
    }
    Ok(written)
}
