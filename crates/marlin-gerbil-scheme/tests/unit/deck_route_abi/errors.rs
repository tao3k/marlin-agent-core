use super::support::{
    fake_initialize_failure, fake_select_invalid_policy_index, fake_select_model_route,
    fake_select_output_abi_mismatch, fake_select_runtime_status,
    fake_select_unmatched_with_policy_index, route_request,
};
use marlin_gerbil_scheme::{
    GERBIL_DECK_ROUTE_ABI_STATUS_ABI_MISMATCH, GERBIL_DECK_ROUTE_ABI_VERSION,
    GerbilDeckRouteAbiError, GerbilDeckRouteAbiModelRouteSelector,
};

#[test]
fn gerbil_deck_route_abi_selector_rejects_output_abi_mismatch() {
    let selector = GerbilDeckRouteAbiModelRouteSelector::new(fake_select_output_abi_mismatch);
    let request = route_request("cargo test", "sub-agent");

    let error = selector
        .evaluate(&request)
        .expect_err("native selector should reject mismatched output ABI version");

    assert_eq!(
        error,
        GerbilDeckRouteAbiError::OutputAbiVersion {
            expected: GERBIL_DECK_ROUTE_ABI_VERSION,
            actual: GERBIL_DECK_ROUTE_ABI_VERSION + 1,
        }
    );
}

#[test]
fn gerbil_deck_route_abi_selector_rejects_invalid_policy_index() {
    let selector = GerbilDeckRouteAbiModelRouteSelector::new(fake_select_invalid_policy_index);
    let request = route_request("cargo test", "sub-agent");

    let error = selector
        .evaluate(&request)
        .expect_err("native selector should reject selected policy index outside request");

    assert_eq!(
        error,
        GerbilDeckRouteAbiError::InvalidPolicyIndex {
            index: 99,
            policies_len: 1,
        }
    );
}

#[test]
fn gerbil_deck_route_abi_selector_rejects_unmatched_policy_index() {
    let selector =
        GerbilDeckRouteAbiModelRouteSelector::new(fake_select_unmatched_with_policy_index);
    let request = route_request("cargo test", "sub-agent");

    let error = selector
        .evaluate(&request)
        .expect_err("native selector should reject unmatched selection with concrete policy index");

    assert_eq!(
        error,
        GerbilDeckRouteAbiError::InvalidUnmatchedPolicyIndex { policy_index: 0 }
    );
}

#[test]
fn gerbil_deck_route_abi_selector_surfaces_runtime_status() {
    let selector = GerbilDeckRouteAbiModelRouteSelector::new(fake_select_runtime_status);
    let request = route_request("cargo test", "sub-agent");

    let error = selector
        .evaluate(&request)
        .expect_err("native selector should surface non-zero runtime status");

    assert_eq!(
        error,
        GerbilDeckRouteAbiError::RuntimeStatus {
            code: GERBIL_DECK_ROUTE_ABI_STATUS_ABI_MISMATCH,
        }
    );
}

#[test]
fn gerbil_deck_route_abi_selector_surfaces_initializer_status() {
    let selector = GerbilDeckRouteAbiModelRouteSelector::with_initializer(
        fake_initialize_failure,
        fake_select_model_route,
    );
    let request = route_request("cargo test", "sub-agent");

    let error = selector
        .evaluate(&request)
        .expect_err("native selector should surface initializer failure");

    assert_eq!(error, GerbilDeckRouteAbiError::RuntimeInit { code: 55 });
}
