//! `Gerbil Scheme` compiler boundary for typed `marlin` artifacts.

mod artifact;
mod command;
mod compile_source;
mod compiler;
mod runtime;

pub use artifact::{GerbilArtifactKind, GerbilArtifactKindMismatch, GerbilCompiledArtifact};
pub use command::{
    GERBIL_COMMAND_PROFILE_ENV, GerbilCommandCompiler, GerbilCommandProfile, GerbilCommandSpec,
    GerbilCompileRequest, GerbilCompileResponse, GerbilRuntimeBinding,
};
pub use compile_source::run_compile_source_cli;
pub use compiler::{GerbilCompiler, GerbilSource, compile_checked};
pub use marlin_gerbil_ir::GerbilWorkspaceContractFacts;
pub use runtime::{
    DEFAULT_GERBIL_GSC_PROGRAM, DEFAULT_GERBIL_GXC_PROGRAM, DEFAULT_GERBIL_GXI_PROGRAM,
    GERBIL_ADAPTER_MODULE, GERBIL_BUILD_SOURCE, GERBIL_COMMAND_ADAPTER_SOURCE, GERBIL_LOADPATH_ENV,
    GERBIL_MARLIN_ADAPTER_SOURCE, GERBIL_MARLIN_PARSER_SOURCE, GERBIL_MARLIN_PROTOCOL_SOURCE,
    GERBIL_MARLIN_REQUEST_SOURCE, GERBIL_RUNTIME_ASSETS, GERBIL_SMOKE_SOURCE,
    GerbilAotBackendShimReceipt, GerbilAotBackendShimStatus, GerbilAotCommandReceipt,
    GerbilAotProbeConfig, GerbilAotProbeReceipt, GerbilAotProbeStatus, GerbilRuntimeAsset,
    MARLIN_GERBIL_GSC_ENV, MARLIN_GERBIL_GXC_ENV, MARLIN_GERBIL_GXI_ENV,
    default_gerbil_gsc_program, default_gerbil_gxc_program, default_gerbil_gxi_program,
    gerbil_runtime_assets, write_gerbil_runtime_assets,
};
