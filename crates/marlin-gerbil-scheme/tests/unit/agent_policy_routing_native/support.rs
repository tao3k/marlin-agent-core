use marlin_gerbil_scheme::{
    GERBIL_AGENT_POLICY_ROUTING_NATIVE_ABI_VERSION, GerbilAgentPolicyRoutingEvidenceKind,
    GerbilAgentPolicyRoutingNativeEvidence, GerbilAgentPolicyRoutingNativeEvidenceList,
    GerbilAgentPolicyRoutingNativeProjection, GerbilAgentPolicyRoutingNativeRequest,
    GerbilAgentPolicyRoutingNativeSelectEdgesRequest, GerbilAgentPolicyRoutingNativeStatus,
    GerbilAgentPolicyRoutingNativeUtf8, GerbilAgentPolicyRoutingNativeUtf8List,
};
use std::{slice, str};

const INVALID_UTF8: &[u8] = &[0xff];

pub(super) fn routing_request() -> GerbilAgentPolicyRoutingNativeSelectEdgesRequest {
    GerbilAgentPolicyRoutingNativeSelectEdgesRequest::new(
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
    request: *const GerbilAgentPolicyRoutingNativeRequest,
    projection: *mut GerbilAgentPolicyRoutingNativeProjection,
) -> GerbilAgentPolicyRoutingNativeStatus {
    if request.is_null() || projection.is_null() {
        return GerbilAgentPolicyRoutingNativeStatus::NULL_POINTER;
    }

    let request = unsafe { &*request };
    assert_eq!(
        request.abi_version,
        GERBIL_AGENT_POLICY_ROUTING_NATIVE_ABI_VERSION
    );
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
        *projection = GerbilAgentPolicyRoutingNativeProjection::with_routing_decision(
            GerbilAgentPolicyRoutingNativeUtf8::from_static("select_edges"),
        );
    }

    GerbilAgentPolicyRoutingNativeStatus::OK
}

pub(super) unsafe extern "C" fn fake_select_output_abi_mismatch(
    request: *const GerbilAgentPolicyRoutingNativeRequest,
    projection: *mut GerbilAgentPolicyRoutingNativeProjection,
) -> GerbilAgentPolicyRoutingNativeStatus {
    if request.is_null() || projection.is_null() {
        return GerbilAgentPolicyRoutingNativeStatus::NULL_POINTER;
    }

    let mut output = GerbilAgentPolicyRoutingNativeProjection::with_routing_decision(
        GerbilAgentPolicyRoutingNativeUtf8::from_static("select_edges"),
    );
    output.abi_version = GERBIL_AGENT_POLICY_ROUTING_NATIVE_ABI_VERSION + 1;
    unsafe {
        *projection = output;
    }

    GerbilAgentPolicyRoutingNativeStatus::OK
}

pub(super) unsafe extern "C" fn fake_select_invalid_utf8(
    request: *const GerbilAgentPolicyRoutingNativeRequest,
    projection: *mut GerbilAgentPolicyRoutingNativeProjection,
) -> GerbilAgentPolicyRoutingNativeStatus {
    if request.is_null() || projection.is_null() {
        return GerbilAgentPolicyRoutingNativeStatus::NULL_POINTER;
    }

    unsafe {
        *projection = GerbilAgentPolicyRoutingNativeProjection::with_routing_decision(
            GerbilAgentPolicyRoutingNativeUtf8 {
                ptr: INVALID_UTF8.as_ptr(),
                len: INVALID_UTF8.len(),
            },
        );
    }

    GerbilAgentPolicyRoutingNativeStatus::OK
}

pub(super) unsafe extern "C" fn fake_select_runtime_status(
    request: *const GerbilAgentPolicyRoutingNativeRequest,
    projection: *mut GerbilAgentPolicyRoutingNativeProjection,
) -> GerbilAgentPolicyRoutingNativeStatus {
    if request.is_null() || projection.is_null() {
        return GerbilAgentPolicyRoutingNativeStatus::NULL_POINTER;
    }

    GerbilAgentPolicyRoutingNativeStatus::ABI_MISMATCH
}

pub(super) unsafe extern "C" fn fake_initialize_failure() -> i32 {
    55
}

fn native_utf8_to_string(value: GerbilAgentPolicyRoutingNativeUtf8) -> String {
    let bytes = if value.ptr.is_null() {
        &[]
    } else {
        unsafe { slice::from_raw_parts(value.ptr, value.len) }
    };
    str::from_utf8(bytes)
        .expect("valid UTF-8 from native request")
        .to_string()
}

fn native_utf8_list_to_strings(value: GerbilAgentPolicyRoutingNativeUtf8List) -> Vec<String> {
    let values = if value.items.is_null() {
        &[]
    } else {
        unsafe { slice::from_raw_parts(value.items, value.len) }
    };
    values.iter().copied().map(native_utf8_to_string).collect()
}

fn native_evidence_list_to_strings(
    value: GerbilAgentPolicyRoutingNativeEvidenceList,
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

fn native_evidence_to_strings(
    evidence: GerbilAgentPolicyRoutingNativeEvidence,
) -> (String, String) {
    (
        native_utf8_to_string(evidence.evidence_kind),
        native_utf8_to_string(evidence.evidence_id),
    )
}
