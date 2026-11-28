// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

pub(super) use marlin_agent_kernel::{
    LoopProgramExecutionDriver, LoopProgramExecutionRequest, LoopProgramExecutionStatus,
    LoopProgramRuntimeHandoffExecutionReportStatus, LoopProgramRuntimeHandoffHandler,
    LoopProgramRuntimeHandoffRouter, LoopProgramRuntimeHandoffRouterHandlers,
    LoopProgramRuntimeOwner, ReceiptDrivenLoopProgramEventMapper, ScriptedLoopProgramEventMapper,
    StaticLoopProgramRuntimeHandoffHandler,
};
pub(super) use marlin_agent_protocol::{LoopProgramActionKind, LoopProgramEventKind};
pub(super) use marlin_gerbil_scheme::{
    GERBIL_DECK_RUNTIME_PROJECT_LOOP_POLICY_PROJECTION_MODULE_SYMBOL,
    GERBIL_LOOP_POLICY_PROJECTION_MODULE_SCHEMA_ID, GERBIL_LOOP_POLICY_PROJECTION_MODULE_TYPE_ID,
    GERBIL_POLICY_MIXIN_DEFINITION_SCHEMA_ID, GERBIL_POLICY_MIXIN_STACK_COMPILER_SCHEMA_ID,
    GERBIL_POLICY_MIXIN_STACK_COMPILER_TYPE_ID, GERBIL_POLICY_PACK_PROJECTION_CHAIN_SCHEMA_ID,
    GERBIL_POLICY_PACK_PROJECTION_CHAIN_TYPE_ID, GERBIL_POLICY_SLOT_MERGE_RECEIPT_SCHEMA_ID,
    GERBIL_POO_LOOP_PROGRAM_COMPILER_SCHEMA_ID, GERBIL_POO_LOOP_PROGRAM_COMPILER_TYPE_ID,
    GERBIL_RESOLVED_LOOP_POLICY_PACK_SCHEMA_ID, GERBIL_RESOLVED_LOOP_POLICY_PACK_TYPE_ID,
    GerbilPooLoopProgramCompilerBoundary, GerbilPooLoopProgramCompilerOwner,
    GerbilPooLoopProgramCompilerSerializationBoundary, GerbilSchemeAbiSymbol, GerbilSchemeSchemaId,
    GerbilSchemeTypeId, GerbilSchemeTypeRegistry, GerbilSchemeTypedValue, GerbilSchemeValue,
    decode_gerbil_loop_policy_projection_module, decode_gerbil_policy_mixin_stack_compiler_receipt,
    decode_gerbil_policy_pack_projection_chain_receipt,
    decode_gerbil_poo_loop_program_compiler_receipt, decode_gerbil_resolved_loop_policy_pack,
    gerbil_deck_route_abi_projection_readiness_plan,
    gerbil_deck_runtime_loop_policy_projection_module_request,
    gerbil_loop_policy_projection_module_type_manifest,
    gerbil_policy_mixin_stack_compiler_type_manifest,
    gerbil_policy_pack_projection_chain_type_manifest,
    gerbil_poo_loop_program_compiler_type_manifest, gerbil_resolved_loop_policy_pack_type_manifest,
};

mod chain;
mod compiler_receipt;
mod fixtures;
mod loop_driver;
mod policy_combination_fixtures;
mod projection_module;
mod resolved_pack;
mod storage_projection;

pub(crate) use policy_combination_fixtures::{
    policy_combination_mixin_stack_compiler_fixture,
    policy_combination_mixin_stack_compiler_fixture_with_mixin_count,
};

use super::assert_rust_projection_decode_error;
use fixtures::{
    handled_by, handled_by_with_event, loop_policy_projection_module_envelope,
    loop_policy_projection_module_registry, policy_mixin_stack_compiler_envelope,
    policy_mixin_stack_compiler_registry, policy_pack_envelope, policy_pack_fixture,
    policy_pack_fixture_with_budget_owner, policy_pack_registry,
    poo_loop_program_compiler_envelope, poo_loop_program_compiler_fixture,
    poo_loop_program_compiler_fixture_with_profile_pack_and_program,
    poo_loop_program_compiler_fixture_with_resolved_policy_pack,
    poo_loop_program_compiler_registry, resolved_loop_policy_audit_fixture_without_merge_receipts,
    resolved_loop_policy_pack_envelope, resolved_loop_policy_pack_fixture,
    resolved_loop_policy_pack_fixture_with_audit, resolved_loop_policy_pack_registry,
    runtime_reactive_tool_loop_script,
};
