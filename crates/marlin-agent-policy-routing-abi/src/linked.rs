//! Real linked selector constructor for Gerbil AgentGraph policy routing.

use marlin_gerbil_scheme::{
    GerbilAgentPolicyRoutingAbiProjection, GerbilAgentPolicyRoutingAbiRequest,
    GerbilAgentPolicyRoutingAbiSelector, GerbilAgentPolicyRoutingAbiStatus,
};

unsafe extern "C" {
    fn marlin_agent_policy_routing_initialize() -> i32;

    fn marlin_agent_policy_routing_select_edges(
        request: *const GerbilAgentPolicyRoutingAbiRequest,
        projection: *mut GerbilAgentPolicyRoutingAbiProjection,
    ) -> GerbilAgentPolicyRoutingAbiStatus;
}

/// Builds a selector from the symbols linked by the `linked-selector` build path.
pub fn linked_agent_policy_routing_abi_selector() -> GerbilAgentPolicyRoutingAbiSelector {
    GerbilAgentPolicyRoutingAbiSelector::with_initializer(
        marlin_agent_policy_routing_initialize,
        marlin_agent_policy_routing_select_edges,
    )
}
