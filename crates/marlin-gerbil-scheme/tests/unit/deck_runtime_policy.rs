use marlin_gerbil_scheme::{
    GERBIL_DECK_ROUTE_ABI_PROJECTION_ABI_ID, GERBIL_DECK_ROUTE_ABI_PROJECTION_ABI_VERSION,
    GERBIL_DECK_RUNTIME_POO_POLICY_PROJECTION_SCHEMA_ID,
    GERBIL_DECK_RUNTIME_POO_POLICY_PROJECTION_TYPE_ID,
    GERBIL_DECK_RUNTIME_PROJECT_LOOP_POLICY_PROJECTION_MODULE_SYMBOL,
    GERBIL_DECK_RUNTIME_PROJECT_POO_POLICY_SYMBOL,
    GERBIL_DECK_RUNTIME_PROJECT_RESOLVED_LOOP_POLICY_PACK_SYMBOL,
    GERBIL_LOOP_POLICY_PROJECTION_MODULE_SCHEMA_ID, GERBIL_LOOP_POLICY_PROJECTION_MODULE_TYPE_ID,
    GERBIL_RESOLVED_LOOP_POLICY_PACK_SCHEMA_ID, GERBIL_RESOLVED_LOOP_POLICY_PACK_TYPE_ID,
    gerbil_deck_route_abi_projection_readiness_plan,
    gerbil_deck_runtime_poo_policy_projection_request,
    gerbil_deck_runtime_resolved_loop_policy_pack_projection_request,
};

#[test]
fn gerbil_deck_runtime_poo_policy_projection_request_matches_scheme_module_contract() {
    let request = gerbil_deck_runtime_poo_policy_projection_request();
    let resolved_request = gerbil_deck_runtime_resolved_loop_policy_pack_projection_request();
    let readiness_plan = gerbil_deck_route_abi_projection_readiness_plan();

    assert_eq!(
        request.abi_id.as_str(),
        GERBIL_DECK_ROUTE_ABI_PROJECTION_ABI_ID
    );
    assert_eq!(
        request.abi_version,
        GERBIL_DECK_ROUTE_ABI_PROJECTION_ABI_VERSION
    );
    assert_eq!(
        request.symbol.as_str(),
        GERBIL_DECK_RUNTIME_PROJECT_POO_POLICY_SYMBOL
    );
    assert_eq!(
        request.contract.type_id.as_str(),
        GERBIL_DECK_RUNTIME_POO_POLICY_PROJECTION_TYPE_ID
    );
    assert_eq!(
        request
            .contract
            .schema_id
            .as_ref()
            .map(|schema| schema.as_str()),
        Some(GERBIL_DECK_RUNTIME_POO_POLICY_PROJECTION_SCHEMA_ID)
    );
    assert_eq!(resolved_request.abi_id, request.abi_id);
    assert_eq!(resolved_request.abi_version, request.abi_version);
    assert_eq!(
        resolved_request.symbol.as_str(),
        GERBIL_DECK_RUNTIME_PROJECT_RESOLVED_LOOP_POLICY_PACK_SYMBOL
    );
    assert_eq!(
        resolved_request.contract.type_id.as_str(),
        GERBIL_RESOLVED_LOOP_POLICY_PACK_TYPE_ID
    );
    assert_eq!(
        resolved_request
            .contract
            .schema_id
            .as_ref()
            .map(|schema| schema.as_str()),
        Some(GERBIL_RESOLVED_LOOP_POLICY_PACK_SCHEMA_ID)
    );
    assert_eq!(readiness_plan.abi_id, request.abi_id);
    assert_eq!(readiness_plan.version, request.abi_version);
    let exported_symbols = readiness_plan
        .exported_symbols
        .iter()
        .map(|symbol| symbol.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        exported_symbols,
        [
            request.symbol.as_str(),
            GERBIL_DECK_RUNTIME_PROJECT_RESOLVED_LOOP_POLICY_PACK_SYMBOL,
            GERBIL_DECK_RUNTIME_PROJECT_LOOP_POLICY_PROJECTION_MODULE_SYMBOL,
        ]
    );

    let abi_projection_source = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../scheme/policy/projection/deck/policy.ss"
    ));
    for expected in [
        GERBIL_DECK_ROUTE_ABI_PROJECTION_ABI_ID,
        GERBIL_DECK_RUNTIME_PROJECT_POO_POLICY_SYMBOL,
        GERBIL_DECK_RUNTIME_PROJECT_RESOLVED_LOOP_POLICY_PACK_SYMBOL,
        GERBIL_DECK_RUNTIME_PROJECT_LOOP_POLICY_PROJECTION_MODULE_SYMBOL,
        GERBIL_DECK_RUNTIME_POO_POLICY_PROJECTION_TYPE_ID,
        GERBIL_DECK_RUNTIME_POO_POLICY_PROJECTION_SCHEMA_ID,
        GERBIL_RESOLVED_LOOP_POLICY_PACK_TYPE_ID,
        GERBIL_RESOLVED_LOOP_POLICY_PACK_SCHEMA_ID,
        GERBIL_LOOP_POLICY_PROJECTION_MODULE_TYPE_ID,
        GERBIL_LOOP_POLICY_PROJECTION_MODULE_SCHEMA_ID,
    ] {
        assert!(
            abi_projection_source.contains(expected),
            "native projection Scheme source should contain {expected}"
        );
    }
}
