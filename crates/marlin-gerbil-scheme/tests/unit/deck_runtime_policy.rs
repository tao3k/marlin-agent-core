use marlin_agent_protocol::{ModelRouteAgentScope, ModelRouteRequest};
use marlin_gerbil_scheme::{
    GERBIL_DECK_RUNTIME_NATIVE_PROJECTION_ABI_ID,
    GERBIL_DECK_RUNTIME_NATIVE_PROJECTION_ABI_VERSION,
    GERBIL_DECK_RUNTIME_POO_POLICY_PROJECTION_SCHEMA_ID,
    GERBIL_DECK_RUNTIME_POO_POLICY_PROJECTION_TYPE_ID,
    GERBIL_DECK_RUNTIME_PROJECT_POO_POLICY_SYMBOL,
    GERBIL_DECK_RUNTIME_PROJECT_RESOLVED_LOOP_POLICY_PACK_SYMBOL,
    GERBIL_RESOLVED_LOOP_POLICY_PACK_SCHEMA_ID, GERBIL_RESOLVED_LOOP_POLICY_PACK_TYPE_ID,
    GerbilDeckRuntimeContextMode, GerbilDeckRuntimeIsolationMode,
    GerbilDeckRuntimeModelRoutePolicy, GerbilDeckRuntimeModelRoutePolicyRequest,
    GerbilDeckRuntimeModelRouteSelectedPolicy, GerbilDeckRuntimeModelRouteSelectionReceipt,
    GerbilDeckRuntimeSelectedPolicyKind, gerbil_deck_runtime_native_projection_readiness_plan,
    gerbil_deck_runtime_poo_policy_projection_request,
    gerbil_deck_runtime_resolved_loop_policy_pack_projection_request, gerbil_runtime_asset,
};

#[test]
fn gerbil_deck_runtime_policy_request_uses_typed_agent_scope() {
    let policy =
        GerbilDeckRuntimeModelRoutePolicy::new("cheap-test-runner", "openai", "gpt-5-mini")
            .with_command_prefix("cargo test")
            .with_agent_scope("sub-agent");
    let request = ModelRouteRequest::command(["cargo", "test"])
        .with_agent_scope(ModelRouteAgentScope::SubAgent)
        .with_sub_agent_role("tester");

    let scheme_request =
        GerbilDeckRuntimeModelRoutePolicyRequest::from_model_route_request([policy], &request);

    assert_eq!(scheme_request.command, "cargo test");
    assert_eq!(scheme_request.agent_scope, "sub-agent");
    assert_eq!(scheme_request.policies[0].provider, "openai");
    assert_eq!(scheme_request.policies[0].model, "gpt-5-mini");
}

#[test]
fn gerbil_deck_runtime_policy_receipt_preserves_provider_model_identity() {
    let receipt = GerbilDeckRuntimeModelRouteSelectionReceipt {
        schema_id: "marlin-deck-runtime.model-route-selection.v1".to_string(),
        command: "cargo test".to_string(),
        agent_scope: "sub-agent".to_string(),
        matched: true,
        policy: Some(GerbilDeckRuntimeModelRouteSelectedPolicy {
            kind: GerbilDeckRuntimeSelectedPolicyKind::new(
                "marlin-deck-runtime.model-route-policy.v1",
            ),
            name: "cheap-test-runner".to_string(),
            provider: "openai".to_string(),
            model: "gpt-5-mini".to_string(),
            command_prefixes: vec!["cargo test".to_string()],
            agent_scopes: vec!["sub-agent".to_string()],
            context_mode: GerbilDeckRuntimeContextMode::new("forked-context"),
            isolation_mode: GerbilDeckRuntimeIsolationMode::new("workspace-isolated"),
        }),
    };

    let policy = receipt.selected_policy().expect("selected policy");
    assert!(receipt.matched);
    assert_eq!(policy.provider, "openai");
    assert_eq!(policy.model, "gpt-5-mini");
    assert_eq!(policy.context_mode.as_str(), "forked-context");
}

#[test]
fn gerbil_deck_runtime_poo_policy_projection_request_matches_scheme_module_contract() {
    let request = gerbil_deck_runtime_poo_policy_projection_request();
    let resolved_request = gerbil_deck_runtime_resolved_loop_policy_pack_projection_request();
    let readiness_plan = gerbil_deck_runtime_native_projection_readiness_plan();

    assert_eq!(
        request.abi_id.as_str(),
        GERBIL_DECK_RUNTIME_NATIVE_PROJECTION_ABI_ID
    );
    assert_eq!(
        request.abi_version,
        GERBIL_DECK_RUNTIME_NATIVE_PROJECTION_ABI_VERSION
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
        ]
    );

    let native_projection_source =
        gerbil_runtime_asset("src/marlin/deck-runtime-native-projection.ss")
            .expect("generated manifest includes native projection source")
            .source;
    for expected in [
        GERBIL_DECK_RUNTIME_NATIVE_PROJECTION_ABI_ID,
        GERBIL_DECK_RUNTIME_PROJECT_POO_POLICY_SYMBOL,
        GERBIL_DECK_RUNTIME_PROJECT_RESOLVED_LOOP_POLICY_PACK_SYMBOL,
        GERBIL_DECK_RUNTIME_POO_POLICY_PROJECTION_TYPE_ID,
        GERBIL_DECK_RUNTIME_POO_POLICY_PROJECTION_SCHEMA_ID,
        GERBIL_RESOLVED_LOOP_POLICY_PACK_TYPE_ID,
        GERBIL_RESOLVED_LOOP_POLICY_PACK_SCHEMA_ID,
    ] {
        assert!(
            native_projection_source.contains(expected),
            "native projection Scheme source should contain {expected}"
        );
    }
}
