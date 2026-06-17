#[test]
fn package_identity_marks_agent_policy_routing_native_bridge() {
    assert_eq!(env!("CARGO_PKG_NAME"), "marlin-agent-policy-routing-native");
}
