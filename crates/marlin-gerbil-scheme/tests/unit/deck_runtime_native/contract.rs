use marlin_gerbil_scheme::{
    GERBIL_DECK_RUNTIME_NATIVE_ABI_VERSION, GERBIL_DECK_RUNTIME_NATIVE_HEADER_SOURCE,
    GERBIL_DECK_RUNTIME_NATIVE_NO_POLICY_INDEX, GERBIL_DECK_RUNTIME_NATIVE_STATUS_ABI_MISMATCH,
    GERBIL_DECK_RUNTIME_NATIVE_STATUS_INVALID_SELECTION,
    GERBIL_DECK_RUNTIME_NATIVE_STATUS_NULL_POINTER, GERBIL_DECK_RUNTIME_NATIVE_STATUS_OK,
    GerbilDeckRuntimeNativeModelRouteSelection, GerbilDeckRuntimeNativeStatus,
};

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
