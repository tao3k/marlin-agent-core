use marlin_gerbil_scheme::{
    GERBIL_AGENT_POLICY_ROUTING_NATIVE_ABI_VERSION,
    GERBIL_AGENT_POLICY_ROUTING_NATIVE_HEADER_SOURCE,
    GERBIL_AGENT_POLICY_ROUTING_NATIVE_STATUS_ABI_MISMATCH,
    GERBIL_AGENT_POLICY_ROUTING_NATIVE_STATUS_INVALID_PROJECTION,
    GERBIL_AGENT_POLICY_ROUTING_NATIVE_STATUS_NULL_POINTER,
    GERBIL_AGENT_POLICY_ROUTING_NATIVE_STATUS_OK, GerbilAgentPolicyRoutingNativeEvidence,
    GerbilAgentPolicyRoutingNativeEvidenceList, GerbilAgentPolicyRoutingNativeProjection,
    GerbilAgentPolicyRoutingNativeRequest, GerbilAgentPolicyRoutingNativeStatus,
    GerbilAgentPolicyRoutingNativeUtf8, GerbilAgentPolicyRoutingNativeUtf8List,
};
use memoffset::offset_of;
use static_assertions::{assert_eq_align, const_assert_eq};
use std::mem::size_of;

const USIZE_BYTES: usize = size_of::<usize>();

const_assert_eq!(size_of::<*const u8>(), USIZE_BYTES);
const_assert_eq!(
    size_of::<GerbilAgentPolicyRoutingNativeUtf8>(),
    USIZE_BYTES * 2
);
const_assert_eq!(
    size_of::<GerbilAgentPolicyRoutingNativeUtf8List>(),
    USIZE_BYTES * 2
);
const_assert_eq!(
    size_of::<GerbilAgentPolicyRoutingNativeEvidence>(),
    USIZE_BYTES * 4
);
const_assert_eq!(
    size_of::<GerbilAgentPolicyRoutingNativeEvidenceList>(),
    USIZE_BYTES * 2
);
const_assert_eq!(
    size_of::<GerbilAgentPolicyRoutingNativeRequest>(),
    USIZE_BYTES * 11
);
const_assert_eq!(
    size_of::<GerbilAgentPolicyRoutingNativeProjection>(),
    USIZE_BYTES * 3
);

assert_eq_align!(GerbilAgentPolicyRoutingNativeUtf8, usize);
assert_eq_align!(GerbilAgentPolicyRoutingNativeUtf8List, usize);
assert_eq_align!(GerbilAgentPolicyRoutingNativeEvidence, usize);
assert_eq_align!(GerbilAgentPolicyRoutingNativeEvidenceList, usize);
assert_eq_align!(GerbilAgentPolicyRoutingNativeRequest, usize);
assert_eq_align!(GerbilAgentPolicyRoutingNativeProjection, usize);

#[test]
fn agent_policy_routing_native_header_exports_c_abi_contract() {
    assert!(
        GERBIL_AGENT_POLICY_ROUTING_NATIVE_HEADER_SOURCE
            .contains("#define MARLIN_AGENT_POLICY_ROUTING_NATIVE_ABI_VERSION 1u")
    );
    assert!(
        GERBIL_AGENT_POLICY_ROUTING_NATIVE_HEADER_SOURCE
            .contains("MarlinAgentPolicyRoutingSelectEdgesFn")
    );
    assert!(
        GERBIL_AGENT_POLICY_ROUTING_NATIVE_HEADER_SOURCE
            .contains("MarlinAgentPolicyRoutingRequest")
    );
    assert!(
        GERBIL_AGENT_POLICY_ROUTING_NATIVE_HEADER_SOURCE
            .contains("MarlinAgentPolicyRoutingProjection")
    );
    assert!(
        GERBIL_AGENT_POLICY_ROUTING_NATIVE_HEADER_SOURCE
            .contains("marlin_agent_policy_routing_select_edges")
    );
    assert!(
        GERBIL_AGENT_POLICY_ROUTING_NATIVE_HEADER_SOURCE
            .contains("MARLIN_AGENT_POLICY_ROUTING_NATIVE_STATUS_INVALID_PROJECTION")
    );
    assert!(!GERBIL_AGENT_POLICY_ROUTING_NATIVE_HEADER_SOURCE.contains("JSON"));
    assert!(!GERBIL_AGENT_POLICY_ROUTING_NATIVE_HEADER_SOURCE.contains("OwnedBytes"));
}

#[test]
fn agent_policy_routing_native_status_constants_are_stable() {
    assert_eq!(GERBIL_AGENT_POLICY_ROUTING_NATIVE_ABI_VERSION, 1);
    assert_eq!(
        GerbilAgentPolicyRoutingNativeStatus::OK.code(),
        GERBIL_AGENT_POLICY_ROUTING_NATIVE_STATUS_OK
    );
    assert_eq!(
        GerbilAgentPolicyRoutingNativeStatus::NULL_POINTER.code(),
        GERBIL_AGENT_POLICY_ROUTING_NATIVE_STATUS_NULL_POINTER
    );
    assert_eq!(
        GerbilAgentPolicyRoutingNativeStatus::ABI_MISMATCH.code(),
        GERBIL_AGENT_POLICY_ROUTING_NATIVE_STATUS_ABI_MISMATCH
    );
    assert_eq!(
        GerbilAgentPolicyRoutingNativeStatus::INVALID_PROJECTION.code(),
        GERBIL_AGENT_POLICY_ROUTING_NATIVE_STATUS_INVALID_PROJECTION
    );
}

#[test]
fn agent_policy_routing_native_layout_offsets_are_stable() {
    assert_eq!(offset_of!(GerbilAgentPolicyRoutingNativeUtf8, ptr), 0);
    assert_eq!(
        offset_of!(GerbilAgentPolicyRoutingNativeUtf8, len),
        USIZE_BYTES
    );

    assert_eq!(offset_of!(GerbilAgentPolicyRoutingNativeUtf8List, items), 0);
    assert_eq!(
        offset_of!(GerbilAgentPolicyRoutingNativeUtf8List, len),
        USIZE_BYTES
    );

    assert_eq!(
        offset_of!(GerbilAgentPolicyRoutingNativeEvidence, evidence_kind),
        0
    );
    assert_eq!(
        offset_of!(GerbilAgentPolicyRoutingNativeEvidence, evidence_id),
        USIZE_BYTES * 2
    );

    assert_eq!(
        offset_of!(GerbilAgentPolicyRoutingNativeEvidenceList, items),
        0
    );
    assert_eq!(
        offset_of!(GerbilAgentPolicyRoutingNativeEvidenceList, len),
        USIZE_BYTES
    );

    assert_eq!(
        offset_of!(GerbilAgentPolicyRoutingNativeRequest, abi_version),
        0
    );
    assert_eq!(
        offset_of!(GerbilAgentPolicyRoutingNativeRequest, graph_id),
        USIZE_BYTES
    );
    assert_eq!(
        offset_of!(GerbilAgentPolicyRoutingNativeRequest, policy_scope),
        USIZE_BYTES * 3
    );
    assert_eq!(
        offset_of!(GerbilAgentPolicyRoutingNativeRequest, root_node),
        USIZE_BYTES * 5
    );
    assert_eq!(
        offset_of!(GerbilAgentPolicyRoutingNativeRequest, candidate_edges),
        USIZE_BYTES * 7
    );
    assert_eq!(
        offset_of!(GerbilAgentPolicyRoutingNativeRequest, routing_evidence),
        USIZE_BYTES * 9
    );

    assert_eq!(
        offset_of!(GerbilAgentPolicyRoutingNativeProjection, abi_version),
        0
    );
    assert_eq!(
        offset_of!(GerbilAgentPolicyRoutingNativeProjection, routing_decision),
        USIZE_BYTES
    );
}
