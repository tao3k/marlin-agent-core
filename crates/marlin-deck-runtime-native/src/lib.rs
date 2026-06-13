//! Runtime bridge for the linked native `marlin-deck-runtime` selector.

mod error;
#[cfg(feature = "linked-native")]
mod linked;
mod policy;
mod resolver;

pub use error::DeckRuntimeNativeRouteError;
#[cfg(feature = "linked-native")]
pub use linked::linked_deck_runtime_native_selector;
pub use resolver::DeckRuntimeNativeRouteResolver;
