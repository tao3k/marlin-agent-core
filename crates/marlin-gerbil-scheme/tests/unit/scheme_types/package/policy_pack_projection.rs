use marlin_gerbil_scheme::{
    GERBIL_POLICY_PACK_PROJECTION_CHAIN_PACKAGE_ID, GERBIL_POLICY_PACK_PROJECTION_CHAIN_SCHEMA_ID,
    GERBIL_POLICY_PACK_PROJECTION_CHAIN_TYPE_ID, GERBIL_RESOLVED_LOOP_POLICY_PACK_PACKAGE_ID,
    GERBIL_RESOLVED_LOOP_POLICY_PACK_SCHEMA_ID, GERBIL_RESOLVED_LOOP_POLICY_PACK_TYPE_ID,
    GerbilSchemePackageId, GerbilSchemeTypeId,
    gerbil_policy_pack_projection_chain_package_manifest,
    gerbil_resolved_loop_policy_pack_package_manifest, validate_gerbil_scheme_package_manifest,
};

#[test]
fn policy_pack_projection_chain_package_manifest_declares_typed_receipt_contract() {
    let manifest = gerbil_policy_pack_projection_chain_package_manifest();

    let receipt = validate_gerbil_scheme_package_manifest(&manifest)
        .expect("policy-pack projection-chain package manifest should validate");

    assert_eq!(
        receipt.package_id,
        GerbilSchemePackageId::new(GERBIL_POLICY_PACK_PROJECTION_CHAIN_PACKAGE_ID)
    );
    assert_eq!(receipt.type_count, 1);
    assert_eq!(receipt.field_count, 16);
    assert_eq!(receipt.projection_contract_count, 1);
    assert_eq!(receipt.native_abi_version, None);
    assert_eq!(receipt.native_symbol_count, 0);

    let projection_type = manifest
        .type_manifest
        .types
        .first()
        .expect("policy-pack projection-chain type manifest entry");
    assert_eq!(
        projection_type.type_id,
        GerbilSchemeTypeId::new(GERBIL_POLICY_PACK_PROJECTION_CHAIN_TYPE_ID)
    );
    assert_eq!(
        projection_type
            .schema_id
            .as_ref()
            .map(|schema| schema.as_str()),
        Some(GERBIL_POLICY_PACK_PROJECTION_CHAIN_SCHEMA_ID)
    );
}

#[test]
fn resolved_loop_policy_pack_package_manifest_declares_hot_audit_projection_contract() {
    let manifest = gerbil_resolved_loop_policy_pack_package_manifest();

    let receipt = validate_gerbil_scheme_package_manifest(&manifest)
        .expect("resolved loop policy pack package manifest should validate");

    assert_eq!(
        receipt.package_id,
        GerbilSchemePackageId::new(GERBIL_RESOLVED_LOOP_POLICY_PACK_PACKAGE_ID)
    );
    assert_eq!(receipt.type_count, 1);
    assert_eq!(receipt.field_count, 5);
    assert_eq!(receipt.projection_contract_count, 1);
    assert_eq!(receipt.native_abi_version, None);
    assert_eq!(receipt.native_symbol_count, 0);

    let projection_type = manifest
        .type_manifest
        .types
        .first()
        .expect("resolved loop policy pack type manifest entry");
    assert_eq!(
        projection_type.type_id,
        GerbilSchemeTypeId::new(GERBIL_RESOLVED_LOOP_POLICY_PACK_TYPE_ID)
    );
    assert_eq!(
        projection_type
            .schema_id
            .as_ref()
            .map(|schema| schema.as_str()),
        Some(GERBIL_RESOLVED_LOOP_POLICY_PACK_SCHEMA_ID)
    );
}
