use marlin_gerbil_scheme::{
    GERBIL_POLICY_PACK_PROJECTION_CHAIN_SCHEMA_ID, GERBIL_POLICY_PACK_PROJECTION_CHAIN_TYPE_ID,
    GERBIL_RESOLVED_LOOP_POLICY_PACK_SCHEMA_ID, GERBIL_RESOLVED_LOOP_POLICY_PACK_TYPE_ID,
    GerbilSchemeSchemaId, GerbilSchemeTypeId, GerbilSchemeTypeRegistry, GerbilSchemeTypedValue,
    GerbilSchemeValue, decode_gerbil_policy_pack_projection_chain_receipt,
    decode_gerbil_resolved_loop_policy_pack, gerbil_policy_pack_projection_chain_type_manifest,
    gerbil_resolved_loop_policy_pack_type_manifest,
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

fn policy_pack_registry() -> GerbilSchemeTypeRegistry {
    GerbilSchemeTypeRegistry::new(gerbil_policy_pack_projection_chain_type_manifest())
        .expect("policy-pack projection-chain manifest")
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
    GerbilSchemeValue::record([
        ("schema_version", i64::from(schema_version).into()),
        ("policy_epoch", 42_i64.into()),
        (
            "policy_digest",
            GerbilSchemeValue::vector((0..32).map(|_| GerbilSchemeValue::from(7_i64))),
        ),
        ("hot", resolved_loop_policy_hot_payload()),
        ("audit", resolved_loop_policy_audit_payload()),
    ])
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
        (
            "merge_receipts",
            GerbilSchemeValue::vector([GerbilSchemeValue::record([
                ("slot_id", 9_i64.into()),
                ("merge", "union".into()),
                ("status", "applied".into()),
            ])]),
        ),
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

fn resolved_loop_policy_pack_schema_id() -> GerbilSchemeSchemaId {
    GerbilSchemeSchemaId::new(GERBIL_RESOLVED_LOOP_POLICY_PACK_SCHEMA_ID)
}
