//! Constants for Gerbil AOT module and executable outputs.

pub(super) const GERBIL_MARLIN_ADAPTER_PATH: &str = "src/marlin/adapter.ss";

pub(super) const GERBIL_AOT_MODULE_SOURCES: &[&str] = &[
    "src/marlin/protocol.ss",
    "src/marlin/request.ss",
    "src/marlin/parser.ss",
    GERBIL_MARLIN_ADAPTER_PATH,
];

pub(super) const GERBIL_AOT_EXECUTABLE_NAME: &str = "marlin-gerbil-typed-runtime-aot";
pub(super) const GERBIL_AOT_OUTPUT_DIR: &str = ".gerbil/lib";
pub(super) const GERBIL_AOT_PROBE_CACHE_SCHEMA_VERSION: u32 = 1;
