//! Real linked selector constructor for the native Deck runtime.

use marlin_gerbil_scheme::{
    GerbilDeckRouteAbiModelRouteRequest, GerbilDeckRouteAbiModelRouteSelection,
    GerbilDeckRouteAbiModelRouteSelector, GerbilDeckRouteAbiStatus,
};

unsafe extern "C" {
    fn marlin_deck_runtime_initialize() -> i32;

    fn marlin_deck_runtime_select_model_route(
        request: *const GerbilDeckRouteAbiModelRouteRequest,
        selection: *mut GerbilDeckRouteAbiModelRouteSelection,
    ) -> GerbilDeckRouteAbiStatus;
}

/// Builds a selector from the symbols linked by the `linked-selector` build path.
pub fn linked_deck_route_selector() -> GerbilDeckRouteAbiModelRouteSelector {
    GerbilDeckRouteAbiModelRouteSelector::with_initializer(
        marlin_deck_runtime_initialize,
        marlin_deck_runtime_select_model_route,
    )
}
