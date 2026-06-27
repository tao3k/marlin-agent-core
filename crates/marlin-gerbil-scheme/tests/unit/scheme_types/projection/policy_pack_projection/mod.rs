use std::sync::Arc;

pub(super) use marlin_agent_kernel::{
    AgentFlowLoopProgramRuntimeHandoffExecutor, LoopProgramDerivedSessionPolicyStatus,
    LoopProgramExecutionDriver, LoopProgramExecutionRequest, LoopProgramExecutionStatus,
    LoopProgramRuntimeHandoffExecutionReportStatus, LoopProgramRuntimeHandoffHandler,
    LoopProgramRuntimeHandoffRouter, LoopProgramRuntimeHandoffRouterHandlers,
    LoopProgramRuntimeOwner, LoopProgramRuntimeSideEffectExecutor,
    LoopProgramRuntimeSideEffectStatus, LoopProgramToolProcessCommandTemplate,
    LoopProgramToolProcessProgram, LoopProgramToolProcessSideEffectStatus,
    PolicyGatedAgentFlowLoopProgramRuntimeHandoffExecutor, ReceiptDrivenLoopProgramEventMapper,
    ScriptedLoopProgramEventMapper, StaticLoopProgramRuntimeHandoffHandler,
    StaticLoopProgramToolProcessResolver,
};
pub(super) use marlin_agent_protocol::{
    AgentFlowMemoryOperation, ContinuationOp, LoopMechanismPolicyId, LoopProgramActionKind,
    LoopProgramEventKind,
};
pub(super) use marlin_agent_runtime::TokioAgentRuntime;
pub(super) use marlin_gerbil_scheme::{
    GERBIL_POLICY_PACK_PROJECTION_CHAIN_SCHEMA_ID, GERBIL_POLICY_PACK_PROJECTION_CHAIN_TYPE_ID,
    GERBIL_POO_LOOP_PROGRAM_COMPILER_SCHEMA_ID, GERBIL_POO_LOOP_PROGRAM_COMPILER_TYPE_ID,
    GERBIL_RESOLVED_LOOP_POLICY_PACK_SCHEMA_ID, GERBIL_RESOLVED_LOOP_POLICY_PACK_TYPE_ID,
    GerbilPooLoopProgramCompilerBoundary, GerbilPooLoopProgramCompilerOwner,
    GerbilPooLoopProgramCompilerSerializationBoundary, GerbilSchemeSchemaId, GerbilSchemeTypeId,
    GerbilSchemeTypeRegistry, GerbilSchemeTypedValue, GerbilSchemeValue,
    decode_gerbil_policy_pack_projection_chain_receipt,
    decode_gerbil_poo_loop_program_compiler_receipt, decode_gerbil_resolved_loop_policy_pack,
    gerbil_policy_pack_projection_chain_type_manifest,
    gerbil_poo_loop_program_compiler_type_manifest, gerbil_resolved_loop_policy_pack_type_manifest,
};

mod chain;
mod compiler_receipt;
mod loop_driver;
mod resolved_pack;

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

fn resolved_loop_policy_pack_registry() -> GerbilSchemeTypeRegistry {
    GerbilSchemeTypeRegistry::new(gerbil_resolved_loop_policy_pack_type_manifest())
        .expect("resolved loop policy pack manifest")
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

fn policy_pack_payload() -> GerbilSchemeValue {
    policy_pack_payload_with_budget_owner("rust")
}

fn policy_pack_payload_with_budget_owner(budget_owner: &str) -> GerbilSchemeValue {
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

fn resolved_loop_policy_pack_payload(schema_version: u32) -> GerbilSchemeValue {
    resolved_loop_policy_pack_payload_with_audit(
        schema_version,
        resolved_loop_policy_audit_payload(),
    )
}

fn resolved_loop_policy_pack_payload_with_audit(
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
        ("hot", resolved_loop_policy_hot_payload()),
        ("audit", audit_payload),
    ])
}

fn poo_loop_program_compiler_payload(loop_program_digest: [u8; 32]) -> GerbilSchemeValue {
    poo_loop_program_compiler_payload_with_resolved_policy_pack(
        loop_program_digest,
        resolved_loop_policy_pack_payload(1),
    )
}

fn poo_loop_program_compiler_payload_with_resolved_policy_pack(
    loop_program_digest: [u8; 32],
    resolved_policy_pack: GerbilSchemeValue,
) -> GerbilSchemeValue {
    GerbilSchemeValue::record([
        ("kind", GERBIL_POO_LOOP_PROGRAM_COMPILER_SCHEMA_ID.into()),
        ("profile-id", "real-repair-001/reactive-tool-loop".into()),
        ("compiler-owner", "gerbil-poo-flow".into()),
        ("resolved-policy-pack", resolved_policy_pack),
        ("loop-program", loop_program_payload(loop_program_digest)),
        ("scheme-boundary", "scheme-types-to-rust-types".into()),
        (
            "serialization-boundary",
            "rust-owned-cli-trace-cross-process".into(),
        ),
    ])
}

fn policy_combination_matrix_poo_loop_program_compiler_payload() -> GerbilSchemeValue {
    GerbilSchemeValue::record([
        ("kind", GERBIL_POO_LOOP_PROGRAM_COMPILER_SCHEMA_ID.into()),
        (
            "profile-id",
            "policy-combination/memory-rewrite-checker".into(),
        ),
        ("compiler-owner", "gerbil-poo-flow".into()),
        (
            "resolved-policy-pack",
            policy_combination_matrix_resolved_policy_pack_payload(),
        ),
        (
            "loop-program",
            policy_combination_matrix_loop_program_payload(),
        ),
        ("scheme-boundary", "scheme-types-to-rust-types".into()),
        (
            "serialization-boundary",
            "rust-owned-cli-trace-cross-process".into(),
        ),
    ])
}

fn failure_retry_poo_loop_program_compiler_payload() -> GerbilSchemeValue {
    GerbilSchemeValue::record([
        ("kind", GERBIL_POO_LOOP_PROGRAM_COMPILER_SCHEMA_ID.into()),
        (
            "profile-id",
            "marlin-failure-retry-profile/typed-recovery".into(),
        ),
        ("compiler-owner", "gerbil-poo-flow".into()),
        (
            "resolved-policy-pack",
            failure_retry_resolved_policy_pack_payload(),
        ),
        ("loop-program", failure_retry_loop_program_payload()),
        ("scheme-boundary", "scheme-types-to-rust-types".into()),
        (
            "serialization-boundary",
            "rust-owned-cli-trace-cross-process".into(),
        ),
    ])
}

fn failure_retry_resolved_policy_pack_payload() -> GerbilSchemeValue {
    GerbilSchemeValue::record([
        ("schema_version", 1_i64.into()),
        ("policy_epoch", 21_i64.into()),
        (
            "policy_digest",
            GerbilSchemeValue::vector((0..32).map(|_| GerbilSchemeValue::from(21_i64))),
        ),
        ("hot", failure_retry_hot_payload()),
        ("audit", failure_retry_audit_payload()),
    ])
}

fn failure_retry_hot_payload() -> GerbilSchemeValue {
    GerbilSchemeValue::record([
        ("capability_mask", 0b111_i64.into()),
        ("human_gate_mask", 0_i64.into()),
        (
            "budget_caps",
            GerbilSchemeValue::record([
                ("max_attempts", 3_i64.into()),
                ("max_cost_units", 300_i64.into()),
                ("max_wall_time_ms", 15000_i64.into()),
            ]),
        ),
        (
            "graph_nodes",
            GerbilSchemeValue::vector([
                GerbilSchemeValue::record([
                    ("node_id", 21_i64.into()),
                    ("executor_id", 31_i64.into()),
                    ("capability_mask", 0b111_i64.into()),
                    ("resource_class_id", 41_i64.into()),
                ]),
                GerbilSchemeValue::record([
                    ("node_id", 22_i64.into()),
                    ("executor_id", 32_i64.into()),
                    ("capability_mask", 0b011_i64.into()),
                    ("resource_class_id", 41_i64.into()),
                ]),
            ]),
        ),
        (
            "graph_edges",
            GerbilSchemeValue::vector([GerbilSchemeValue::record([
                ("from", 21_i64.into()),
                ("to", 22_i64.into()),
            ])]),
        ),
        (
            "route_index",
            GerbilSchemeValue::record([(
                "buckets",
                GerbilSchemeValue::vector([GerbilSchemeValue::record([
                    ("bucket_id", 21_i64.into()),
                    ("scope_mask", 127_i64.into()),
                    ("target_id", 31_i64.into()),
                ])]),
            )]),
        ),
        (
            "resource_classes",
            GerbilSchemeValue::vector([GerbilSchemeValue::record([
                ("resource_class_id", 41_i64.into()),
                ("exclusive", true.into()),
            ])]),
        ),
        (
            "continuation_table",
            GerbilSchemeValue::vector([
                GerbilSchemeValue::record([
                    ("op", "retry".into()),
                    ("graph_template", 1_i64.into()),
                    ("max_attempts", 3_i64.into()),
                ]),
                GerbilSchemeValue::record([("op", "stop_failed".into())]),
            ]),
        ),
        (
            "maker_profiles",
            GerbilSchemeValue::vector([GerbilSchemeValue::from(21_i64)]),
        ),
        (
            "checker_profiles",
            GerbilSchemeValue::vector([GerbilSchemeValue::from(22_i64)]),
        ),
    ])
}

fn failure_retry_audit_payload() -> GerbilSchemeValue {
    GerbilSchemeValue::record([
        (
            "provenance",
            GerbilSchemeValue::vector([
                GerbilSchemeValue::record([
                    ("slot_id", 21_i64.into()),
                    ("winner_role", "retry-governor".into()),
                    (
                        "source_role_order",
                        GerbilSchemeValue::vector(["failure-observer".into(), "retry-governor".into()]),
                    ),
                    ("merge", "min".into()),
                ]),
                GerbilSchemeValue::record([
                    ("slot_id", 22_i64.into()),
                    ("winner_role", "runtime-kernel".into()),
                    (
                        "source_role_order",
                        GerbilSchemeValue::vector(["retry-governor".into(), "runtime-kernel".into()]),
                    ),
                    ("merge", "union".into()),
                ]),
            ]),
        ),
        (
            "linearization",
            GerbilSchemeValue::vector([
                "failure-observer".into(),
                "retry-governor".into(),
                "runtime-kernel".into(),
            ]),
        ),
        (
            "diagnostics",
            GerbilSchemeValue::vector([GerbilSchemeValue::record([
                ("code", "failure-retry-policy-pack-ok".into()),
                ("severity", "info".into()),
            ])]),
        ),
        (
            "source_locations",
            GerbilSchemeValue::vector([GerbilSchemeValue::record([
                ("source_location_id", 21_i64.into()),
                (
                    "path",
                    "gerbil/src/config-interface/custom/marline-kernel/policies/loops/profiles/failure-retry.ss".into(),
                ),
                ("line", 1_i64.into()),
                ("column", 1_i64.into()),
            ])]),
        ),
        (
            "explanation_strings",
            GerbilSchemeValue::vector([
                "failure-retry lowers POO retry budget into Rust loop IR".into(),
            ]),
        ),
        (
            "forced_slots",
            GerbilSchemeValue::vector([
                GerbilSchemeValue::record([("slot_id", 21_i64.into()), ("hotness", "hot".into())]),
                GerbilSchemeValue::record([("slot_id", 22_i64.into()), ("hotness", "hot".into())]),
            ]),
        ),
        (
            "merge_receipts",
            GerbilSchemeValue::vector([
                GerbilSchemeValue::record([
                    ("slot_id", 21_i64.into()),
                    ("merge", "min".into()),
                    ("status", "applied".into()),
                ]),
                GerbilSchemeValue::record([
                    ("slot_id", 22_i64.into()),
                    ("merge", "union".into()),
                    ("status", "applied".into()),
                ]),
            ]),
        ),
    ])
}

fn failure_retry_loop_program_payload() -> GerbilSchemeValue {
    GerbilSchemeValue::record([
        ("schema_version", 1_i64.into()),
        ("program_id", "failure-retry-typed-recovery".into()),
        ("policy_epoch", 21_i64.into()),
        (
            "policy_digest",
            GerbilSchemeValue::vector((0..32).map(|_| GerbilSchemeValue::from(21_i64))),
        ),
        (
            "mechanism_policies",
            GerbilSchemeValue::vector([
                "failure-retry-budget".into(),
                "typed-recovery".into(),
                "verification-gate".into(),
            ]),
        ),
        ("initial_state", "start".into()),
        (
            "transitions",
            GerbilSchemeValue::vector([
                GerbilSchemeValue::record([
                    ("transition_id", "start-classify-failure".into()),
                    ("from", "start".into()),
                    ("event", "start".into()),
                    ("action", "invoke_model".into()),
                    ("to", "await-classification".into()),
                ]),
                GerbilSchemeValue::record([
                    ("transition_id", "classification-plan-retry".into()),
                    ("from", "await-classification".into()),
                    ("event", "model_event".into()),
                    ("action", "runtime_handoff".into()),
                    ("to", "retry-planned".into()),
                ]),
                GerbilSchemeValue::record([
                    ("transition_id", "retry-plan-dispatch".into()),
                    ("from", "retry-planned".into()),
                    ("event", "runtime_receipt".into()),
                    ("action", "dispatch_tools".into()),
                    ("to", "await-retry-tool".into()),
                ]),
                GerbilSchemeValue::record([
                    ("transition_id", "retry-tool-verify".into()),
                    ("from", "await-retry-tool".into()),
                    ("event", "tool_receipt".into()),
                    ("action", "verify".into()),
                    ("to", "await-verification".into()),
                ]),
                GerbilSchemeValue::record([
                    ("transition_id", "verification-stop".into()),
                    ("from", "await-verification".into()),
                    ("event", "verification_receipt".into()),
                    ("action", "stop".into()),
                    ("to", "stopped".into()),
                ]),
            ]),
        ),
    ])
}

fn policy_combination_matrix_resolved_policy_pack_payload() -> GerbilSchemeValue {
    GerbilSchemeValue::record([
        ("schema_version", 1_i64.into()),
        ("policy_epoch", 15_i64.into()),
        (
            "policy_digest",
            GerbilSchemeValue::vector((0..32).map(|_| GerbilSchemeValue::from(15_i64))),
        ),
        ("hot", resolved_loop_policy_hot_payload()),
        (
            "audit",
            resolved_loop_policy_audit_payload_with_merge_receipts(GerbilSchemeValue::vector([
                GerbilSchemeValue::record([
                    ("slot_id", 31_i64.into()),
                    ("merge", "ordered_append".into()),
                    ("status", "applied".into()),
                ]),
                GerbilSchemeValue::record([
                    ("slot_id", 32_i64.into()),
                    ("merge", "ordered_append".into()),
                    ("status", "applied".into()),
                ]),
                GerbilSchemeValue::record([
                    ("slot_id", 33_i64.into()),
                    ("merge", "ordered_append".into()),
                    ("status", "applied".into()),
                ]),
            ])),
        ),
    ])
}

fn policy_combination_matrix_loop_program_payload() -> GerbilSchemeValue {
    GerbilSchemeValue::record([
        ("schema_version", 1_i64.into()),
        (
            "program_id",
            "policy-combination-memory-rewrite-checker".into(),
        ),
        ("policy_epoch", 15_i64.into()),
        (
            "policy_digest",
            GerbilSchemeValue::vector((0..32).map(|_| GerbilSchemeValue::from(15_i64))),
        ),
        (
            "mechanism_policies",
            GerbilSchemeValue::vector([
                "real-policy-003-maker-checker".into(),
                "real-policy-004-dynamic-rewrite".into(),
                "real-policy-005-memory-recall".into(),
            ]),
        ),
        ("initial_state", "start".into()),
        (
            "transitions",
            GerbilSchemeValue::vector([
                GerbilSchemeValue::record([
                    ("transition_id", "start-memory".into()),
                    ("from", "start".into()),
                    ("event", "start".into()),
                    ("action", "read_memory".into()),
                    ("to", "memory-ready".into()),
                ]),
                GerbilSchemeValue::record([
                    ("transition_id", "memory-maker".into()),
                    ("from", "memory-ready".into()),
                    ("event", "runtime_receipt".into()),
                    ("action", "invoke_model".into()),
                    ("to", "await-maker".into()),
                ]),
                GerbilSchemeValue::record([
                    ("transition_id", "maker-rewrite".into()),
                    ("from", "await-maker".into()),
                    ("event", "model_event".into()),
                    ("action", "rewrite_graph".into()),
                    ("to", "rewritten".into()),
                ]),
                GerbilSchemeValue::record([
                    ("transition_id", "rewrite-tool".into()),
                    ("from", "rewritten".into()),
                    ("event", "runtime_receipt".into()),
                    ("action", "dispatch_tools".into()),
                    ("to", "await-tool".into()),
                ]),
                GerbilSchemeValue::record([
                    ("transition_id", "tool-checker".into()),
                    ("from", "await-tool".into()),
                    ("event", "tool_receipt".into()),
                    ("action", "verify".into()),
                    ("to", "await-checker".into()),
                ]),
                GerbilSchemeValue::record([
                    ("transition_id", "checker-stop".into()),
                    ("from", "await-checker".into()),
                    ("event", "verification_receipt".into()),
                    ("action", "stop".into()),
                    ("to", "stopped".into()),
                ]),
            ]),
        ),
    ])
}

fn loop_program_payload(policy_digest: [u8; 32]) -> GerbilSchemeValue {
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

fn real_repair_script() -> ScriptedLoopProgramEventMapper {
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

fn policy_combination_matrix_script() -> ScriptedLoopProgramEventMapper {
    ScriptedLoopProgramEventMapper::new(
        vec![
            (
                LoopProgramActionKind::ReadMemory,
                LoopProgramEventKind::RuntimeReceipt,
            ),
            (
                LoopProgramActionKind::InvokeModel,
                LoopProgramEventKind::ModelEvent,
            ),
            (
                LoopProgramActionKind::RewriteGraph,
                LoopProgramEventKind::RuntimeReceipt,
            ),
            (
                LoopProgramActionKind::DispatchTools,
                LoopProgramEventKind::ToolReceipt,
            ),
            (
                LoopProgramActionKind::Verify,
                LoopProgramEventKind::VerificationReceipt,
            ),
        ]
        .into_boxed_slice(),
    )
}

fn failure_retry_script() -> ScriptedLoopProgramEventMapper {
    ScriptedLoopProgramEventMapper::new(
        vec![
            (
                LoopProgramActionKind::InvokeModel,
                LoopProgramEventKind::ModelEvent,
            ),
            (
                LoopProgramActionKind::RuntimeHandoff,
                LoopProgramEventKind::RuntimeReceipt,
            ),
            (
                LoopProgramActionKind::DispatchTools,
                LoopProgramEventKind::ToolReceipt,
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

fn resolved_loop_policy_hot_payload() -> GerbilSchemeValue {
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

fn resolved_loop_policy_audit_payload() -> GerbilSchemeValue {
    resolved_loop_policy_audit_payload_with_merge_receipts(GerbilSchemeValue::vector([
        GerbilSchemeValue::record([
            ("slot_id", 9_i64.into()),
            ("merge", "union".into()),
            ("status", "applied".into()),
        ]),
    ]))
}

fn resolved_loop_policy_audit_payload_without_merge_receipts() -> GerbilSchemeValue {
    resolved_loop_policy_audit_payload_with_merge_receipts(GerbilSchemeValue::vector([]))
}

fn resolved_loop_policy_audit_payload_with_merge_receipts(
    merge_receipts: GerbilSchemeValue,
) -> GerbilSchemeValue {
    GerbilSchemeValue::record([
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

fn resolved_loop_policy_pack_schema_id() -> GerbilSchemeSchemaId {
    GerbilSchemeSchemaId::new(GERBIL_RESOLVED_LOOP_POLICY_PACK_SCHEMA_ID)
}

fn poo_loop_program_compiler_schema_id() -> GerbilSchemeSchemaId {
    GerbilSchemeSchemaId::new(GERBIL_POO_LOOP_PROGRAM_COMPILER_SCHEMA_ID)
}
