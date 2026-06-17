//! Linked native bridge for Gerbil AgentGraph policy-routing selectors.

#[cfg(feature = "linked-native")]
mod linked;

#[cfg(feature = "linked-native")]
pub use linked::linked_agent_policy_routing_native_selector;
