use std::sync::Arc;

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
    GerbilPooLoopProgramCompilerSerializationBoundary, GerbilSchemeNativeSymbol,
    GerbilSchemeSchemaId, GerbilSchemeTypeId, GerbilSchemeTypeRegistry, GerbilSchemeTypedValue,
    GerbilSchemeValue, decode_gerbil_loop_policy_projection_module,
    decode_gerbil_policy_mixin_stack_compiler_receipt,
    decode_gerbil_policy_pack_projection_chain_receipt,
    decode_gerbil_poo_loop_program_compiler_receipt, decode_gerbil_resolved_loop_policy_pack,
    gerbil_deck_runtime_loop_policy_projection_module_request,
    gerbil_deck_runtime_native_projection_readiness_plan,
    gerbil_loop_policy_projection_module_type_manifest,
    gerbil_policy_mixin_stack_compiler_type_manifest,
    gerbil_policy_pack_projection_chain_type_manifest,
    gerbil_poo_loop_program_compiler_type_manifest, gerbil_resolved_loop_policy_pack_type_manifest,
};

mod chain;
mod compiler_receipt;
mod loop_driver;
mod projection_module;
mod resolved_pack;
mod storage_projection;

fn assert_rust_projection_decode_error(
    error: marlin_gerbil_scheme::GerbilSchemeTypeDecodeError,
    needle: &str,
) {
    super::assert_rust_projection_decode_error(error, needle);
}

fn policy_pack_registry() -> GerbilSchemeTypeRegistry {
    GerbilSchemeTypeRegistry::new(gerbil_policy_pack_projection_chain_type_manifest())
        .expect("policy-pack projection-chain manifest")
}

fn poo_loop_program_compiler_registry() -> GerbilSchemeTypeRegistry {
    GerbilSchemeTypeRegistry::new(gerbil_poo_loop_program_compiler_type_manifest())
        .expect("POO loop program compiler manifest")
}

fn policy_mixin_stack_compiler_registry() -> GerbilSchemeTypeRegistry {
    GerbilSchemeTypeRegistry::new(gerbil_policy_mixin_stack_compiler_type_manifest())
        .expect("POO mixin-stack compiler manifest")
}

fn resolved_loop_policy_pack_registry() -> GerbilSchemeTypeRegistry {
    GerbilSchemeTypeRegistry::new(gerbil_resolved_loop_policy_pack_type_manifest())
        .expect("resolved loop policy pack manifest")
}

fn loop_policy_projection_module_registry() -> GerbilSchemeTypeRegistry {
    GerbilSchemeTypeRegistry::new(gerbil_loop_policy_projection_module_type_manifest())
        .expect("loop policy projection module manifest")
}

fn policy_pack_envelope(payload: GerbilSchemeValue) -> GerbilSchemeTypedValue {
    GerbilSchemeTypedValue::new(policy_pack_type_id(), payload)
        .with_schema_id(policy_pack_schema_id())
}

fn resolved_loop_policy_pack_envelope(payload: GerbilSchemeValue) -> GerbilSchemeTypedValue {
    GerbilSchemeTypedValue::new(resolved_loop_policy_pack_type_id(), payload)
        .with_schema_id(resolved_loop_policy_pack_schema_id())
}

fn poo_loop_program_compiler_envelope(payload: GerbilSchemeValue) -> GerbilSchemeTypedValue {
    GerbilSchemeTypedValue::new(poo_loop_program_compiler_type_id(), payload)
        .with_schema_id(poo_loop_program_compiler_schema_id())
}

fn policy_mixin_stack_compiler_envelope(payload: GerbilSchemeValue) -> GerbilSchemeTypedValue {
    GerbilSchemeTypedValue::new(policy_mixin_stack_compiler_type_id(), payload)
        .with_schema_id(policy_mixin_stack_compiler_schema_id())
}

fn loop_policy_projection_module_envelope(payload: GerbilSchemeValue) -> GerbilSchemeTypedValue {
    GerbilSchemeTypedValue::new(
        GerbilSchemeTypeId::new(GERBIL_LOOP_POLICY_PROJECTION_MODULE_TYPE_ID),
        payload,
    )
    .with_schema_id(GerbilSchemeSchemaId::new(
        GERBIL_LOOP_POLICY_PROJECTION_MODULE_SCHEMA_ID,
    ))
}

fn policy_pack_fixture() -> GerbilSchemeValue {
    policy_pack_fixture_with_budget_owner("rust")
}

fn policy_pack_fixture_with_budget_owner(budget_owner: &str) -> GerbilSchemeValue {
    GerbilSchemeValue::record([
        ("kind", GERBIL_POLICY_PACK_PROJECTION_CHAIN_SCHEMA_ID.into()),
        ("pack-id", "inventory-conflict-pack".into()),
        ("receipt-family-count", 5.into()),
        (
            "receipt-family-ids",
            GerbilSchemeValue::vector([
                "module_evaluation_receipt".into(),
                "policy_projection_receipt".into(),
                "native_projection_payload".into(),
                "budget_receipt".into(),
                "catalog_resolution_receipt".into(),
            ]),
        ),
        (
            "module-evaluation-receipt",
            nested_receipt(
                "marlin.config-interface.policy-pack.module-evaluation-receipt.v1",
                "gerbil-module-system",
            ),
        ),
        (
            "policy-projection-receipt",
            nested_receipt("marlin.config-interface.policy-projection.v1", "gerbil-poo"),
        ),
        (
            "native-projection-payload",
            nested_receipt(
                "marlin.config-interface.policy-pack-presentation.v1",
                "rust",
            ),
        ),
        (
            "budget-receipt",
            nested_receipt("marlin.runtime.policy-budget-receipt.v1", "rust"),
        ),
        (
            "catalog-resolution-receipt",
            nested_receipt(
                "marlin.runtime.policy-catalog-resolution-receipt.v1",
                "rust",
            ),
        ),
        (
            "module-evaluation-receipt-owner",
            "gerbil-module-system".into(),
        ),
        ("policy-projection-receipt-owner", "gerbil-poo".into()),
        ("native-projection-payload-owner", "rust".into()),
        ("budget-receipt-owner", budget_owner.into()),
        ("catalog-resolution-receipt-owner", "rust".into()),
        ("catalog-resolution-allowed-hook-count", 2.into()),
        ("replayable", true.into()),
    ])
}

fn nested_receipt(kind: &str, owner: &str) -> GerbilSchemeValue {
    GerbilSchemeValue::record([
        ("kind", kind.into()),
        ("pack-id", "inventory-conflict-pack".into()),
        ("owner", owner.into()),
        ("replayable", true.into()),
    ])
}

fn resolved_loop_policy_pack_fixture(schema_version: u32) -> GerbilSchemeValue {
    resolved_loop_policy_pack_fixture_with_audit(
        schema_version,
        resolved_loop_policy_audit_fixture(),
    )
}

fn resolved_loop_policy_pack_fixture_with_audit(
    schema_version: u32,
    audit_payload: GerbilSchemeValue,
) -> GerbilSchemeValue {
    GerbilSchemeValue::record([
        ("schema_version", i64::from(schema_version).into()),
        ("policy_epoch", 42_i64.into()),
        (
            "policy_digest",
            GerbilSchemeValue::vector((0..32).map(|_| GerbilSchemeValue::from(7_i64))),
        ),
        ("hot", resolved_loop_policy_hot_fixture()),
        ("audit", audit_payload),
    ])
}

fn poo_loop_program_compiler_fixture(loop_program_digest: [u8; 32]) -> GerbilSchemeValue {
    poo_loop_program_compiler_fixture_with_resolved_policy_pack(
        loop_program_digest,
        resolved_loop_policy_pack_fixture(1),
    )
}

fn poo_loop_program_compiler_fixture_with_resolved_policy_pack(
    loop_program_digest: [u8; 32],
    resolved_policy_pack: GerbilSchemeValue,
) -> GerbilSchemeValue {
    poo_loop_program_compiler_fixture_with_profile_pack_and_program(
        "runtime-reactive-tool-loop",
        resolved_policy_pack,
        loop_program_fixture(loop_program_digest),
    )
}

fn poo_loop_program_compiler_fixture_with_profile_pack_and_program(
    profile_id: &str,
    resolved_policy_pack: GerbilSchemeValue,
    loop_program: GerbilSchemeValue,
) -> GerbilSchemeValue {
    GerbilSchemeValue::record([
        ("kind", GERBIL_POO_LOOP_PROGRAM_COMPILER_SCHEMA_ID.into()),
        ("profile-id", profile_id.into()),
        ("compiler-owner", "gerbil-poo-flow".into()),
        ("resolved-policy-pack", resolved_policy_pack),
        ("loop-program", loop_program),
        ("scheme-boundary", "scheme-types-to-rust-types".into()),
        (
            "serialization-boundary",
            "rust-owned-cli-trace-cross-process".into(),
        ),
    ])
}

mod policy_combination_fixtures;
pub(crate) use policy_combination_fixtures::{
    policy_combination_mixin_stack_compiler_fixture,
    policy_combination_mixin_stack_compiler_fixture_with_mixin_count,
};

fn loop_program_fixture(policy_digest: [u8; 32]) -> GerbilSchemeValue {
    GerbilSchemeValue::record([
        ("schema_version", 1_i64.into()),
        ("program_id", "poo-reactive-tool-loop".into()),
        ("policy_epoch", 42_i64.into()),
        (
            "policy_digest",
            GerbilSchemeValue::vector(
                policy_digest
                    .iter()
                    .copied()
                    .map(|byte| GerbilSchemeValue::from(i64::from(byte))),
            ),
        ),
        (
            "mechanism_policies",
            GerbilSchemeValue::vector([
                "reactive-tool-loop-base".into(),
                "dynamic-graph-rewrite".into(),
                "verification-gate".into(),
            ]),
        ),
        ("initial_state", "start".into()),
        (
            "transitions",
            GerbilSchemeValue::vector([
                GerbilSchemeValue::record([
                    ("transition_id", "start-model".into()),
                    ("from", "start".into()),
                    ("event", "start".into()),
                    ("action", "invoke_model".into()),
                    ("to", "await-model".into()),
                ]),
                GerbilSchemeValue::record([
                    ("transition_id", "model-tools".into()),
                    ("from", "await-model".into()),
                    ("event", "tool_request".into()),
                    ("action", "dispatch_tools".into()),
                    ("to", "await-tools".into()),
                ]),
                GerbilSchemeValue::record([
                    ("transition_id", "tools-continue".into()),
                    ("from", "await-tools".into()),
                    ("event", "tool_receipt".into()),
                    ("action", "continue".into()),
                    ("to", "await-model".into()),
                ]),
                GerbilSchemeValue::record([
                    ("transition_id", "dynamic-rewrite".into()),
                    ("from", "await-model".into()),
                    ("event", "model_event".into()),
                    ("action", "rewrite_graph".into()),
                    ("to", "rewritten".into()),
                ]),
                GerbilSchemeValue::record([
                    ("transition_id", "verify-rewrite".into()),
                    ("from", "rewritten".into()),
                    ("event", "runtime_receipt".into()),
                    ("action", "verify".into()),
                    ("to", "verifying".into()),
                ]),
                GerbilSchemeValue::record([
                    ("transition_id", "verification-stop".into()),
                    ("from", "verifying".into()),
                    ("event", "verification_receipt".into()),
                    ("action", "stop".into()),
                    ("to", "stopped".into()),
                ]),
            ]),
        ),
    ])
}

fn runtime_reactive_tool_loop_script() -> ScriptedLoopProgramEventMapper {
    ScriptedLoopProgramEventMapper::new(
        vec![
            (
                LoopProgramActionKind::InvokeModel,
                LoopProgramEventKind::ToolRequest,
            ),
            (
                LoopProgramActionKind::DispatchTools,
                LoopProgramEventKind::ToolReceipt,
            ),
            (
                LoopProgramActionKind::Continue,
                LoopProgramEventKind::ModelEvent,
            ),
            (
                LoopProgramActionKind::RewriteGraph,
                LoopProgramEventKind::RuntimeReceipt,
            ),
            (
                LoopProgramActionKind::Verify,
                LoopProgramEventKind::VerificationReceipt,
            ),
        ]
        .into_boxed_slice(),
    )
}

fn handled_by(owner: &'static str) -> Arc<dyn LoopProgramRuntimeHandoffHandler> {
    Arc::new(StaticLoopProgramRuntimeHandoffHandler::handled(
        LoopProgramRuntimeOwner::new(owner),
    ))
}

fn handled_by_with_event(
    owner: &'static str,
    next_event: LoopProgramEventKind,
) -> Arc<dyn LoopProgramRuntimeHandoffHandler> {
    Arc::new(
        StaticLoopProgramRuntimeHandoffHandler::handled_with_next_event(
            LoopProgramRuntimeOwner::new(owner),
            next_event,
        ),
    )
}

fn resolved_loop_policy_hot_fixture() -> GerbilSchemeValue {
    GerbilSchemeValue::record([
        ("capability_mask", 0b101_i64.into()),
        ("human_gate_mask", 0b001_i64.into()),
        (
            "budget_caps",
            GerbilSchemeValue::record([
                ("max_attempts", 3_i64.into()),
                ("max_cost_units", 1000_i64.into()),
                ("max_wall_time_ms", 30000_i64.into()),
            ]),
        ),
        (
            "graph_nodes",
            GerbilSchemeValue::vector([GerbilSchemeValue::record([
                ("node_id", 1_i64.into()),
                ("executor_id", 2_i64.into()),
                ("capability_mask", 0b101_i64.into()),
                ("resource_class_id", 4_i64.into()),
            ])]),
        ),
        (
            "graph_edges",
            GerbilSchemeValue::vector([GerbilSchemeValue::record([
                ("from", 1_i64.into()),
                ("to", 2_i64.into()),
            ])]),
        ),
        (
            "route_index",
            GerbilSchemeValue::record([(
                "buckets",
                GerbilSchemeValue::vector([GerbilSchemeValue::record([
                    ("bucket_id", 1_i64.into()),
                    ("scope_mask", 255_i64.into()),
                    ("target_id", 3_i64.into()),
                ])]),
            )]),
        ),
        (
            "resource_classes",
            GerbilSchemeValue::vector([GerbilSchemeValue::record([
                ("resource_class_id", 4_i64.into()),
                ("exclusive", true.into()),
            ])]),
        ),
        (
            "continuation_table",
            GerbilSchemeValue::vector([GerbilSchemeValue::record([(
                "op",
                "stop_completed".into(),
            )])]),
        ),
        (
            "maker_profiles",
            GerbilSchemeValue::vector([GerbilSchemeValue::from(11_i64)]),
        ),
        (
            "checker_profiles",
            GerbilSchemeValue::vector([GerbilSchemeValue::from(12_i64)]),
        ),
    ])
}

fn resolved_loop_policy_audit_fixture() -> GerbilSchemeValue {
    resolved_loop_policy_audit_fixture_with_merge_receipts(GerbilSchemeValue::vector([
        GerbilSchemeValue::record([
            ("slot_id", 9_i64.into()),
            ("merge", "union".into()),
            ("status", "applied".into()),
        ]),
        GerbilSchemeValue::record([
            ("slot_id", 10_i64.into()),
            ("merge", "intersection".into()),
            ("status", "applied".into()),
        ]),
        GerbilSchemeValue::record([
            ("slot_id", 11_i64.into()),
            ("merge", "min".into()),
            ("status", "applied".into()),
        ]),
        GerbilSchemeValue::record([
            ("slot_id", 12_i64.into()),
            ("merge", "ordered_append".into()),
            ("status", "applied".into()),
        ]),
        GerbilSchemeValue::record([
            ("slot_id", 13_i64.into()),
            ("merge", "conflict_error".into()),
            ("status", "conflict".into()),
        ]),
    ]))
}

fn resolved_loop_policy_audit_fixture_without_merge_receipts() -> GerbilSchemeValue {
    resolved_loop_policy_audit_fixture_with_merge_receipts(GerbilSchemeValue::vector([]))
}

fn resolved_loop_policy_audit_fixture_with_merge_receipts(
    merge_receipts: GerbilSchemeValue,
) -> GerbilSchemeValue {
    GerbilSchemeValue::record([
        (
            "policy_mixins",
            GerbilSchemeValue::vector([
                "reactive-tool-loop-base".into(),
                "workspace-write-policy".into(),
                "sandbox-denylist-policy".into(),
                "retry-budget-policy".into(),
                "artifact-policy".into(),
                "trace-policy".into(),
            ]),
        ),
        (
            "provenance",
            GerbilSchemeValue::vector([GerbilSchemeValue::record([
                ("slot_id", 9_i64.into()),
                ("winner_role", "planner".into()),
                (
                    "source_role_order",
                    GerbilSchemeValue::vector(["planner".into(), "reviewer".into()]),
                ),
                ("merge", "union".into()),
            ])]),
        ),
        (
            "linearization",
            GerbilSchemeValue::vector(["planner".into(), "reviewer".into()]),
        ),
        (
            "diagnostics",
            GerbilSchemeValue::vector([GerbilSchemeValue::record([
                ("code", "policy-pack-ok".into()),
                ("severity", "info".into()),
            ])]),
        ),
        (
            "source_locations",
            GerbilSchemeValue::vector([GerbilSchemeValue::record([
                ("source_location_id", 1_i64.into()),
                (
                    "path",
                    "gerbil/src/config-interface/modules/policy-pack.ss".into(),
                ),
                ("line", 10_i64.into()),
                ("column", 2_i64.into()),
            ])]),
        ),
        (
            "explanation_strings",
            GerbilSchemeValue::vector(["forced policy pack before native handoff".into()]),
        ),
        (
            "forced_slots",
            GerbilSchemeValue::vector([GerbilSchemeValue::record([
                ("slot_id", 9_i64.into()),
                ("hotness", "hot".into()),
            ])]),
        ),
        ("merge_receipts", merge_receipts),
    ])
}

fn policy_pack_type_id() -> GerbilSchemeTypeId {
    GerbilSchemeTypeId::new(GERBIL_POLICY_PACK_PROJECTION_CHAIN_TYPE_ID)
}

fn policy_pack_schema_id() -> GerbilSchemeSchemaId {
    GerbilSchemeSchemaId::new(GERBIL_POLICY_PACK_PROJECTION_CHAIN_SCHEMA_ID)
}

fn resolved_loop_policy_pack_type_id() -> GerbilSchemeTypeId {
    GerbilSchemeTypeId::new(GERBIL_RESOLVED_LOOP_POLICY_PACK_TYPE_ID)
}

fn poo_loop_program_compiler_type_id() -> GerbilSchemeTypeId {
    GerbilSchemeTypeId::new(GERBIL_POO_LOOP_PROGRAM_COMPILER_TYPE_ID)
}

fn policy_mixin_stack_compiler_type_id() -> GerbilSchemeTypeId {
    GerbilSchemeTypeId::new(GERBIL_POLICY_MIXIN_STACK_COMPILER_TYPE_ID)
}

fn resolved_loop_policy_pack_schema_id() -> GerbilSchemeSchemaId {
    GerbilSchemeSchemaId::new(GERBIL_RESOLVED_LOOP_POLICY_PACK_SCHEMA_ID)
}

fn poo_loop_program_compiler_schema_id() -> GerbilSchemeSchemaId {
    GerbilSchemeSchemaId::new(GERBIL_POO_LOOP_PROGRAM_COMPILER_SCHEMA_ID)
}

fn policy_mixin_stack_compiler_schema_id() -> GerbilSchemeSchemaId {
    GerbilSchemeSchemaId::new(GERBIL_POLICY_MIXIN_STACK_COMPILER_SCHEMA_ID)
}
