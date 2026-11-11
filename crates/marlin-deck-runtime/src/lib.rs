//! Single-VM Deck Actor execution and Scheme-owned runtime metadata.

mod metadata;
#[cfg(feature = "actor-runtime")]
mod runtime;

pub use metadata::{DeckRuntimeMetadata, DeckRuntimeModuleId};
#[cfg(feature = "actor-runtime")]
pub use runtime::{
    DeckRuntime, DeckRuntimeConfig, DeckRuntimeMetrics, DeckRuntimeRejection,
    DeckRuntimeShutdownError, DeckRuntimeShutdownReceipt, DeckRuntimeStartupError,
    DeckRuntimeStopReason,
};
