//! Real linked selector constructor for the native Deck runtime.

use marlin_gerbil_scheme::{
    GerbilDeckRuntimeNativeModelRouteRequest, GerbilDeckRuntimeNativeModelRouteSelection,
    GerbilDeckRuntimeNativeModelRouteSelector, GerbilDeckRuntimeNativeStatus,
};

unsafe extern "C" {
    fn marlin_deck_runtime_initialize() -> i32;

    fn marlin_deck_runtime_select_model_route(
        request: *const GerbilDeckRuntimeNativeModelRouteRequest,
        selection: *mut GerbilDeckRuntimeNativeModelRouteSelection,
    ) -> GerbilDeckRuntimeNativeStatus;
}

/// Builds a selector from the symbols linked by the `linked-native` build path.
pub fn linked_deck_runtime_native_selector() -> GerbilDeckRuntimeNativeModelRouteSelector {
    GerbilDeckRuntimeNativeModelRouteSelector::with_initializer(
        marlin_deck_runtime_initialize,
        marlin_deck_runtime_select_model_route,
    )
}
