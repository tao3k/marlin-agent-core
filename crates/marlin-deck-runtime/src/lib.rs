// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Single-VM Deck Actor execution and Scheme-owned runtime metadata.

mod metadata;
#[cfg(feature = "actor-runtime")]
mod runtime;

#[cfg(feature = "actor-runtime")]
pub use metadata::DeckRuntimeMetadataError;
pub use metadata::{DeckRuntimeMetadata, DeckRuntimeModuleId};
#[cfg(feature = "actor-runtime")]
pub use runtime::{
    DeckContextModule, DeckMailboxMetrics, DeckModuleInstanceId, DeckModuleRetirementReceipt,
    DeckRuntime, DeckRuntimeConfig, DeckRuntimeMetrics, DeckRuntimeRejection,
    DeckRuntimeShutdownError, DeckRuntimeShutdownReceipt, DeckRuntimeStartupError,
    DeckRuntimeStopReason, DeckTimerId, DeckTimerReceipt,
};
