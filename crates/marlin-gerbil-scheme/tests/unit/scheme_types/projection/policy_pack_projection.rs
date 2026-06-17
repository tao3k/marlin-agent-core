use marlin_gerbil_scheme::{
    GERBIL_POLICY_PACK_PROJECTION_CHAIN_SCHEMA_ID, GERBIL_POLICY_PACK_PROJECTION_CHAIN_TYPE_ID,
    GerbilSchemeSchemaId, GerbilSchemeTypeId, GerbilSchemeTypeRegistry, GerbilSchemeTypedValue,
    GerbilSchemeValue, decode_gerbil_policy_pack_projection_chain_receipt,
    gerbil_policy_pack_projection_chain_type_manifest,
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

fn policy_pack_registry() -> GerbilSchemeTypeRegistry {
    GerbilSchemeTypeRegistry::new(gerbil_policy_pack_projection_chain_type_manifest())
        .expect("policy-pack projection-chain manifest")
}

fn policy_pack_envelope(payload: GerbilSchemeValue) -> GerbilSchemeTypedValue {
    GerbilSchemeTypedValue::new(policy_pack_type_id(), payload)
        .with_schema_id(policy_pack_schema_id())
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
                "marlin.modules.policy-pack.module-evaluation-receipt.v1",
                "gerbil-module-system",
            ),
        ),
        (
            "policy-projection-receipt",
            nested_receipt("marlin.modules.policy-projection.v1", "gerbil-poo"),
        ),
        (
            "native-projection-payload",
            nested_receipt("marlin.modules.policy-pack-presentation.v1", "rust"),
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

fn policy_pack_type_id() -> GerbilSchemeTypeId {
    GerbilSchemeTypeId::new(GERBIL_POLICY_PACK_PROJECTION_CHAIN_TYPE_ID)
}

fn policy_pack_schema_id() -> GerbilSchemeSchemaId {
    GerbilSchemeSchemaId::new(GERBIL_POLICY_PACK_PROJECTION_CHAIN_SCHEMA_ID)
}
