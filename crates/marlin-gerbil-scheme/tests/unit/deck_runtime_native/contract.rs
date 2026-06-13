use marlin_gerbil_scheme::{
    GERBIL_DECK_RUNTIME_NATIVE_ABI_VERSION, GERBIL_DECK_RUNTIME_NATIVE_HEADER_SOURCE,
    GERBIL_DECK_RUNTIME_NATIVE_NO_POLICY_INDEX, GERBIL_DECK_RUNTIME_NATIVE_STATUS_ABI_MISMATCH,
    GERBIL_DECK_RUNTIME_NATIVE_STATUS_INVALID_SELECTION,
    GERBIL_DECK_RUNTIME_NATIVE_STATUS_NULL_POINTER, GERBIL_DECK_RUNTIME_NATIVE_STATUS_OK,
    GerbilDeckRuntimeNativeModelRoutePolicy, GerbilDeckRuntimeNativeModelRouteRequest,
    GerbilDeckRuntimeNativeModelRouteSelection, GerbilDeckRuntimeNativeStatus,
    GerbilDeckRuntimeNativeUtf8, GerbilDeckRuntimeNativeUtf8List,
};
use memoffset::offset_of;
use static_assertions::{assert_eq_align, const_assert_eq};
use std::mem::size_of;

const USIZE_BYTES: usize = size_of::<usize>();

const_assert_eq!(size_of::<*const u8>(), USIZE_BYTES);
const_assert_eq!(size_of::<GerbilDeckRuntimeNativeUtf8>(), USIZE_BYTES * 2);
const_assert_eq!(
    size_of::<GerbilDeckRuntimeNativeUtf8List>(),
    USIZE_BYTES * 2
);
const_assert_eq!(
    size_of::<GerbilDeckRuntimeNativeModelRoutePolicy>(),
    USIZE_BYTES * 14
);
const_assert_eq!(
    size_of::<GerbilDeckRuntimeNativeModelRouteRequest>(),
    USIZE_BYTES * 7
);
const_assert_eq!(
    size_of::<GerbilDeckRuntimeNativeModelRouteSelection>(),
    USIZE_BYTES + 8
);

assert_eq_align!(GerbilDeckRuntimeNativeUtf8, usize);
assert_eq_align!(GerbilDeckRuntimeNativeUtf8List, usize);
assert_eq_align!(GerbilDeckRuntimeNativeModelRoutePolicy, usize);
assert_eq_align!(GerbilDeckRuntimeNativeModelRouteRequest, usize);
assert_eq_align!(GerbilDeckRuntimeNativeModelRouteSelection, usize);

#[test]
fn gerbil_deck_runtime_native_header_exports_c_abi_contract() {
    assert!(
        GERBIL_DECK_RUNTIME_NATIVE_HEADER_SOURCE
            .contains("#define MARLIN_DECK_RUNTIME_NATIVE_ABI_VERSION 1u")
    );
    assert!(
        GERBIL_DECK_RUNTIME_NATIVE_HEADER_SOURCE.contains("MarlinDeckRuntimeSelectModelRouteFn")
    );
    assert!(
        GERBIL_DECK_RUNTIME_NATIVE_HEADER_SOURCE.contains("MarlinDeckRuntimeModelRouteRequest")
    );
    assert!(
        GERBIL_DECK_RUNTIME_NATIVE_HEADER_SOURCE.contains("MarlinDeckRuntimeModelRouteSelection")
    );
    assert!(
        GERBIL_DECK_RUNTIME_NATIVE_HEADER_SOURCE.contains("MARLIN_DECK_RUNTIME_NATIVE_STATUS_OK")
    );
    assert!(
        GERBIL_DECK_RUNTIME_NATIVE_HEADER_SOURCE
            .contains("MARLIN_DECK_RUNTIME_NATIVE_STATUS_ABI_MISMATCH")
    );
    assert!(
        GERBIL_DECK_RUNTIME_NATIVE_HEADER_SOURCE
            .contains("MARLIN_DECK_RUNTIME_NATIVE_NO_POLICY_INDEX")
    );
    assert!(!GERBIL_DECK_RUNTIME_NATIVE_HEADER_SOURCE.contains("MarlinDeckRuntimeOwnedBytes"));
}

#[test]
fn gerbil_deck_runtime_native_status_constants_are_stable() {
    assert_eq!(GERBIL_DECK_RUNTIME_NATIVE_ABI_VERSION, 1);
    assert_eq!(
        GerbilDeckRuntimeNativeStatus::OK.code(),
        GERBIL_DECK_RUNTIME_NATIVE_STATUS_OK
    );
    assert_eq!(
        GerbilDeckRuntimeNativeStatus::NULL_POINTER.code(),
        GERBIL_DECK_RUNTIME_NATIVE_STATUS_NULL_POINTER
    );
    assert_eq!(
        GerbilDeckRuntimeNativeStatus::ABI_MISMATCH.code(),
        GERBIL_DECK_RUNTIME_NATIVE_STATUS_ABI_MISMATCH
    );
    assert_eq!(
        GerbilDeckRuntimeNativeStatus::INVALID_SELECTION.code(),
        GERBIL_DECK_RUNTIME_NATIVE_STATUS_INVALID_SELECTION
    );
    assert_eq!(
        GerbilDeckRuntimeNativeModelRouteSelection::NO_POLICY_INDEX,
        GERBIL_DECK_RUNTIME_NATIVE_NO_POLICY_INDEX
    );
}

#[test]
fn gerbil_deck_runtime_native_layout_offsets_are_stable() {
    assert_eq!(offset_of!(GerbilDeckRuntimeNativeUtf8, ptr), 0);
    assert_eq!(offset_of!(GerbilDeckRuntimeNativeUtf8, len), USIZE_BYTES);

    assert_eq!(offset_of!(GerbilDeckRuntimeNativeUtf8List, items), 0);
    assert_eq!(
        offset_of!(GerbilDeckRuntimeNativeUtf8List, len),
        USIZE_BYTES
    );

    assert_eq!(offset_of!(GerbilDeckRuntimeNativeModelRoutePolicy, name), 0);
    assert_eq!(
        offset_of!(GerbilDeckRuntimeNativeModelRoutePolicy, provider),
        USIZE_BYTES * 2
    );
    assert_eq!(
        offset_of!(GerbilDeckRuntimeNativeModelRoutePolicy, model),
        USIZE_BYTES * 4
    );
    assert_eq!(
        offset_of!(GerbilDeckRuntimeNativeModelRoutePolicy, command_prefixes),
        USIZE_BYTES * 6
    );
    assert_eq!(
        offset_of!(GerbilDeckRuntimeNativeModelRoutePolicy, agent_scopes),
        USIZE_BYTES * 8
    );
    assert_eq!(
        offset_of!(GerbilDeckRuntimeNativeModelRoutePolicy, context_mode),
        USIZE_BYTES * 10
    );
    assert_eq!(
        offset_of!(GerbilDeckRuntimeNativeModelRoutePolicy, isolation_mode),
        USIZE_BYTES * 12
    );

    assert_eq!(
        offset_of!(GerbilDeckRuntimeNativeModelRouteRequest, abi_version),
        0
    );
    assert_eq!(
        offset_of!(GerbilDeckRuntimeNativeModelRouteRequest, command),
        USIZE_BYTES
    );
    assert_eq!(
        offset_of!(GerbilDeckRuntimeNativeModelRouteRequest, agent_scope),
        USIZE_BYTES * 3
    );
    assert_eq!(
        offset_of!(GerbilDeckRuntimeNativeModelRouteRequest, policies),
        USIZE_BYTES * 5
    );
    assert_eq!(
        offset_of!(GerbilDeckRuntimeNativeModelRouteRequest, policies_len),
        USIZE_BYTES * 6
    );

    assert_eq!(
        offset_of!(GerbilDeckRuntimeNativeModelRouteSelection, abi_version),
        0
    );
    assert_eq!(
        offset_of!(GerbilDeckRuntimeNativeModelRouteSelection, matched),
        4
    );
    assert_eq!(
        offset_of!(GerbilDeckRuntimeNativeModelRouteSelection, reserved),
        5
    );
    assert_eq!(
        offset_of!(GerbilDeckRuntimeNativeModelRouteSelection, policy_index),
        8
    );
}
