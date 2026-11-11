// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Native C ABI boundary for Gerbil AgentGraph policy routing projections.
//!
//! Gerbil owns policy semantics. Rust owns the ABI wrapper, the typed Scheme
//! value envelope, and projection into `marlin-agent-graph` receipts.

mod abi;
mod request;
mod selector;

pub use abi::{
    GERBIL_AGENT_POLICY_ROUTING_ABI_HEADER_PATH, GERBIL_AGENT_POLICY_ROUTING_ABI_HEADER_SOURCE,
    GERBIL_AGENT_POLICY_ROUTING_ABI_ID, GERBIL_AGENT_POLICY_ROUTING_ABI_STATUS_ABI_MISMATCH,
    GERBIL_AGENT_POLICY_ROUTING_ABI_STATUS_INVALID_PROJECTION,
    GERBIL_AGENT_POLICY_ROUTING_ABI_STATUS_NULL_POINTER, GERBIL_AGENT_POLICY_ROUTING_ABI_STATUS_OK,
    GERBIL_AGENT_POLICY_ROUTING_ABI_VERSION, GerbilAgentPolicyRoutingAbiEvidence,
    GerbilAgentPolicyRoutingAbiEvidenceList, GerbilAgentPolicyRoutingAbiInitializeFn,
    GerbilAgentPolicyRoutingAbiProjection, GerbilAgentPolicyRoutingAbiRequest,
    GerbilAgentPolicyRoutingAbiSelectEdgesFn, GerbilAgentPolicyRoutingAbiStatus,
    GerbilAgentPolicyRoutingAbiUtf8, GerbilAgentPolicyRoutingAbiUtf8List,
};
pub use request::{
    GerbilAgentPolicyRoutingAbiEpochBacking, GerbilAgentPolicyRoutingAbiEvidenceRef,
    GerbilAgentPolicyRoutingAbiMatchKey, GerbilAgentPolicyRoutingAbiPayload,
    GerbilAgentPolicyRoutingAbiRequestConversionProfile,
    GerbilAgentPolicyRoutingAbiSelectEdgesRequest,
    gerbil_agent_policy_routing_abi_request_conversion_profile,
};
pub use selector::{GerbilAgentPolicyRoutingAbiError, GerbilAgentPolicyRoutingAbiSelector};
