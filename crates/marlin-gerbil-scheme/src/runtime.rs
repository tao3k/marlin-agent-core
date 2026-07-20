//! Crate-shipped `Gerbil` runtime assets for the `marlin` adapter loadpath.

use std::{
    env,
    ffi::OsString,
    fs, io,
    path::{Path, PathBuf},
};

/// Environment variable that overrides the `gxi` executable path.
pub const MARLIN_GERBIL_GXI_ENV: &str = "MARLIN_GERBIL_GXI";
/// Environment variable that overrides the `gxpkg` executable path.
pub const MARLIN_GERBIL_GXPKG_ENV: &str = "MARLIN_GERBIL_GXPKG";
/// Environment variable that overrides the `gxc` executable path.
pub const MARLIN_GERBIL_GXC_ENV: &str = "MARLIN_GERBIL_GXC";
/// Environment variable that identifies the paired `Gerbil` Gambit compiler.
pub const MARLIN_GERBIL_GSC_ENV: &str = "MARLIN_GERBIL_GSC";

/// Gerbil loadpath environment variable consumed by `gxi`.
pub const GERBIL_LOADPATH_ENV: &str = "GERBIL_LOADPATH";

/// Module entry point for the crate-shipped Marlin command adapter.
pub const GERBIL_ADAPTER_MODULE: &str = ":marlin/adapter";
/// Gerbil package name provided by the `poo-flow` dependency.
pub const GERBIL_POO_PACKAGE_NAME: &str = "clan/poo";
/// Object prototype module provided by the `poo-flow` dependency.
pub const GERBIL_POO_OBJECT_MODULE: &str = ":clan/poo/object";
/// Meta-object protocol module provided by the `poo-flow` dependency.
pub const GERBIL_POO_MOP_MODULE: &str = ":clan/poo/mop";
/// Prototype composition module provided by the `poo-flow` dependency.
pub const GERBIL_POO_PROTO_MODULE: &str = ":clan/poo/proto";
/// Source directory inside the crate-shipped `Gerbil` runtime package.
pub const GERBIL_PACKAGE_SOURCE_PATH: &str = "src";
/// Directory that contains the crate-shipped `Gerbil` runtime package.
pub const GERBIL_PACKAGE_ROOT_PATH: &str = "gerbil";
/// Package-owned build script for crate-shipped `Gerbil` runtime assets.
pub const GERBIL_PACKAGE_BUILD_SCRIPT: &str = "build.ss";
/// Gerbil package name owned by the crate-shipped runtime package.
pub const GERBIL_RUNTIME_PACKAGE_NAME: &str = "marlin-deck-runtime";
/// Build stages exposed by the crate-shipped Gerbil package driver.
pub const GERBIL_RUNTIME_BUILD_STAGES: &[&str] = &["spec", "compile", "clean", "test"];
/// Gerbil package dependencies declared by the crate-shipped package driver.
pub const GERBIL_RUNTIME_BUILD_DEPS: &[&str] = &["poo-flow", "gslph"];
/// Source roots scanned by the crate-shipped package driver.
pub const GERBIL_RUNTIME_SOURCE_ROOTS: &[&str] = &["src"];
/// Runtime roots activated through the Scheme harness source coverage API.
pub const GERBIL_RUNTIME_COVERAGE_ROOTS: &[&str] = &["src"];
/// Native sources compiled through dedicated native-link lanes, not package make.
pub const GERBIL_RUNTIME_SPECIAL_SOURCE_FILES: &[&str] = &[
    "marlin/_deck-runtime-native.ssi",
    "marlin/deck-runtime-native.ss",
    "marlin/_agent-policy-routing-native.ssi",
    "marlin/agent-policy-routing-native.ss",
];
/// Sources excluded from the generic package make spec.
pub const GERBIL_RUNTIME_EXCLUDED_PACKAGE_SOURCE_FILES: &[&str] =
    GERBIL_RUNTIME_SPECIAL_SOURCE_FILES;
/// Environment variables accepted by the Gerbil package build worker selector.
pub const GERBIL_RUNTIME_BUILD_WORKER_ENV: &[&str] =
    &["GERBIL_BUILD_CORES", "CARGO_BUILD_JOBS", "NUM_JOBS"];

/// Rust-owned contract facts for the crate-shipped Gerbil package driver.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GerbilRuntimeBuildScriptContract {
    pub package_name: &'static str,
    pub stages: &'static [&'static str],
    pub deps: &'static [&'static str],
    pub source_roots: &'static [&'static str],
    pub coverage_roots: &'static [&'static str],
    pub special_source_files: &'static [&'static str],
    pub excluded_package_source_files: &'static [&'static str],
    pub worker_env: &'static [&'static str],
}

/// Returns the Rust-owned contract facts expected from `gerbil/build.ss`.
pub const fn gerbil_runtime_build_script_contract() -> GerbilRuntimeBuildScriptContract {
    GerbilRuntimeBuildScriptContract {
        package_name: GERBIL_RUNTIME_PACKAGE_NAME,
        stages: GERBIL_RUNTIME_BUILD_STAGES,
        deps: GERBIL_RUNTIME_BUILD_DEPS,
        source_roots: GERBIL_RUNTIME_SOURCE_ROOTS,
        coverage_roots: GERBIL_RUNTIME_COVERAGE_ROOTS,
        special_source_files: GERBIL_RUNTIME_SPECIAL_SOURCE_FILES,
        excluded_package_source_files: GERBIL_RUNTIME_EXCLUDED_PACKAGE_SOURCE_FILES,
        worker_env: GERBIL_RUNTIME_BUILD_WORKER_ENV,
    }
}
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

/// Returns the dependency loadpath managed by `gxpkg` next to `gerbil.pkg`.
pub fn gerbil_runtime_dependency_loadpath() -> PathBuf {
    gerbil_package_root().join(".gerbil").join("lib")
}

/// Returns a `GERBIL_LOADPATH` value for package dependencies plus runtime assets.
///
/// Package-local compiled dependencies come first so runtime execution uses
/// built package modules before falling back to source assets.
pub fn gerbil_runtime_loadpath_with_dependencies(root: impl AsRef<Path>) -> OsString {
    let mut loadpaths = vec![gerbil_runtime_dependency_loadpath()];
    loadpaths.extend(gerbil_user_dependency_loadpaths());
    loadpaths.push(gerbil_runtime_loadpath(root));
    if let Some(existing_loadpath) = env::var_os(GERBIL_LOADPATH_ENV) {
        loadpaths.extend(env::split_paths(&existing_loadpath));
    }
    env::join_paths(loadpaths).expect("Gerbil loadpath entries should be joinable")
}

fn gerbil_user_dependency_loadpaths() -> Vec<PathBuf> {
    let Some(home) = env::var_os("HOME") else {
        return Vec::new();
    };
    let gerbil_home = PathBuf::from(home).join(".gerbil");
    let mut loadpaths = Vec::new();
    let user_lib = gerbil_home.join("lib");
    if user_lib.is_dir() {
        loadpaths.push(user_lib);
    }
    let user_pkg = gerbil_home.join("pkg");
    collect_gerbil_compiled_libs(&user_pkg, &mut loadpaths);
    loadpaths
}

fn collect_gerbil_compiled_libs(path: &Path, loadpaths: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(path) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if path.file_name().and_then(|name| name.to_str()) == Some(".gerbil") {
            let lib = path.join("lib");
            if lib.is_dir() {
                loadpaths.push(lib);
            }
            continue;
        }
        collect_gerbil_compiled_libs(&path, loadpaths);
    }
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
