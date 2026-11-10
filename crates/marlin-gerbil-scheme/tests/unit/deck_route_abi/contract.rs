use marlin_gerbil_scheme::{
    GERBIL_DECK_ROUTE_ABI_HEADER_SOURCE, GERBIL_DECK_ROUTE_ABI_NO_POLICY_INDEX,
    GERBIL_DECK_ROUTE_ABI_STATUS_ABI_MISMATCH, GERBIL_DECK_ROUTE_ABI_STATUS_INVALID_SELECTION,
    GERBIL_DECK_ROUTE_ABI_STATUS_NULL_POINTER, GERBIL_DECK_ROUTE_ABI_STATUS_OK,
    GERBIL_DECK_ROUTE_ABI_VERSION, GerbilDeckRouteAbiModelRoutePolicy,
    GerbilDeckRouteAbiModelRouteRequest, GerbilDeckRouteAbiModelRouteSelection,
    GerbilDeckRouteAbiStatus, GerbilDeckRouteAbiUtf8, GerbilDeckRouteAbiUtf8List,
};
use memoffset::offset_of;
use static_assertions::{assert_eq_align, const_assert_eq};
use std::mem::size_of;

const USIZE_BYTES: usize = size_of::<usize>();

const_assert_eq!(size_of::<*const u8>(), USIZE_BYTES);
const_assert_eq!(size_of::<GerbilDeckRouteAbiUtf8>(), USIZE_BYTES * 2);
const_assert_eq!(size_of::<GerbilDeckRouteAbiUtf8List>(), USIZE_BYTES * 2);
const_assert_eq!(
    size_of::<GerbilDeckRouteAbiModelRoutePolicy>(),
    USIZE_BYTES * 14
);
const_assert_eq!(
    size_of::<GerbilDeckRouteAbiModelRouteRequest>(),
    USIZE_BYTES * 7
);
const_assert_eq!(
    size_of::<GerbilDeckRouteAbiModelRouteSelection>(),
    USIZE_BYTES + 8
);

assert_eq_align!(GerbilDeckRouteAbiUtf8, usize);
assert_eq_align!(GerbilDeckRouteAbiUtf8List, usize);
assert_eq_align!(GerbilDeckRouteAbiModelRoutePolicy, usize);
assert_eq_align!(GerbilDeckRouteAbiModelRouteRequest, usize);
assert_eq_align!(GerbilDeckRouteAbiModelRouteSelection, usize);

#[test]
fn gerbil_deck_route_abi_header_exports_c_abi_contract() {
    assert!(
        GERBIL_DECK_ROUTE_ABI_HEADER_SOURCE.contains("#define MARLIN_DECK_ROUTE_ABI_VERSION 1u")
    );
    assert!(GERBIL_DECK_ROUTE_ABI_HEADER_SOURCE.contains("MarlinDeckRuntimeSelectModelRouteFn"));
    assert!(GERBIL_DECK_ROUTE_ABI_HEADER_SOURCE.contains("MarlinDeckRuntimeModelRouteRequest"));
    assert!(GERBIL_DECK_ROUTE_ABI_HEADER_SOURCE.contains("MarlinDeckRuntimeModelRouteSelection"));
    assert!(GERBIL_DECK_ROUTE_ABI_HEADER_SOURCE.contains("MARLIN_DECK_ROUTE_ABI_STATUS_OK"));
    assert!(
        GERBIL_DECK_ROUTE_ABI_HEADER_SOURCE.contains("MARLIN_DECK_ROUTE_ABI_STATUS_ABI_MISMATCH")
    );
    assert!(GERBIL_DECK_ROUTE_ABI_HEADER_SOURCE.contains("MARLIN_DECK_ROUTE_ABI_NO_POLICY_INDEX"));
    assert!(!GERBIL_DECK_ROUTE_ABI_HEADER_SOURCE.contains("MarlinDeckRuntimeOwnedBytes"));
}

#[test]
fn gerbil_deck_route_abi_status_constants_are_stable() {
    assert_eq!(GERBIL_DECK_ROUTE_ABI_VERSION, 1);
    assert_eq!(
        GerbilDeckRouteAbiStatus::OK.code(),
        GERBIL_DECK_ROUTE_ABI_STATUS_OK
    );
    assert_eq!(
        GerbilDeckRouteAbiStatus::NULL_POINTER.code(),
        GERBIL_DECK_ROUTE_ABI_STATUS_NULL_POINTER
    );
    assert_eq!(
        GerbilDeckRouteAbiStatus::ABI_MISMATCH.code(),
        GERBIL_DECK_ROUTE_ABI_STATUS_ABI_MISMATCH
    );
    assert_eq!(
        GerbilDeckRouteAbiStatus::INVALID_SELECTION.code(),
        GERBIL_DECK_ROUTE_ABI_STATUS_INVALID_SELECTION
    );
    assert_eq!(
        GerbilDeckRouteAbiModelRouteSelection::NO_POLICY_INDEX,
        GERBIL_DECK_ROUTE_ABI_NO_POLICY_INDEX
    );
}

#[test]
fn gerbil_deck_route_abi_layout_offsets_are_stable() {
    assert_eq!(offset_of!(GerbilDeckRouteAbiUtf8, ptr), 0);
    assert_eq!(offset_of!(GerbilDeckRouteAbiUtf8, len), USIZE_BYTES);

    assert_eq!(offset_of!(GerbilDeckRouteAbiUtf8List, items), 0);
    assert_eq!(offset_of!(GerbilDeckRouteAbiUtf8List, len), USIZE_BYTES);

    assert_eq!(offset_of!(GerbilDeckRouteAbiModelRoutePolicy, name), 0);
    assert_eq!(
        offset_of!(GerbilDeckRouteAbiModelRoutePolicy, provider),
        USIZE_BYTES * 2
    );
    assert_eq!(
        offset_of!(GerbilDeckRouteAbiModelRoutePolicy, model),
        USIZE_BYTES * 4
    );
    assert_eq!(
        offset_of!(GerbilDeckRouteAbiModelRoutePolicy, command_prefixes),
        USIZE_BYTES * 6
    );
    assert_eq!(
        offset_of!(GerbilDeckRouteAbiModelRoutePolicy, agent_scopes),
        USIZE_BYTES * 8
    );
    assert_eq!(
        offset_of!(GerbilDeckRouteAbiModelRoutePolicy, context_mode),
        USIZE_BYTES * 10
    );
    assert_eq!(
        offset_of!(GerbilDeckRouteAbiModelRoutePolicy, isolation_mode),
        USIZE_BYTES * 12
    );

    assert_eq!(
        offset_of!(GerbilDeckRouteAbiModelRouteRequest, abi_version),
        0
    );
    assert_eq!(
        offset_of!(GerbilDeckRouteAbiModelRouteRequest, command),
        USIZE_BYTES
    );
    assert_eq!(
        offset_of!(GerbilDeckRouteAbiModelRouteRequest, agent_scope),
        USIZE_BYTES * 3
    );
    assert_eq!(
        offset_of!(GerbilDeckRouteAbiModelRouteRequest, policies),
        USIZE_BYTES * 5
    );
    assert_eq!(
        offset_of!(GerbilDeckRouteAbiModelRouteRequest, policies_len),
        USIZE_BYTES * 6
    );

    assert_eq!(
        offset_of!(GerbilDeckRouteAbiModelRouteSelection, abi_version),
        0
    );
    assert_eq!(
        offset_of!(GerbilDeckRouteAbiModelRouteSelection, matched),
        4
    );
    assert_eq!(
        offset_of!(GerbilDeckRouteAbiModelRouteSelection, reserved),
        5
    );
    assert_eq!(
        offset_of!(GerbilDeckRouteAbiModelRouteSelection, policy_index),
        8
    );
}
