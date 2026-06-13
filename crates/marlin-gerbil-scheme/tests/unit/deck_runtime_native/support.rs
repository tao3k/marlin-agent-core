use marlin_gerbil_scheme::{
    GERBIL_DECK_RUNTIME_NATIVE_ABI_VERSION, GerbilDeckRuntimeModelRoutePolicy,
    GerbilDeckRuntimeModelRoutePolicyRequest, GerbilDeckRuntimeNativeModelRouteRequest,
    GerbilDeckRuntimeNativeModelRouteSelection, GerbilDeckRuntimeNativeStatus,
    GerbilDeckRuntimeNativeUtf8,
};
use std::{slice, str};

pub(super) fn route_request(
    command: &str,
    agent_scope: &str,
) -> GerbilDeckRuntimeModelRoutePolicyRequest {
    GerbilDeckRuntimeModelRoutePolicyRequest::new(command, agent_scope).with_policy(
        GerbilDeckRuntimeModelRoutePolicy::new("cheap-test-runner", "openai", "gpt-5-mini")
            .with_command_prefix("cargo test")
            .with_agent_scope("sub-agent"),
    )
}

pub(super) unsafe extern "C" fn fake_select_model_route(
    request: *const GerbilDeckRuntimeNativeModelRouteRequest,
    selection: *mut GerbilDeckRuntimeNativeModelRouteSelection,
) -> GerbilDeckRuntimeNativeStatus {
    if request.is_null() || selection.is_null() {
        return GerbilDeckRuntimeNativeStatus::NULL_POINTER;
    }

    let request = unsafe { &*request };
    assert_eq!(request.abi_version, GERBIL_DECK_RUNTIME_NATIVE_ABI_VERSION);
    assert_eq!(native_utf8_to_string(request.command), "cargo test");
    assert_eq!(native_utf8_to_string(request.agent_scope), "sub-agent");
    assert_eq!(request.policies_len, 1);

    let policy = unsafe { &*request.policies };
    assert_eq!(native_utf8_to_string(policy.name), "cheap-test-runner");
    assert_eq!(native_utf8_to_string(policy.provider), "openai");
    assert_eq!(native_utf8_to_string(policy.model), "gpt-5-mini");

    unsafe {
        *selection = GerbilDeckRuntimeNativeModelRouteSelection::matched(0);
    }

    GerbilDeckRuntimeNativeStatus::OK
}

pub(super) unsafe extern "C" fn fake_select_model_route_fast(
    request: *const GerbilDeckRuntimeNativeModelRouteRequest,
    selection: *mut GerbilDeckRuntimeNativeModelRouteSelection,
) -> GerbilDeckRuntimeNativeStatus {
    if request.is_null() || selection.is_null() {
        return GerbilDeckRuntimeNativeStatus::NULL_POINTER;
    }

    unsafe {
        *selection = GerbilDeckRuntimeNativeModelRouteSelection::matched(0);
    }

    GerbilDeckRuntimeNativeStatus::OK
}

pub(super) unsafe extern "C" fn fake_select_no_match(
    request: *const GerbilDeckRuntimeNativeModelRouteRequest,
    selection: *mut GerbilDeckRuntimeNativeModelRouteSelection,
) -> GerbilDeckRuntimeNativeStatus {
    if request.is_null() || selection.is_null() {
        return GerbilDeckRuntimeNativeStatus::NULL_POINTER;
    }

    unsafe {
        *selection = GerbilDeckRuntimeNativeModelRouteSelection::empty();
    }

    GerbilDeckRuntimeNativeStatus::OK
}

pub(super) unsafe extern "C" fn fake_select_output_abi_mismatch(
    request: *const GerbilDeckRuntimeNativeModelRouteRequest,
    selection: *mut GerbilDeckRuntimeNativeModelRouteSelection,
) -> GerbilDeckRuntimeNativeStatus {
    if request.is_null() || selection.is_null() {
        return GerbilDeckRuntimeNativeStatus::NULL_POINTER;
    }

    let mut output = GerbilDeckRuntimeNativeModelRouteSelection::matched(0);
    output.abi_version = GERBIL_DECK_RUNTIME_NATIVE_ABI_VERSION + 1;
    unsafe {
        *selection = output;
    }

    GerbilDeckRuntimeNativeStatus::OK
}

pub(super) unsafe extern "C" fn fake_select_invalid_policy_index(
    request: *const GerbilDeckRuntimeNativeModelRouteRequest,
    selection: *mut GerbilDeckRuntimeNativeModelRouteSelection,
) -> GerbilDeckRuntimeNativeStatus {
    if request.is_null() || selection.is_null() {
        return GerbilDeckRuntimeNativeStatus::NULL_POINTER;
    }

    unsafe {
        *selection = GerbilDeckRuntimeNativeModelRouteSelection::matched(99);
    }

    GerbilDeckRuntimeNativeStatus::OK
}

pub(super) unsafe extern "C" fn fake_select_unmatched_with_policy_index(
    request: *const GerbilDeckRuntimeNativeModelRouteRequest,
    selection: *mut GerbilDeckRuntimeNativeModelRouteSelection,
) -> GerbilDeckRuntimeNativeStatus {
    if request.is_null() || selection.is_null() {
        return GerbilDeckRuntimeNativeStatus::NULL_POINTER;
    }

    let mut output = GerbilDeckRuntimeNativeModelRouteSelection::empty();
    output.policy_index = 0;
    unsafe {
        *selection = output;
    }

    GerbilDeckRuntimeNativeStatus::OK
}

pub(super) unsafe extern "C" fn fake_select_runtime_status(
    request: *const GerbilDeckRuntimeNativeModelRouteRequest,
    selection: *mut GerbilDeckRuntimeNativeModelRouteSelection,
) -> GerbilDeckRuntimeNativeStatus {
    if request.is_null() || selection.is_null() {
        return GerbilDeckRuntimeNativeStatus::NULL_POINTER;
    }

    GerbilDeckRuntimeNativeStatus::ABI_MISMATCH
}

pub(super) unsafe extern "C" fn fake_initialize_failure() -> i32 {
    55
}

fn native_utf8_to_string(value: GerbilDeckRuntimeNativeUtf8) -> String {
    let bytes = if value.ptr.is_null() {
        &[]
    } else {
        unsafe { slice::from_raw_parts(value.ptr, value.len) }
    };
    str::from_utf8(bytes)
        .expect("valid UTF-8 from native request")
        .to_string()
}
