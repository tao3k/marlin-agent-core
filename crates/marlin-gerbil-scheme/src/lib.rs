//! `Gerbil Scheme` compiler boundary for typed `marlin` artifacts.

mod agent_policy_routing;
mod agent_policy_routing_abi;
mod aot_runtime;
mod deck_runtime_policy;
mod graph_loop_continuation;
mod loop_case_driver;
mod policy_pack_projection;
mod runtime;
#[doc(hidden)]
pub mod scheme_type_fixtures;
mod scheme_types;
mod working_copy_policy;

pub use agent_policy_routing::{
    GERBIL_AGENT_POLICY_ROUTING_ABI_PROJECTION_ABI_ID,
    GERBIL_AGENT_POLICY_ROUTING_ABI_PROJECTION_ABI_VERSION,
    GERBIL_AGENT_POLICY_ROUTING_ABI_PROJECTION_PACKAGE_ID, GERBIL_AGENT_POLICY_ROUTING_ABI_SYMBOL,
    GERBIL_AGENT_POLICY_ROUTING_SCHEMA_ID, GERBIL_AGENT_POLICY_ROUTING_TYPE_ID,
    GerbilAgentPolicyRoutingDecision, GerbilAgentPolicyRoutingEvidence,
    GerbilAgentPolicyRoutingEvidenceKind, GerbilAgentPolicyRoutingProjection,
    decode_gerbil_agent_policy_routing_abi_projection,
    decode_gerbil_agent_policy_routing_projection,
    gerbil_agent_policy_routing_abi_projection_abi_contract,
    gerbil_agent_policy_routing_abi_projection_package_manifest,
    gerbil_agent_policy_routing_abi_projection_readiness_plan,
    gerbil_agent_policy_routing_abi_projection_request,
    gerbil_agent_policy_routing_projection_contract, gerbil_agent_policy_routing_type_manifest,
    project_gerbil_agent_policy_routing_abi_receipt, project_gerbil_agent_policy_routing_receipt,
};
pub use agent_policy_routing_abi::{
    GERBIL_AGENT_POLICY_ROUTING_ABI_HEADER_PATH, GERBIL_AGENT_POLICY_ROUTING_ABI_HEADER_SOURCE,
    GERBIL_AGENT_POLICY_ROUTING_ABI_ID, GERBIL_AGENT_POLICY_ROUTING_ABI_STATUS_ABI_MISMATCH,
    GERBIL_AGENT_POLICY_ROUTING_ABI_STATUS_INVALID_PROJECTION,
    GERBIL_AGENT_POLICY_ROUTING_ABI_STATUS_NULL_POINTER, GERBIL_AGENT_POLICY_ROUTING_ABI_STATUS_OK,
    GERBIL_AGENT_POLICY_ROUTING_ABI_VERSION, GerbilAgentPolicyRoutingAbiEpochBacking,
    GerbilAgentPolicyRoutingAbiError, GerbilAgentPolicyRoutingAbiEvidence,
    GerbilAgentPolicyRoutingAbiEvidenceList, GerbilAgentPolicyRoutingAbiEvidenceRef,
    GerbilAgentPolicyRoutingAbiInitializeFn, GerbilAgentPolicyRoutingAbiMatchKey,
    GerbilAgentPolicyRoutingAbiPayload, GerbilAgentPolicyRoutingAbiProjection,
    GerbilAgentPolicyRoutingAbiRequest, GerbilAgentPolicyRoutingAbiRequestConversionProfile,
    GerbilAgentPolicyRoutingAbiSelectEdgesFn, GerbilAgentPolicyRoutingAbiSelectEdgesRequest,
    GerbilAgentPolicyRoutingAbiSelector, GerbilAgentPolicyRoutingAbiStatus,
    GerbilAgentPolicyRoutingAbiUtf8, GerbilAgentPolicyRoutingAbiUtf8List,
    gerbil_agent_policy_routing_abi_request_conversion_profile,
};
pub use aot_runtime::{
    GerbilAotBuildReceipt, GerbilAotBuildStatus, GerbilAotCargoDirective,
    GerbilAotCargoDirectiveKind, GerbilAotCommandPlan, GerbilAotCommandReceipt, GerbilAotConfig,
    GerbilAotPlan, GerbilAotProfile, GerbilAotStaticLinkPlan, GerbilAotStaticLinkStatus,
    GerbilAotStatus, GerbilAotSymbol, GerbilAotSymbolAuditMethod, GerbilCCompiler,
    GerbilLinkLibrary, GerbilSymbolAuditor,
};
pub use deck_runtime_policy::{
    GERBIL_DECK_ROUTE_ABI_PROJECTION_ABI_ID, GERBIL_DECK_ROUTE_ABI_PROJECTION_ABI_VERSION,
    GERBIL_DECK_ROUTE_ABI_PROJECTION_PACKAGE_ID,
    GERBIL_DECK_RUNTIME_POO_POLICY_PROJECTION_SCHEMA_ID,
    GERBIL_DECK_RUNTIME_POO_POLICY_PROJECTION_TYPE_ID,
    GERBIL_DECK_RUNTIME_PROJECT_LOOP_POLICY_PROJECTION_MODULE_SYMBOL,
    GERBIL_DECK_RUNTIME_PROJECT_POO_POLICY_SYMBOL,
    GERBIL_DECK_RUNTIME_PROJECT_RESOLVED_LOOP_POLICY_PACK_SYMBOL,
    GerbilDeckRuntimePooPolicyProjection, decode_gerbil_deck_runtime_loop_policy_projection_module,
    decode_gerbil_deck_runtime_poo_policy_projection,
    decode_gerbil_deck_runtime_resolved_loop_policy_pack_projection,
    gerbil_deck_route_abi_projection_abi_contract,
    gerbil_deck_route_abi_projection_package_manifest,
    gerbil_deck_route_abi_projection_readiness_plan,
    gerbil_deck_route_abi_projection_type_manifest,
    gerbil_deck_runtime_loop_policy_projection_module_request,
    gerbil_deck_runtime_poo_policy_projection_contract,
    gerbil_deck_runtime_poo_policy_projection_request,
    gerbil_deck_runtime_poo_policy_projection_type_manifest,
    gerbil_deck_runtime_resolved_loop_policy_pack_projection_request,
};
pub use graph_loop_continuation::{
    GERBIL_LOOP_GRAPH_CONTINUATION_ABI_PROJECTION_ABI_ID,
    GERBIL_LOOP_GRAPH_CONTINUATION_ABI_PROJECTION_ABI_VERSION,
    GERBIL_LOOP_GRAPH_CONTINUATION_ABI_PROJECTION_PACKAGE_ID,
    GERBIL_LOOP_GRAPH_CONTINUATION_ABI_SYMBOL, GERBIL_LOOP_GRAPH_CONTINUATION_TYPE_ID,
    decode_gerbil_loop_graph_continuation_abi_projection,
    decode_gerbil_loop_graph_continuation_projection,
    gerbil_loop_graph_continuation_abi_projection_abi_contract,
    gerbil_loop_graph_continuation_abi_projection_package_manifest,
    gerbil_loop_graph_continuation_abi_projection_readiness_plan,
    gerbil_loop_graph_continuation_abi_projection_request,
    gerbil_loop_graph_continuation_projection_contract,
    gerbil_loop_graph_continuation_type_manifest,
    project_gerbil_loop_graph_continuation_abi_action,
    project_gerbil_loop_graph_continuation_action,
};
pub use loop_case_driver::{
    GERBIL_LOOP_CASE_DRIVER_INTENT_CASE_RUNTIME_OWNER,
    GERBIL_LOOP_CASE_DRIVER_RUST_LOOP_RECEIPT_SCHEMA_ID,
    GERBIL_LOOP_CASE_DRIVER_SCHEME_RECEIPT_KIND, GerbilLoopCaseCommandKind,
    GerbilLoopCaseDriverCapability, GerbilLoopCaseDriverCaseId, GerbilLoopCaseDriverLoopProgramId,
    GerbilLoopCaseDriverLoopProgramProjectionError, GerbilLoopCaseDriverProfileRef,
    GerbilLoopCaseDriverRustLoopReceipt, GerbilLoopCaseDriverSchemeReceipt,
    GerbilLoopCaseDriverSchemeReceiptKind, GerbilLoopCaseDriverVerticalTraceError,
    GerbilLoopCaseDriverVerticalTraceReceipt, GerbilLoopCaseRuntimeHandoffStatus,
    GerbilLoopCaseRuntimeMode, GerbilLoopCaseSchemeBoundary, GerbilLoopCaseSerializationBoundary,
    GerbilLoopCaseSmokeStatus, parse_gerbil_loop_case_driver_vertical_trace,
    project_gerbil_loop_case_driver_intent_case_artifact_manifest,
    project_gerbil_loop_case_driver_intent_case_run_receipt,
    project_gerbil_loop_case_driver_loop_action_kind,
    project_gerbil_loop_case_driver_loop_event_kind, project_gerbil_loop_case_driver_loop_program,
    project_gerbil_loop_case_driver_rust_loop_receipt,
    project_gerbil_loop_case_driver_vertical_trace_rust_loop_receipt,
    verify_gerbil_loop_case_driver_vertical_trace,
};
pub use loop_case_driver::{
    GerbilLoopCaseDriverRealLlmCaseReceipt, GerbilLoopCaseDriverRealLlmCaseReceiptError,
    parse_gerbil_loop_case_driver_real_llm_case_receipt,
};
pub use policy_pack_projection::{
    GERBIL_LOOP_POLICY_PROJECTION_MODULE_PACKAGE_ID,
    GERBIL_LOOP_POLICY_PROJECTION_MODULE_SCHEMA_ID, GERBIL_LOOP_POLICY_PROJECTION_MODULE_TYPE_ID,
    GERBIL_POLICY_MIXIN_DEFINITION_SCHEMA_ID, GERBIL_POLICY_MIXIN_STACK_COMPILER_PACKAGE_ID,
    GERBIL_POLICY_MIXIN_STACK_COMPILER_SCHEMA_ID, GERBIL_POLICY_MIXIN_STACK_COMPILER_TYPE_ID,
    GERBIL_POLICY_PACK_PROJECTION_CHAIN_PACKAGE_ID, GERBIL_POLICY_PACK_PROJECTION_CHAIN_SCHEMA_ID,
    GERBIL_POLICY_PACK_PROJECTION_CHAIN_TYPE_ID, GERBIL_POLICY_SLOT_MERGE_RECEIPT_SCHEMA_ID,
    GERBIL_POO_LOOP_PROGRAM_COMPILER_PACKAGE_ID, GERBIL_POO_LOOP_PROGRAM_COMPILER_SCHEMA_ID,
    GERBIL_POO_LOOP_PROGRAM_COMPILER_TYPE_ID, GERBIL_RESOLVED_LOOP_POLICY_PACK_PACKAGE_ID,
    GERBIL_RESOLVED_LOOP_POLICY_PACK_SCHEMA_ID, GERBIL_RESOLVED_LOOP_POLICY_PACK_TYPE_ID,
    GerbilLoopPolicyProjectionModule, GerbilPolicyMixinDefinition,
    GerbilPolicyMixinStackCompilerReceipt, GerbilPolicyPackProjectionChainReceipt,
    GerbilPolicyPackReceiptKind, GerbilPolicyPackReceiptSummary, GerbilPolicySlotMergeReceipt,
    GerbilPooLoopProgramCompilerBoundary, GerbilPooLoopProgramCompilerOwner,
    GerbilPooLoopProgramCompilerReceipt, GerbilPooLoopProgramCompilerSerializationBoundary,
    GerbilPooLoopProgramProfileId, decode_gerbil_loop_policy_projection_module,
    decode_gerbil_policy_mixin_stack_compiler_receipt,
    decode_gerbil_policy_pack_projection_chain_receipt,
    decode_gerbil_poo_loop_program_compiler_receipt, decode_gerbil_resolved_loop_policy_pack,
    gerbil_loop_policy_projection_module_contract,
    gerbil_loop_policy_projection_module_package_manifest,
    gerbil_loop_policy_projection_module_type_manifest,
    gerbil_policy_mixin_stack_compiler_contract,
    gerbil_policy_mixin_stack_compiler_package_manifest,
    gerbil_policy_mixin_stack_compiler_type_manifest, gerbil_policy_pack_projection_chain_contract,
    gerbil_policy_pack_projection_chain_package_manifest,
    gerbil_policy_pack_projection_chain_type_manifest, gerbil_poo_loop_program_compiler_contract,
    gerbil_poo_loop_program_compiler_package_manifest,
    gerbil_poo_loop_program_compiler_type_manifest, gerbil_resolved_loop_policy_pack_contract,
    gerbil_resolved_loop_policy_pack_package_manifest,
    gerbil_resolved_loop_policy_pack_type_manifest,
};
pub use runtime::{
    default_gerbil_gsc_program, default_gerbil_gxc_program, default_gerbil_gxpkg_program,
    gerbil_package_root,
};
pub use scheme_types::{
    GerbilSchemeAbiProjectionReceipt, GerbilSchemeAbiProjectionRequest,
    GerbilSchemeAbiProjectionStatus, GerbilSchemeAbiSymbol,
    GerbilSchemeBackedBindingCatalogReceipt, GerbilSchemeCAbiContract, GerbilSchemeCAbiId,
    GerbilSchemeCAbiReadinessPlan, GerbilSchemeFieldName, GerbilSchemePackageAbiReadinessReceipt,
    GerbilSchemePackageId, GerbilSchemePackageManifest,
    GerbilSchemePackageManifestValidationReceipt, GerbilSchemeProjectionContract,
    GerbilSchemeSchemaId, GerbilSchemeTypeDecodeError, GerbilSchemeTypeFieldSpec,
    GerbilSchemeTypeId, GerbilSchemeTypeManifest, GerbilSchemeTypeManifestValidationReceipt,
    GerbilSchemeTypeRegistry, GerbilSchemeTypeSpec, GerbilSchemeTypedProjection,
    GerbilSchemeTypedValue, GerbilSchemeTypedValueValidationReceipt, GerbilSchemeValue,
    decode_gerbil_scheme_abi_projection, gerbil_scheme_backed_binding_catalog,
    validate_gerbil_scheme_abi_projection, validate_gerbil_scheme_package_abi_readiness,
    validate_gerbil_scheme_package_manifest, validate_gerbil_scheme_type_manifest,
    validate_gerbil_scheme_typed_value,
};
pub use working_copy_policy::{GerbilWorkingCopyPolicyOperation, GerbilWorkingCopyPolicySelection};
