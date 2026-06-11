//! Crate-shipped `Gerbil` runtime assets for the `marlin` adapter loadpath.

use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

/// Environment variable that overrides the `gxi` executable path.
pub const MARLIN_GERBIL_GXI_ENV: &str = "MARLIN_GERBIL_GXI";

/// Homebrew `gerbil-scheme` executable path used when no override is present.
pub const DEFAULT_GERBIL_GXI_PROGRAM: &str = "/opt/homebrew/opt/gerbil-scheme/bin/gxi";

/// Gerbil loadpath environment variable consumed by `gxi`.
pub const GERBIL_LOADPATH_ENV: &str = "GERBIL_LOADPATH";

/// Module entry point for the crate-shipped Marlin command adapter.
pub const GERBIL_ADAPTER_MODULE: &str = ":marlin/adapter";

/// Runtime source asset that can be written into a `gxi` loadpath root.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GerbilRuntimeAsset {
    pub path: &'static str,
    pub source: &'static str,
}

/// Source-file launcher that runs the `:marlin/adapter` command adapter.
pub const GERBIL_COMMAND_ADAPTER_SOURCE: &str =
    include_str!("../fixtures/gerbil/command-adapter.ss");
/// Source-file launcher that runs newline-delimited command adapter requests.
pub const GERBIL_COMMAND_ADAPTER_BATCH_SOURCE: &str =
    include_str!("../fixtures/gerbil/command-adapter-batch.ss");
/// Build script for compiling the crate-shipped Gerbil runtime assets.
pub const GERBIL_BUILD_SOURCE: &str = include_str!("../fixtures/gerbil/build.ss");
/// Standalone smoke source used to verify `Gerbil` module loading.
pub const GERBIL_SMOKE_SOURCE: &str = include_str!("../fixtures/gerbil/smoke.ss");
/// Library module that reads a compile request and emits a compile response.
pub const GERBIL_MARLIN_ADAPTER_SOURCE: &str = include_str!("../fixtures/gerbil/marlin/adapter.ss");
/// Reader-backed source parser for `marlin` smoke artifact forms.
pub const GERBIL_MARLIN_PARSER_SOURCE: &str = include_str!("../fixtures/gerbil/marlin/parser.ss");
/// Protocol binding constructors and JSON serializers for `marlin` artifacts.
pub const GERBIL_MARLIN_PROTOCOL_SOURCE: &str =
    include_str!("../fixtures/gerbil/marlin/protocol.ss");
/// JSON request decoder for the Rust-to-`Gerbil` command protocol.
pub const GERBIL_MARLIN_REQUEST_SOURCE: &str = include_str!("../fixtures/gerbil/marlin/request.ss");

/// Complete file manifest required under a `GERBIL_LOADPATH` root.
pub const GERBIL_RUNTIME_ASSETS: &[GerbilRuntimeAsset] = &[
    GerbilRuntimeAsset {
        path: "command-adapter.ss",
        source: GERBIL_COMMAND_ADAPTER_SOURCE,
    },
    GerbilRuntimeAsset {
        path: "command-adapter-batch.ss",
        source: GERBIL_COMMAND_ADAPTER_BATCH_SOURCE,
    },
    GerbilRuntimeAsset {
        path: "build.ss",
        source: GERBIL_BUILD_SOURCE,
    },
    GerbilRuntimeAsset {
        path: "smoke.ss",
        source: GERBIL_SMOKE_SOURCE,
    },
    GerbilRuntimeAsset {
        path: "marlin/adapter.ss",
        source: GERBIL_MARLIN_ADAPTER_SOURCE,
    },
    GerbilRuntimeAsset {
        path: "marlin/parser.ss",
        source: GERBIL_MARLIN_PARSER_SOURCE,
    },
    GerbilRuntimeAsset {
        path: "marlin/protocol.ss",
        source: GERBIL_MARLIN_PROTOCOL_SOURCE,
    },
    GerbilRuntimeAsset {
        path: "marlin/request.ss",
        source: GERBIL_MARLIN_REQUEST_SOURCE,
    },
];

/// Returns the crate-owned `Gerbil` runtime asset manifest.
pub fn gerbil_runtime_assets() -> &'static [GerbilRuntimeAsset] {
    GERBIL_RUNTIME_ASSETS
}

/// Returns the configured `gxi` executable path without checking filesystem state.
pub fn default_gerbil_gxi_program() -> PathBuf {
    env::var_os(MARLIN_GERBIL_GXI_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_GERBIL_GXI_PROGRAM))
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
