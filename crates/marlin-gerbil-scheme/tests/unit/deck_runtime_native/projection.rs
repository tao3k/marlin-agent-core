use super::support::{fake_select_model_route, fake_select_no_match, route_request};
use marlin_gerbil_scheme::GerbilDeckRuntimeNativeModelRouteSelector;

#[test]
fn gerbil_deck_runtime_native_selector_uses_typed_c_abi_selection() {
    let selector = GerbilDeckRuntimeNativeModelRouteSelector::new(fake_select_model_route);
    let request = route_request("cargo test", "sub-agent");

    let policy_index = selector
        .select_policy_index(&request)
        .expect("native C ABI selector should return a typed policy index");
    assert_eq!(policy_index, Some(0));

    let receipt = selector
        .evaluate(&request)
        .expect("native C ABI selector should project typed runtime selection");
    let selected = receipt.selected_policy().expect("selected policy");

    assert!(receipt.matched);
    assert_eq!(selected.name, "cheap-test-runner");
    assert_eq!(selected.provider, "openai");
    assert_eq!(selected.model, "gpt-5-mini");
}

#[test]
fn gerbil_deck_runtime_native_selector_projects_no_match_selection() {
    let selector = GerbilDeckRuntimeNativeModelRouteSelector::new(fake_select_no_match);
    let request = route_request("cargo clippy", "sub-agent");

    let policy_index = selector
        .select_policy_index(&request)
        .expect("native C ABI selector should return unmatched policy index");
    assert_eq!(policy_index, None);

    let receipt = selector
        .evaluate(&request)
        .expect("native C ABI selector should project unmatched selection");

    assert!(!receipt.matched);
    assert!(receipt.selected_policy().is_none());
    assert_eq!(receipt.command, "cargo clippy");
    assert_eq!(receipt.agent_scope, "sub-agent");
}
