// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Converged Marlin search engine constructor.

use std::sync::Arc;

use agent_semantic_search::{MarlinConvergedSearchEngine, SearchEngineStoragePort};
use agent_semantic_topology::OrgDocumentProjectionPort;

/// Construct a converged ASP search engine backed by Marlin's Org and Storage ports.
#[must_use]
pub fn marlin_search_engine(
    org_port: Arc<dyn OrgDocumentProjectionPort>,
    storage_port: Arc<dyn SearchEngineStoragePort>,
) -> MarlinConvergedSearchEngine {
    MarlinConvergedSearchEngine::new(org_port, storage_port)
}
