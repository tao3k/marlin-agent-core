//! Real linked selector constructor for Gerbil AgentGraph policy routing.

use marlin_gerbil_scheme::{
    GerbilAgentPolicyRoutingNativeProjection, GerbilAgentPolicyRoutingNativeRequest,
    GerbilAgentPolicyRoutingNativeSelector, GerbilAgentPolicyRoutingNativeStatus,
};

unsafe extern "C" {
    fn marlin_agent_policy_routing_initialize() -> i32;

    fn marlin_agent_policy_routing_select_edges(
        request: *const GerbilAgentPolicyRoutingNativeRequest,
        projection: *mut GerbilAgentPolicyRoutingNativeProjection,
    ) -> GerbilAgentPolicyRoutingNativeStatus;
}

/// Builds a selector from the symbols linked by the `linked-native` build path.
pub fn linked_agent_policy_routing_native_selector() -> GerbilAgentPolicyRoutingNativeSelector {
    GerbilAgentPolicyRoutingNativeSelector::with_initializer(
        marlin_agent_policy_routing_initialize,
        marlin_agent_policy_routing_select_edges,
    )
}
