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
mod loop_case_driver;
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
    GerbilAgentPolicyRoutingNativeEpochBacking, GerbilAgentPolicyRoutingNativeEvidence,
    GerbilAgentPolicyRoutingNativeEvidenceList, GerbilAgentPolicyRoutingNativeEvidenceRef,
    GerbilAgentPolicyRoutingNativeInitializeFn, GerbilAgentPolicyRoutingNativeMatchKey,
    GerbilAgentPolicyRoutingNativePayload, GerbilAgentPolicyRoutingNativeProjection,
    GerbilAgentPolicyRoutingNativeRequest, GerbilAgentPolicyRoutingNativeRequestConversionProfile,
    GerbilAgentPolicyRoutingNativeSelectEdgesFn, GerbilAgentPolicyRoutingNativeSelectEdgesRequest,
    GerbilAgentPolicyRoutingNativeSelector, GerbilAgentPolicyRoutingNativeStatus,
    GerbilAgentPolicyRoutingNativeUtf8, GerbilAgentPolicyRoutingNativeUtf8List,
    gerbil_agent_policy_routing_native_request_conversion_profile,
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
    GERBIL_DECK_RUNTIME_MODEL_ROUTE_SELECTION_SCHEMA_ID,
    GERBIL_DECK_RUNTIME_NATIVE_PROJECTION_ABI_ID,
    GERBIL_DECK_RUNTIME_NATIVE_PROJECTION_ABI_VERSION,
    GERBIL_DECK_RUNTIME_NATIVE_PROJECTION_PACKAGE_ID,
    GERBIL_DECK_RUNTIME_NATIVE_TYPED_BRIDGE_RECEIPT_SCHEMA_ID,
    GERBIL_DECK_RUNTIME_POO_POLICY_PROJECTION_SCHEMA_ID,
    GERBIL_DECK_RUNTIME_POO_POLICY_PROJECTION_TYPE_ID,
    GERBIL_DECK_RUNTIME_PROJECT_LOOP_POLICY_PROJECTION_MODULE_SYMBOL,
    GERBIL_DECK_RUNTIME_PROJECT_POO_POLICY_SYMBOL,
    GERBIL_DECK_RUNTIME_PROJECT_RESOLVED_LOOP_POLICY_PACK_SYMBOL, GerbilDeckRuntimeContextMode,
    GerbilDeckRuntimeIsolationMode, GerbilDeckRuntimeModelRoutePolicy,
    GerbilDeckRuntimeModelRoutePolicyRequest, GerbilDeckRuntimeModelRouteSelectedPolicy,
    GerbilDeckRuntimeModelRouteSelectionReceipt, GerbilDeckRuntimeNativeBridgeBoundary,
    GerbilDeckRuntimeNativeBridgeReceipt, GerbilDeckRuntimeNativeBridgeStatus,
    GerbilDeckRuntimePooPolicyProjection, GerbilDeckRuntimeSelectedPolicyKind,
    GerbilDeckRuntimeSerializationBoundary,
    decode_gerbil_deck_runtime_loop_policy_projection_module,
    decode_gerbil_deck_runtime_poo_policy_projection,
    decode_gerbil_deck_runtime_resolved_loop_policy_pack_projection,
    gerbil_deck_runtime_loop_policy_projection_module_request,
    gerbil_deck_runtime_native_projection_abi_contract,
    gerbil_deck_runtime_native_projection_package_manifest,
    gerbil_deck_runtime_native_projection_readiness_plan,
    gerbil_deck_runtime_native_projection_type_manifest,
    gerbil_deck_runtime_poo_policy_projection_contract,
    gerbil_deck_runtime_poo_policy_projection_request,
    gerbil_deck_runtime_poo_policy_projection_type_manifest,
    gerbil_deck_runtime_resolved_loop_policy_pack_projection_request,
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
pub use loop_case_driver::{
    GERBIL_LOOP_CASE_DRIVER_RUST_LOOP_RECEIPT_SCHEMA_ID,
    GERBIL_LOOP_CASE_DRIVER_SCHEME_RECEIPT_KIND, GerbilLoopCaseCommandKind,
    GerbilLoopCaseDriverCapability, GerbilLoopCaseDriverCaseId, GerbilLoopCaseDriverLoopProgramId,
    GerbilLoopCaseDriverLoopProgramProjectionError, GerbilLoopCaseDriverProfileRef,
    GerbilLoopCaseDriverProjectedLoopProgram, GerbilLoopCaseDriverProjectedLoopProgramError,
    GerbilLoopCaseDriverProjectedLoopProgramRequest, GerbilLoopCaseDriverRustLoopReceipt,
    GerbilLoopCaseDriverSchemeReceipt, GerbilLoopCaseDriverSchemeReceiptKind,
    GerbilLoopCaseDriverVerticalTraceError, GerbilLoopCaseDriverVerticalTraceReceipt,
    GerbilLoopCaseRuntimeHandoffStatus, GerbilLoopCaseRuntimeMode, GerbilLoopCaseSchemeBoundary,
    GerbilLoopCaseSerializationBoundary, GerbilLoopCaseSmokeStatus,
    default_gerbil_config_interface_root, gerbil_config_interface_loadpath_with_src,
    load_gerbil_loop_case_driver_projected_loop_program,
    parse_gerbil_loop_case_driver_vertical_trace, project_gerbil_loop_case_driver_loop_action_kind,
    project_gerbil_loop_case_driver_loop_event_kind, project_gerbil_loop_case_driver_loop_program,
    project_gerbil_loop_case_driver_rust_loop_receipt,
    run_gerbil_config_interface_case_driver_smoke,
    run_gerbil_config_interface_case_driver_smoke_in,
    verify_gerbil_loop_case_driver_vertical_trace,
};
pub use loop_case_driver::{
    GerbilLoopCaseDriverRealLlmCaseReceipt, GerbilLoopCaseDriverRealLlmCaseReceiptError,
    parse_gerbil_loop_case_driver_real_llm_case_receipt,
};
pub use marlin_gerbil_ir::GerbilWorkspaceContractFacts;
pub use native_aot_cli::run_gerbil_native_aot_cli;
pub use policy_pack_projection::{
    GERBIL_LOOP_POLICY_PROJECTION_MODULE_PACKAGE_ID,
    GERBIL_LOOP_POLICY_PROJECTION_MODULE_SCHEMA_ID, GERBIL_LOOP_POLICY_PROJECTION_MODULE_TYPE_ID,
    GERBIL_POLICY_PACK_PROJECTION_CHAIN_PACKAGE_ID, GERBIL_POLICY_PACK_PROJECTION_CHAIN_SCHEMA_ID,
    GERBIL_POLICY_PACK_PROJECTION_CHAIN_TYPE_ID, GERBIL_POO_LOOP_PROGRAM_COMPILER_PACKAGE_ID,
    GERBIL_POO_LOOP_PROGRAM_COMPILER_SCHEMA_ID, GERBIL_POO_LOOP_PROGRAM_COMPILER_TYPE_ID,
    GERBIL_RESOLVED_LOOP_POLICY_PACK_PACKAGE_ID, GERBIL_RESOLVED_LOOP_POLICY_PACK_SCHEMA_ID,
    GERBIL_RESOLVED_LOOP_POLICY_PACK_TYPE_ID, GerbilLoopPolicyProjectionModule,
    GerbilPolicyPackProjectionChainReceipt, GerbilPolicyPackReceiptKind,
    GerbilPolicyPackReceiptSummary, GerbilPooLoopProgramCompilerBoundary,
    GerbilPooLoopProgramCompilerOwner, GerbilPooLoopProgramCompilerReceipt,
    GerbilPooLoopProgramCompilerSerializationBoundary, GerbilPooLoopProgramProfileId,
    decode_gerbil_loop_policy_projection_module,
    decode_gerbil_policy_pack_projection_chain_receipt,
    decode_gerbil_poo_loop_program_compiler_receipt, decode_gerbil_resolved_loop_policy_pack,
    gerbil_loop_policy_projection_module_contract,
    gerbil_loop_policy_projection_module_package_manifest,
    gerbil_loop_policy_projection_module_type_manifest,
    gerbil_policy_pack_projection_chain_contract,
    gerbil_policy_pack_projection_chain_package_manifest,
    gerbil_policy_pack_projection_chain_type_manifest, gerbil_poo_loop_program_compiler_contract,
    gerbil_poo_loop_program_compiler_package_manifest,
    gerbil_poo_loop_program_compiler_type_manifest, gerbil_resolved_loop_policy_pack_contract,
    gerbil_resolved_loop_policy_pack_package_manifest,
    gerbil_resolved_loop_policy_pack_type_manifest,
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
    GerbilResidentRuntimeShutdownStatus, GerbilResidentStrategyEventKind,
    GerbilResidentStrategyExecutionPerformanceReceipt,
    GerbilResidentStrategyExecutionPerformanceScope, GerbilResidentStrategyExecutionReceipt,
    GerbilResidentStrategyExecutionRequest, GerbilResidentStrategyExecutionResponse,
    GerbilResidentStrategyExecutionStatus, GerbilResidentStrategyExecutor,
    GerbilResidentStrategyGxiSmokeBridge, GerbilResidentStrategyLaneId,
    GerbilResidentStrategyLanePlan, GerbilResidentStrategyLaneStatus,
    GerbilResidentStrategyRequest, GerbilResidentStrategyRequestId,
    GerbilResidentStrategyRequestReceipt, GerbilResidentStrategyRequestStatus,
    GerbilResidentStrategyServicePlan, GerbilResidentStrategyServiceReceipt,
};
pub use runtime::{
    GERBIL_ADAPTER_MODULE, GERBIL_LOADPATH_ENV, GERBIL_PACKAGE_BUILD_SCRIPT,
    GERBIL_PACKAGE_ROOT_PATH, GERBIL_PACKAGE_SOURCE_PATH, GERBIL_POO_MOP_MODULE,
    GERBIL_POO_OBJECT_MODULE, GERBIL_POO_PACKAGE_NAME, GERBIL_POO_PROTO_MODULE,
    GERBIL_RUNTIME_ASSETS, GERBIL_RUNTIME_BUILD_DEPS, GERBIL_RUNTIME_BUILD_STAGES,
    GERBIL_RUNTIME_BUILD_WORKER_ENV, GERBIL_RUNTIME_COVERAGE_ROOTS,
    GERBIL_RUNTIME_EXCLUDED_PACKAGE_SOURCE_FILES, GERBIL_RUNTIME_PACKAGE_NAME,
    GERBIL_RUNTIME_SOURCE_ROOTS, GERBIL_RUNTIME_SPECIAL_SOURCE_FILES, GerbilRuntimeAsset,
    GerbilRuntimeBuildScriptContract, MARLIN_GERBIL_GSC_ENV, MARLIN_GERBIL_GXC_ENV,
    MARLIN_GERBIL_GXI_ENV, MARLIN_GERBIL_GXPKG_ENV, default_gerbil_gsc_program,
    default_gerbil_gxc_program, default_gerbil_gxi_program, default_gerbil_gxpkg_program,
    gerbil_package_build_script, gerbil_package_root, gerbil_runtime_asset, gerbil_runtime_assets,
    gerbil_runtime_build_script_contract, gerbil_runtime_dependency_loadpath,
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
