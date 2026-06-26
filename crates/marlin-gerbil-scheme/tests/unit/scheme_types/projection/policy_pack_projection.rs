use std::sync::Arc;

use marlin_agent_kernel::{
    LoopProgramExecutionDriver, LoopProgramExecutionRequest, LoopProgramExecutionStatus,
    LoopProgramRuntimeHandoffExecutionReportStatus, LoopProgramRuntimeHandoffHandler,
    LoopProgramRuntimeHandoffRouter, LoopProgramRuntimeHandoffRouterHandlers,
    LoopProgramRuntimeOwner, ScriptedLoopProgramEventMapper,
    StaticLoopProgramRuntimeHandoffHandler,
};
use marlin_agent_protocol::{LoopProgramActionKind, LoopProgramEventKind};
use marlin_gerbil_scheme::{
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

#[test]
fn policy_pack_projection_chain_decodes_typed_poo_receipts_without_json_boundary() {
    let registry = policy_pack_registry();
    let envelope = policy_pack_envelope(policy_pack_payload());

    let receipt = decode_gerbil_policy_pack_projection_chain_receipt(&registry, &envelope)
        .expect("policy-pack projection-chain receipt decodes");

    assert!(receipt.has_current_schema());
    assert_eq!(receipt.pack_id, "inventory-conflict-pack");
    assert_eq!(receipt.receipt_family_count, 5);
    assert_eq!(
        receipt.receipt_family_ids,
        vec![
            "module_evaluation_receipt",
            "policy_projection_receipt",
            "native_projection_payload",
            "budget_receipt",
            "catalog_resolution_receipt"
        ]
    );
    assert_eq!(
        receipt.module_evaluation_receipt.owner,
        "gerbil-module-system"
    );
    assert_eq!(receipt.policy_projection_receipt.owner, "gerbil-poo");
    assert_eq!(receipt.native_projection_payload.owner, "rust");
    assert_eq!(receipt.budget_receipt.owner, "rust");
    assert_eq!(receipt.catalog_resolution_receipt.owner, "rust");
    assert_eq!(receipt.catalog_resolution_allowed_hook_count, 2);
    assert!(receipt.replayable);
}

#[test]
fn policy_pack_projection_chain_rejects_owner_drift_between_chain_and_nested_receipt() {
    let registry = policy_pack_registry();
    let envelope = policy_pack_envelope(policy_pack_payload_with_budget_owner("gerbil-poo"));

    let error = decode_gerbil_policy_pack_projection_chain_receipt(&registry, &envelope)
        .expect_err("owner drift should be rejected by Rust projection");
    super::assert_rust_projection_decode_error(error, "budget-receipt-owner");
}

#[test]
fn resolved_loop_policy_pack_decodes_hot_and_audit_ir_without_json_boundary() {
    let registry = resolved_loop_policy_pack_registry();
    let envelope = resolved_loop_policy_pack_envelope(resolved_loop_policy_pack_payload(1));

    let pack = decode_gerbil_resolved_loop_policy_pack(&registry, &envelope)
        .expect("resolved loop policy pack decodes");

    assert!(pack.has_current_schema());
    assert_eq!(pack.policy_epoch.get(), 42);
    assert_eq!(pack.policy_digest.as_bytes(), &[7_u8; 32]);
    assert_eq!(pack.hot.capability_mask, 0b101);
    assert_eq!(pack.hot.graph_nodes.len(), 1);
    assert_eq!(pack.hot.graph_nodes[0].node_id.get(), 1);
    assert_eq!(pack.hot.continuation_table.len(), 1);
    assert_eq!(pack.audit.provenance.len(), 1);
    assert_eq!(pack.audit.provenance[0].winner_role.as_str(), "planner");
    assert_eq!(pack.audit.forced_slots.len(), 1);
    assert_eq!(pack.audit.merge_receipts.len(), 1);
}

#[test]
fn resolved_loop_policy_pack_rejects_schema_version_drift() {
    let registry = resolved_loop_policy_pack_registry();
    let envelope = resolved_loop_policy_pack_envelope(resolved_loop_policy_pack_payload(99));

    let error = decode_gerbil_resolved_loop_policy_pack(&registry, &envelope)
        .expect_err("schema drift should be rejected by Rust projection");
    super::assert_rust_projection_decode_error(error, "schema version 99");
}

#[test]
fn poo_loop_program_compiler_receipt_decodes_program_bound_to_resolved_pack() {
    let registry = poo_loop_program_compiler_registry();
    let envelope = poo_loop_program_compiler_envelope(poo_loop_program_compiler_payload([7; 32]));

    let receipt = decode_gerbil_poo_loop_program_compiler_receipt(&registry, &envelope)
        .expect("POO loop program compiler receipt decodes");

    assert!(receipt.has_current_schema());
    assert_eq!(
        receipt.profile_id.as_str(),
        "real-repair-001/reactive-tool-loop"
    );
    assert_eq!(
        receipt.compiler_owner,
        GerbilPooLoopProgramCompilerOwner::GerbilPooFlow
    );
    assert_eq!(
        receipt.scheme_boundary,
        GerbilPooLoopProgramCompilerBoundary::SchemeTypesToRustTypes
    );
    assert_eq!(
        receipt.serialization_boundary,
        GerbilPooLoopProgramCompilerSerializationBoundary::RustOwnedCliTraceCrossProcess
    );
    assert_eq!(
        receipt.loop_program.policy_epoch,
        receipt.resolved_policy_pack.policy_epoch
    );
    assert_eq!(
        receipt.loop_program.policy_digest,
        receipt.resolved_policy_pack.policy_digest
    );
    assert_eq!(receipt.loop_program.mechanism_policies.len(), 3);
    assert_eq!(receipt.loop_program.transitions.len(), 6);
    assert!(
        !format!("{:?}", receipt.scheme_boundary)
            .to_ascii_lowercase()
            .contains("json")
    );
    assert!(receipt.resolved_policy_pack.has_current_schema());
    assert!(receipt.loop_program.has_current_schema());
    assert!(!receipt.resolved_policy_pack.hot.graph_nodes.is_empty());
    assert!(!receipt.resolved_policy_pack.audit.provenance.is_empty());
    assert!(!receipt.resolved_policy_pack.audit.linearization.is_empty());
    assert!(!receipt.resolved_policy_pack.audit.forced_slots.is_empty());
    assert!(!receipt.resolved_policy_pack.audit.merge_receipts.is_empty());
}

#[test]
fn poo_loop_program_compiler_receipt_runs_scripted_loop_through_kernel_driver() {
    let registry = poo_loop_program_compiler_registry();
    let envelope = poo_loop_program_compiler_envelope(poo_loop_program_compiler_payload([7; 32]));
    let compiler_receipt = decode_gerbil_poo_loop_program_compiler_receipt(&registry, &envelope)
        .expect("POO loop program compiler receipt decodes");

    let handlers = LoopProgramRuntimeHandoffRouterHandlers {
        control_handler: handled_by("runtime.control"),
        model_handler: handled_by("runtime.model"),
        tool_handler: handled_by("runtime.tool"),
        graph_handler: handled_by("runtime.graph"),
        verification_handler: handled_by("runtime.verification"),
        ..LoopProgramRuntimeHandoffRouterHandlers::default()
    };
    let driver = LoopProgramExecutionDriver::new(LoopProgramRuntimeHandoffRouter::new(handlers))
        .with_event_mapper(real_repair_script())
        .with_max_steps(16);

    let execution_receipt = driver.run(LoopProgramExecutionRequest::new(
        compiler_receipt.loop_program,
        vec![LoopProgramEventKind::Start],
    ));

    assert_eq!(
        execution_receipt.status,
        LoopProgramExecutionStatus::Stopped
    );
    assert!(execution_receipt.error.is_none());
    assert_eq!(execution_receipt.steps.len(), 6);
    assert_eq!(
        execution_receipt
            .steps
            .iter()
            .map(|step| step.machine_receipt.action.clone())
            .collect::<Vec<_>>(),
        vec![
            LoopProgramActionKind::InvokeModel,
            LoopProgramActionKind::DispatchTools,
            LoopProgramActionKind::Continue,
            LoopProgramActionKind::RewriteGraph,
            LoopProgramActionKind::Verify,
            LoopProgramActionKind::Stop,
        ]
    );
    assert_eq!(
        execution_receipt
            .steps
            .iter()
            .map(|step| step.generated_event.clone())
            .collect::<Vec<_>>(),
        vec![
            Some(LoopProgramEventKind::ToolRequest),
            Some(LoopProgramEventKind::ToolReceipt),
            Some(LoopProgramEventKind::ModelEvent),
            Some(LoopProgramEventKind::RuntimeReceipt),
            Some(LoopProgramEventKind::VerificationReceipt),
            None,
        ]
    );
    assert!(execution_receipt.steps.iter().all(|step| {
        step.runtime_handoff_plan.handoffs.len() == 1
            && step.runtime_handoff_execution.status
                == LoopProgramRuntimeHandoffExecutionReportStatus::Completed
    }));
    assert_eq!(
        execution_receipt
            .steps
            .iter()
            .map(|step| step.runtime_handoff_execution.executions[0].owner.as_str())
            .collect::<Vec<_>>(),
        vec![
            "runtime.model",
            "runtime.tool",
            "runtime.control",
            "runtime.graph",
            "runtime.verification",
            "runtime.control",
        ]
    );
}

#[test]
fn poo_loop_program_compiler_receipt_rejects_digest_drift() {
    let registry = poo_loop_program_compiler_registry();
    let envelope = poo_loop_program_compiler_envelope(poo_loop_program_compiler_payload([8; 32]));

    let error = decode_gerbil_poo_loop_program_compiler_receipt(&registry, &envelope)
        .expect_err("digest drift should be rejected by Rust projection");
    super::assert_rust_projection_decode_error(error, "policy digest");
}

#[test]
fn poo_loop_program_compiler_receipt_rejects_missing_merge_receipts() {
    let registry = poo_loop_program_compiler_registry();
    let payload = poo_loop_program_compiler_payload_with_resolved_policy_pack(
        [7; 32],
        resolved_loop_policy_pack_payload_with_audit(
            1,
            resolved_loop_policy_audit_payload_without_merge_receipts(),
        ),
    );
    let envelope = poo_loop_program_compiler_envelope(payload);

    let error = decode_gerbil_poo_loop_program_compiler_receipt(&registry, &envelope)
        .expect_err("missing merge receipts should be rejected by Rust projection");
    super::assert_rust_projection_decode_error(error, "merge receipts");
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

fn handled_by(owner: &'static str) -> Arc<dyn LoopProgramRuntimeHandoffHandler> {
    Arc::new(StaticLoopProgramRuntimeHandoffHandler::handled(
        LoopProgramRuntimeOwner::new(owner),
    ))
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
