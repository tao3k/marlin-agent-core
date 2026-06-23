//! `Gerbil Scheme` compiler boundary for typed `marlin` artifacts.

mod agent_policy_routing;
mod agent_policy_routing_native;
mod aot_repair_cli;
mod aot_runtime;
mod artifact;
mod command;
mod compile_source;
mod compiler;
mod deck_runtime_native;
mod deck_runtime_policy;
mod deck_runtime_script;
mod deps;
mod graph_loop_continuation;
mod native_aot_cli;
mod policy_pack_projection;
mod real_gxi_gate;
mod resident_runtime;
mod runtime;
#[doc(hidden)]
pub mod scheme_type_fixtures;
mod scheme_types;
mod working_copy_policy;

pub use agent_policy_routing::{
    GERBIL_AGENT_POLICY_ROUTING_NATIVE_PROJECTION_ABI_ID,
    GERBIL_AGENT_POLICY_ROUTING_NATIVE_PROJECTION_ABI_VERSION,
    GERBIL_AGENT_POLICY_ROUTING_NATIVE_PROJECTION_PACKAGE_ID,
    GERBIL_AGENT_POLICY_ROUTING_NATIVE_SYMBOL, GERBIL_AGENT_POLICY_ROUTING_SCHEMA_ID,
    GERBIL_AGENT_POLICY_ROUTING_TYPE_ID, GerbilAgentPolicyRoutingDecision,
    GerbilAgentPolicyRoutingEvidence, GerbilAgentPolicyRoutingEvidenceKind,
    GerbilAgentPolicyRoutingProjection, decode_gerbil_agent_policy_routing_native_projection,
    decode_gerbil_agent_policy_routing_projection,
    gerbil_agent_policy_routing_native_projection_abi_contract,
    gerbil_agent_policy_routing_native_projection_package_manifest,
    gerbil_agent_policy_routing_native_projection_readiness_plan,
    gerbil_agent_policy_routing_native_projection_request,
    gerbil_agent_policy_routing_projection_contract, gerbil_agent_policy_routing_type_manifest,
    project_gerbil_agent_policy_routing_native_receipt,
    project_gerbil_agent_policy_routing_receipt,
};
pub use agent_policy_routing_native::{
    GERBIL_AGENT_POLICY_ROUTING_NATIVE_ABI_ID, GERBIL_AGENT_POLICY_ROUTING_NATIVE_ABI_VERSION,
    GERBIL_AGENT_POLICY_ROUTING_NATIVE_HEADER_PATH,
    GERBIL_AGENT_POLICY_ROUTING_NATIVE_HEADER_SOURCE,
    GERBIL_AGENT_POLICY_ROUTING_NATIVE_STATUS_ABI_MISMATCH,
    GERBIL_AGENT_POLICY_ROUTING_NATIVE_STATUS_INVALID_PROJECTION,
    GERBIL_AGENT_POLICY_ROUTING_NATIVE_STATUS_NULL_POINTER,
    GERBIL_AGENT_POLICY_ROUTING_NATIVE_STATUS_OK, GerbilAgentPolicyRoutingNativeAbiError,
    GerbilAgentPolicyRoutingNativeEvidence, GerbilAgentPolicyRoutingNativeEvidenceList,
    GerbilAgentPolicyRoutingNativeEvidenceRef, GerbilAgentPolicyRoutingNativeInitializeFn,
    GerbilAgentPolicyRoutingNativeProjection, GerbilAgentPolicyRoutingNativeRequest,
    GerbilAgentPolicyRoutingNativeSelectEdgesFn, GerbilAgentPolicyRoutingNativeSelectEdgesRequest,
    GerbilAgentPolicyRoutingNativeSelector, GerbilAgentPolicyRoutingNativeStatus,
    GerbilAgentPolicyRoutingNativeUtf8, GerbilAgentPolicyRoutingNativeUtf8List,
};
pub use aot_repair_cli::run_gerbil_aot_repair_cli;
pub use aot_runtime::{
    GerbilAotBackendRepairReceipt, GerbilAotBackendRepairStatus, GerbilAotBackendShimReceipt,
    GerbilAotBackendShimStatus, GerbilAotCommandReceipt, GerbilAotProbeConfig,
    GerbilAotProbeReceipt, GerbilAotProbeStatus, GerbilDeckRuntimeNativeAotBuildReceipt,
    GerbilDeckRuntimeNativeAotBuildStatus, GerbilDeckRuntimeNativeAotCommandPlan,
    GerbilDeckRuntimeNativeAotCommandReceipt, GerbilDeckRuntimeNativeAotConfig,
    GerbilDeckRuntimeNativeAotPlan, GerbilDeckRuntimeNativeAotProfile,
    GerbilDeckRuntimeNativeAotStatus, GerbilDeckRuntimeNativeCargoDirective,
    GerbilDeckRuntimeNativeCargoDirectiveKind, GerbilDeckRuntimeNativeStaticLinkPlan,
    GerbilDeckRuntimeNativeStaticLinkStatus, GerbilDeckRuntimeNativeSymbol,
    GerbilDeckRuntimeNativeSymbolAuditMethod, GerbilNativeCCompiler, GerbilNativeLinkLibrary,
    GerbilNativeSymbolAuditor,
};
pub use artifact::{GerbilArtifactKind, GerbilArtifactKindMismatch, GerbilCompiledArtifact};
pub use command::{
    GERBIL_COMMAND_PROFILE_ENV, GerbilCommandProfile, GerbilCommandSpec, GerbilCompileRequest,
};
pub use compile_source::run_compile_source_cli;
pub use compiler::{GerbilCompiler, GerbilSource, compile_checked};
pub use deck_runtime_native::{
    GERBIL_DECK_RUNTIME_NATIVE_ABI_ID, GERBIL_DECK_RUNTIME_NATIVE_ABI_VERSION,
    GERBIL_DECK_RUNTIME_NATIVE_HEADER_PATH, GERBIL_DECK_RUNTIME_NATIVE_HEADER_SOURCE,
    GERBIL_DECK_RUNTIME_NATIVE_NO_POLICY_INDEX, GERBIL_DECK_RUNTIME_NATIVE_STATUS_ABI_MISMATCH,
    GERBIL_DECK_RUNTIME_NATIVE_STATUS_INVALID_SELECTION,
    GERBIL_DECK_RUNTIME_NATIVE_STATUS_NULL_POINTER, GERBIL_DECK_RUNTIME_NATIVE_STATUS_OK,
    GerbilDeckRuntimeNativeAbiError, GerbilDeckRuntimeNativeModelRoutePolicy,
    GerbilDeckRuntimeNativeModelRouteRequest, GerbilDeckRuntimeNativeModelRouteSelection,
    GerbilDeckRuntimeNativeModelRouteSelector, GerbilDeckRuntimeNativeSelectModelRouteFn,
    GerbilDeckRuntimeNativeStatus, GerbilDeckRuntimeNativeUtf8, GerbilDeckRuntimeNativeUtf8List,
};
pub use deck_runtime_policy::{
    GERBIL_DECK_RUNTIME_NATIVE_PROJECTION_ABI_ID,
    GERBIL_DECK_RUNTIME_NATIVE_PROJECTION_ABI_VERSION,
    GERBIL_DECK_RUNTIME_NATIVE_PROJECTION_PACKAGE_ID,
    GERBIL_DECK_RUNTIME_POO_POLICY_PROJECTION_SCHEMA_ID,
    GERBIL_DECK_RUNTIME_POO_POLICY_PROJECTION_TYPE_ID,
    GERBIL_DECK_RUNTIME_PROJECT_POO_POLICY_SYMBOL, GerbilDeckRuntimeContextMode,
    GerbilDeckRuntimeIsolationMode, GerbilDeckRuntimeModelRoutePolicy,
    GerbilDeckRuntimeModelRoutePolicyRequest, GerbilDeckRuntimeModelRouteSelectedPolicy,
    GerbilDeckRuntimeModelRouteSelectionReceipt, GerbilDeckRuntimePooPolicyProjection,
    GerbilDeckRuntimeSelectedPolicyKind, decode_gerbil_deck_runtime_poo_policy_projection,
    gerbil_deck_runtime_native_projection_abi_contract,
    gerbil_deck_runtime_native_projection_package_manifest,
    gerbil_deck_runtime_native_projection_readiness_plan,
    gerbil_deck_runtime_poo_policy_projection_contract,
    gerbil_deck_runtime_poo_policy_projection_request,
    gerbil_deck_runtime_poo_policy_projection_type_manifest,
};
pub use deck_runtime_script::{
    GERBIL_DECK_RUNTIME_SCRIPT_BATCH_DEFAULT_ITERATIONS,
    GERBIL_DECK_RUNTIME_SCRIPT_BATCH_MAX_ELAPSED_US,
    GERBIL_DECK_RUNTIME_SCRIPT_BATCH_METRICS_SCHEMA_ID,
    GERBIL_DECK_RUNTIME_SCRIPT_BATCH_METRICS_TYPE_ID, GERBIL_DECK_RUNTIME_SCRIPT_INTERFACE_KIND,
    GERBIL_DECK_RUNTIME_SCRIPT_INTERFACE_RECEIPT_SCHEMA_ID,
    GERBIL_DECK_RUNTIME_SCRIPT_INTERFACE_RECEIPT_TYPE_ID, GERBIL_DECK_RUNTIME_SCRIPT_KIND,
    GerbilDeckRuntimeScriptAction, GerbilDeckRuntimeScriptBatchMetrics,
    GerbilDeckRuntimeScriptBatchMetricsKind, GerbilDeckRuntimeScriptBatchPerformanceBudget,
    GerbilDeckRuntimeScriptBatchPerformanceReceipt, GerbilDeckRuntimeScriptBatchPerformanceStatus,
    GerbilDeckRuntimeScriptExtensionId, GerbilDeckRuntimeScriptId,
    GerbilDeckRuntimeScriptInterfaceKind, GerbilDeckRuntimeScriptInterfaceReceipt,
    GerbilDeckRuntimeScriptInterfaceReceiptKind, decode_gerbil_deck_runtime_script_batch_metrics,
    decode_gerbil_deck_runtime_script_interface_receipt,
    evaluate_gerbil_deck_runtime_script_batch_performance,
    gerbil_deck_runtime_script_batch_metrics_contract,
    gerbil_deck_runtime_script_interface_receipt_contract,
    gerbil_deck_runtime_script_interface_type_manifest,
};
pub use deps::{
    GerbilDepsAction, GerbilDepsConfig, GerbilDepsError, run_gerbil_deps_cli,
    run_gerbil_deps_from_args,
};
pub use graph_loop_continuation::{
    GERBIL_LOOP_GRAPH_CONTINUATION_NATIVE_PROJECTION_ABI_ID,
    GERBIL_LOOP_GRAPH_CONTINUATION_NATIVE_PROJECTION_ABI_VERSION,
    GERBIL_LOOP_GRAPH_CONTINUATION_NATIVE_PROJECTION_PACKAGE_ID,
    GERBIL_LOOP_GRAPH_CONTINUATION_NATIVE_SYMBOL, GERBIL_LOOP_GRAPH_CONTINUATION_TYPE_ID,
    decode_gerbil_loop_graph_continuation_native_projection,
    decode_gerbil_loop_graph_continuation_projection,
    gerbil_loop_graph_continuation_native_projection_abi_contract,
    gerbil_loop_graph_continuation_native_projection_package_manifest,
    gerbil_loop_graph_continuation_native_projection_readiness_plan,
    gerbil_loop_graph_continuation_native_projection_request,
    gerbil_loop_graph_continuation_projection_contract,
    gerbil_loop_graph_continuation_type_manifest, project_gerbil_loop_graph_continuation_action,
    project_gerbil_loop_graph_continuation_native_action,
};
pub use marlin_gerbil_ir::GerbilWorkspaceContractFacts;
pub use native_aot_cli::run_gerbil_native_aot_cli;
pub use policy_pack_projection::{
    GERBIL_POLICY_PACK_PROJECTION_CHAIN_PACKAGE_ID, GERBIL_POLICY_PACK_PROJECTION_CHAIN_SCHEMA_ID,
    GERBIL_POLICY_PACK_PROJECTION_CHAIN_TYPE_ID, GerbilPolicyPackProjectionChainReceipt,
    GerbilPolicyPackReceiptKind, GerbilPolicyPackReceiptSummary,
    decode_gerbil_policy_pack_projection_chain_receipt,
    gerbil_policy_pack_projection_chain_contract,
    gerbil_policy_pack_projection_chain_package_manifest,
    gerbil_policy_pack_projection_chain_type_manifest,
};
pub use real_gxi_gate::{
    RealGxiGateCommand, RealGxiGateError, run_real_gxi_gate_cli, run_real_gxi_gate_from_args,
};
pub use resident_runtime::{
    GerbilResidentRuntimeHandle, GerbilResidentRuntimeHealthReceipt,
    GerbilResidentRuntimeHealthStatus, GerbilResidentRuntimePlan,
    GerbilResidentRuntimePrepareReceipt, GerbilResidentRuntimeProcess,
    GerbilResidentRuntimeProcessPlan, GerbilResidentRuntimeProcessReceipt,
    GerbilResidentRuntimeProcessStatus, GerbilResidentRuntimeSessionId,
    GerbilResidentRuntimeSessionMode, GerbilResidentRuntimeShutdownReceipt,
    GerbilResidentRuntimeShutdownStatus,
};
pub use runtime::{
    GERBIL_ADAPTER_MODULE, GERBIL_LOADPATH_ENV, GERBIL_PACKAGE_BUILD_SCRIPT,
    GERBIL_PACKAGE_ROOT_PATH, GERBIL_PACKAGE_SOURCE_PATH, GERBIL_POO_MOP_MODULE,
    GERBIL_POO_OBJECT_MODULE, GERBIL_POO_PACKAGE_NAME, GERBIL_POO_PROTO_MODULE,
    GERBIL_RUNTIME_ASSETS, GerbilRuntimeAsset, MARLIN_GERBIL_GSC_ENV, MARLIN_GERBIL_GXC_ENV,
    MARLIN_GERBIL_GXI_ENV, default_gerbil_gsc_program, default_gerbil_gxc_program,
    default_gerbil_gxi_program, gerbil_package_build_script, gerbil_package_root,
    gerbil_runtime_asset, gerbil_runtime_assets, gerbil_runtime_dependency_loadpath,
    gerbil_runtime_loadpath, gerbil_runtime_loadpath_with_dependencies, resolve_gerbil_executable,
    write_gerbil_runtime_assets,
};
pub use scheme_types::{
    GerbilSchemeFieldName, GerbilSchemeNativeAbiContract, GerbilSchemeNativeAbiId,
    GerbilSchemeNativeAbiReadinessPlan, GerbilSchemeNativeProjectionReceipt,
    GerbilSchemeNativeProjectionRequest, GerbilSchemeNativeProjectionStatus,
    GerbilSchemeNativeSymbol, GerbilSchemePackageId, GerbilSchemePackageManifest,
    GerbilSchemePackageManifestValidationReceipt, GerbilSchemePackageNativeReadinessReceipt,
    GerbilSchemeProjectionContract, GerbilSchemeSchemaId, GerbilSchemeTypeDecodeError,
    GerbilSchemeTypeFieldSpec, GerbilSchemeTypeId, GerbilSchemeTypeManifest,
    GerbilSchemeTypeManifestValidationReceipt, GerbilSchemeTypeRegistry, GerbilSchemeTypeSpec,
    GerbilSchemeTypedProjection, GerbilSchemeTypedValue, GerbilSchemeTypedValueValidationReceipt,
    GerbilSchemeValue, decode_gerbil_scheme_native_projection,
    validate_gerbil_scheme_native_projection, validate_gerbil_scheme_package_manifest,
    validate_gerbil_scheme_package_native_readiness, validate_gerbil_scheme_type_manifest,
    validate_gerbil_scheme_typed_value,
};
pub use working_copy_policy::{GerbilWorkingCopyPolicyOperation, GerbilWorkingCopyPolicySelection};
