// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Marlin's typed Context admission requests and Lab scope identities.

#[cfg(feature = "artifacts-abi")]
pub mod artifacts;
#[cfg(feature = "gerbil-scheme-abi")]
pub mod gerbil;
#[cfg(feature = "gerbil")]
pub mod gerbil_org;
/// Structural Org `:ID:` region leases and concurrent multi-agent subtree mediation.
pub mod governance;
/// Bridges `OrgGovernanceLedger` subtree mediation with `marlin-agent-graph`
/// `GovernedContextCapsule`, Scoped Detours, and Two-Zone KV-Prefix prompt assembly.
#[cfg(feature = "org-memory")]
pub mod governed_bridge;
/// Org-backed Context source and optional memory projections.
#[cfg(feature = "org-source")]
pub mod org;
mod request;
mod scope;

pub use request::{ContextAdmissionRequest, ContextScope, ContextSession, ContextTurn};
pub use scope::{
    CONTEXT_BATCH_MAX_ROWS, CONTEXT_BATCH_MAX_TEXT_BYTES, ContextAdmissionError, ContextBundleId,
    ContextDestinationId, ContextOrganizationId, ContextScopeBatch, ContextScopeDigest,
    ContextScopeEpoch, ContextScopeKey, ContextScopeRow, ContextSessionTurn, ContextSourceCutId,
    ContextTaskId, ContextTurnIndex,
};
