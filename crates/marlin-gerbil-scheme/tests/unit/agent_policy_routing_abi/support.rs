use marlin_gerbil_scheme::{
    GERBIL_AGENT_POLICY_ROUTING_ABI_VERSION, GerbilAgentPolicyRoutingAbiEvidence,
    GerbilAgentPolicyRoutingAbiEvidenceList, GerbilAgentPolicyRoutingAbiProjection,
    GerbilAgentPolicyRoutingAbiRequest, GerbilAgentPolicyRoutingAbiSelectEdgesRequest,
    GerbilAgentPolicyRoutingAbiStatus, GerbilAgentPolicyRoutingAbiUtf8,
    GerbilAgentPolicyRoutingAbiUtf8List, GerbilAgentPolicyRoutingEvidenceKind,
};
use std::{slice, str};

const INVALID_UTF8: &[u8] = &[0xff];

pub(super) fn routing_request() -> GerbilAgentPolicyRoutingAbiSelectEdgesRequest {
    GerbilAgentPolicyRoutingAbiSelectEdgesRequest::new(
        "agent-graph.policy",
        "gerbil.scope.agent-topology",
        "planner",
    )
    .with_candidate_edge("planner-to-custom")
    .with_evidence(
        GerbilAgentPolicyRoutingEvidenceKind::GerbilPolicyReceipt,
        "gerbil.policy.receipt.1",
    )
}

pub(super) unsafe extern "C" fn fake_select_edges(
    request: *const GerbilAgentPolicyRoutingAbiRequest,
    projection: *mut GerbilAgentPolicyRoutingAbiProjection,
) -> GerbilAgentPolicyRoutingAbiStatus {
    if request.is_null() || projection.is_null() {
        return GerbilAgentPolicyRoutingAbiStatus::NULL_POINTER;
    }

    let request = unsafe { &*request };
    assert_eq!(request.abi_version, GERBIL_AGENT_POLICY_ROUTING_ABI_VERSION);
    assert_eq!(
        native_utf8_to_string(request.graph_id),
        "agent-graph.policy"
    );
    assert_eq!(
        native_utf8_to_string(request.policy_scope),
        "gerbil.scope.agent-topology"
    );
    assert_eq!(native_utf8_to_string(request.root_node), "planner");
    assert_eq!(
        native_utf8_list_to_strings(request.candidate_edges),
        vec!["planner-to-custom"]
    );

    let evidence = native_evidence_list_to_strings(request.routing_evidence);
    assert_eq!(
        evidence,
        vec![(
            "gerbil_policy_receipt".to_string(),
            "gerbil.policy.receipt.1".to_string()
        )]
    );

    unsafe {
        *projection = GerbilAgentPolicyRoutingAbiProjection::with_routing_decision(
            GerbilAgentPolicyRoutingAbiUtf8::from_static("select_edges"),
        );
    }

    GerbilAgentPolicyRoutingAbiStatus::OK
}

pub(super) unsafe extern "C" fn fake_select_output_abi_mismatch(
    request: *const GerbilAgentPolicyRoutingAbiRequest,
    projection: *mut GerbilAgentPolicyRoutingAbiProjection,
) -> GerbilAgentPolicyRoutingAbiStatus {
    if request.is_null() || projection.is_null() {
        return GerbilAgentPolicyRoutingAbiStatus::NULL_POINTER;
    }

    let mut output = GerbilAgentPolicyRoutingAbiProjection::with_routing_decision(
        GerbilAgentPolicyRoutingAbiUtf8::from_static("select_edges"),
    );
    output.abi_version = GERBIL_AGENT_POLICY_ROUTING_ABI_VERSION + 1;
    unsafe {
        *projection = output;
    }

    GerbilAgentPolicyRoutingAbiStatus::OK
}

pub(super) unsafe extern "C" fn fake_select_invalid_utf8(
    request: *const GerbilAgentPolicyRoutingAbiRequest,
    projection: *mut GerbilAgentPolicyRoutingAbiProjection,
) -> GerbilAgentPolicyRoutingAbiStatus {
    if request.is_null() || projection.is_null() {
        return GerbilAgentPolicyRoutingAbiStatus::NULL_POINTER;
    }

    unsafe {
        *projection = GerbilAgentPolicyRoutingAbiProjection::with_routing_decision(
            GerbilAgentPolicyRoutingAbiUtf8 {
                ptr: INVALID_UTF8.as_ptr(),
                len: INVALID_UTF8.len(),
            },
        );
    }

    GerbilAgentPolicyRoutingAbiStatus::OK
}

pub(super) unsafe extern "C" fn fake_select_runtime_status(
    request: *const GerbilAgentPolicyRoutingAbiRequest,
    projection: *mut GerbilAgentPolicyRoutingAbiProjection,
) -> GerbilAgentPolicyRoutingAbiStatus {
    if request.is_null() || projection.is_null() {
        return GerbilAgentPolicyRoutingAbiStatus::NULL_POINTER;
    }

    GerbilAgentPolicyRoutingAbiStatus::ABI_MISMATCH
}

pub(super) unsafe extern "C" fn fake_initialize_failure() -> i32 {
    55
}

fn native_utf8_to_string(value: GerbilAgentPolicyRoutingAbiUtf8) -> String {
    let bytes = if value.ptr.is_null() {
        &[]
    } else {
        unsafe { slice::from_raw_parts(value.ptr, value.len) }
    };
    str::from_utf8(bytes)
        .expect("valid UTF-8 from native request")
        .to_string()
}

fn native_utf8_list_to_strings(value: GerbilAgentPolicyRoutingAbiUtf8List) -> Vec<String> {
    let values = if value.items.is_null() {
        &[]
    } else {
        unsafe { slice::from_raw_parts(value.items, value.len) }
    };
    values.iter().copied().map(native_utf8_to_string).collect()
}

fn native_evidence_list_to_strings(
    value: GerbilAgentPolicyRoutingAbiEvidenceList,
) -> Vec<(String, String)> {
    let values = if value.items.is_null() {
        &[]
    } else {
        unsafe { slice::from_raw_parts(value.items, value.len) }
    };
    values
        .iter()
        .copied()
        .map(native_evidence_to_strings)
        .collect()
}

fn native_evidence_to_strings(evidence: GerbilAgentPolicyRoutingAbiEvidence) -> (String, String) {
    (
        native_utf8_to_string(evidence.evidence_kind),
        native_utf8_to_string(evidence.evidence_id),
    )
}
