use marlin_agent_protocol::{
    ModelCommandMatcher, ModelEndpoint, ModelRouteRequest, ModelRouteRule,
};
use marlin_deck_runtime_native::{DeckRuntimeNativeRouteError, DeckRuntimeNativeRouteResolver};
use marlin_gerbil_scheme::{
    GerbilDeckRuntimeNativeAbiError, GerbilDeckRuntimeNativeModelRouteRequest,
    GerbilDeckRuntimeNativeModelRouteSelection, GerbilDeckRuntimeNativeModelRouteSelector,
    GerbilDeckRuntimeNativeStatus,
};

#[test]
fn native_route_resolver_maps_priority_order_to_original_policy_index() {
    let resolver = DeckRuntimeNativeRouteResolver::new(
        GerbilDeckRuntimeNativeModelRouteSelector::new(fake_select_native_index_zero),
        vec![
            ModelRouteRule::new(
                "cheap-original-zero",
                1,
                ModelCommandMatcher::new().with_argv_glob("cargo test*"),
                ModelEndpoint::new("openai", "gpt-5.4-mini"),
            ),
            ModelRouteRule::new(
                "expensive-original-one",
                100,
                ModelCommandMatcher::new().with_argv_glob("cargo test*"),
                ModelEndpoint::new("anthropic", "claude-opus-4-8"),
            ),
        ],
    )
    .expect("native route resolver compiles");

    let decision = resolver
        .resolve(&ModelRouteRequest::command(["cargo", "test", "-p", "demo"]))
        .expect("native route resolves")
        .expect("native policy matches");

    assert_eq!(decision.receipt.rule_id.as_str(), "expensive-original-one");
    assert_eq!(
        decision.receipt.litellm_model_id.as_str(),
        "anthropic/claude-opus-4-8"
    );
}

#[test]
fn native_route_resolver_returns_none_without_native_match() {
    let resolver = DeckRuntimeNativeRouteResolver::new(
        GerbilDeckRuntimeNativeModelRouteSelector::new(fake_select_no_match),
        vec![ModelRouteRule::new(
            "route",
            1,
            ModelCommandMatcher::new().with_argv_glob("cargo test*"),
            ModelEndpoint::new("openai", "gpt-5.4-mini"),
        )],
    )
    .expect("native route resolver compiles");

    let decision = resolver
        .resolve(&ModelRouteRequest::command(["cargo", "test"]))
        .expect("native no-match resolves");

    assert!(decision.is_none());
}

#[test]
fn native_route_resolver_rejects_out_of_range_native_index() {
    let resolver = DeckRuntimeNativeRouteResolver::new(
        GerbilDeckRuntimeNativeModelRouteSelector::new(fake_select_out_of_range),
        vec![ModelRouteRule::new(
            "route",
            1,
            ModelCommandMatcher::new().with_argv_glob("cargo test*"),
            ModelEndpoint::new("openai", "gpt-5.4-mini"),
        )],
    )
    .expect("native route resolver compiles");

    let error = resolver
        .resolve(&ModelRouteRequest::command(["cargo", "test"]))
        .expect_err("native policy index should be rejected");

    assert_eq!(
        error,
        DeckRuntimeNativeRouteError::NativeAbi(
            GerbilDeckRuntimeNativeAbiError::InvalidPolicyIndex {
                index: 9,
                policies_len: 1,
            }
        )
    );
}

#[test]
fn native_route_resolver_revalidates_selected_policy_with_runtime_resolver() {
    let resolver = DeckRuntimeNativeRouteResolver::new(
        GerbilDeckRuntimeNativeModelRouteSelector::new(fake_select_native_index_zero),
        vec![ModelRouteRule::new(
            "cargo-route",
            1,
            ModelCommandMatcher::new().with_executable_glob("cargo"),
            ModelEndpoint::new("openai", "gpt-5.4-mini"),
        )],
    )
    .expect("native route resolver compiles");

    let error = resolver
        .resolve(&ModelRouteRequest::command(["just", "test"]))
        .expect_err("stale native selection should be rejected");

    assert!(matches!(error, DeckRuntimeNativeRouteError::Projection(_)));
}

unsafe extern "C" fn fake_select_native_index_zero(
    request: *const GerbilDeckRuntimeNativeModelRouteRequest,
    selection: *mut GerbilDeckRuntimeNativeModelRouteSelection,
) -> GerbilDeckRuntimeNativeStatus {
    if request.is_null() || selection.is_null() {
        return GerbilDeckRuntimeNativeStatus::NULL_POINTER;
    }

    unsafe {
        *selection = GerbilDeckRuntimeNativeModelRouteSelection::matched(0);
    }

    GerbilDeckRuntimeNativeStatus::OK
}

unsafe extern "C" fn fake_select_no_match(
    request: *const GerbilDeckRuntimeNativeModelRouteRequest,
    selection: *mut GerbilDeckRuntimeNativeModelRouteSelection,
) -> GerbilDeckRuntimeNativeStatus {
    if request.is_null() || selection.is_null() {
        return GerbilDeckRuntimeNativeStatus::NULL_POINTER;
    }

    unsafe {
        *selection = GerbilDeckRuntimeNativeModelRouteSelection::empty();
    }

    GerbilDeckRuntimeNativeStatus::OK
}

unsafe extern "C" fn fake_select_out_of_range(
    request: *const GerbilDeckRuntimeNativeModelRouteRequest,
    selection: *mut GerbilDeckRuntimeNativeModelRouteSelection,
) -> GerbilDeckRuntimeNativeStatus {
    if request.is_null() || selection.is_null() {
        return GerbilDeckRuntimeNativeStatus::NULL_POINTER;
    }

    unsafe {
        *selection = GerbilDeckRuntimeNativeModelRouteSelection::matched(9);
    }

    GerbilDeckRuntimeNativeStatus::OK
}
