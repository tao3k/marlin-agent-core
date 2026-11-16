// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Linked native bridge for Gerbil AgentGraph policy-routing selectors.

#[cfg(feature = "linked-selector")]
mod linked;

#[cfg(feature = "linked-selector")]
pub use linked::linked_agent_policy_routing_abi_selector;
