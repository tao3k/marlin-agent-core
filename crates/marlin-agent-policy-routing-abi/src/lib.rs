//! Linked native bridge for Gerbil AgentGraph policy-routing selectors.

#[cfg(feature = "linked-selector")]
mod linked;

#[cfg(feature = "linked-selector")]
pub use linked::linked_agent_policy_routing_abi_selector;
