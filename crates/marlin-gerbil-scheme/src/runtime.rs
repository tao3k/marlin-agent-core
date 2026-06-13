//! Crate-shipped `Gerbil` runtime assets for the `marlin` adapter loadpath.

use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

/// Environment variable that overrides the `gxi` executable path.
pub const MARLIN_GERBIL_GXI_ENV: &str = "MARLIN_GERBIL_GXI";
/// Environment variable that overrides the `gxc` executable path.
pub const MARLIN_GERBIL_GXC_ENV: &str = "MARLIN_GERBIL_GXC";
/// Environment variable that identifies the paired `Gerbil` Gambit compiler.
pub const MARLIN_GERBIL_GSC_ENV: &str = "MARLIN_GERBIL_GSC";

/// Homebrew `gerbil-scheme` executable path used when no override is present.
pub const DEFAULT_GERBIL_GXI_PROGRAM: &str = "/opt/homebrew/opt/gerbil-scheme/bin/gxi";
/// Homebrew `gerbil-scheme` compiler path used when no override is present.
pub const DEFAULT_GERBIL_GXC_PROGRAM: &str = "/opt/homebrew/opt/gerbil-scheme/bin/gxc";
/// Homebrew `gerbil-scheme` Gambit compiler path used when no override is present.
pub const DEFAULT_GERBIL_GSC_PROGRAM: &str = "/opt/homebrew/opt/gerbil-scheme/bin/gsc";

/// Gerbil loadpath environment variable consumed by `gxi`.
pub const GERBIL_LOADPATH_ENV: &str = "GERBIL_LOADPATH";

/// Module entry point for the crate-shipped Marlin command adapter.
pub const GERBIL_ADAPTER_MODULE: &str = ":marlin/adapter";
/// Package manifest path for the crate-shipped `Gerbil` runtime package.
pub const GERBIL_PACKAGE_MANIFEST_PATH: &str = "gerbil.pkg";
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
/// Package manifest source for the crate-shipped `Gerbil` runtime package.
pub const GERBIL_PACKAGE_MANIFEST_SOURCE: &str = include_str!("../gerbil/gerbil.pkg");
/// Source directory inside the crate-shipped `Gerbil` runtime package.
pub const GERBIL_PACKAGE_SOURCE_PATH: &str = "src";
/// Executable launcher directory inside the crate-shipped `Gerbil` runtime package.
pub const GERBIL_PACKAGE_BIN_PATH: &str = "bin";

/// Runtime source asset that can be written into a `gxi` loadpath root.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GerbilRuntimeAsset {
    pub path: &'static str,
    pub source: &'static str,
}

/// Source-file launcher that runs the `:marlin/adapter` command adapter.
pub const GERBIL_COMMAND_ADAPTER_PATH: &str = "bin/command-adapter.ss";
/// Source text for the `:marlin/adapter` command adapter launcher.
pub const GERBIL_COMMAND_ADAPTER_SOURCE: &str = include_str!("../gerbil/bin/command-adapter.ss");
/// Source-file launcher that runs newline-delimited command adapter requests.
pub const GERBIL_COMMAND_ADAPTER_BATCH_PATH: &str = "bin/command-adapter-batch.ss";
/// Source text for the newline-delimited command adapter launcher.
pub const GERBIL_COMMAND_ADAPTER_BATCH_SOURCE: &str =
    include_str!("../gerbil/bin/command-adapter-batch.ss");
/// Source-file launcher that runs a configured `Gerbil Scheme` hook policy.
pub const GERBIL_HOOK_POLICY_ADAPTER_PATH: &str = "bin/hook-policy-adapter.ss";
/// Source text for the configured `Gerbil Scheme` hook policy launcher.
pub const GERBIL_HOOK_POLICY_ADAPTER_SOURCE: &str =
    include_str!("../gerbil/bin/hook-policy-adapter.ss");
/// Source-file launcher that runs the Deck runtime model-route policy selector.
pub const GERBIL_DECK_RUNTIME_POLICY_ADAPTER_PATH: &str = "bin/deck-runtime-policy-adapter.ss";
/// Source text for the Deck runtime model-route policy selector launcher.
pub const GERBIL_DECK_RUNTIME_POLICY_ADAPTER_SOURCE: &str =
    include_str!("../gerbil/bin/deck-runtime-policy-adapter.ss");
/// Build script for compiling the crate-shipped Gerbil runtime assets.
pub const GERBIL_BUILD_SOURCE: &str = include_str!("../gerbil/build.ss");
/// Smoke launcher path inside the crate-shipped `Gerbil` runtime package.
pub const GERBIL_SMOKE_PATH: &str = "bin/smoke.ss";
/// Standalone smoke source used to verify `Gerbil` module loading.
pub const GERBIL_SMOKE_SOURCE: &str = include_str!("../gerbil/bin/smoke.ss");
/// Library module that reads a compile request and emits a compile response.
pub const GERBIL_MARLIN_ADAPTER_PATH: &str = "src/marlin/adapter.ss";
/// Source text for the `:marlin/adapter` library module.
pub const GERBIL_MARLIN_ADAPTER_SOURCE: &str = include_str!("../gerbil/src/marlin/adapter.ss");
/// Library module that exposes the Deck runtime capability bridge.
pub const GERBIL_MARLIN_DECK_RUNTIME_PATH: &str = "src/marlin/deck-runtime.ss";
/// Source text for the Deck runtime capability bridge.
pub const GERBIL_MARLIN_DECK_RUNTIME_SOURCE: &str =
    include_str!("../gerbil/src/marlin/deck-runtime.ss");
/// Library module that exposes macro-specialized Deck runtime policy selectors.
pub const GERBIL_MARLIN_DECK_RUNTIME_COMPILED_POLICY_PATH: &str =
    "src/marlin/deck-runtime-compiled-policy.ss";
/// Source text for macro-specialized Deck runtime policy selectors.
pub const GERBIL_MARLIN_DECK_RUNTIME_COMPILED_POLICY_SOURCE: &str =
    include_str!("../gerbil/src/marlin/deck-runtime-compiled-policy.ss");
/// Package-compiled sample policy template used by compiled macro performance gates.
pub const GERBIL_MARLIN_DECK_RUNTIME_COMPILED_POLICY_SAMPLE_PATH: &str =
    "src/marlin/deck-runtime-compiled-policy-sample.ss";
/// Source text for the package-compiled sample policy template.
pub const GERBIL_MARLIN_DECK_RUNTIME_COMPILED_POLICY_SAMPLE_SOURCE: &str =
    include_str!("../gerbil/src/marlin/deck-runtime-compiled-policy-sample.ss");
/// Native C ABI source for the Deck runtime selector.
pub const GERBIL_MARLIN_DECK_RUNTIME_NATIVE_PATH: &str = "src/marlin/deck-runtime-native.ss";
/// Source text for the native C ABI Deck runtime selector.
pub const GERBIL_MARLIN_DECK_RUNTIME_NATIVE_SOURCE: &str =
    include_str!("../gerbil/src/marlin/deck-runtime-native.ss");
/// Library module that adapts JSON requests into Deck runtime policy selection.
pub const GERBIL_MARLIN_DECK_RUNTIME_POLICY_PATH: &str = "src/marlin/deck-runtime-policy.ss";
/// Source text for the Deck runtime policy selector adapter module.
pub const GERBIL_MARLIN_DECK_RUNTIME_POLICY_SOURCE: &str =
    include_str!("../gerbil/src/marlin/deck-runtime-policy.ss");
/// Library module that dynamically invokes configured hook policy procedures.
pub const GERBIL_MARLIN_HOOK_POLICY_PATH: &str = "src/marlin/hook-policy.ss";
/// Source text for dynamically invoking configured hook policy procedures.
pub const GERBIL_MARLIN_HOOK_POLICY_SOURCE: &str =
    include_str!("../gerbil/src/marlin/hook-policy.ss");
/// Reader-backed source parser for `marlin` smoke artifact forms.
pub const GERBIL_MARLIN_PARSER_PATH: &str = "src/marlin/parser.ss";
/// Source text for the reader-backed source parser.
pub const GERBIL_MARLIN_PARSER_SOURCE: &str = include_str!("../gerbil/src/marlin/parser.ss");
/// Protocol binding constructors and JSON serializers for `marlin` artifacts.
pub const GERBIL_MARLIN_PROTOCOL_PATH: &str = "src/marlin/protocol.ss";
/// Source text for protocol binding constructors and JSON serializers.
pub const GERBIL_MARLIN_PROTOCOL_SOURCE: &str = include_str!("../gerbil/src/marlin/protocol.ss");
/// JSON request decoder for the Rust-to-`Gerbil` command protocol.
pub const GERBIL_MARLIN_REQUEST_PATH: &str = "src/marlin/request.ss";
/// Source text for the Rust-to-`Gerbil` command protocol request decoder.
pub const GERBIL_MARLIN_REQUEST_SOURCE: &str = include_str!("../gerbil/src/marlin/request.ss");

include!(concat!(env!("OUT_DIR"), "/gerbil_runtime_assets.rs"));

/// Returns the crate-owned `Gerbil` runtime asset manifest.
pub fn gerbil_runtime_assets() -> &'static [GerbilRuntimeAsset] {
    GERBIL_RUNTIME_ASSETS
}

/// Returns the package source path that must be exposed through `GERBIL_LOADPATH`.
pub fn gerbil_runtime_loadpath(root: impl AsRef<Path>) -> PathBuf {
    root.as_ref().join(GERBIL_PACKAGE_SOURCE_PATH)
}

/// Returns the configured `gxi` executable path without checking filesystem state.
pub fn default_gerbil_gxi_program() -> PathBuf {
    env::var_os(MARLIN_GERBIL_GXI_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_GERBIL_GXI_PROGRAM))
}

/// Returns the configured `gxc` executable path without checking filesystem state.
pub fn default_gerbil_gxc_program() -> PathBuf {
    env::var_os(MARLIN_GERBIL_GXC_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_GERBIL_GXC_PROGRAM))
}

/// Returns the configured `Gerbil` Gambit compiler path without checking filesystem state.
pub fn default_gerbil_gsc_program() -> PathBuf {
    env::var_os(MARLIN_GERBIL_GSC_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_GERBIL_GSC_PROGRAM))
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
