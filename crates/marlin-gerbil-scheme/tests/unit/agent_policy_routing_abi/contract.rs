use marlin_gerbil_scheme::{
    GERBIL_AGENT_POLICY_ROUTING_ABI_HEADER_SOURCE,
    GERBIL_AGENT_POLICY_ROUTING_ABI_STATUS_ABI_MISMATCH,
    GERBIL_AGENT_POLICY_ROUTING_ABI_STATUS_INVALID_PROJECTION,
    GERBIL_AGENT_POLICY_ROUTING_ABI_STATUS_NULL_POINTER, GERBIL_AGENT_POLICY_ROUTING_ABI_STATUS_OK,
    GERBIL_AGENT_POLICY_ROUTING_ABI_VERSION, GerbilAgentPolicyRoutingAbiEvidence,
    GerbilAgentPolicyRoutingAbiEvidenceList, GerbilAgentPolicyRoutingAbiProjection,
    GerbilAgentPolicyRoutingAbiRequest, GerbilAgentPolicyRoutingAbiStatus,
    GerbilAgentPolicyRoutingAbiUtf8, GerbilAgentPolicyRoutingAbiUtf8List,
};
use memoffset::offset_of;
use static_assertions::{assert_eq_align, const_assert_eq};
use std::mem::size_of;

const USIZE_BYTES: usize = size_of::<usize>();

const_assert_eq!(size_of::<*const u8>(), USIZE_BYTES);
const_assert_eq!(
    size_of::<GerbilAgentPolicyRoutingAbiUtf8>(),
    USIZE_BYTES * 2
);
const_assert_eq!(
    size_of::<GerbilAgentPolicyRoutingAbiUtf8List>(),
    USIZE_BYTES * 2
);
const_assert_eq!(
    size_of::<GerbilAgentPolicyRoutingAbiEvidence>(),
    USIZE_BYTES * 4
);
const_assert_eq!(
    size_of::<GerbilAgentPolicyRoutingAbiEvidenceList>(),
    USIZE_BYTES * 2
);
const_assert_eq!(
    size_of::<GerbilAgentPolicyRoutingAbiRequest>(),
    USIZE_BYTES * 11
);
const_assert_eq!(
    size_of::<GerbilAgentPolicyRoutingAbiProjection>(),
    USIZE_BYTES * 3
);

assert_eq_align!(GerbilAgentPolicyRoutingAbiUtf8, usize);
assert_eq_align!(GerbilAgentPolicyRoutingAbiUtf8List, usize);
assert_eq_align!(GerbilAgentPolicyRoutingAbiEvidence, usize);
assert_eq_align!(GerbilAgentPolicyRoutingAbiEvidenceList, usize);
assert_eq_align!(GerbilAgentPolicyRoutingAbiRequest, usize);
assert_eq_align!(GerbilAgentPolicyRoutingAbiProjection, usize);

#[test]
fn agent_policy_routing_abi_header_exports_c_abi_contract() {
    assert!(
        GERBIL_AGENT_POLICY_ROUTING_ABI_HEADER_SOURCE
            .contains("#define MARLIN_AGENT_POLICY_ROUTING_ABI_VERSION 1u")
    );
    assert!(
        GERBIL_AGENT_POLICY_ROUTING_ABI_HEADER_SOURCE
            .contains("MarlinAgentPolicyRoutingSelectEdgesFn")
    );
    assert!(
        GERBIL_AGENT_POLICY_ROUTING_ABI_HEADER_SOURCE.contains("MarlinAgentPolicyRoutingRequest")
    );
    assert!(
        GERBIL_AGENT_POLICY_ROUTING_ABI_HEADER_SOURCE
            .contains("MarlinAgentPolicyRoutingProjection")
    );
    assert!(
        GERBIL_AGENT_POLICY_ROUTING_ABI_HEADER_SOURCE
            .contains("marlin_agent_policy_routing_select_edges")
    );
    assert!(
        GERBIL_AGENT_POLICY_ROUTING_ABI_HEADER_SOURCE
            .contains("MARLIN_AGENT_POLICY_ROUTING_ABI_STATUS_INVALID_PROJECTION")
    );
    assert!(!GERBIL_AGENT_POLICY_ROUTING_ABI_HEADER_SOURCE.contains("JSON"));
    assert!(!GERBIL_AGENT_POLICY_ROUTING_ABI_HEADER_SOURCE.contains("OwnedBytes"));
}

#[test]
fn agent_policy_routing_abi_status_constants_are_stable() {
    assert_eq!(GERBIL_AGENT_POLICY_ROUTING_ABI_VERSION, 1);
    assert_eq!(
        GerbilAgentPolicyRoutingAbiStatus::OK.code(),
        GERBIL_AGENT_POLICY_ROUTING_ABI_STATUS_OK
    );
    assert_eq!(
        GerbilAgentPolicyRoutingAbiStatus::NULL_POINTER.code(),
        GERBIL_AGENT_POLICY_ROUTING_ABI_STATUS_NULL_POINTER
    );
    assert_eq!(
        GerbilAgentPolicyRoutingAbiStatus::ABI_MISMATCH.code(),
        GERBIL_AGENT_POLICY_ROUTING_ABI_STATUS_ABI_MISMATCH
    );
    assert_eq!(
        GerbilAgentPolicyRoutingAbiStatus::INVALID_PROJECTION.code(),
        GERBIL_AGENT_POLICY_ROUTING_ABI_STATUS_INVALID_PROJECTION
    );
}

#[test]
fn agent_policy_routing_abi_layout_offsets_are_stable() {
    assert_eq!(offset_of!(GerbilAgentPolicyRoutingAbiUtf8, ptr), 0);
    assert_eq!(
        offset_of!(GerbilAgentPolicyRoutingAbiUtf8, len),
        USIZE_BYTES
    );

    assert_eq!(offset_of!(GerbilAgentPolicyRoutingAbiUtf8List, items), 0);
    assert_eq!(
        offset_of!(GerbilAgentPolicyRoutingAbiUtf8List, len),
        USIZE_BYTES
    );

    assert_eq!(
        offset_of!(GerbilAgentPolicyRoutingAbiEvidence, evidence_kind),
        0
    );
    assert_eq!(
        offset_of!(GerbilAgentPolicyRoutingAbiEvidence, evidence_id),
        USIZE_BYTES * 2
    );

    assert_eq!(
        offset_of!(GerbilAgentPolicyRoutingAbiEvidenceList, items),
        0
    );
    assert_eq!(
        offset_of!(GerbilAgentPolicyRoutingAbiEvidenceList, len),
        USIZE_BYTES
    );

    assert_eq!(
        offset_of!(GerbilAgentPolicyRoutingAbiRequest, abi_version),
        0
    );
    assert_eq!(
        offset_of!(GerbilAgentPolicyRoutingAbiRequest, graph_id),
        USIZE_BYTES
    );
    assert_eq!(
        offset_of!(GerbilAgentPolicyRoutingAbiRequest, policy_scope),
        USIZE_BYTES * 3
    );
    assert_eq!(
        offset_of!(GerbilAgentPolicyRoutingAbiRequest, root_node),
        USIZE_BYTES * 5
    );
    assert_eq!(
        offset_of!(GerbilAgentPolicyRoutingAbiRequest, candidate_edges),
        USIZE_BYTES * 7
    );
    assert_eq!(
        offset_of!(GerbilAgentPolicyRoutingAbiRequest, routing_evidence),
        USIZE_BYTES * 9
    );

    assert_eq!(
        offset_of!(GerbilAgentPolicyRoutingAbiProjection, abi_version),
        0
    );
    assert_eq!(
        offset_of!(GerbilAgentPolicyRoutingAbiProjection, routing_decision),
        USIZE_BYTES
    );
}
