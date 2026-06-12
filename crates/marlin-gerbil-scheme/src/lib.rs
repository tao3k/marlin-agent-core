//! `Gerbil Scheme` compiler boundary for typed `marlin` artifacts.

mod aot_repair_cli;
mod aot_runtime;
mod artifact;
mod command;
mod compile_source;
mod compiler;
mod hook_policy;
mod real_gxi_gate;
mod runtime;

pub use aot_repair_cli::run_gerbil_aot_repair_cli;
pub use aot_runtime::{
    GerbilAotBackendRepairReceipt, GerbilAotBackendRepairStatus, GerbilAotBackendShimReceipt,
    GerbilAotBackendShimStatus, GerbilAotCommandReceipt, GerbilAotProbeConfig,
    GerbilAotProbeReceipt, GerbilAotProbeStatus,
};
pub use artifact::{GerbilArtifactKind, GerbilArtifactKindMismatch, GerbilCompiledArtifact};
pub use command::{
    GERBIL_COMMAND_PROFILE_ENV, GerbilCommandCompiler, GerbilCommandProfile, GerbilCommandSpec,
    GerbilCompileRequest, GerbilCompileResponse, GerbilRuntimeBinding,
};
pub use compile_source::run_compile_source_cli;
pub use compiler::{GerbilCompiler, GerbilSource, compile_checked};
pub use hook_policy::{
    GerbilHookPolicyCommandEvaluator, GerbilHookPolicyDiagnostic,
    GerbilHookPolicyEvaluationDecodeInput, GerbilHookPolicyEvaluationError,
    GerbilHookPolicyEvaluationOutput, GerbilHookPolicyEvaluationReceipt,
    GerbilHookPolicyInvocation, GerbilHookPolicyInvocationError, GerbilHookPolicyInvocationInput,
    GerbilHookPolicyRuntimeBinding, build_gerbil_hook_policy_invocation,
    decode_gerbil_hook_policy_evaluation,
};
pub use marlin_gerbil_ir::GerbilWorkspaceContractFacts;
pub use real_gxi_gate::{
    RealGxiGateCommand, RealGxiGateError, run_real_gxi_gate_cli, run_real_gxi_gate_from_args,
};
pub use runtime::{
    DEFAULT_GERBIL_GSC_PROGRAM, DEFAULT_GERBIL_GXC_PROGRAM, DEFAULT_GERBIL_GXI_PROGRAM,
    GERBIL_ADAPTER_MODULE, GERBIL_BUILD_SOURCE, GERBIL_COMMAND_ADAPTER_BATCH_PATH,
    GERBIL_COMMAND_ADAPTER_BATCH_SOURCE, GERBIL_COMMAND_ADAPTER_PATH,
    GERBIL_COMMAND_ADAPTER_SOURCE, GERBIL_HOOK_POLICY_ADAPTER_PATH,
    GERBIL_HOOK_POLICY_ADAPTER_SOURCE, GERBIL_LOADPATH_ENV, GERBIL_MARLIN_ADAPTER_PATH,
    GERBIL_MARLIN_ADAPTER_SOURCE, GERBIL_MARLIN_HOOK_POLICY_PATH, GERBIL_MARLIN_HOOK_POLICY_SOURCE,
    GERBIL_MARLIN_PARSER_PATH, GERBIL_MARLIN_PARSER_SOURCE, GERBIL_MARLIN_PROTOCOL_PATH,
    GERBIL_MARLIN_PROTOCOL_SOURCE, GERBIL_MARLIN_REQUEST_PATH, GERBIL_MARLIN_REQUEST_SOURCE,
    GERBIL_PACKAGE_BIN_PATH, GERBIL_PACKAGE_MANIFEST_PATH, GERBIL_PACKAGE_MANIFEST_SOURCE,
    GERBIL_PACKAGE_SOURCE_PATH, GERBIL_RUNTIME_ASSETS, GERBIL_SMOKE_PATH, GERBIL_SMOKE_SOURCE,
    GerbilRuntimeAsset, MARLIN_GERBIL_GSC_ENV, MARLIN_GERBIL_GXC_ENV, MARLIN_GERBIL_GXI_ENV,
    default_gerbil_gsc_program, default_gerbil_gxc_program, default_gerbil_gxi_program,
    gerbil_runtime_assets, gerbil_runtime_loadpath, write_gerbil_runtime_assets,
};
