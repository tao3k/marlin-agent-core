//! Rust host types and linked selector for the Scheme Deck Runtime.

mod error;
#[cfg(feature = "linked-selector")]
mod linked;
mod metadata;
mod policy;
mod resolver;
#[cfg(feature = "actor-runtime")]
mod route_admission;

pub use error::DeckLinkedRouteError;
#[cfg(feature = "linked-selector")]
pub use linked::linked_deck_route_selector;
pub use metadata::{DeckRuntimeMetadata, DeckRuntimeModuleId};
pub use resolver::DeckLinkedRouteResolver;
#[cfg(feature = "actor-runtime")]
pub use route_admission::{
    DeckRouteAdmission, DeckRouteAdmissionConfig, DeckRouteAdmissionRejection,
    DeckRouteAdmissionStartupError,
};
