//! Public boundary for Gerbil policy-pack projection types.
//!
//! This module exposes Rust-owned typed projection contracts while the concrete
//! Gerbil config-interface cases stay in Scheme modules.

mod mixin_stack;
mod optional_scheme;
mod surface;

pub use mixin_stack::{
    GERBIL_POLICY_MIXIN_DEFINITION_SCHEMA_ID, GERBIL_POLICY_MIXIN_STACK_COMPILER_PACKAGE_ID,
    GERBIL_POLICY_MIXIN_STACK_COMPILER_SCHEMA_ID, GERBIL_POLICY_MIXIN_STACK_COMPILER_TYPE_ID,
    GERBIL_POLICY_SLOT_MERGE_RECEIPT_SCHEMA_ID, GerbilPolicyMixinDefinition,
    GerbilPolicyMixinStackCompilerReceipt, GerbilPolicySlotMergeReceipt,
    decode_gerbil_policy_mixin_stack_compiler_receipt, gerbil_policy_mixin_stack_compiler_contract,
    gerbil_policy_mixin_stack_compiler_package_manifest,
    gerbil_policy_mixin_stack_compiler_type_manifest,
};
pub use surface::{
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
