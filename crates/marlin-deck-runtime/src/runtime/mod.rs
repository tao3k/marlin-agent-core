// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Admission handles and the public receipts for the single VM.

mod handle;

pub use handle::{
    DeckContextModule, DeckMailboxMetrics, DeckModuleInstanceId, DeckModuleRetirementReceipt,
    DeckRuntime, DeckRuntimeConfig, DeckRuntimeMetrics, DeckRuntimeRejection,
    DeckRuntimeShutdownError, DeckRuntimeShutdownReceipt, DeckRuntimeStartupError,
    DeckRuntimeStopReason, DeckTimerId, DeckTimerReceipt,
};
