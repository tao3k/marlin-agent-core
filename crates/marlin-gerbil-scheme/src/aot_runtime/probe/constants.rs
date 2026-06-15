//! Constants for Gerbil AOT module and executable outputs.

use crate::runtime::{
    GERBIL_MARLIN_ADAPTER_PATH, GERBIL_MARLIN_PARSER_PATH, GERBIL_MARLIN_PROTOCOL_PATH,
    GERBIL_MARLIN_REQUEST_PATH,
};

pub(super) const GERBIL_AOT_MODULE_SOURCES: &[&str] = &[
    GERBIL_MARLIN_PROTOCOL_PATH,
    GERBIL_MARLIN_REQUEST_PATH,
    GERBIL_MARLIN_PARSER_PATH,
    GERBIL_MARLIN_ADAPTER_PATH,
];

pub(super) const GERBIL_AOT_EXECUTABLE_NAME: &str = "marlin-gerbil-typed-runtime-aot";
pub(super) const GERBIL_AOT_OUTPUT_DIR: &str = ".gerbil/lib";
pub(super) const GERBIL_AOT_PROBE_CACHE_SCHEMA_VERSION: u32 = 1;
