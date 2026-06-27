use marlin_gerbil_scheme::{
    GERBIL_DECK_RUNTIME_NATIVE_PROJECTION_ABI_ID,
    GERBIL_DECK_RUNTIME_NATIVE_PROJECTION_ABI_VERSION,
    GERBIL_DECK_RUNTIME_NATIVE_PROJECTION_PACKAGE_ID,
    GERBIL_DECK_RUNTIME_POO_POLICY_PROJECTION_SCHEMA_ID,
    GERBIL_DECK_RUNTIME_POO_POLICY_PROJECTION_TYPE_ID,
    GERBIL_DECK_RUNTIME_PROJECT_LOOP_POLICY_PROJECTION_MODULE_SYMBOL,
    GERBIL_DECK_RUNTIME_PROJECT_POO_POLICY_SYMBOL,
    GERBIL_DECK_RUNTIME_PROJECT_RESOLVED_LOOP_POLICY_PACK_SYMBOL,
    GERBIL_LOOP_GRAPH_CONTINUATION_NATIVE_PROJECTION_ABI_ID,
    GERBIL_LOOP_GRAPH_CONTINUATION_NATIVE_PROJECTION_ABI_VERSION,
    GERBIL_LOOP_GRAPH_CONTINUATION_NATIVE_PROJECTION_PACKAGE_ID,
    GERBIL_LOOP_GRAPH_CONTINUATION_NATIVE_SYMBOL, GERBIL_LOOP_GRAPH_CONTINUATION_TYPE_ID,
    GERBIL_LOOP_POLICY_PROJECTION_MODULE_SCHEMA_ID, GERBIL_LOOP_POLICY_PROJECTION_MODULE_TYPE_ID,
    GERBIL_RESOLVED_LOOP_POLICY_PACK_SCHEMA_ID, GERBIL_RESOLVED_LOOP_POLICY_PACK_TYPE_ID,
    GerbilSchemeNativeAbiId, GerbilSchemeNativeSymbol, GerbilSchemePackageId, GerbilSchemeTypeId,
    gerbil_deck_runtime_native_projection_package_manifest,
    gerbil_deck_runtime_native_projection_readiness_plan,
    gerbil_loop_graph_continuation_native_projection_package_manifest,
    gerbil_loop_graph_continuation_native_projection_readiness_plan,
    validate_gerbil_scheme_package_manifest, validate_gerbil_scheme_package_native_readiness,
};

#[test]
fn deck_runtime_native_projection_package_manifest_connects_poo_type_contract_and_abi() {
    let manifest = gerbil_deck_runtime_native_projection_package_manifest();

    let receipt = validate_gerbil_scheme_package_manifest(&manifest)
        .expect("Deck runtime POO native projection package manifest should validate");

    assert_eq!(
        receipt.package_id,
        GerbilSchemePackageId::new(GERBIL_DECK_RUNTIME_NATIVE_PROJECTION_PACKAGE_ID)
    );
    assert_eq!(receipt.type_count, 3);
    assert_eq!(receipt.field_count, 25);
    assert_eq!(receipt.projection_contract_count, 3);
    assert_eq!(
        receipt.native_abi_version,
        Some(GERBIL_DECK_RUNTIME_NATIVE_PROJECTION_ABI_VERSION)
    );
    assert_eq!(receipt.native_symbol_count, 3);

    let projection_types = &manifest.type_manifest.types;
    let projection_type = projection_types
        .iter()
        .find(|type_spec| {
            type_spec.type_id
                == GerbilSchemeTypeId::new(GERBIL_DECK_RUNTIME_POO_POLICY_PROJECTION_TYPE_ID)
        })
        .expect("POO projection type manifest entry");
    assert_eq!(
        projection_type
            .schema_id
            .as_ref()
            .map(|schema| schema.as_str()),
        Some(GERBIL_DECK_RUNTIME_POO_POLICY_PROJECTION_SCHEMA_ID)
    );

    let resolved_type = projection_types
        .iter()
        .find(|type_spec| {
            type_spec.type_id == GerbilSchemeTypeId::new(GERBIL_RESOLVED_LOOP_POLICY_PACK_TYPE_ID)
        })
        .expect("resolved loop policy pack type manifest entry");
    assert_eq!(
        resolved_type
            .schema_id
            .as_ref()
            .map(|schema| schema.as_str()),
        Some(GERBIL_RESOLVED_LOOP_POLICY_PACK_SCHEMA_ID)
    );

    let projection_module_type = projection_types
        .iter()
        .find(|type_spec| {
            type_spec.type_id
                == GerbilSchemeTypeId::new(GERBIL_LOOP_POLICY_PROJECTION_MODULE_TYPE_ID)
        })
        .expect("loop policy projection module type manifest entry");
    assert_eq!(
        projection_module_type
            .schema_id
            .as_ref()
            .map(|schema| schema.as_str()),
        Some(GERBIL_LOOP_POLICY_PROJECTION_MODULE_SCHEMA_ID)
    );

    let native_abi = manifest
        .native_abi
        .as_ref()
        .expect("POO projection package native ABI");
    assert_eq!(
        native_abi.abi_id,
        GerbilSchemeNativeAbiId::new(GERBIL_DECK_RUNTIME_NATIVE_PROJECTION_ABI_ID)
    );
    assert_eq!(
        native_abi.version,
        GERBIL_DECK_RUNTIME_NATIVE_PROJECTION_ABI_VERSION
    );
    assert_eq!(
        native_abi.exported_symbols,
        vec![
            GerbilSchemeNativeSymbol::new(GERBIL_DECK_RUNTIME_PROJECT_POO_POLICY_SYMBOL),
            GerbilSchemeNativeSymbol::new(
                GERBIL_DECK_RUNTIME_PROJECT_RESOLVED_LOOP_POLICY_PACK_SYMBOL
            ),
            GerbilSchemeNativeSymbol::new(
                GERBIL_DECK_RUNTIME_PROJECT_LOOP_POLICY_PROJECTION_MODULE_SYMBOL
            ),
        ]
    );

    let readiness = validate_gerbil_scheme_package_native_readiness(
        &manifest,
        &gerbil_deck_runtime_native_projection_readiness_plan(),
    )
    .expect("Deck runtime POO native projection package should match readiness plan");

    assert_eq!(readiness.required_symbol_count, 3);
    assert_eq!(readiness.available_symbol_count, 3);
    assert_eq!(readiness.matched_symbol_count, 3);
}

#[test]
fn graph_loop_continuation_native_projection_package_manifest_connects_type_contract_and_abi() {
    let manifest = gerbil_loop_graph_continuation_native_projection_package_manifest();

    let receipt = validate_gerbil_scheme_package_manifest(&manifest)
        .expect("Gerbil continuation native projection package manifest should validate");

    assert_eq!(
        receipt.package_id,
        GerbilSchemePackageId::new(GERBIL_LOOP_GRAPH_CONTINUATION_NATIVE_PROJECTION_PACKAGE_ID)
    );
    assert_eq!(receipt.type_count, 1);
    assert_eq!(receipt.field_count, 3);
    assert_eq!(receipt.projection_contract_count, 1);
    assert_eq!(
        receipt.native_abi_version,
        Some(GERBIL_LOOP_GRAPH_CONTINUATION_NATIVE_PROJECTION_ABI_VERSION)
    );
    assert_eq!(receipt.native_symbol_count, 1);

    let projection_type = manifest
        .type_manifest
        .types
        .first()
        .expect("continuation projection type manifest entry");
    assert_eq!(
        projection_type.type_id,
        GerbilSchemeTypeId::new(GERBIL_LOOP_GRAPH_CONTINUATION_TYPE_ID)
    );

    let native_abi = manifest
        .native_abi
        .as_ref()
        .expect("continuation projection package native ABI");
    assert_eq!(
        native_abi.abi_id,
        GerbilSchemeNativeAbiId::new(GERBIL_LOOP_GRAPH_CONTINUATION_NATIVE_PROJECTION_ABI_ID)
    );
    assert_eq!(
        native_abi.version,
        GERBIL_LOOP_GRAPH_CONTINUATION_NATIVE_PROJECTION_ABI_VERSION
    );
    assert_eq!(
        native_abi.exported_symbols,
        vec![GerbilSchemeNativeSymbol::new(
            GERBIL_LOOP_GRAPH_CONTINUATION_NATIVE_SYMBOL
        )]
    );

    let readiness = validate_gerbil_scheme_package_native_readiness(
        &manifest,
        &gerbil_loop_graph_continuation_native_projection_readiness_plan(),
    )
    .expect("Gerbil continuation native projection package should match readiness plan");

    assert_eq!(readiness.required_symbol_count, 1);
    assert_eq!(readiness.available_symbol_count, 1);
    assert_eq!(readiness.matched_symbol_count, 1);
}
