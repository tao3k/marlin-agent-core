use super::support::{
    fake_initialize_failure, fake_select_edges, fake_select_invalid_utf8,
    fake_select_output_abi_mismatch, fake_select_runtime_status, routing_request,
};
use marlin_gerbil_scheme::{
    GERBIL_AGENT_POLICY_ROUTING_NATIVE_ABI_VERSION,
    GERBIL_AGENT_POLICY_ROUTING_NATIVE_STATUS_ABI_MISMATCH, GerbilAgentPolicyRoutingNativeAbiError,
    GerbilAgentPolicyRoutingNativeSelector,
};

#[test]
fn agent_policy_routing_native_selector_rejects_output_abi_mismatch() {
    let selector = GerbilAgentPolicyRoutingNativeSelector::new(fake_select_output_abi_mismatch);
    let request = routing_request();

    let error = selector
        .project_policy_receipt(&request)
        .expect_err("native selector should reject mismatched output ABI version");

    assert_eq!(
        error,
        GerbilAgentPolicyRoutingNativeAbiError::OutputAbiVersion {
            expected: GERBIL_AGENT_POLICY_ROUTING_NATIVE_ABI_VERSION,
            actual: GERBIL_AGENT_POLICY_ROUTING_NATIVE_ABI_VERSION + 1,
        }
    );
}

#[test]
fn agent_policy_routing_native_selector_surfaces_runtime_status() {
    let selector = GerbilAgentPolicyRoutingNativeSelector::new(fake_select_runtime_status);
    let request = routing_request();

    let error = selector
        .project_policy_receipt(&request)
        .expect_err("native selector should surface non-zero runtime status");

    assert_eq!(
        error,
        GerbilAgentPolicyRoutingNativeAbiError::RuntimeStatus {
            code: GERBIL_AGENT_POLICY_ROUTING_NATIVE_STATUS_ABI_MISMATCH,
        }
    );
}

#[test]
fn agent_policy_routing_native_selector_rejects_invalid_utf8_projection() {
    let selector = GerbilAgentPolicyRoutingNativeSelector::new(fake_select_invalid_utf8);
    let request = routing_request();

    let error = selector
        .project_policy_receipt(&request)
        .expect_err("native selector should reject invalid UTF-8 projection fields");

    assert!(matches!(
        error,
        GerbilAgentPolicyRoutingNativeAbiError::InvalidUtf8 {
            field: "routing_decision",
            ..
        }
    ));
}

#[test]
fn agent_policy_routing_native_selector_surfaces_initializer_status() {
    let selector = GerbilAgentPolicyRoutingNativeSelector::with_initializer(
        fake_initialize_failure,
        fake_select_edges,
    );
    let request = routing_request();

    let error = selector
        .project_policy_receipt(&request)
        .expect_err("native selector should surface initializer failure");

    assert_eq!(
        error,
        GerbilAgentPolicyRoutingNativeAbiError::RuntimeInit { code: 55 }
    );
}
