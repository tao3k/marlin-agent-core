//! Native C ABI boundary for Gerbil AgentGraph policy routing projections.
//!
//! Gerbil owns policy semantics. Rust owns the ABI wrapper, the typed Scheme
//! value envelope, and projection into `marlin-agent-graph` receipts.

mod abi;
mod request;
mod selector;

pub use abi::{
    GERBIL_AGENT_POLICY_ROUTING_NATIVE_ABI_ID, GERBIL_AGENT_POLICY_ROUTING_NATIVE_ABI_VERSION,
    GERBIL_AGENT_POLICY_ROUTING_NATIVE_HEADER_PATH,
    GERBIL_AGENT_POLICY_ROUTING_NATIVE_HEADER_SOURCE,
    GERBIL_AGENT_POLICY_ROUTING_NATIVE_STATUS_ABI_MISMATCH,
    GERBIL_AGENT_POLICY_ROUTING_NATIVE_STATUS_INVALID_PROJECTION,
    GERBIL_AGENT_POLICY_ROUTING_NATIVE_STATUS_NULL_POINTER,
    GERBIL_AGENT_POLICY_ROUTING_NATIVE_STATUS_OK, GerbilAgentPolicyRoutingNativeEvidence,
    GerbilAgentPolicyRoutingNativeEvidenceList, GerbilAgentPolicyRoutingNativeInitializeFn,
    GerbilAgentPolicyRoutingNativeProjection, GerbilAgentPolicyRoutingNativeRequest,
    GerbilAgentPolicyRoutingNativeSelectEdgesFn, GerbilAgentPolicyRoutingNativeStatus,
    GerbilAgentPolicyRoutingNativeUtf8, GerbilAgentPolicyRoutingNativeUtf8List,
};
pub use request::{
    GerbilAgentPolicyRoutingNativeEpochBacking, GerbilAgentPolicyRoutingNativeEvidenceRef,
    GerbilAgentPolicyRoutingNativeMatchKey, GerbilAgentPolicyRoutingNativePayload,
    GerbilAgentPolicyRoutingNativeRequestConversionProfile,
    GerbilAgentPolicyRoutingNativeSelectEdgesRequest,
    gerbil_agent_policy_routing_native_request_conversion_profile,
};
pub use selector::{
    GerbilAgentPolicyRoutingNativeAbiError, GerbilAgentPolicyRoutingNativeSelector,
};
