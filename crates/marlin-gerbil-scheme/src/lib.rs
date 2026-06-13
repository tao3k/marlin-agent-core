//! `Gerbil Scheme` compiler boundary for typed `marlin` artifacts.

mod aot_repair_cli;
mod aot_runtime;
mod artifact;
mod command;
mod compile_source;
mod compiler;
mod deck_runtime_native;
mod deck_runtime_policy;
mod deps;
mod hook_policy;
mod native_aot_cli;
mod real_gxi_gate;
mod runtime;
mod scheme_types;

pub use aot_repair_cli::run_gerbil_aot_repair_cli;
pub use aot_runtime::{
    GerbilAotBackendRepairReceipt, GerbilAotBackendRepairStatus, GerbilAotBackendShimReceipt,
    GerbilAotBackendShimStatus, GerbilAotCommandReceipt, GerbilAotProbeConfig,
    GerbilAotProbeReceipt, GerbilAotProbeStatus, GerbilDeckRuntimeNativeAotBuildReceipt,
    GerbilDeckRuntimeNativeAotBuildStatus, GerbilDeckRuntimeNativeAotCommandPlan,
    GerbilDeckRuntimeNativeAotCommandReceipt, GerbilDeckRuntimeNativeAotConfig,
    GerbilDeckRuntimeNativeAotPlan, GerbilDeckRuntimeNativeAotStatus,
    GerbilDeckRuntimeNativeCargoDirective, GerbilDeckRuntimeNativeCargoDirectiveKind,
    GerbilDeckRuntimeNativeStaticLinkPlan, GerbilDeckRuntimeNativeStaticLinkStatus,
    GerbilDeckRuntimeNativeSymbol, GerbilNativeCCompiler, GerbilNativeLinkLibrary,
    GerbilNativeSymbolAuditor,
};
pub use artifact::{GerbilArtifactKind, GerbilArtifactKindMismatch, GerbilCompiledArtifact};
pub use command::{
    GERBIL_COMMAND_PROFILE_ENV, GerbilCommandCompiler, GerbilCommandProfile, GerbilCommandSpec,
    GerbilCompileRequest, GerbilCompileResponse, GerbilRuntimeBinding,
};
pub use compile_source::run_compile_source_cli;
pub use compiler::{GerbilCompiler, GerbilSource, compile_checked};
pub use deck_runtime_native::{
    GERBIL_DECK_RUNTIME_NATIVE_ABI_VERSION, GERBIL_DECK_RUNTIME_NATIVE_HEADER_PATH,
    GERBIL_DECK_RUNTIME_NATIVE_HEADER_SOURCE, GERBIL_DECK_RUNTIME_NATIVE_NO_POLICY_INDEX,
    GERBIL_DECK_RUNTIME_NATIVE_STATUS_ABI_MISMATCH,
    GERBIL_DECK_RUNTIME_NATIVE_STATUS_INVALID_SELECTION,
    GERBIL_DECK_RUNTIME_NATIVE_STATUS_NULL_POINTER, GERBIL_DECK_RUNTIME_NATIVE_STATUS_OK,
    GerbilDeckRuntimeNativeAbiError, GerbilDeckRuntimeNativeModelRoutePolicy,
    GerbilDeckRuntimeNativeModelRouteRequest, GerbilDeckRuntimeNativeModelRouteSelection,
    GerbilDeckRuntimeNativeModelRouteSelector, GerbilDeckRuntimeNativeSelectModelRouteFn,
    GerbilDeckRuntimeNativeStatus, GerbilDeckRuntimeNativeUtf8, GerbilDeckRuntimeNativeUtf8List,
};
pub use deck_runtime_policy::{
    GerbilDeckRuntimeContextMode, GerbilDeckRuntimeIsolationMode,
    GerbilDeckRuntimeModelRoutePolicy, GerbilDeckRuntimeModelRoutePolicyError,
    GerbilDeckRuntimeModelRoutePolicyEvaluator, GerbilDeckRuntimeModelRoutePolicyRequest,
    GerbilDeckRuntimeModelRoutePolicyRuntimeBinding, GerbilDeckRuntimeModelRouteSelectedPolicy,
    GerbilDeckRuntimeModelRouteSelectionReceipt, GerbilDeckRuntimeSelectedPolicyKind,
    decode_gerbil_deck_runtime_model_route_selection,
};
pub use deps::{
    GerbilDepsAction, GerbilDepsConfig, GerbilDepsError, run_gerbil_deps_cli,
    run_gerbil_deps_from_args,
};
pub use hook_policy::{
    GerbilHookPolicyCommandEvaluator, GerbilHookPolicyDiagnostic,
    GerbilHookPolicyEvaluationDecodeInput, GerbilHookPolicyEvaluationError,
    GerbilHookPolicyEvaluationOutput, GerbilHookPolicyEvaluationReceipt,
    GerbilHookPolicyInvocation, GerbilHookPolicyInvocationError, GerbilHookPolicyInvocationInput,
    GerbilHookPolicyRuntimeBinding, build_gerbil_hook_policy_invocation,
    decode_gerbil_hook_policy_evaluation,
};
pub use marlin_gerbil_ir::GerbilWorkspaceContractFacts;
pub use native_aot_cli::run_gerbil_native_aot_cli;
pub use real_gxi_gate::{
    RealGxiGateCommand, RealGxiGateError, run_real_gxi_gate_cli, run_real_gxi_gate_from_args,
};
pub use runtime::{
    DEFAULT_GERBIL_GSC_PROGRAM, DEFAULT_GERBIL_GXC_PROGRAM, DEFAULT_GERBIL_GXI_PROGRAM,
    GERBIL_ADAPTER_MODULE, GERBIL_BUILD_SOURCE, GERBIL_COMMAND_ADAPTER_BATCH_PATH,
    GERBIL_COMMAND_ADAPTER_BATCH_SOURCE, GERBIL_COMMAND_ADAPTER_PATH,
    GERBIL_COMMAND_ADAPTER_SOURCE, GERBIL_DECK_RUNTIME_POLICY_ADAPTER_PATH,
    GERBIL_DECK_RUNTIME_POLICY_ADAPTER_SOURCE, GERBIL_HOOK_POLICY_ADAPTER_PATH,
    GERBIL_HOOK_POLICY_ADAPTER_SOURCE, GERBIL_LOADPATH_ENV, GERBIL_MARLIN_ADAPTER_PATH,
    GERBIL_MARLIN_ADAPTER_SOURCE, GERBIL_MARLIN_DECK_RUNTIME_NATIVE_PATH,
    GERBIL_MARLIN_DECK_RUNTIME_NATIVE_SOURCE, GERBIL_MARLIN_DECK_RUNTIME_PATH,
    GERBIL_MARLIN_DECK_RUNTIME_POLICY_PATH, GERBIL_MARLIN_DECK_RUNTIME_POLICY_SOURCE,
    GERBIL_MARLIN_DECK_RUNTIME_SOURCE, GERBIL_MARLIN_HOOK_POLICY_PATH,
    GERBIL_MARLIN_HOOK_POLICY_SOURCE, GERBIL_MARLIN_PARSER_PATH, GERBIL_MARLIN_PARSER_SOURCE,
    GERBIL_MARLIN_PROTOCOL_PATH, GERBIL_MARLIN_PROTOCOL_SOURCE, GERBIL_MARLIN_REQUEST_PATH,
    GERBIL_MARLIN_REQUEST_SOURCE, GERBIL_PACKAGE_BIN_PATH, GERBIL_PACKAGE_MANIFEST_PATH,
    GERBIL_PACKAGE_MANIFEST_SOURCE, GERBIL_PACKAGE_SOURCE_PATH, GERBIL_POO_DEPENDENCY,
    GERBIL_POO_MOP_MODULE, GERBIL_POO_OBJECT_MODULE, GERBIL_POO_PACKAGE_NAME,
    GERBIL_POO_PROTO_MODULE, GERBIL_RUNTIME_ASSETS, GERBIL_SMOKE_PATH, GERBIL_SMOKE_SOURCE,
    GerbilRuntimeAsset, MARLIN_GERBIL_GSC_ENV, MARLIN_GERBIL_GXC_ENV, MARLIN_GERBIL_GXI_ENV,
    default_gerbil_gsc_program, default_gerbil_gxc_program, default_gerbil_gxi_program,
    gerbil_runtime_assets, gerbil_runtime_loadpath, write_gerbil_runtime_assets,
};
pub use scheme_types::{
    GerbilSchemeFieldName, GerbilSchemeJsonTypeKind, GerbilSchemeSchemaId,
    GerbilSchemeTypeDecodeError, GerbilSchemeTypeFieldSpec, GerbilSchemeTypeId,
    GerbilSchemeTypeManifest, GerbilSchemeTypeManifestValidationReceipt, GerbilSchemeTypeRegistry,
    GerbilSchemeTypeSpec, GerbilSchemeTypedValue, GerbilSchemeTypedValueValidationReceipt,
    decode_gerbil_scheme_type_manifest, decode_gerbil_scheme_typed_value,
    validate_gerbil_scheme_type_manifest, validate_gerbil_scheme_typed_value,
    validate_gerbil_scheme_value_as_type,
};
