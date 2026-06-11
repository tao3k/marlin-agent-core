//! Crate-shipped `Gerbil` runtime assets for the `marlin` adapter loadpath.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

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
