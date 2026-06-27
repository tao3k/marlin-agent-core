use super::{
    decode_gerbil_policy_pack_projection_chain_receipt, policy_pack_envelope, policy_pack_fixture,
    policy_pack_fixture_with_budget_owner, policy_pack_registry,
};

#[test]
fn policy_pack_projection_chain_decodes_typed_poo_receipts_without_json_boundary() {
    let registry = policy_pack_registry();
    let envelope = policy_pack_envelope(policy_pack_fixture());

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
    let envelope = policy_pack_envelope(policy_pack_fixture_with_budget_owner("gerbil-poo"));

    let error = decode_gerbil_policy_pack_projection_chain_receipt(&registry, &envelope)
        .expect_err("owner drift should be rejected by Rust projection");
    super::assert_rust_projection_decode_error(error, "budget-receipt-owner");
}
